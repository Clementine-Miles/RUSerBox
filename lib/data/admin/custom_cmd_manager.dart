import 'package:server_box/data/model/app/scripts/shell_func.dart';
import 'package:server_box/data/model/server/server_exec.dart';
import 'package:server_box/data/model/server/system.dart';
import 'package:server_box/src/rust/api/script.dart' as ffi;

/// A server's custom status commands: files in a fixed directory under the
/// account's home, which the status script runs. Commands and the listing's
/// format are `sbm_parser`'s; this file only runs them.
abstract final class CustomCmdManager {
  /// In the order the status page lists them. A directory that does not exist
  /// yet reads as none: the first [save] creates it.
  static Future<List<ffi.CustomCmd>> list(
    CommandExec exec,
    SystemType system,
  ) async {
    final res = await exec.run(
      ffi.readCustomCmdsCommand(system: ShellFuncManager.ffiSystem(system)),
      entry: _entry(system),
    );
    if (!res.succeeded) throw CommandFailed('read custom commands', res);
    final cmds = ffi.parseCustomCmdsListing(raw: res.stdout);
    if (cmds == null) throw CommandFailed('read custom commands', res);
    return cmds;
  }

  /// Replaces the set in one round trip. Each file's name carries its position
  /// in [cmds], spaced so a later move renames one file.
  static Future<void> save(
    CommandExec exec,
    SystemType system,
    List<ffi.CustomCmd> cmds,
  ) async {
    final res = await exec.run(
      ffi.installCustomCmdsCommand(
        system: ShellFuncManager.ffiSystem(system),
        cmds: cmds,
      ),
      entry: _entry(system),
    );
    if (!res.succeeded) throw CommandFailed('save custom commands', res);
  }

  /// The script is fed to `sh` on stdin, so a user's command never has to
  /// survive shell quoting. (A Windows host gets an empty script.)
  static String? _entry(SystemType system) => 'sh';
}
