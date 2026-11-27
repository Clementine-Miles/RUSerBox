import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:fl_lib/src/core/ext/ctx/l10n.dart';
import 'package:fl_lib/src/core/ext/datetime.dart';
import 'package:fl_lib/src/core/ext/string.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:share_plus/share_plus.dart';

/// Platforms
enum Pfs {
  android,
  ios,
  linux,
  macos,
  windows,
  web,
  fuchsia,
  unknown;

  /// The current platform
  static final type = () {
    if (kIsWeb) {
      return web;
    }
    return switch (Platform.operatingSystem) {
      'android' => android,
      'ios' => ios,
      'linux' => linux,
      'macos' => macos,
      'windows' => windows,
      'fuchsia' => fuchsia,
      // If the platform is not recognized, return unknown
      _ => unknown,
    };
  }();

  @override
  String toString() => switch (this) {
    macos => 'macOS',
    ios => 'iOS',
    final val => val.name.capitalize,
  };

  static final String seperator = isWindows ? '\\' : '/';

  /// The home directory on the Linux test host; null on Android, whose app
  /// sandbox has no user home.
  static final String? homeDir = isLinux ? Platform.environment['HOME'] : null;

  /// Share files through the system share sheet.
  static Future<ShareResult?> sharePaths({required List<String> paths, String? title}) async {
    title ??= libL10n.share;
    final files = paths.map((path) => XFile(path)).toList();
    final params = ShareParams(title: title, files: files);
    return SharePlus.instance.share(params);
  }

  /// Share string data with a file name.
  static Future<ShareResult> shareStr(String data, {String? title}) async {
    title ??= libL10n.share;
    final params = ShareParams(text: data, title: title);
    return SharePlus.instance.share(params);
  }

  /// Share bytes data.
  static Future<ShareResult> shareBytes({
    required Uint8List bytes,
    String? title,
    String? fileName,
    String mime = 'application/octet-stream',
  }) async {
    title;
    final xfile = XFile.fromData(bytes, mimeType: mime, name: fileName ?? 'shared_file_${DateTimeX.timestamp}.bin');
    final params = ShareParams(files: [xfile], title: title ?? libL10n.share);
    return SharePlus.instance.share(params);
  }

  /// Pick a file and return the [PlatformFile] object.
  static Future<PlatformFile?> pickFile() {
    return FilePicker.pickFile(type: FileType.any);
  }

  /// Pick a file and return the file path.
  static Future<String?> pickFilePath() async {
    final picked = await pickFile();
    return picked?.path;
  }

  /// Pick a file and return its bytes.
  static Future<Uint8List?> pickFileBytes() async {
    final picked = await pickFile();
    if (picked == null) return null;

    final path = picked.path;
    if (path == null) return await picked.readAsBytes();
    return await File(path).readAsBytes();
  }

  /// Pick a file and return the file String.
  static Future<String?> pickFileString() async {
    final bytes = await pickFileBytes();
    return bytes == null ? null : utf8.decode(bytes);
  }

  /// Copy the data to the clipboard.
  static void copy(dynamic data) => switch (data.runtimeType) {
    const (String) => Clipboard.setData(ClipboardData(text: data)),
    final val => throw UnimplementedError('Not supported type: $val(${val.runtimeType})'),
  };

  /// Paste the data from the clipboard.
  static Future<String?> paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    return data?.text;
  }

}

final isAndroid = Pfs.type == Pfs.android;
final isIOS = Pfs.type == Pfs.ios;
final isLinux = Pfs.type == Pfs.linux;
final isMacOS = Pfs.type == Pfs.macos;
final isWindows = Pfs.type == Pfs.windows;
final isWeb = Pfs.type == Pfs.web;
final isMobile = Pfs.type == Pfs.ios || Pfs.type == Pfs.android;
final isDesktop = Pfs.type == Pfs.linux || Pfs.type == Pfs.macos || Pfs.type == Pfs.windows;
