import 'dart:async';

import 'package:server_box/data/model/server/server_exec.dart';
import 'package:server_box/src/rust/api/zellij.dart' as ffi;

/// One SSH connection's Zellij CLI. The interactive client runs through the
/// terminal's existing SSH PTY, while these commands use SSH exec channels.
final class ZellijSessions {
  const ZellijSessions(this._exec, {this.environment});

  final CommandExec _exec;
  final Map<String, String>? environment;

  static bool validName(String name) => ffi.zellijValidName(name: name);

  Future<String?> find() async {
    final result = await _run(ffi.zellijFindCommand());
    return result.exitCode == 0
        ? ffi.zellijParseFind(stdout: result.stdout)
        : null;
  }

  Future<List<String>> list(String bin) async {
    final result = await _run(ffi.zellijListSessionsCommand(bin: bin));
    final sessions = ffi.zellijReadSessions(
      stdout: result.stdout,
      stderr: result.stderr,
      exitCode: result.exitCode ?? -1,
    );
    if (sessions != null && !result.outputIncomplete) return sessions;
    throw CommandFailed('Zellij list-sessions', result);
  }

  String? attachCommand(String bin, String name, {bool create = false}) =>
      ffi.zellijAttachCommand(bin: bin, name: name, create: create);

  Future<void> terminate(String bin, String name) async {
    final command = ffi.zellijKillSessionCommand(bin: bin, name: name);
    if (command == null) throw ArgumentError.value(name, 'name');
    final result = await _run(command);
    if (result.exitCode != 0) {
      throw CommandFailed('Zellij kill-session', result);
    }
  }

  Future<ExecResult> _run(String command) async {
    const deadline = Duration(seconds: 8);
    final cancel = Completer<void>();
    final timer = Timer(deadline, cancel.complete);
    try {
      return await _exec
          .run(command, env: environment, cancel: cancel.future)
          .timeout(deadline);
    } finally {
      timer.cancel();
    }
  }
}
