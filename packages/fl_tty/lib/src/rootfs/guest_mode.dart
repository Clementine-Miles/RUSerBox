import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

/// `chmod`, which `dart:io` has not got and a file written into a guest cannot
/// do without.
///
/// Falls back to the ordinary permission bits if a special bit is refused.
void chmodGuestFile(String path, int mode) {
  final chmod = _chmod;
  if (chmod == null) return;
  final pointer = path.toNativeUtf8();
  try {
    if (chmod(pointer.cast(), mode) == 0) return;
    final special = mode & _specialBits;
    if (special == 0) return;
    final baseMode = mode & ~_specialBits;
    final rwMode = baseMode | 0x180;
    if (chmod(pointer.cast(), rwMode) != 0) return;
    if (baseMode != rwMode) {
      chmod(pointer.cast(), baseMode);
    }
  } finally {
    malloc.free(pointer);
  }
}

/// Set-id and sticky: `S_ISUID | S_ISGID | S_ISVTX`.
const _specialBits = 0xE00;

/// `link`, for the same reason [chmodGuestFile] exists: `dart:io` offers only
/// symbolic links, and a rootfs is full of hard ones.
///
/// Ubuntu 26.04 ships uutils coreutils as one 10 MB multi-call binary with 115
/// hard links pointing at it, and Perl adds another. Turning those into copies
/// would cost a gigabyte; turning them into symlinks is what the tar reader
/// already did by accident, and it produced links pointing nowhere — see the
/// unpacking code. A real hard link is what the archive asked for and costs
/// nothing.
///
/// Returns whether it worked. Failure is worth handling rather than throwing:
/// the caller has a correct fallback, and a system missing `ls` is worse than
/// one whose `ls` is a symlink.
bool linkGuestFile(String target, String path) {
  final link = _link;
  if (link == null) return false;
  final targetPtr = target.toNativeUtf8();
  final pathPtr = path.toNativeUtf8();
  try {
    return link(targetPtr.cast(), pathPtr.cast()) == 0;
  } catch (_) {
    return false;
  } finally {
    malloc.free(targetPtr);
    malloc.free(pathPtr);
  }
}

final _link = () {
  try {
    return DynamicLibrary.process().lookupFunction<
      Int Function(Pointer<Char>, Pointer<Char>),
      int Function(Pointer<Char>, Pointer<Char>)
    >('link');
  } catch (_) {
    return null;
  }
}();

final _chmod = () {
  try {
    final process = DynamicLibrary.process();
    return process.lookupFunction<
      Int Function(Pointer<Char>, UnsignedInt),
      int Function(Pointer<Char>, int)
    >('chmod');
  } catch (_) {
    return null;
  }
}();

/// Writes [content] to [file] and assigns [mode] if [file] did not exist yet,
/// keeping the default permission bits independent of the host umask.
Future<void> writeGuestTextFile(
  File file,
  String content, {
  int mode = 0x1A4,
}) async {
  final existed = await file.exists();
  await file.writeAsString(content);
  if (!existed) chmodGuestFile(file.path, mode);
}

/// Repairs guest directory modes on an existing rootfs so `/` is `0755` (not
/// `0777` from host `umask(0)`) and `/tmp` + `/var/tmp` are `01777`.
void repairGuestModes(String root) {
  if (FileSystemEntity.typeSync(root, followLinks: false) ==
      FileSystemEntityType.directory) {
    if ((FileStat.statSync(root).mode & 0xfff) == 0x1ff) {
      chmodGuestFile(root, 0x1ED);
    }
  }
  for (final rel in const ['tmp', 'var/tmp']) {
    final dir =
        '$root${Platform.pathSeparator}${rel.replaceAll('/', Platform.pathSeparator)}';
    if (FileSystemEntity.typeSync(dir, followLinks: false) ==
        FileSystemEntityType.directory) {
      chmodGuestFile(dir, 0x3FF);
    }
  }
}
