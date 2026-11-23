import 'dart:collection';

import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/server/server_exec.dart';
import 'package:server_box/data/terminal/zellij.dart';

import '../../helpers/rust_lib_helper.dart';

const _bin = '/home/test/.cargo/bin/zellij';

void main() {
  setUpAll(initRustLibForTest);

  test('a host without Zellij has no session manager', () async {
    final zellij = ZellijSessions(_Exec([_result(1)]));
    expect(await zellij.find(), isNull);
  });

  test('list active sessions and terminate the chosen full name', () async {
    final exec = _Exec([
      _result(0, stdout: 'Welcome!\n$_bin\n'),
      _result(
        0,
        stdout:
            'work here [Created 1h ago] \nold [Created 2h ago] (EXITED - attach to resurrect)\n',
      ),
      _result(0),
    ]);
    final zellij = ZellijSessions(exec);
    expect(await zellij.find(), _bin);
    expect(await zellij.list(_bin), ['work here']);
    expect(
      zellij.attachCommand(_bin, 'work here'),
      "exec '$_bin' attach 'work here' options --on-force-close detach",
    );
    expect(
      zellij.attachCommand(_bin, 'new', create: true),
      "exec '$_bin' attach --create 'new' options --on-force-close detach",
    );
    await zellij.terminate(_bin, 'work here');
    expect(exec.commands.skip(1), [
      "'$_bin' list-sessions --no-formatting",
      "'$_bin' kill-session 'work here'",
    ]);
  });

  test('distinguishes an empty list from a CLI failure', () async {
    final zellij = ZellijSessions(
      _Exec([
        _result(1, stderr: 'No active zellij sessions found.'),
        _result(1, stderr: 'Permission denied'),
      ]),
    );
    expect(await zellij.list(_bin), isEmpty);
    await expectLater(zellij.list(_bin), throwsA(isA<CommandFailed>()));
  });
}

ExecResult _result(int code, {String stdout = '', String stderr = ''}) =>
    ExecResult(exitCode: code, stdout: stdout, stderr: stderr);

final class _Exec implements CommandExec {
  _Exec(Iterable<ExecResult> results) : _results = Queue.of(results);

  final Queue<ExecResult> _results;
  final commands = <String>[];

  @override
  Future<ExecResult> run(
    String script, {
    String? entry,
    Map<String, String>? env,
    String? stdin,
    OnExecOutput? onStdout,
    OnExecOutput? onStderr,
    Future<void>? cancel,
  }) async {
    commands.add(script);
    return _results.removeFirst();
  }
}
