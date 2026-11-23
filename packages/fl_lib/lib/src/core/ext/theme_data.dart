import 'package:fl_lib/src/res/l10n.dart';
import 'package:material_ui/material_ui.dart';

extension ThemeModeX on ThemeMode {
  String get i18n => switch (this) {
        ThemeMode.dark => l10n.dark,
        ThemeMode.light => l10n.bright,
        ThemeMode.system => l10n.auto,
      };
}
