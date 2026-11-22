import 'package:fl_lib/fl_lib.dart' show Hairline;
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/gruvbox.dart';
import 'package:server_box/view/widget/page_transitions.dart';

ThemeMode appThemeMode(int value) => switch (value) {
  1 => ThemeMode.light,
  2 || 3 => ThemeMode.dark,
  _ => ThemeMode.system,
};

ThemeData buildAppTheme(Gruvbox theme, Brightness brightness) {
  final palette = theme.palette(brightness);
  final background = Color(palette.background);
  final surface = Color(palette.surface);
  final raised = Color(palette.raised);
  final text = Color(palette.foreground);
  final muted = Color(palette.muted);
  final accent = Color(palette.blue);
  final scheme = ColorScheme(
    brightness: brightness,
    primary: accent,
    onPrimary: background,
    primaryContainer: raised,
    onPrimaryContainer: text,
    secondary: Color(palette.green),
    onSecondary: background,
    secondaryContainer: raised,
    onSecondaryContainer: text,
    tertiary: Color(palette.aqua),
    onTertiary: background,
    tertiaryContainer: raised,
    onTertiaryContainer: text,
    error: Color(palette.red),
    onError: background,
    errorContainer: raised,
    onErrorContainer: Color(palette.red),
    surface: background,
    onSurface: text,
    surfaceDim: background,
    surfaceBright: raised,
    surfaceContainerLowest: background,
    surfaceContainerLow: surface,
    surfaceContainer: surface,
    surfaceContainerHigh: raised,
    surfaceContainerHighest: raised,
    onSurfaceVariant: muted,
    outline: muted,
    outlineVariant: raised,
    inverseSurface: text,
    onInverseSurface: background,
    inversePrimary: Color(
      theme
          .palette(
            brightness == Brightness.dark ? Brightness.light : Brightness.dark,
          )
          .blue,
    ),
  );
  final cardShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(13),
  );
  final tileShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(9),
  );
  final buttonShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(30),
  );
  final buttonStyle = ButtonStyle(shape: WidgetStatePropertyAll(buttonShape));
  final base = ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    pageTransitionsTheme: AppPageTransitions.plain,
    cardTheme: CardThemeData(shape: cardShape, elevation: 0),
    dividerTheme: DividerThemeData(color: Hairline.of(scheme)),
    elevatedButtonTheme: ElevatedButtonThemeData(style: buttonStyle),
    filledButtonTheme: FilledButtonThemeData(style: buttonStyle),
    outlinedButtonTheme: OutlinedButtonThemeData(style: buttonStyle),
    textButtonTheme: TextButtonThemeData(style: buttonStyle),
    navigationBarTheme: NavigationBarThemeData(indicatorShape: buttonShape),
    dialogTheme: DialogThemeData(shape: cardShape),
    appBarTheme: const AppBarTheme(
      scrolledUnderElevation: 0,
      centerTitle: false,
    ),
    actionIconTheme: const ActionIconThemeData(
      backButtonIconBuilder: _backButtonIcon,
    ),
    switchTheme: const SwitchThemeData(
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
    ),
  );
  return base.copyWith(
    iconTheme: base.iconTheme.copyWith(size: 19),
    appBarTheme: base.appBarTheme.copyWith(
      titleTextStyle: base.textTheme.titleLarge?.copyWith(
        inherit: false,
        fontSize: 17,
        fontWeight: FontWeight.w500,
        color: text,
      ),
    ),
    listTileTheme: ListTileThemeData(
      shape: tileShape,
      contentPadding: const EdgeInsets.symmetric(horizontal: 13),
      horizontalTitleGap: 13,
      minLeadingWidth: 0,
      minVerticalPadding: 9,
      minTileHeight: 44,
      titleTextStyle: base.textTheme.bodyLarge?.copyWith(
        inherit: false,
        fontSize: 14,
        color: text,
      ),
      subtitleTextStyle: base.textTheme.bodyMedium?.copyWith(
        inherit: false,
        fontSize: 12,
        color: muted,
      ),
    ),
  );
}

Widget _backButtonIcon(BuildContext context) => const Icon(Icons.chevron_left);
