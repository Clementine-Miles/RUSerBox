import 'package:server_box/data/terminal/terminal_status.dart';
import 'package:xterm/core.dart';

/// Something a terminal's programs said that is worth telling a user who is
/// not looking at it.
sealed class TerminalStatusAlert {
  const TerminalStatusAlert();

  ProgramState get state;
}

/// A record became `blocked`, `done` or `error`, or changed what it says
/// while it is one.
final class RecordAlert extends TerminalStatusAlert {
  const RecordAlert(this.headline);

  final TerminalStatusHeadline headline;

  @override
  ProgramState get state => headline.state;
}

/// A command that ran for a while finished (OSC 133).
final class CommandAlert extends TerminalStatusAlert {
  const CommandAlert(this.exitCode);

  final int? exitCode;

  @override
  ProgramState get state => exitCode == null || exitCode == 0
      ? ProgramState.done
      : ProgramState.error;
}

/// Picks out of a [TerminalStatus] what changed into something worth an alert
/// since it was last asked. Asked on every change, on screen or not, so what
/// the user watched happen is not announced once they look away.
final class TerminalStatusAlerts {
  TerminalStatusAlerts({
    this.longCommand = const Duration(seconds: 30),
    DateTime Function()? now,
  }) : _now = now ?? DateTime.now;

  /// How long a command has to run for its end to be worth an alert.
  final Duration longCommand;

  final DateTime Function() _now;

  /// What each alerting record last said.
  final _said = <String, String>{};

  /// When the current command was first seen running.
  DateTime? _commandSince;

  /// The alerts since the last call, the most urgent first.
  List<TerminalStatusAlert> take(TerminalStatus status) {
    final alerts = <TerminalStatusAlert>[];
    final present = <String>{};
    final records = status.shell;
    for (final record in records.records) {
      if (!_alerting(record.state)) continue;
      final key = record.key;
      present.add(key);
      final report = record.report;
      final said =
          '${record.state.name}\u0000${report.title}\u0000${report.msg}';
      if (_said[key] == said) continue;
      _said[key] = said;
      alerts.add(
        RecordAlert(
          TerminalStatusHeadline(
            state: record.state,
            report: report,
            progress: report.progress,
          ),
        ),
      );
    }

    const progressKey = '\u0000progress';
    if (records.progress?.state == TerminalProgressState.error) {
      present.add(progressKey);
      if (_said[progressKey] == null) {
        _said[progressKey] = 'error';
        alerts.add(
          RecordAlert(const TerminalStatusHeadline(state: ProgramState.error)),
        );
      }
    }

    final command = records.command;
    if (command != null && command.running) {
      _commandSince ??= _now();
    } else {
      final since = _commandSince;
      _commandSince = null;
      if (command != null &&
          since != null &&
          _now().difference(since) >= longCommand) {
        alerts.add(CommandAlert(command.exitCode));
      }
    }

    _said.removeWhere((key, _) => !present.contains(key));
    alerts.sort((a, b) => a.state.index.compareTo(b.state.index));
    return alerts;
  }

  static bool _alerting(ProgramState state) =>
      state == ProgramState.blocked ||
      state == ProgramState.done ||
      state == ProgramState.error;
}
