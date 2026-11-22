import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
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

  static TerminalTheme themeOf(BuildContext context) {
    final palette = Stores.setting.appThemePreset.fetch().palette(
      isDark(context) ? Brightness.dark : Brightness.light,
    );
    final background = Color(palette.background);
    final foreground = Color(palette.foreground);
    final ansi = palette.ansi;
    return TerminalTheme(
      cursor: foreground,
      selectionCursor: Color(palette.blue),
      selection: Color(palette.raised),
      foreground: foreground,
      background: background,
      searchHitBackground: Color(palette.yellow),
      searchHitBackgroundCurrent: Color(palette.green),
      searchHitForeground: background,
      black: const Color(0xFF282828),
      red: Color(ansi?[0] ?? palette.red),
      green: Color(ansi?[1] ?? palette.green),
      yellow: Color(ansi?[2] ?? palette.yellow),
      blue: Color(ansi?[3] ?? palette.blue),
      magenta: Color(ansi?[4] ?? palette.purple),
      cyan: Color(ansi?[5] ?? palette.aqua),
      white: const Color(0xFFEBDBB2),
      brightBlack: const Color(0xFF928374),
      brightRed: Color(palette.red),
      brightGreen: Color(palette.green),
      brightYellow: Color(palette.yellow),
      brightBlue: Color(palette.blue),
      brightMagenta: Color(palette.purple),
      brightCyan: Color(palette.aqua),
      brightWhite: const Color(0xFFFBF1C7),
    );
  }
}
