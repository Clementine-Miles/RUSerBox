import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:xterm/core.dart';

/// What the programs of one terminal session say about themselves: OSC 7501
/// reports, OSC 9;4 progress and OSC 133 marks, by the lifetime rules of
/// [ProgramStatusRecords].
final class TerminalStatus extends ChangeNotifier {
  TerminalStatus() {
    shell.addListener(notifyListeners);
  }

  final shell = ProgramStatusRecords();

  bool get hasReports => shell.records.isNotEmpty || shell.progress != null;

  /// Every record as plain lines, for a reader that is not a person: the
  /// Agent. Empty when nothing is reported. The text in it is the programs'
  /// own, and so data rather than anything to follow.
  String describeForAgent() {
    final lines = <String>[];
    for (final record in shell.records) {
      final report = record.report;
      lines.add(
        [
          'shell${record.key.isEmpty ? '' : ' [${record.key}]'}:',
          record.state.name,
          if (report.kind case final kind?) '(${kind.name})',
          if (shell.appOf(record) case final app?) 'app=$app',
          if (report.progress case final progress?) 'progress=$progress%',
          if (report.title case final title?) 'title=${jsonEncode(title)}',
          if (report.msg case final msg?) 'msg=${jsonEncode(msg)}',
        ].join(' '),
      );
    }
    if (shell.progress case final bar?) {
      lines.add(
        'shell: progress bar ${bar.state!.name}'
        '${bar.percent == null ? '' : ' ${bar.percent}%'}',
      );
    }
    if (shell.command case final command?) {
      lines.add(
        command.running
            ? 'shell: a command is running'
            : 'shell: last command exited ${command.exitCode ?? 'unknown'}',
      );
    }
    return lines.join('\n');
  }

  void dismiss(List<String> id) =>
      shell.apply(ProgramStatusReport.clear(id: id));

  ProgramState? get state => shell.state;

  /// The most urgent report, preferring the most recent among equals.
  TerminalStatusHeadline? get headline {
    final current = state;
    if (current == null) return null;
    ProgramStatusReport? report;
    for (final record in shell.records) {
      if (record.state == current) report = record.report;
    }
    return TerminalStatusHeadline(
      state: current,
      report: report,
      progress:
          report?.progress ?? (report == null ? shell.progress?.percent : null),
    );
  }

  @override
  void dispose() {
    shell.removeListener(notifyListeners);
    super.dispose();
  }
}

/// See [TerminalStatus.headline].
final class TerminalStatusHeadline {
  const TerminalStatusHeadline({
    required this.state,
    this.report,
    this.progress,
  });

  final ProgramState state;

  /// Null when only an OSC 9;4 progress bar says it.
  final ProgramStatusReport? report;

  /// 0–100, or null when unknown.
  final int? progress;
}
