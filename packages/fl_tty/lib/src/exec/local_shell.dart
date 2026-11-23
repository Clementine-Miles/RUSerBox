import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:fl_lib/fl_lib.dart';
import 'package:fl_tty/src/pty/pty.dart';
import 'package:fl_tty/src/rootfs/android_rootfs.dart';
import 'package:fl_tty/src/exec/shell_backend.dart';

/// [ShellBackend] on the machine the app is running on.
///
/// Opens an Android or Linux host pseudo-terminal, optionally inside a guest.
class LocalShellBackend implements ShellBackend {
  LocalShellBackend({this.inRootfs = false, this.profileId});

  /// Which installed system, when [inRootfs], or null for the selected one.
  ///
  /// proot is a host process per session, so two backends with different ids
  /// are two systems running at once and nothing has to coordinate them.
  final String? profileId;

  /// Whether shells start inside the Linux userland rather than the platform's
  /// own. See [AndroidRootfs].
  final bool inRootfs;

  /// Android shells and Linux hosts use the same PTY implementation.
  static bool get isSupported => Platform.isAndroid || Platform.isLinux;

  /// The user's own shell where the OS says so, and a shell that certainly
  /// exists where it does not.
  ///
  /// `$SHELL` rather than a hard-coded path, because a terminal that ignores
  /// which shell someone chose is a terminal they have to `exec` out of every
  /// time they open it.
  static String get shellPath {
    if (Platform.isAndroid) return '/system/bin/sh';
    final preferred = Platform.environment['SHELL'];
    if (preferred != null && preferred.isNotEmpty) return preferred;
    return '/bin/sh';
  }

  /// Where a new shell starts, or null where there is nowhere to say.
  ///
  /// Android has neither: an app has no `HOME`, so a shell inherits the
  /// process's working directory and opens at `/`, which is read-only and has
  /// nothing in it. The app's own directory is the one place it can write, so
  /// that is this device's home — and it is exported as `HOME` too, or `cd`
  /// with no argument and every `~` in a command would still point at `/`.
  static String? get homeDir {
    if (Platform.isAndroid) return Paths.doc;
    final home = Platform.environment['HOME'];
    return home?.isNotEmpty == true ? home : null;
  }

  var _closed = false;
  final _sessions = <ShellSession>[];

  @override
  bool get isClosed => _closed;

  @override
  bool get supportsExec => true;

  @override
  Future<ShellSession> openShell({
    required int width,
    required int height,
    Map<String, String>? environment,
  }) async {
    final guest = inRootfs
        ? await AndroidRootfs.enter(profileId: profileId)
        : null;
    if (inRootfs && guest == null) {
      throw StateError('The selected Linux system is no longer available.');
    }
    try {
      return _start(
        guest?.executable ?? shellPath,
        arguments: guest?.arguments ?? const [],
        width: width,
        height: height,
        environment: environment,
        onDone: guest?.release,
      );
    } catch (_) {
      guest?.release();
      rethrow;
    }
  }

  @override
  Future<ShellSession> execute(
    String command, {
    required int width,
    required int height,
    Map<String, String>? environment,
  }) async {
    if (_closed) {
      throw StateError('This local shell backend is closed');
    }
    final guest = inRootfs
        ? await AndroidRootfs.enter(command: command, profileId: profileId)
        : null;
    if (inRootfs && guest == null) {
      throw StateError('The selected Linux system is no longer available.');
    }
    try {
      return _start(
        guest?.executable ?? shellPath,
        arguments: guest?.arguments ?? ['-c', command],
        width: width,
        height: height,
        environment: environment,
        onDone: guest?.release,
      );
    } catch (_) {
      guest?.release();
      rethrow;
    }
  }

  _LocalShellSession _start(
    String executable, {
    required List<String> arguments,
    required int width,
    required int height,
    Map<String, String>? environment,
    void Function()? onDone,
  }) {
    if (_closed) {
      throw StateError('This local shell backend is closed');
    }
    final home = homeDir;
    final session = _LocalShellSession(
      Pty.start(
        executable,
        arguments: arguments,
        // Android's application directory is the local shell's home.
        environment: Platform.isAndroid && home != null
            ? {'HOME': home, ...?environment}
            : environment,
        workingDirectory: home,
        // A terminal that has not been laid out yet reports zero, and a pty of
        // no size makes programs that ask draw nothing.
        rows: height > 0 ? height : 25,
        columns: width > 0 ? width : 80,
      ),
    );
    _sessions.add(session);
    unawaited(
      session.done.whenComplete(() {
        _sessions.remove(session);
        onDone?.call();
      }),
    );
    return session;
  }

  /// Nothing to reach, so nothing can be unreachable.
  ///
  /// The keep-alive this drives is about a link going away underneath a
  /// session. There is no link: a local shell is either running or has exited,
  /// and its exit is reported by [ShellSession.done] like any other.
  @override
  Future<void> ping() async {}

  @override
  void close() {
    if (_closed) return;
    _closed = true;
    // A copy: closing each one removes it from the list.
    for (final session in [..._sessions]) {
      session.close();
    }
  }
}

/// [ShellSession] on a local pseudo-terminal.
class _LocalShellSession implements ShellSession {
  _LocalShellSession(this._pty) {
    unawaited(_pty.exitCode.then((_) => _exited = true));
  }

  final Pty _pty;

  /// Whether the shell is already gone, so [close] neither signals a pid that
  /// may since have been reused nor schedules a second attempt at nothing.
  var _exited = false;

  @override
  Stream<Uint8List>? get stdout => _pty.output;

  /// Null, like the monitor agent's PTY: a pseudo-terminal merges the two the
  /// way a real terminal does, and there is no second stream to offer.
  @override
  Stream<Uint8List>? get stderr => null;

  @override
  void write(List<int> data) =>
      _pty.write(data is Uint8List ? data : Uint8List.fromList(data));

  @override
  void resizeTerminal(int width, int height) => _pty.resize(height, width);

  @override
  Future<void> get done => _pty.exitCode;

  @override
  void close() {
    if (_exited) return;

    // Interactive shells can ignore SIGTERM. Signal the terminal hangup.
    //
    // The whole process group, not only the shell. `forkpty` makes the child a
    // session leader, so its pid is its group id, and whatever is in the
    // foreground — `top`, an editor, a build — is in that group. Signalling
    // the leader alone leaves those running with their terminal gone.
    _signal(ProcessSignal.sighup);

    // For a shell, or a foreground process, that trapped the hangup. Nothing
    // is reading this terminal any more, so leaving it running only leaks it.
    Timer(const Duration(seconds: 3), () {
      if (_exited) return;
      _signal(ProcessSignal.sigkill);
    });
  }

  void _signal(ProcessSignal signal) {
    try {
      Process.killPid(-_pty.pid, signal);
    } catch (_) {
      // No such group, or a platform that will not take a negative pid. The
      // leader itself is still signalled below.
    }
    try {
      _pty.kill(signal);
    } catch (_) {
      // Already gone. `done` has completed or is about to.
    }
  }
}
