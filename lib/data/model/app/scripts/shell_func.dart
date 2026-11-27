import 'package:server_box/data/model/app/scripts/script_consts.dart';
import 'package:server_box/data/model/server/system.dart';
import 'package:server_box/data/res/build_data.dart';
import 'package:server_box/src/rust/api/parser.dart' as rs show SystemType;
import 'package:server_box/src/rust/api/script.dart' as ffi;

/// Shell functions of the generated script.
///
/// The FRB-generated enum is the single source; function names, flags, and
/// script generation live in the shared Rust library (sbm_parser::script, see
/// the shared-parser design). This file only maps app types onto the FFI and keeps the
/// connection-state concerns (script paths, custom dirs).
typedef ShellFunc = ffi.ShellFuncKind;

extension ShellFuncX on ShellFunc {
  /// Command line flag for this function (wire format owned by Rust)
  String get flag => ffi.shellFuncFlag(func: this);

  /// Execute this shell function on the specified server
  String exec(String id, {SystemType? systemType, required String? customDir}) {
    final scriptPath = ShellFuncManager.getScriptPath(
      id,
      systemType: systemType,
      customDir: customDir,
    );
    return ffi.execCommand(
      system: ShellFuncManager.ffiSystem(systemType),
      scriptPath: scriptPath,
      func: this,
    );
  }
}

/// Resolves script paths and builds commands for shell functions.
class ShellFuncManager {
  const ShellFuncManager._();

  /// The platform the FFI builds for: Linux where none is known, the
  /// historical default.
  static rs.SystemType ffiSystem(SystemType? systemType) => (systemType ?? SystemType.linux).ffi;

  /// Removes trailing path separators from [dir].
  static String _normalizeDir(String dir) =>
      dir.replaceAll(RegExp('${RegExp.escape(ScriptConstants.unixPathSeparator)}+\$'), '');

  /// Returns the script directory for [id].
  ///
  /// Uses [customDir] when provided; otherwise uses the stored default.
  /// [systemType] is accepted so callers state the host; every administered
  /// host takes the same POSIX layout.
  static String getScriptDir(
    String id, {
    SystemType? systemType,
    required String? customDir,
  }) {
    if (customDir != null) return _normalizeDir(customDir);
    return ScriptPaths.getScriptDir(id);
  }

  /// Switches between the temporary and home script directories.
  static void switchScriptDir(String id, {SystemType? systemType}) {
    ScriptPaths.switchScriptDir(id);
  }

  /// Returns the full script path for [id].
  static String getScriptPath(
    String id, {
    SystemType? systemType,
    required String? customDir,
  }) {
    if (customDir != null) {
      return '${_normalizeDir(customDir)}${ScriptConstants.unixPathSeparator}'
          '${ScriptConstants.scriptFile}';
    }
    return ScriptPaths.getScriptPath(id);
  }

  /// Builds the command that installs the script.
  static String getInstallShellCmd(
    String id, {
    SystemType? systemType,
    required String? customDir,
  }) {
    final normalizedDir = _normalizeDir(
      getScriptDir(id, systemType: systemType, customDir: customDir),
    );
    final scriptPath =
        '$normalizedDir${ScriptConstants.unixPathSeparator}${ScriptConstants.scriptFile}';
    return ffi.installCommand(
      system: ffiSystem(systemType),
      scriptDir: normalizedDir,
      scriptPath: scriptPath,
    );
  }

  /// What to write to [getInstallShellCmd]'s stdin to install [content].
  ///
  /// The payload's shape is `sbm_parser::script`'s; today it is [content]
  /// unchanged.
  static String installPayload(String content, {SystemType? systemType}) {
    return ffi.installPayload(system: ffiSystem(systemType), content: content);
  }

  /// Generate complete script based on system type.
  ///
  /// No longer a function of the user's custom commands: those are files in a
  /// directory of their own, which the script reads. So these bytes
  /// change only when the app does, and a command with a stray quote in it
  /// breaks that command rather than the whole status page.
  static String allScript({
    SystemType? systemType,
    List<String>? disabledCmdTypes,
  }) {
    return ffi.buildScript(
      system: ffiSystem(systemType),
      disabled: disabledCmdTypes ?? const [],
      buildNumber: '${BuildData.build}',
    );
  }
}
