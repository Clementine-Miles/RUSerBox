import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/gruvbox.dart';
import 'package:server_box/data/store/stores.dart';
import 'package:xterm/ui.dart';

/// How a terminal is configured to look, wherever one is shown.
///
/// Shared rather than read where it is needed, so the terminal in a dialog and
/// the terminal in a tab cannot end up on different fonts or a different theme.
abstract final class TerminalLook {
  /// The bounds of [SettingStore.termLineHeight].
  static const minLineHeight = 1.0;
  static const maxLineHeight = 2.0;

  static TerminalStyle styleOf(BuildContext context) {
    final family = Stores.setting.terminalTypeface.fetch().family;
    final size = Stores.setting.termFontSize.fetch();
    final height = Stores.setting.termLineHeight.fetch().clamp(
      minLineHeight,
      maxLineHeight,
    );
    final style = TerminalStyle.fromTextStyle(
      TextStyle(fontFamily: family, fontSize: size, height: height),
    );
    // Keep the terminal's monospace fonts ahead of UI fallbacks,
    // and a family already on the terminal's list is not added again:
    // `sans-serif` is on both, and the UI list is appended on every theme
    // change.
    final own = style.fontFamilyFallback;
    final uiFallbacks =
        Theme.of(context).textTheme.bodyMedium?.fontFamilyFallback ?? const [];
    return style.copyWith(
      fontFamilyFallback: [
        ...own,
        ...uiFallbacks.where((f) => !own.contains(f)),
      ],
    );
  }

  /// The terminal's own theme setting, falling back to the app's and then to
  /// what the system asked for.
  static bool isDark(BuildContext context) =>
      switch (Stores.setting.termTheme.fetch()) {
        1 => false,
        2 => true,
        _ => context.isDark,
      };

  /// The terminal's colours from the active Gruvbox variant.
  ///
  /// The normal ANSI colours are each family's standard terminal set
  /// (classic Gruvbox's neutral tones, Gruvbox Material's own); the bright
  /// ones are the family's anchors. Default foreground and background are
  /// the variant's surface pair, and the selection its accent container. A
  /// program's own 24-bit colours are drawn as sent: only the 16-colour
  /// table and the defaults come from here.
  static TerminalTheme themeOf(BuildContext context) {
    final brightness = isDark(context) ? Brightness.dark : Brightness.light;
    final theme = Stores.setting.appTheme;
    final c = theme.colors(brightness);
    final dark = brightness == Brightness.dark;
    final classic = theme.family == GruvboxFamily.gruvbox;
    // Classic Gruvbox's terminal palette: the "neutral" ANSI tones, the same
    // in light and dark, with the faded anchors as the bright half in light.
    const neutral = [0xFFCC241D, 0xFF98971A, 0xFFD79921, 0xFF458588, 0xFFB16286, 0xFF689D6A];
    final normal = classic
        ? [for (final v in neutral) Color(v)]
        : [c.red, c.green, c.yellow, c.blue, c.purple, c.aqua];
    final bright = [c.red, c.green, c.yellow, c.blue, c.purple, c.aqua];
    return TerminalTheme(
      cursor: c.foreground,
      selectionCursor: c.accent,
      selection: c.selection,
      foreground: c.foreground,
      background: c.background,
      searchHitBackground: c.yellow,
      searchHitBackgroundCurrent: c.orange,
      searchHitForeground: c.background,
      black: dark ? const Color(0xFF282828) : const Color(0xFFFBF1C7),
      red: normal[0],
      green: normal[1],
      yellow: normal[2],
      blue: normal[3],
      magenta: normal[4],
      cyan: normal[5],
      white: dark ? const Color(0xFFA89984) : const Color(0xFF7C6F64),
      brightBlack: const Color(0xFF928374),
      brightRed: bright[0],
      brightGreen: bright[1],
      brightYellow: bright[2],
      brightBlue: bright[3],
      brightMagenta: bright[4],
      brightCyan: bright[5],
      brightWhite: dark ? const Color(0xFFEBDBB2) : const Color(0xFF3C3836),
    );
  }
}
