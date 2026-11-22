import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/app_theme.dart';
import 'package:server_box/data/res/gruvbox.dart';

void main() {
  test('each Gruvbox family defines a readable light and dark scheme', () {
    for (final family in Gruvbox.values) {
      for (final brightness in Brightness.values) {
        final scheme = buildAppTheme(family, brightness).colorScheme;
        expect(scheme.brightness, brightness);
        expect(scheme.surface, Color(family.palette(brightness).background));
        final luminances = [
          scheme.surface.computeLuminance(),
          scheme.onSurface.computeLuminance(),
        ]..sort();
        expect(
          (luminances.last + .05) / (luminances.first + .05),
          greaterThan(4.5),
        );
      }
    }
  });

  test(
    'legacy appearance settings resolve to a supported palette and mode',
    () {
      expect(Gruvbox.fromSetting('amoled'), Gruvbox.material);
      expect(Gruvbox.fromSetting('original'), Gruvbox.original);
      expect(appThemeMode(1), ThemeMode.light);
      expect(appThemeMode(2), ThemeMode.dark);
      expect(appThemeMode(3), ThemeMode.dark);
      expect(appThemeMode(4), ThemeMode.system);
    },
  );
}
