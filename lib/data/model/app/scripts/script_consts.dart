
/// Path constants and per-connection script directory state.
///
/// Script content, markers, headers, and output splitting live in the shared
/// Rust library (`sbm_parser::script`); only path/filename conventions and
/// Flutter-connection state remain here.
// TODO(migration): residue of the Dart script layer. What still has to move
// into sbm_parser's script API: the script directory constants below and
// [ScriptPaths]' per-server directory state (which directory the last write
// fell back to).
class ScriptConstants {
  const ScriptConstants._();

  /// The number in the remote script's filename.
  ///
  /// It must only increase, and it must increase whenever the generated script
  /// changes: the name is what decides whether a server reuses the copy it
  /// already has. Two builds writing different scripts under one name is the
  /// failure this exists to prevent.
  ///
  /// A plain constant, here. It was a Git-history count computed during the
  /// build, whose result varied with the source paths and with whether a build
  /// went through `fl_build` at all — so it became a hand-maintained number,
  /// and then a hand-maintained number in *two* files with a test holding them
  /// level, because `fl_build` regenerates `BuildData` and drops anything it
  /// was not fed.
  static const int version = 90;

  static const String scriptFile = 'srvboxm_v$version.sh';

  // Script directories
  static const String scriptDirHome = '~/.config/server_box';
  static const String scriptDirTmp = '/tmp/server_box';

  static const String unixPathSeparator = '/';
}

/// Script path configuration and management
class ScriptPaths {
  ScriptPaths._();

  static final Map<String, String> _scriptDirMap = <String, String>{};

  /// Returns the current script directory for [id].
  ///
  /// Starts with the temporary directory. [switchScriptDir] changes it to the
  /// home-directory location when the temporary path is unavailable.
  static String getScriptDir(String id) =>
      _scriptDirMap[id] ??= ScriptConstants.scriptDirTmp;

  /// Switches between the temporary and home script directories.
  static String switchScriptDir(String id) => _scriptDirMap[id] =
      _scriptDirMap[id] == ScriptConstants.scriptDirHome
      ? ScriptConstants.scriptDirTmp
      : ScriptConstants.scriptDirHome;

  /// Returns the full script path for [id].
  static String getScriptPath(String id) =>
      '${getScriptDir(id)}${ScriptConstants.unixPathSeparator}${ScriptConstants.scriptFile}';
}
