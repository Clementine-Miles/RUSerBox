import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:math' as math;

import 'package:fl_lib/fl_lib.dart';
import 'package:flutter/foundation.dart';
import 'package:server_box/core/chan.dart';
import 'package:server_box/core/utils/coalesced.dart';
import 'package:server_box/data/store/stores.dart';
import 'package:server_box/data/terminal/android_service_policy.dart';
import 'package:xterm/core.dart' show ProgramState;

enum TermSessionStatus {
  connecting,
  connected,
  disconnected;

  @override
  String toString() {
    return name.capitalize;
  }
}

/// A running SSH terminal session shown in Android notifications.
class TermSessionInfo {
  final String id;
  final String title; // Usually the server name.
  final String subtitle; // Usually `user@host:port`.

  /// The server the session is on; null for this device. Sessions on one
  /// server are shown as one.
  final String? serverId;
  final int startTimeMs;
  final TermSessionStatus status;

  /// The most urgent thing the session's programs report (OSC 7501 and
  /// friends), and that in words. Null while they report nothing.
  final ProgramState? programState;
  final String? program;

  TermSessionInfo({
    required this.id,
    required this.title,
    required this.subtitle,
    this.serverId,
    required this.startTimeMs,
    required this.status,
    this.programState,
    this.program,
  });

  TermSessionInfo copyWith({
    TermSessionStatus? status,
    (ProgramState?, String?)? program,
  }) => TermSessionInfo(
    id: id,
    title: title,
    subtitle: subtitle,
    serverId: serverId,
    startTimeMs: startTimeMs,
    status: status ?? this.status,
    programState: program == null ? programState : program.$1,
    program: program == null ? this.program : program.$2,
  );

  Map<String, Object> toJson() => {
    'id': id,
    'title': title,
    'subtitle': subtitle,
    'startTimeMs': startTimeMs,
    'status': status.toString(),
    'program': ?program,
  };
}

/// The sessions [TermSessionManager] shows, and which one is on screen: the
/// state alone, with nothing of the platforms it is pushed to.
///
/// Each change answers whether anything changed, which is what decides that
/// the notification is updated.
final class TermSessionRegistry {
  final _entries = <String, TermSessionEntry>{};
  String? _activeId;

  /// Every session, by id, in the order they were added.
  Map<String, TermSessionEntry> get entries => UnmodifiableMapView(_entries);

  /// The session on screen.
  String? get activeId => _activeId;

  void add(
    TermSessionInfo info,
    VoidCallback disconnect, {
    bool setAsActive = false,
    bool hasTerminal = true,
  }) {
    _entries[info.id] = TermSessionEntry(info, disconnect, hasTerminalUI: hasTerminal);
    if (setAsActive) _activeId = info.id;
  }

  bool updateStatus(String id, TermSessionStatus status) {
    final old = _entries[id];
    if (old == null || old.info.status == status) return false;
    _entries[id] = TermSessionEntry(
      old.info.copyWith(status: status),
      old.disconnect,
      hasTerminalUI: old.hasTerminalUI,
    );
    return true;
  }

  bool updateProgram(String id, ProgramState? state, String? text) {
    final old = _entries[id];
    if (old == null) return false;
    if (old.info.programState == state && old.info.program == text) return false;
    _entries[id] = TermSessionEntry(
      old.info.copyWith(program: (state, text)),
      old.disconnect,
      hasTerminalUI: old.hasTerminalUI,
    );
    return true;
  }

  /// One notification entry per server, with its
  /// status connection and its terminals merged. Each session of this device
  /// is its own.
  List<TermSessionInfo> get shown {
    final byServer = <String, TermSessionInfo>{};
    for (final entry in _entries.values) {
      final info = entry.info;
      final key = info.serverId ?? info.id;
      final seen = byServer[key];
      byServer[key] = seen == null ? info : _merge(seen, info);
    }
    return byServer.values.toList();
  }

  static TermSessionInfo _merge(TermSessionInfo a, TermSessionInfo b) {
    final (programState, program) = _moreUrgent(a, b)
        ? (a.programState, a.program)
        : (b.programState, b.program);
    return TermSessionInfo(
      id: a.id,
      title: a.title,
      subtitle: a.subtitle,
      serverId: a.serverId,
      startTimeMs: math.min(a.startTimeMs, b.startTimeMs),
      // Connected if any is: the server is reachable.
      status: _statusRank(a.status) <= _statusRank(b.status) ? a.status : b.status,
      programState: programState,
      program: program,
    );
  }

  static int _statusRank(TermSessionStatus status) => switch (status) {
    TermSessionStatus.connected => 0,
    TermSessionStatus.connecting => 1,
    TermSessionStatus.disconnected => 2,
  };

  /// Whether [a]'s program report is at least as urgent as [b]'s.
  static bool _moreUrgent(TermSessionInfo a, TermSessionInfo b) {
    final stateA = a.program == null ? null : a.programState;
    final stateB = b.program == null ? null : b.programState;
    if (stateB == null) return true;
    if (stateA == null) return false;
    return stateA.index <= stateB.index;
  }

  /// Of [shown], the one whose programs report the most urgent state, if any
  /// do.
  TermSessionInfo? get mostUrgentProgram {
    TermSessionInfo? best;
    for (final info in shown) {
      if (info.program == null) continue;
      if (best == null || !_moreUrgent(best, info)) best = info;
    }
    return best;
  }

  void remove(String id) {
    _entries.remove(id);
    if (_activeId == id) _activeId = _entries.keys.firstOrNull;
  }

  /// Marks [id] as the session on screen; answers whether it is a session.
  bool setActive(String id, {bool hasTerminal = true}) {
    _activeId = id;
    final old = _entries[id];
    if (old == null) return false;
    _entries[id] = TermSessionEntry(old.info, old.disconnect, hasTerminalUI: hasTerminal);
    return true;
  }

  /// Marks [id]'s terminal as hidden without making another one active.
  bool hideTerminal(String id) {
    final old = _entries[id];
    if (old == null) return false;
    _entries[id] = TermSessionEntry(old.info, old.disconnect, hasTerminalUI: false);
    if (_activeId == id) _activeId = null;
    return true;
  }

  /// Forgets every session.
  void clear() {
    _entries.clear();
    _activeId = null;
  }
}

/// Singleton to track active SSH sessions and sync to Android notifications.
abstract final class TermSessionManager {
  static var _registry = TermSessionRegistry();

  @visibleForTesting
  static TermSessionRegistry get registry => _registry;

  static Map<String, TermSessionEntry> get _entries => _registry.entries;
  static var _syncs = Coalesced(() => _push());

  /// What a drain pushes each time: the platforms, outside tests.
  static Future<void> Function() _push = _syncLatest;

  @visibleForTesting
  static set push(Future<void> Function() push) => _push = push;

  /// Back to how the process starts: no sessions, no flags, nothing pending.
  @visibleForTesting
  static void reset() {
    _registry = TermSessionRegistry();
    _syncs.cancelPending();
    _syncs = Coalesced(() => _push());
    _push = _syncLatest;
    _stoodDown = false;
    _backgrounded = false;
    _transferring = false;
    _hasServers = false;
  }
  static Future<void> init() async {
    if (isAndroid) {
      MethodChans.registerHandler(
        onStopAll: stopAllConnections,
        // The first attempt to start the service raised the permission prompt
        // and was refused by the same call, because asking is asynchronous.
        // This is the answer arriving, and syncing is what asks again.
        onNotificationPermissionGranted: _sync,
        onBackgrounded: setBackgrounded,
      );
      _pushKeepAlive();
      Stores.setting.bgRun.listenable().addListener(_pushKeepAlive);
    }
  }

  /// Called when Android notification "Stop All" button is pressed
  static void stopAllConnections() {
    // Disconnect all sessions, then forget them
    for (final disconnect in [for (final e in _entries.values) e.disconnect]) {
      disconnect();
    }
    _registry.clear();
    // And stand down, rather than letting the keep-alive put the notification
    // straight back: pressing Stop All in the background would otherwise clear
    // every session and then start the service again for [_backgrounded], so
    // the one control the notification offers would appear not to work.
    // In the foreground (the tile over the app) there is no keep-alive to give
    // up, and the flag would outlive this visit.
    _stoodDown = _backgrounded;
    _sync();
  }

  /// Whether the user has asked, from the notification, to be left alone.
  ///
  /// Cleared on the next resume, not on the next connection: it is an answer
  /// about this trip to the background, and coming back to the app is what
  /// ends that.
  static var _stoodDown = false;

  /// The id of [serverId]'s status connection, whichever transport it uses.
  static String statusSessionId(String serverId) => 'server_$serverId';

  /// Whether session [id] is registered.
  static bool contains(String id) => _entries.containsKey(id);

  /// Add a session record and push update to Android.
  static void add({
    required String id,
    required String title,
    String? serverId,

    /// What the title is not enough to tell apart — `user@ip:port` for a
    /// server. Empty where there is nothing to add, which is what a terminal
    /// on this device has.
    String subtitle = '',
    required int startTimeMs,
    required VoidCallback disconnect,
    TermSessionStatus status = TermSessionStatus.connecting,
    bool setAsActive = false,
    bool hasTerminal = true,
  }) {
    final info = TermSessionInfo(
      id: id,
      title: title,
      subtitle: subtitle,
      serverId: serverId,
      startTimeMs: startTimeMs,
      status: status,
    );
    _registry.add(info, disconnect, setAsActive: setAsActive, hasTerminal: hasTerminal);
    _sync();
  }

  static void updateStatus(String id, TermSessionStatus status) {
    if (_registry.updateStatus(id, status)) _sync();
  }

  /// Updates what the terminal's programs report in the notification.
  static void updateProgram(String id, ProgramState? state, String? text) {
    if (_registry.updateProgram(id, state, text)) _sync();
  }

  static void remove(String id) {
    _registry.remove(id);
    _sync();
  }

  static void _sync() {
    unawaited(_syncs());
  }

  /// Whether the activity is paused, from its native `onPause`/`onResume`.
  ///
  /// `onPause` is the last moment Android 12+ lets an app *start* a foreground
  /// service; Flutter's `paused` comes later, from `onStop`. Flutter's
  /// `inactive` and `resumed` are not used: they also follow window focus, so
  /// a pulled-down notification shade would read as leaving.
  static var _backgrounded = false;

  static void setBackgrounded(bool value) {
    if (_backgrounded == value) return;
    _backgrounded = value;
    // Coming back to the app ends a stand-down: it was an answer about being
    // in the background, and the user is not there any more.
    if (!value) _stoodDown = false;
    _sync();
  }

  /// Whether the process must stay out of the freezer right now.
  ///
  /// Two reasons, either of which is enough. Something is connected, which is
  /// what the notification is *for*. Or the app is in the background with
  /// [SettingStore.bgRun] on and a server to keep, which is what that switch
  /// has always said it means — "the program will try to run in the
  /// background" — and never did:
  /// it only kept the poll timer scheduled, and a frozen process runs no
  /// timers.
  ///
  /// The zero-connection case is deliberate. Dropping the service the moment
  /// the last connection goes is what made a background disconnect permanent:
  /// the app cannot start a foreground service from the background, so nothing
  /// was left running to reconnect with. Keeping it means a notification while
  /// backgrounded even with nothing connected, which is the cost of being able
  /// to come back.
  ///
  /// A file transfer in progress is a third: it runs on this process, and a
  /// frozen one leaves it, and its notification, standing still. Stop All
  /// does not end it, and the native side keeps the service up for it.
  static bool get _serviceWanted =>
      _entries.isNotEmpty ||
      _transferring ||
      (_backgrounded && !_stoodDown && _keepAliveWanted);

  static var _transferring = false;

  /// Whether any file transfer is running, as `AndroidSystemSync` sees them.
  static void setTransferring(bool value) {
    if (_transferring == value) return;
    _transferring = value;
    _sync();
  }

  static var _hasServers = false;

  /// Whether any server is configured, as `AndroidSystemSync` sees them.
  static void setHasServers(bool value) {
    if (_hasServers == value) return;
    _hasServers = value;
    _pushKeepAlive();
    _sync();
  }

  /// Keeping the process alive in the background is for the servers: their
  /// status and their reconnects. With none there is nothing to keep.
  static bool get _keepAliveWanted =>
      _hasServers && Stores.setting.bgRun.fetch();

  /// The native side starts the keep-alive on `onPause` from this: the
  /// `activityPaused` round trip can land after the app is background.
  static void _pushKeepAlive() {
    if (!isAndroid) return;
    unawaited(MethodChans.setKeepAliveInBackground(_keepAliveWanted));
  }

  static Future<void> _syncLatest() async {
    // Android: update foreground service notifications
    if (isAndroid) {
      final wanted = _serviceWanted;
      // No query is needed for an update, and no stop is allowed while the app
      // is backgrounded, unless stood down. Apart from saving a platform round
      // trip, that keeps an empty state from starting a foreground service just
      // to stop it again.
      final mayStop = !_backgrounded || _stoodDown;
      final running = !wanted && mayStop
          ? await MethodChans.isServiceRunning()
          : false;
      switch (decideAndroidSessionServiceAction(
        wanted: wanted,
        running: running,
        backgrounded: _backgrounded,
        stoodDown: _stoodDown,
      )) {
        case AndroidSessionServiceAction.update:
          await MethodChans.updateSessions(
            jsonEncode({
              'sessions': _registry.shown.map((e) => e.toJson()).toList(),
              // Tells the service that an empty list is not the same as
              // "nothing to do" while background execution is wanted.
              'keepAlive': wanted,
            }),
          );
        case AndroidSessionServiceAction.stop:
          // Serialized through the service itself, so a pending foreground
          // start is promoted before the stop command is handled.
          await MethodChans.stopService();
        case AndroidSessionServiceAction.none:
          break;
      }
    }

  }

  /// Mark which session is actively displayed in UI.
  static void setActive(String id, {bool hasTerminal = true}) {
    if (_registry.setActive(id, hasTerminal: hasTerminal)) _sync();
  }

  /// Mark a session's terminal UI as hidden without promoting it to active.
  static void hideTerminal(String id) {
    if (_registry.hideTerminal(id)) _sync();
  }
}

/// One session as [TermSessionRegistry] holds it.
class TermSessionEntry {
  final TermSessionInfo info;
  final VoidCallback disconnect;
  final bool hasTerminalUI;
  TermSessionEntry(this.info, this.disconnect, {this.hasTerminalUI = true});
}
