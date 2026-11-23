part of 'page.dart';

extension _Keyboard on SSHPageState {
  void _handleEscKeyOrBackButton() {
    _terminal.keyInput(TerminalKey.escape);
    HapticFeedback.lightImpact();
  }

  bool _handleKeyEvent(KeyEvent event) {
    // `HardwareKeyboard` handlers are global and every open terminal keeps one
    // — the pages stay alive, that is what `wantKeepAlive` is for. Without this
    // an Escape typed in one terminal reached all of them, and every key was
    // handled as many times as there were tabs.
    if (!mounted || !_keyboardHandlerActive || !_isVisibleSessionPage) {
      return false;
    }
    if (event is KeyDownEvent) {
      // The clipboard chords are deliberately *not* here. `TerminalView` binds
      // all of them already — Cmd+C/V, Ctrl+Shift+C/V and Ctrl+V, in
      // `defaultTerminalShortcuts` — and its intents paste and copy for this
      // page (`onCopied` is the one it passes). Handling them here as well
      // ran both: this is a `HardwareKeyboard` handler and that is the focus
      // tree, two paths that each see every event, so one Cmd+V pasted twice.
      //
      // Plain Ctrl+C is bound by neither, which is what keeps SIGINT reaching
      // the shell — the one key nobody will accept losing.
      if (event.logicalKey == LogicalKeyboardKey.escape) {
        // Consume Escape locally after forwarding it to the terminal.
        _handleEscKeyOrBackButton();
        return true;
      }
      if (event.logicalKey == LogicalKeyboardKey.shiftLeft ||
          event.logicalKey == LogicalKeyboardKey.shiftRight) {
        _terminal.keyInput(TerminalKey.shift);
        HapticFeedback.lightImpact();
        return true;
      }
    }
    return false;
  }
}
