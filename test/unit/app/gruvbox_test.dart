import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/app_theme.dart';
import 'package:server_box/data/res/gruvbox.dart';

double _contrast(Color a, Color b) {
  final l = [a.computeLuminance(), b.computeLuminance()]..sort();
  return (l.last + .05) / (l.first + .05);
}

void main() {
  test('exactly eight variants: two classic, six Material', () {
    final all = GruvboxTheme.all;
    expect(all, hasLength(8));
    expect(all.toSet(), hasLength(8));
    expect(all.where((t) => t.family == GruvboxFamily.gruvbox), hasLength(2));
    expect(
      {
        for (final t in all.where((t) => t.family == GruvboxFamily.material))
          (t.accent, t.contrast),
      },
      hasLength(6),
    );
  });

  test('every variant is readable in light and dark', () {
    for (final theme in GruvboxTheme.all) {
      for (final brightness in Brightness.values) {
        final scheme = buildAppTheme(theme, brightness).colorScheme;
        final why = '$theme $brightness';
        expect(scheme.brightness, brightness, reason: why);
        expect(_contrast(scheme.surface, scheme.onSurface), greaterThan(4.5), reason: why);
        expect(_contrast(scheme.surface, scheme.onSurfaceVariant), greaterThan(4.5), reason: why);
        expect(_contrast(scheme.surface, scheme.primary), greaterThan(4.5), reason: why);
        expect(_contrast(scheme.surface, scheme.error), greaterThan(4.5), reason: why);
        expect(_contrast(scheme.primaryContainer, scheme.onPrimaryContainer), greaterThan(4.5), reason: why);
        expect(_contrast(scheme.primary, scheme.onPrimary), greaterThan(4.5), reason: why);
      }
    }
  });

  test('the accent changes the primary; contrast changes only Material', () {
    for (final b in Brightness.values) {
      const green = GruvboxTheme(accent: GruvboxAccent.green);
      const blue = GruvboxTheme(accent: GruvboxAccent.blue);
      expect(green.scheme(b).primary, isNot(blue.scheme(b).primary));
      final hard = const GruvboxTheme(contrast: GruvboxContrast.hard).scheme(b);
      final soft = const GruvboxTheme(contrast: GruvboxContrast.soft).scheme(b);
      expect(hard.surface, isNot(soft.surface));
      // A Material contrast kept in settings does nothing to classic Gruvbox.
      const classicHard = GruvboxTheme(
        family: GruvboxFamily.gruvbox,
        contrast: GruvboxContrast.hard,
      );
      expect(classicHard.contrast, GruvboxContrast.medium);
      expect(
        classicHard.scheme(b),
        const GruvboxTheme(family: GruvboxFamily.gruvbox).scheme(b),
      );
    }
  });

  test('the seven hues all take part, by meaning', () {
    final c = const GruvboxTheme().colors(Brightness.dark);
    final hues = {c.red, c.orange, c.yellow, c.green, c.aqua, c.blue, c.purple};
    expect(hues, hasLength(7));
    expect(c.danger, c.red);
    expect(c.warning, c.yellow);
    expect(c.success, c.green);
    expect(c.memory, c.purple);
    expect(c.netUpload, c.orange);
    // CPU is the accent and never the download colour.
    for (final accent in GruvboxAccent.values) {
      final a = GruvboxTheme(accent: accent).colors(Brightness.dark);
      expect(a.cpu, isNot(a.netDownload));
    }
  });

  test('stored values resolve deterministically', () {
    expect(GruvboxFamily.fromSetting('original'), GruvboxFamily.gruvbox);
    expect(GruvboxFamily.fromSetting('material'), GruvboxFamily.material);
    // Retired values: a store theme's id, the old AMOLED preset.
    expect(GruvboxFamily.fromSetting('someone.dracula'), GruvboxFamily.material);
    expect(GruvboxFamily.fromSetting('amoled'), GruvboxFamily.material);
    expect(GruvboxAccent.fromSetting(null), GruvboxAccent.blue);
    expect(GruvboxAccent.fromSetting('green'), GruvboxAccent.green);
    expect(GruvboxContrast.fromSetting('x'), GruvboxContrast.medium);
    expect(appThemeMode(1), ThemeMode.light);
    expect(appThemeMode(2), ThemeMode.dark);
    expect(appThemeMode(3), ThemeMode.dark);
    expect(appThemeMode(4), ThemeMode.system);
  });
}
