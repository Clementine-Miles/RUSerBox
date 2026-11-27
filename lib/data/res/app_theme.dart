import 'package:fl_lib/fl_lib.dart' show Hairline;
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/gruvbox.dart';
import 'package:server_box/view/widget/page_transitions.dart';

ThemeMode appThemeMode(int value) => switch (value) {
  1 => ThemeMode.light,
  2 || 3 => ThemeMode.dark,
  _ => ThemeMode.system,
};

/// The app's [ThemeData] for one built-in Gruvbox variant: the generated
/// [ColorScheme] (`gruvbox_palettes.g.dart`) plus the app's shapes and type.
ThemeData buildAppTheme(GruvboxTheme theme, Brightness brightness) {
  final scheme = theme.scheme(brightness);
  final text = scheme.onSurface;
  final muted = scheme.onSurfaceVariant;
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
