part of 'page.dart';

extension _Init on SSHPageState {
  /// Connects a new source of shells. The provider's grant is a fallback hint;
  /// [TerminalSession.connect] asks the agent directly before opening a PTY.
  ///
  /// The status poller's connection first, when it is on its way — see
  /// [_awaitServerClient] — and a connection of the terminal's own otherwise.
  Future<ShellBackend> _connectBackend() async {
    final granted = switch (widget.args.spi) {
      final spi? => ref.readPresentServer(spi.id)?.agentAccess,
      null => null,
    };
    final shared = await _awaitServerClient();
    // Gone while waiting: nothing is left to show a shell in.
    if (!mounted) throw const TerminalSessionClosed();
    if (shared != null) {
      _sess.adopt(shared, granted: granted);
      if (_sess.backend case final backend?) return backend;
    }
    return _sess.connect(granted: granted, context: mounted ? context : null);
  }

  /// The status poller's connection, when it has one or is making one — see
  /// [awaitServerClient]. Null for a terminal on no server, or one deleted
  /// since the tab opened.
  Future<SSHClient?> _awaitServerClient() async {
    final id = widget.args.spi?.id;
    if (id == null || ref.readPresentServer(id) == null) return null;
    return awaitServerClientOf(ref, id);
  }

  void _bindForegroundSession(ShellSession session, {String? zellijName}) {
    _sess.bindForeground(session, zellijSessionName: zellijName);
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.connected);
  }

  void _onForegroundSessionDone(ShellSession session) {
    // A transport that closed under the session is a connection loss, not the
    // end of one — route it to the reconnect path rather than tearing the tab
    // down. Normal session end (shell exit) and user-initiated disconnect
    // leave the transport open, so they still take the path below.
    //
    final transportClosed = _backend != null && _backend!.isClosed;
    if (mounted && transportClosed && !_endRequested) {
      unawaited(_onConnectionLossSuspected());
      return;
    }

    if (mounted && !_endRequested && _sess.zellijSessionName != null) {
      unawaited(_returnToShell());
      return;
    }

    if (mounted && widget.args.notFromTab) {
      context.pop();
    }
    widget.args.onSessionEnd?.call();
    TermSessionManager.remove(_sessionId);
  }

  void _initStoredCfg() {
    _terminalStyle = TerminalLook.styleOf(context);
  }

  Future<void> _showHelp() async {
    if (Stores.setting.sshTermHelpShown.fetch()) return;
    // Where there are virtual keys, the walkthrough says all of this and says
    // it beside the keys it is naming. This paragraph is what a desktop gets
    // instead, having no such row to point at.
    if (_virtKeysHeight > 0 && !Stores.setting.virtualKeyIntroShown.fetch()) {
      Stores.setting.sshTermHelpShown.put(true);
      return;
    }

    return await context.showSheet(
      title: libL10n.doc,
      child: Text(l10n.sshTermHelp),
      actions: [
        TextButton(
          onPressed: () {
            Stores.setting.sshTermHelpShown.put(true);
            context.popSheet();
          },
          child: Text(l10n.noPromptAgain),
        ),
      ],
    );
  }

  Future<void> _initTerminal() async {
    if (_openingTerminal) return;
    _endRequested = false;
    // A session handed to this page is already connected and already running
    // something — the dialog that started it did all of this. Opening a second
    // shell here would replace what the user asked to carry on watching.
    if (_adopted) {
      // It may also have finished on the way here, in which case this tab is
      // the output and nothing more — said plainly rather than left looking
      // like a shell that stopped answering.
      TermSessionManager.updateStatus(
        _sessionId,
        _session != null
            ? TermSessionStatus.connected
            : TermSessionStatus.disconnected,
      );
      _focusTerminal(keyboard: false);
      return;
    }

    _openingTerminal = true;
    _retryInitialConnectionOnResume = false;
    _setConnectionStep(TerminalConnectionStep.connecting);
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.connecting);
    try {
      // Startup may precede the network. Permission and auth failures are not
      // retried; only transport failures use these short backoffs. Four
      // tries: a server whose `MaxStartups` is shedding connections turns
      // each one away at random, a third of the time or more, and three
      // tries still lost about one launch in thirty.
      const retryDelays = [
        Duration.zero,
        Duration(milliseconds: 500),
        Duration(seconds: 2),
        Duration(seconds: 5),
      ];
      for (final delay in retryDelays) {
        if (delay != Duration.zero) {
          await Future.delayed(delay);
          if (!mounted) return;
          _setConnectionStep(TerminalConnectionStep.connecting);
        }

        try {
          if (_backend?.isClosed == true) _sess.dropBackend(alive: false);
          if (_backend == null) await _connectBackend();
          if (!mounted) {
            _sess.close();
            return;
          }

          _setConnectionStep(TerminalConnectionStep.openingShell);
          final (session, zellijName) = await _openForegroundSession();
          if (!mounted) {
            session?.close();
            _sess.close();
            return;
          }
          if (session == null) {
            // Asked, as a reconnect asks: a borrowed link that stopped
            // answering but is not closed yet would be adopted again.
            final alive = await _answers(_backend);
            if (!mounted) return;
            _sess.dropBackend(alive: alive);
            _setConnectionStep(TerminalConnectionStep.shellFailed);
            TermSessionManager.updateStatus(
              _sessionId,
              TermSessionStatus.disconnected,
            );
            return;
          }

          _bindForegroundSession(session, zellijName: zellijName);
          _setConnectionStep(TerminalConnectionStep.ready);
          break;
        } catch (error, stackTrace) {
          Loggers.app.warning('Failed to open terminal', error, stackTrace);
          final alive = await _answers(_backend);
          if (!mounted) return;
          _sess.dropBackend(alive: alive);
          final retryable = isRetryableTerminalConnectionError(error);
          if (delay != retryDelays.last && retryable) {
            continue;
          }
          _retryInitialConnectionOnResume = retryable;
          _setConnectionStep(
            _connectionStep == TerminalConnectionStep.openingShell
                ? TerminalConnectionStep.shellFailed
                : TerminalConnectionStep.connectionFailed,
            detail: switch (error) {
              TerminalRemoteAccessUnavailable() => l10n.monitorNoRemoteAccess,
              TerminalConsoleErr(:final message) => message,
              LocalServerErr() => error.solution,
              _ => null,
            },
          );
          TermSessionManager.updateStatus(
            _sessionId,
            TermSessionStatus.disconnected,
          );
          return;
        }
      }
    } finally {
      _openingTerminal = false;
    }

    // Not awaited: a snippet can `${sleep N}`, and the page must not wait for
    // it before taking keyboard input.
    if (_sess.zellijSessionName == null) unawaited(_runStartupInput());

    _focusTerminal(keyboard: false);
  }

  /// Auto-run snippets, then `initCmd`, then `initSnippet`, one after another:
  /// a snippet with placeholders types across awaits, so running them
  /// concurrently would interleave their input.
  ///
  /// All of it is for the shell that was in front when startup began. A
  /// snippet can wait (`${sleep N}`) through a reconnect or a switch to Zellij,
  /// and what is left must not be typed into the shell that replaced it.
  Future<void> _runStartupInput() async {
    final shell = _session;
    bool current() =>
        mounted &&
        shell != null &&
        identical(_session, shell) &&
        _sess.zellijSessionName == null;

    // Snippets name the server they run on, and their scripts are written
    // against one. A terminal on this device has neither.
    final spi = widget.args.spi;
    final snippets = ref.read(snippetProvider.select((p) => p.snippets));
    if (spi != null) {
      for (final snippet in snippets) {
        if (snippet.autoRunOn?.contains(spi.id) == true) {
          if (!current()) return;
          await _runStartupSnippet(snippet, spi, current);
        }
      }
    }

    if (!current()) return;
    final initCmd = widget.args.initCmd;
    if (initCmd != null) {
      await _runAnsweringSudo(initCmd);
    }

    final initSnippet = widget.args.initSnippet;
    if (initSnippet != null && (spi != null || !initSnippet.needsServer)) {
      if (!current()) return;
      await _runStartupSnippet(initSnippet, spi, current);
    }
  }

  Future<void> _runStartupSnippet(
    Snippet snippet,
    Spi? spi,
    bool Function() current,
  ) async {
    try {
      await snippet.runInTerm(_terminal, spi, alive: current);
    } catch (e, s) {
      if (!mounted) return;
      context.showErrSheet(e, s, '${libL10n.snippet}: ${snippet.name}');
    }
  }

  void _setupDiscontinuityTimer() {
    _discontinuityTimer?.cancel();
    if (!mounted) return;

    _missedKeepAliveCount = 0;
    _discontinuityTimer = Timer.periodic(
      SSHPageState._connectionCheckInterval,
      (_) => _checkConnectionHealth(),
    );
  }

  /// Asks the far side whether it is still there.
  ///
  /// [immediate] means "answer now" — a resume, or the page becoming visible —
  /// rather than waiting for the timer's turn.
  ///
  /// It used to also mean "one missed reply is the verdict", which is the worst
  /// rule at the worst moment. A resume is when the radio is coldest, and
  /// `home.dart` starts a refresh of every configured server on the same frame;
  /// an SSH handshake is pure Dart on this isolate, so each server that cannot
  /// be reached costs a connect timeout of this isolate's attention. A few of
  /// those in the list were enough to push one ping past its ten seconds and
  /// close a session that was fine — which is how "I have servers that don't
  /// connect" turned into "switching apps drops my shell" (#1287).
  ///
  /// It now asks as many times as the timer would before concluding anything.
  /// It just does not wait a minute between the tries.
  Future<void> _checkConnectionHealth({bool immediate = false}) async {
    if (!mounted || _backend == null) return;
    if (_isCheckingConnection) {
      if (immediate) _hasPendingImmediateCheck = true;
      return;
    }
    _isCheckingConnection = true;

    try {
      final attempts = immediate ? SSHPageState._maxKeepAliveFailures : 1;
      Object? lastError;
      StackTrace? lastStackTrace;
      // A reconnect may replace the backend while a ping is out; what the old
      // one answers says nothing about the new one.
      final backend = _backend!;
      for (var attempt = 0; attempt < attempts; attempt++) {
        if (attempt > 0) {
          // Only ever paid by a link that is slow. One that is gone says so at
          // once, and the check below leaves the loop without waiting.
          await Future.delayed(SSHPageState._connectionCheckRetryDelay);
          if (!mounted || !identical(_backend, backend)) return;
        }
        try {
          await backend.ping().timeout(SSHPageState._connectionCheckTimeout);
          if (!mounted || !identical(_backend, backend)) return;
          _missedKeepAliveCount = 0;
          if (_reportedDisconnected) {
            _reportedDisconnected = false;
            TermSessionManager.updateStatus(
              _sessionId,
              TermSessionStatus.connected,
            );
          }
          return;
        } on Object catch (error, stackTrace) {
          if (!mounted || !identical(_backend, backend)) return;
          lastError = error;
          lastStackTrace = stackTrace;
          // The transport itself says it is gone. Nothing to wait for.
          if (backend.isClosed) break;
        }
      }
      _handleConnectionCheckFailure(
        lastError ?? StateError('SSH keep-alive did not run'),
        stackTrace: lastStackTrace,
        immediate: immediate,
      );
    } finally {
      _isCheckingConnection = false;
      if (_hasPendingImmediateCheck) {
        _hasPendingImmediateCheck = false;
        unawaited(_checkConnectionHealth(immediate: true));
      }
    }
  }

  void _handleConnectionCheckFailure(
    Object error, {
    StackTrace? stackTrace,
    bool immediate = false,
  }) {
    Loggers.root.warning('SSH keep-alive failed', error, stackTrace);
    _missedKeepAliveCount += 1;

    if (!immediate &&
        _missedKeepAliveCount < SSHPageState._maxKeepAliveFailures) {
      return;
    }

    _missedKeepAliveCount = 0;
    unawaited(_onConnectionLossSuspected());
  }

  Future<void> _onConnectionLossSuspected() async {
    if (!mounted || _disconnectDialogOpen || _endRequested) return;

    _disconnectDialogOpen = true;
    _reportedDisconnected = true;
    _discontinuityTimer?.cancel();
    _writeLn('\n\n${libL10n.disconnected}\r\n');
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.disconnected);

    final restored = await _tryReconnect(zellijName: _sess.zellijSessionName);
    if (!mounted) return;
    if (restored) {
      _disconnectDialogOpen = false;
      _reportedDisconnected = false;
      return;
    }

    // Reconnect failed or was cancelled -> ask whether to leave.
    unawaited(_showDisconnectDialog());
  }

  /// Reconnects, because the user asked for it — the toolbar button.
  ///
  /// Not [_onConnectionLossSuspected]: that path reports a link it found dead
  /// and, when it cannot get it back, asks whether to leave the page. A
  /// deliberate reconnect has neither problem. The link may well have been
  /// fine, and the question would be an answer to a loss that never happened —
  /// pressing reconnect on a slow server must not offer to throw the terminal
  /// away. So this says what happened and stops there.
  ///
  /// Re-attaching Zellij keeps what runs in it. Without a session to re-attach,
  /// the reconnect opens a new shell and the one on screen — perhaps fine, only
  /// slow — ends with whatever it was running and what it printed, so that is
  /// asked first.
  ///
  /// A link that still answers is not closed: it may be the status poller's,
  /// shared with other tabs and the file browser. Only this terminal's shell
  /// is replaced then.
  Future<void> reconnect() async {
    if (!mounted || !widget.args.canReconnect) return;
    if (_disconnectDialogOpen || _openingTerminal) return;
    _endRequested = false;
    // Never connected: there is no shell to end, and the first connection is
    // what the overlay's retry makes.
    if (!_adopted && _connectionStep != TerminalConnectionStep.ready) {
      unawaited(_initTerminal());
      return;
    }
    final shell = _session;
    final backend = _backend;
    final zellij = _sess.zellijSessionName;
    if (zellij == null) {
      final confirmed = await context.showSheet<bool>(
        title: l10n.reconnect,
        child: Text(l10n.reconnectEndsShell),
        actions: Btnx.cancelOk,
      );
      if (confirmed != true || !mounted) return;
      // The keep-alive may have found the link dead and replaced it while the
      // question was up: what was asked about is gone.
      if (_disconnectDialogOpen ||
          _openingTerminal ||
          !identical(_session, shell) ||
          !identical(_backend, backend)) {
        return;
      }
    }

    _disconnectDialogOpen = true;
    // Stop the keep-alive from firing the automatic path underneath this one.
    _discontinuityTimer?.cancel();
    _missedKeepAliveCount = 0;
    _writeLn('\n\n${libL10n.reconnecting}\r\n');

    final answers = await _answers(backend);
    if (!mounted) return;
    final restored = await _tryReconnect(
      zellijName: _sess.zellijSessionName,
      keepLink: answers,
    );
    if (!mounted) return;

    // [_reconnect] armed the keep-alive and binding the shell said connected.
    if (restored) {
      _disconnectDialogOpen = false;
      _reportedDisconnected = false;
      return;
    }

    // Cancelled, or the reconnect itself gave up. The old shell was let go
    // before the first attempt, so the terminal is disconnected and the
    // keep-alive has nothing to ask; the button tries again — a server that is
    // merely slow comes back on the next press.
    _writeLn(libL10n.fail);
    _disconnectDialogOpen = false;
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.disconnected);
  }

  /// Whether [backend] still answers, within the keep-alive's own timeout.
  Future<bool> _answers(ShellBackend? backend) async {
    if (backend == null || backend.isClosed) return false;
    try {
      await backend.ping().timeout(SSHPageState._connectionCheckTimeout);
      return true;
    } catch (_) {
      return false;
    }
  }

  /// Re-establish SSH, re-attaching [zellijName] when there is one, and
  /// showing a cancellable progress dialog while it runs. [keepLink] lets go
  /// of a borrowed link that still answers instead of closing it.
  ///
  /// Returns `true` when the terminal was restored.
  Future<bool> _tryReconnect({
    String? zellijName,
    bool keepLink = false,
  }) async {
    _reconnectCancelled = false;
    if (mounted) _showReconnectingDialog();
    try {
      return await _reconnect(zellijName: zellijName, keepLink: keepLink);
    } catch (e, st) {
      Loggers.app.warning('SSH reconnect threw', e, st);
      await _failReconnect();
      return false;
    } finally {
      // Not gated on `mounted`, which is what this used to be. The dialog is
      // on the root navigator and outlives the page that raised it, so a page
      // torn down mid-reconnect — its tab closed, the app moved on — left it
      // spinning over every tab with nothing able to close it: this pop was
      // skipped, and the cancel button did not close it either.
      //
      // Before returning, so the caller's next dialog goes up over an empty
      // navigator rather than over this one. See [_dismissReconnectingDialog].
      _dismissReconnectingDialog();
    }
  }

  /// Cancellable progress dialog shown while reconnecting.
  void _showReconnectingDialog() {
    unawaited(
      context.showSheet(
        onShown: (close) => _closeReconnecting = close,
        child: Row(
          children: [
            const SizedBox(
              width: 24,
              height: 24,
              child: CircularProgressIndicator(strokeWidth: 2.5),
            ),
            const SizedBox(width: 16),
            Expanded(child: Text(libL10n.reconnecting)),
            // Closes the dialog itself. An `onTap` replaces the pop `Btn`
            // would otherwise do from its own context, so a callback that only
            // set the flag left the button doing nothing anyone could see: the
            // loop reads the flag between attempts, and an attempt is a whole
            // SSH connect timeout long.
            Btn.cancel(
              onTap: () {
                _reconnectCancelled = true;
                _dismissReconnectingDialog();
              },
            ),
          ],
        ),
        dismissible: false,
      ),
    );
  }

  /// Closes the reconnecting dialog, once. Safe when it is already gone.
  ///
  /// Now, not next frame. A pop takes whatever is on top of the root
  /// navigator, and the caller that closes this dialog is usually about to put
  /// another one up — [_onConnectionLossSuspected] asks "go back?" the moment
  /// the reconnect it was waiting on gives up. Scheduled, this pop landed
  /// after that question was already on screen and closed *it*: the spinner
  /// stayed, and the dialog meant to replace it was gone in a frame.
  ///
  /// [deferred] is for [dispose] alone, which runs inside the frame that is
  /// unmounting this page — popping a route from there is a `markNeedsBuild`
  /// during build. Nothing follows it, so nothing can be popped by mistake.
  void _dismissReconnectingDialog({bool deferred = false}) {
    final close = _closeReconnecting;
    if (close == null) return;
    _closeReconnecting = null;
    if (!deferred) {
      close();
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) => close());
  }

  Future<void> _showDisconnectDialog() async {
    final shouldLeave = await context.showSheet<bool>(
      title: libL10n.attention,
      child: Text('${libL10n.disconnected}\n${libL10n.goBackQ}'),
      dismissible: false,
      actions: [
        TextButton(
          onPressed: () => context.popSheet(false),
          child: Text(libL10n.cancel),
        ),
        TextButton(
          onPressed: () => context.popSheet(true),
          child: Text(libL10n.ok),
        ),
      ],
    );

    if (!mounted) return;

    _disconnectDialogOpen = false;

    if (shouldLeave == true) {
      contextSafe?.pop(); // Pop the SSHPage
      return;
    }

    // If the client is gone or its transport already closed, "stay" means
    // "try again" rather than resuming monitoring of a dead connection.
    if (_backend == null || _backend!.isClosed) {
      unawaited(_onConnectionLossSuspected());
      return;
    }

    _reportedDisconnected = false;
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.connected);
    _setupDiscontinuityTimer();
  }

  /// Reconnect SSH after a connection loss, re-attaching [zellijName] when
  /// there is one to re-attach.
  ///
  /// Returns `true` when a usable terminal was restored (either the Zellij
  /// session was re-attached or, as a fallback, a raw shell was opened). Returns
  /// `false` only when the SSH transport itself could not be re-established, in
  /// which case the caller falls back to the disconnect prompt.
  Future<bool> _reconnect({String? zellijName, bool keepLink = false}) async {
    // Tear down the stale SSH session/client first — only the shell, when the
    // link is borrowed and still answers.
    _sess.dropBackend(alive: keepLink);

    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.connecting);

    const maxAttempts = 10;
    const baseInterval = Duration(milliseconds: 200);
    const maxInterval = Duration(seconds: 3);
    var connected = false;
    for (
      var attempt = 0;
      attempt < maxAttempts && mounted && !_reconnectCancelled;
      attempt++
    ) {
      if (attempt > 0) {
        final backoffMs = (baseInterval.inMilliseconds << (attempt - 1))
            .clamp(0, maxInterval.inMilliseconds)
            .toInt();
        await Future.delayed(Duration(milliseconds: backoffMs));
        if (!mounted || _reconnectCancelled) {
          await _failReconnect();
          return false;
        }
      }
      try {
        await _connectBackend();
        connected = true;
        break;
      } catch (_) {
        Loggers.app.info(
          'SSH reconnect attempt ${attempt + 1}/$maxAttempts failed',
        );
      }
    }
    if (!mounted || _reconnectCancelled) {
      await _failReconnect();
      return false;
    }
    if (!connected) {
      Loggers.app.info('SSH reconnect failed after $maxAttempts attempts');
      await _failReconnect();
      return false;
    }

    if (zellijName != null) {
      final attached = await _reattachZellij(zellijName);
      if (!mounted || _reconnectCancelled) {
        await _failReconnect(unbound: attached);
        return false;
      }
      if (attached != null) {
        _bindForegroundSession(attached, zellijName: zellijName);
        _setupDiscontinuityTimer();
        _focusTerminal(keyboard: false);
        return true;
      }
    }

    final shell = await _sess.openShell();
    if (shell == null || !mounted || _reconnectCancelled) {
      await _failReconnect(unbound: shell);
      return false;
    }
    _bindForegroundSession(shell);
    // A new shell is not the program the old one was running: what the
    // session is for is started again in it.
    if (_sess.reenter case final cmd?) {
      unawaited(_runAnsweringSudo(cmd));
    }
    _setupDiscontinuityTimer();
    _focusTerminal(keyboard: false);
    return true;
  }

  /// After a reconnect that did not restore the terminal — see
  /// [TerminalSession.dropBackend], asked whether the link still answers.
  ///
  /// [unbound] is a shell it opened and did not bind, its own to close either
  /// way. The session only while this page holds it: once the page is gone it
  /// was closed, handed on or left to `onLeave`, and what its new holder bound
  /// is not this reconnect's to end.
  ///
  /// Cancelled, it does not wait on a ping: the user has stopped waiting, and
  /// a link that never answers would hold the result for the whole timeout.
  Future<void> _failReconnect({ShellSession? unbound}) async {
    final alive =
        mounted &&
        (_reconnectCancelled
            ? _backend?.isClosed == false
            : await _answers(_backend));
    if (mounted) {
      _sess.dropBackend(alive: alive, unbound: unbound);
      return;
    }
    try {
      unbound?.close();
    } catch (e, st) {
      Loggers.app.warning('Failed to close shell', e, st);
    }
  }

  void _writeLn(String p0) => _sess.writeLn(p0);

  ZellijSessions? get _zellij {
    final client = _client;
    if (client == null || widget.args.source is! ServerSource) return null;
    return ZellijSessions(SshExec(client), environment: _sess.environment);
  }

  String get _defaultZellijName {
    final name = Stores.setting.zellijSessionName.fetch().trim();
    return ZellijSessions.validName(name) ? name : 'server_box';
  }

  Future<(ShellSession?, String?)> _openForegroundSession() async {
    final zellij = _zellij;
    if (zellij != null &&
        Stores.setting.zellijAuto.fetch() &&
        widget.args.initCmd == null &&
        widget.args.initSnippet == null) {
      try {
        final bin = await zellij.find();
        if (!mounted) return (null, null);
        if (bin != null) {
          final sessions = await zellij.list(bin);
          if (!mounted) return (null, null);
          final defaultName = _defaultZellijName;
          final ZellijChoice? choice;
          if (Stores.setting.zellijShowSelector.fetch()) {
            choice = await showZellijSessionPickerSheet(
              context,
              sessions: sessions,
              defaultSessionName: defaultName,
              allowTerminate: false,
            );
          } else if (sessions.isEmpty) {
            choice = (action: ZellijAction.create, name: defaultName);
          } else {
            final name = sessions.contains(defaultName)
                ? defaultName
                : sessions.first;
            choice = (action: ZellijAction.attach, name: name);
          }
          if (!mounted) return (null, null);
          if (choice != null) {
            final shell = await _openZellij(zellij, bin, choice);
            if (shell != null) return (shell, choice.name);
          }
        }
      } catch (error, stack) {
        Loggers.app.warning('Could not open Zellij on connect', error, stack);
      }
    }
    return (mounted ? await _sess.openShell() : null, null);
  }

  Future<ShellSession?> _openZellij(
    ZellijSessions zellij,
    String bin,
    ZellijChoice choice,
  ) async {
    final backend = _backend;
    final create = choice.action == ZellijAction.create;
    // Zellij accepts unique prefixes for attach. Check the full name again
    // when acting on a picker result, which may have been open for a while.
    if (!create && !(await zellij.list(bin)).contains(choice.name)) return null;
    if (!mounted || _endRequested || !identical(_backend, backend)) return null;
    final command = zellij.attachCommand(bin, choice.name, create: create);
    return command == null ? null : _sess.execute(command);
  }

  Future<ShellSession?> _reattachZellij(String name) async {
    final zellij = _zellij;
    if (zellij == null) return null;
    try {
      final bin = await zellij.find();
      if (bin == null) return null;
      return await _openZellij(zellij, bin, (
        action: ZellijAction.attach,
        name: name,
      ));
    } catch (error, stack) {
      Loggers.app.warning('Could not reattach Zellij', error, stack);
      return null;
    }
  }

  Future<void> _returnToShell() async {
    final old = _session;
    final backend = _backend;
    ShellSession? shell;
    try {
      shell = await _sess.openShell();
    } catch (error, stack) {
      Loggers.app.warning('Could not return to the SSH shell', error, stack);
    }
    if (!mounted ||
        _endRequested ||
        !identical(_backend, backend) ||
        !identical(_session, old)) {
      shell?.close();
      return;
    }
    if (shell != null) {
      _replaceForeground(shell);
    } else if (old == null) {
      _setConnectionStep(TerminalConnectionStep.shellFailed);
      TermSessionManager.updateStatus(
        _sessionId,
        TermSessionStatus.disconnected,
      );
    } else {
      Toast.error(libL10n.fail);
    }
  }

  void _replaceForeground(ShellSession shell, {String? zellijName}) {
    final old = _session;
    _sess.unbindForeground();
    try {
      // Closing the SSH PTY disconnects only this Zellij client. Its forced
      // close behavior was set when launching the client.
      old?.close();
    } catch (error, stack) {
      Loggers.app.warning('Could not close the previous shell', error, stack);
    }
    _bindForegroundSession(shell, zellijName: zellijName);
    _followBottom();
    _focusTerminal(keyboard: false);
  }

  Future<void> _showZellijSwitcher() async {
    final zellij = _zellij;
    final old = _session;
    final backend = _backend;
    if (zellij == null || old == null || _openingTerminal || _switchingZellij) {
      return;
    }
    bool current() =>
        mounted &&
        !_endRequested &&
        identical(_backend, backend) &&
        identical(_session, old);
    _switchingZellij = true;
    try {
      final bin = await zellij.find();
      if (!current()) return;
      if (bin == null) {
        Toast.show(l10n.zellijNotAvailable);
        return;
      }
      final sessions = await zellij.list(bin);
      if (!current()) return;
      final selected = _sess.zellijSessionName;
      final choice = await showZellijSessionPickerSheet(
        context,
        sessions: sessions,
        defaultSessionName: _defaultZellijName,
        currentSessionName: selected,
      );
      if (choice == null || !current()) return;
      switch (choice.action) {
        case ZellijAction.attach when choice.name == selected:
          return;
        case ZellijAction.attach:
        case ZellijAction.create:
          final shell = await _openZellij(zellij, bin, choice);
          if (!current()) {
            shell?.close();
          } else if (shell != null) {
            _replaceForeground(shell, zellijName: choice.name);
          } else {
            Toast.error(libL10n.fail);
          }
        case ZellijAction.detach:
          await _returnToShell();
        case ZellijAction.terminate:
          final confirm = await showDialog<bool>(
            context: context,
            builder: (dialogContext) => AlertDialog(
              title: Text(l10n.zellijTerminate),
              content: Text(l10n.zellijTerminateConfirm(choice.name)),
              actions: [
                TextButton(
                  onPressed: () => Navigator.of(dialogContext).pop(false),
                  child: Text(libL10n.cancel),
                ),
                TextButton(
                  onPressed: () => Navigator.of(dialogContext).pop(true),
                  child: Text(l10n.zellijTerminate),
                ),
              ],
            ),
          );
          if (confirm == true && current()) {
            await zellij.terminate(bin, choice.name);
          }
      }
    } catch (error, stack) {
      Loggers.app.warning('Could not manage Zellij sessions', error, stack);
      if (mounted) Toast.error(libL10n.fail);
    } finally {
      _switchingZellij = false;
    }
  }
}

extension on SSHPageState {
  void _disconnectFromNotification() {
    // Stop All runs every disconnect in one synchronous pass; the shell's
    // `done` arrives after it, whichever closed the shared client first.
    _endRequested = true;
    // Update observers before closing the channel so the UI and notification
    // respond immediately.
    TermSessionManager.updateStatus(_sessionId, TermSessionStatus.disconnected);

    try {
      _session?.close();
    } catch (e, stackTrace) {
      Loggers.app.warning('Error closing SSH session: $e\n$stackTrace');
    }
  }
}
