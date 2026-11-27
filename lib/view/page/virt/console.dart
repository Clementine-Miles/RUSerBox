part of 'guest.dart';

/// Where a guest's console goes.
///
/// Which consoles there are comes from the guest's detail
/// (`VirtGuestDetail.consoles`); how it is opened is [VirtConsoleConnect].
///
/// The text console is the terminal page, in place here: the one terminal the app
///   has, with its virtual keys, theme and reconnect, given a console instead
///   of a shell (PVE), or a shell on the host with `virsh console` typed into
///   it (libvirt). A bar under it says what it is connected through, and that
///   a console prints nothing until it is sent something. Leaving it
///   leaves the console running ([VirtTextConsoles]), and coming back takes it
///   up again.
///
/// Left — this view gone, another guest, another tab — it is closed by
/// [SessionKeepAlive] after the idle time the settings give, with a notice
/// first; coming back before then finds it as it was. A guest that stops
/// closes it (`_syncDetail` in guest.dart).
class VirtConsoleView extends StatelessWidget {
  const VirtConsoleView({
    super.key,
    required this.serverId,
    required this.guest,
    required this.state,
    required this.detail,
    this.kind,
    this.onStart,
    this.onRetryDetail,
  });

  final String serverId;
  final VirtGuest guest;
  final VirtGuestState state;

  /// The guest's detail, which says which consoles it has.
  final Future<VirtGuestDetail>? detail;

  /// The console asked for — the second level of the guest's Console
  /// segment. Null, or one the guest does not have, is [defaultKind].
  final VirtConsoleKind? kind;

  /// The text console, the only kind there is.
  static VirtConsoleKind defaultKind(Set<VirtConsoleKind> consoles) =>
      VirtConsoleKind.text;

  /// Starts the guest, when that is on offer: a console of a guest that is
  /// off has nothing on it.
  final VoidCallback? onStart;

  /// Fetches [detail] again, after it failed.
  final VoidCallback? onRetryDetail;

  @override
  Widget build(BuildContext context) {
    if (!state.isActive) {
      return EmptyPane(
        icon: Icons.desktop_access_disabled_outlined,
        title: state.label,
        action: onStart == null
            ? null
            : Btn.elevated(
                mainAxisSize: MainAxisSize.min,
                text: libL10n.start,
                icon: const Icon(Icons.play_arrow),
                onTap: onStart,
              ),
      );
    }
    return FutureBuilder<VirtGuestDetail>(
      future: detail,
      builder: (context, snap) {
        if (snap.hasError) {
          final err = snap.error;
          return EmptyPane(
            icon: Icons.error_outline,
            title: err is VirtErr ? err.title : libL10n.fail,
            label: err is VirtErr ? err.detail : '$err',
            action: onRetryDetail == null
                ? null
                : Btn.elevated(
                    mainAxisSize: MainAxisSize.min,
                    text: libL10n.retry,
                    icon: const Icon(Icons.refresh),
                    onTap: onRetryDetail,
                  ),
          );
        }
        final data = snap.data;
        if (data == null) return const Center(child: SizedLoading.medium);
        if (data.consoles.isEmpty) {
          return EmptyPane(
            icon: Icons.desktop_access_disabled_outlined,
            title: l10n.virtConsoleNone,
          );
        }
        return _VirtConsoles(
          // A different guest is a different console: nothing of the last
          // one's state.
          key: ValueKey(guest.id),
          serverId: serverId,
          guest: guest,
          kind: data.consoles.contains(kind)
              ? kind!
              : defaultKind(data.consoles),
        );
      },
    );
  }
}

class _VirtConsoles extends ConsumerStatefulWidget {
  const _VirtConsoles({
    super.key,
    required this.serverId,
    required this.guest,
    required this.kind,
  });

  final String serverId;
  final VirtGuest guest;

  /// The console shown: one the guest has.
  final VirtConsoleKind kind;

  @override
  ConsumerState<_VirtConsoles> createState() => _VirtConsolesState();
}

class _VirtConsolesState extends ConsumerState<_VirtConsoles> {
  VirtConsoleKind get _kind => widget.kind;

  var _opening = false;

  late final String _textId = VirtConsoleConnect.textSessionId(
    widget.serverId,
    widget.guest.id,
  );

  /// The text console on screen here: its page's arguments, null when there
  /// is none. Cleared whenever it leaves the screen — its page then hands the
  /// session to [VirtTextConsoles], and showing it again takes it back.
  SshPageArgs? _text;

  /// [_text]'s page is on screen, for the page's own keyboard and focus.
  final _textVisible = ValueNotifier(false);
  final _textFocus = FocusNode();

  /// Close, rather than park, the session the text page leaves behind next.
  var _endText = false;

  final _textPage = GlobalKey<SSHPageState>();

  /// Enter for a console that waits silently, on [_textPage]'s terminal:
  /// a serial port (libvirt's `virsh console`, a PVE VM's `serialN`), and a
  /// PVE container's default console, which joins a getty that drew its
  /// prompt long ago. Null while no terminal is shown.
  SerialWake? _wake;

  @override
  void dispose() {
    _wake?.dispose();
    // After the page under them, which unmounts first and lets go of both.
    _textVisible.dispose();
    _textFocus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final onScreen = ref.watch(
      currentHomeTabProvider.select((tab) => tab == AppTab.virt),
    );
    final textRunning = ref.watch(
      virtTextConsolesProvider.select((ids) => ids.contains(_textId)),
    );
    _syncText(
      shown: onScreen && _kind == VirtConsoleKind.text,
      running: textRunning,
    );

    final text = _text;
    final body = switch (_kind) {
      VirtConsoleKind.text when text != null && onScreen => _buildText(text),
      // Being taken back: the page comes with the next frame.
      VirtConsoleKind.text when textRunning => UIs.placeholder,
      VirtConsoleKind.text => _launcher(
        icon: Icons.terminal,
        text: l10n.connect,
        tip: switch (ref.watch(virtHostProvider(widget.serverId)).kind) {
          VirtHostKind.libvirt => l10n.virtConsoleSerialTip,
          _ => null,
        },
        onTap: _openText,
      ),
    };
    return body;
  }

  /// [onClose], given, is a second button: the session is running, and this
  /// ends it rather than waiting for it to be closed as idle.
  Widget _launcher({
    required IconData icon,
    required String text,
    required VoidCallback onTap,
    String? tip,
    VoidCallback? onClose,
  }) {
    final open = Btn.elevated(
      text: text,
      icon: Icon(icon),
      mainAxisSize: MainAxisSize.min,
      onTap: onTap,
    );
    return EmptyPane(
      icon: icon,
      title: widget.guest.name,
      label: tip,
      action: _opening
          ? SizedLoading.medium
          : onClose == null
          ? open
          : Wrap(
              spacing: 7,
              runSpacing: 7,
              alignment: WrapAlignment.center,
              children: [
                open,
                Btn.text(text: libL10n.close, onTap: onClose),
              ],
            ),
    );
  }

  /// Keeps [_text] to when it can be seen: gone from the screen, its page is
  /// let go (and parks the session); back, a parked session is taken up.
  void _syncText({required bool shown, required bool running}) {
    if (_textVisible.value != shown) {
      // After the frame: the page listening to it rebuilds on a change.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _textVisible.value = shown;
      });
    }
    if ((shown && _text != null) || _wake != null) {
      // The page is built (or gone) this frame; its terminal is there after.
      WidgetsBinding.instance.addPostFrameCallback((_) => _attachWake());
    }
    if (!shown && _text != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) setState(() => _text = null);
      });
    } else if (shown && _text == null && running && !_opening) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && _text == null) _resumeText();
      });
    }
  }

  /// What a text page does with its session when it goes: back to
  /// [VirtTextConsoles] to be kept, or closed when the bar's close asked.
  ///
  /// Captured rather than read through `ref` when it runs: it runs after the
  /// page has gone, and this view may have too.
  void Function(TerminalSession) _leaveText() {
    final consoles = ref.read(virtTextConsolesProvider.notifier);
    final id = _textId;
    // Which of the guest's consoles, in the notice that says it is idle.
    final name = '${widget.guest.name} · ${VirtConsoleKind.text.label}';
    final host =
        ref.read(serversProvider).servers[widget.serverId]?.name ??
        widget.serverId;
    return (session) {
      if (_endText) {
        _endText = false;
        session.close();
        return;
      }
      consoles.park(id, session, name: name, host: host);
    };
  }

  /// Watches the shown terminal for a silent console — a new one when
  /// the page shows another terminal, none once no page does.
  void _attachWake() {
    if (!mounted) return;
    final terminal = _text == null || !_textVisible.value
        ? null
        : _textPage.currentState?.terminal;
    if (identical(_wake?.terminal, terminal)) return;
    _wake?.dispose();
    _wake = terminal == null
        ? null
        : (SerialWake(terminal)..addListener(_onWake));
  }

  void _onWake() {
    if (mounted) setState(() {});
  }

  SshPageArgs _embed(SshPageArgs args) => args.embeddedIn(
    AppTab.virt,
    visible: _textVisible,
    focusNode: _textFocus,
    // The console hung up, or failed to connect: back to "Connect".
    onSessionEnd: () {
      if (mounted) setState(() => _text = null);
    },
  );

  void _resumeText() {
    final session = ref.read(virtTextConsolesProvider.notifier).take(_textId);
    if (session == null) return;
    setState(() {
      _text = _embed(
        VirtConsoleConnect.resumedTextArgs(session, onLeave: _leaveText()),
      );
    });
  }

  Future<void> _openText() async {
    if (_opening) return;
    setState(() => _opening = true);
    try {
      final args = await VirtConsoleConnect.textArgs(
        ProviderScope.containerOf(context),
        serverId: widget.serverId,
        guest: widget.guest,
        onLeave: _leaveText(),
      );
      if (!mounted) return;
      setState(() => _text = _embed(args));
    } catch (e, s) {
      Loggers.app.warning('Opening a text console', e, s);
      Toast.error(libL10n.fail, body: VirtConsoleConnect.describe(e));
    } finally {
      if (mounted && _opening) setState(() => _opening = false);
    }
  }

  /// The countdown to Enter on a silent console, with its two answers;
  /// otherwise the hint to press it — a console prints nothing until written
  /// to, whatever this saw.
  List<Widget> _buildWake() {
    final wake = _wake;
    final left = wake?.remaining;
    // Bounded, not flexible: a flexible one shared the row with the label on
    // the left, and what it left over sat after the buttons, which then
    // stood in the middle of the bar instead of at its end.
    const hint = BoxConstraints(maxWidth: 260);
    if (wake == null || left == null) {
      return [
        ConstrainedBox(
          constraints: hint,
          child: Text(
            l10n.virtConsoleEnterTip,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: UIs.text12Grey,
          ),
        ),
        UIs.width7,
      ];
    }
    return [
      ConstrainedBox(
        constraints: hint,
        child: Text(
          l10n.virtConsoleAutoEnter(left),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: UIs.text12Grey.copyWith(
            color: Theme.of(context).colorScheme.primary,
          ),
        ),
      ),
      Btn.text(text: l10n.virtConsoleEnterNow, onTap: wake.now),
      Btn.text(text: libL10n.cancel, onTap: wake.cancel),
    ];
  }

  /// Ends the text console: its page goes, and closes the session it leaves.
  void _closeText() {
    _endText = true;
    setState(() => _text = null);
  }

  /// The terminal, and under it what it is connected through.
  Widget _buildText(SshPageArgs args) {
    final spi = ref.watch(
      serversProvider.select((s) => s.servers[widget.serverId]),
    );
    final what = switch (args.source) {
      ServerSource() => 'virsh console',
      _ => 'termproxy',
    };
    final via = switch (spi?.transport) {
      ServerTransport.ssh => 'SSH',
      ServerTransport.monitorHttp => 'monitor',
      ServerTransport.local => 'localhost',
      null => null,
    };
    return Column(
      children: [
        Expanded(
          child: SSHPage(key: _textPage, args: args),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(13, 5, 7, 5),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  [what, if (via != null) l10n.virtConsoleVia(via)].join(' · '),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: UIs.text12Grey,
                ),
              ),
              ..._buildWake(),
              // libvirt's: back to the host's shell, as Ctrl+] does.
              if (args.detachInput != null)
                Btn.icon(
                  icon: const Icon(Icons.link_off, size: 17),
                  text: l10n.disconnect,
                  onTap: () => _textPage.currentState?.detach(),
                ),
              Btn.icon(
                icon: const Icon(Icons.close, size: 17),
                text: libL10n.close,
                onTap: _closeText,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

extension on VirtConsoleKind {
  String get label => switch (this) {
    VirtConsoleKind.text => libL10n.terminal,
  };

  IconData get icon => switch (this) {
    VirtConsoleKind.text => Icons.terminal,
  };
}
