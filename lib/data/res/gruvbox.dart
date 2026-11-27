import 'package:flutter/foundation.dart' show LicenseEntryWithLineBreaks, LicenseRegistry;
import 'package:flutter/services.dart' show rootBundle;
import 'package:material_ui/material_ui.dart';

part 'gruvbox_palettes.g.dart';

/// The two built-in theme families. Stored by [name] under `appThemePreset`:
/// `original` and `material`, the identifiers earlier builds already wrote.
enum GruvboxFamily {
  gruvbox('original', 'Gruvbox'),
  material('material', 'Gruvbox Material');

  const GruvboxFamily(this.id, this.label);

  /// The stored identifier.
  final String id;
  final String label;

  /// Whether a contrast level can be chosen: Gruvbox Material only.
  bool get hasContrast => this == material;

  /// Any unknown or retired value (a downloaded theme's id, an old preset)
  /// falls back to Gruvbox Material, the default.
  static GruvboxFamily fromSetting(Object? value) =>
      value == gruvbox.id ? gruvbox : material;
}

/// The selectable primary accents. Every other Gruvbox hue keeps its own
/// semantic role (see [GruvboxColors]).
enum GruvboxAccent {
  green,
  blue;

  static GruvboxAccent fromSetting(Object? value) =>
      value == green.name ? green : blue;
}

/// Gruvbox Material's background contrast. Classic Gruvbox has Medium only.
enum GruvboxContrast {
  medium,
  hard,
  soft;

  static GruvboxContrast fromSetting(Object? value) => switch (value) {
    'hard' => hard,
    'soft' => soft,
    _ => medium,
  };
}

/// One of the eight built-in variants.
///
/// A Material contrast chosen earlier is kept in settings when the family is
/// switched to classic Gruvbox, and simply not applied there ([contrast]).
final class GruvboxTheme {
  const GruvboxTheme({
    this.family = GruvboxFamily.material,
    this.accent = GruvboxAccent.blue,
    GruvboxContrast contrast = GruvboxContrast.medium,
  }) : _contrast = contrast;

  final GruvboxFamily family;
  final GruvboxAccent accent;
  final GruvboxContrast _contrast;

  /// The contrast in effect: always Medium for classic Gruvbox.
  GruvboxContrast get contrast =>
      family.hasContrast ? _contrast : GruvboxContrast.medium;

  /// Every variant there is: 2 classic, 6 Material.
  static List<GruvboxTheme> get all => [
    for (final family in GruvboxFamily.values)
      for (final contrast in family.hasContrast
          ? GruvboxContrast.values
          : const [GruvboxContrast.medium])
        for (final accent in GruvboxAccent.values)
          GruvboxTheme(family: family, accent: accent, contrast: contrast),
  ];

  ColorScheme scheme(Brightness brightness) {
    final v = _schemes[(family, contrast, accent, brightness)]!;
    Color r(String role) => Color(v[_roleOrder.indexOf(role)]);
    return ColorScheme(
      brightness: brightness,
      primary: r('primary'),
      onPrimary: r('onPrimary'),
      primaryContainer: r('primaryContainer'),
      onPrimaryContainer: r('onPrimaryContainer'),
      primaryFixed: r('primaryFixed'),
      primaryFixedDim: r('primaryFixedDim'),
      onPrimaryFixed: r('onPrimaryFixed'),
      onPrimaryFixedVariant: r('onPrimaryFixedVariant'),
      secondary: r('secondary'),
      onSecondary: r('onSecondary'),
      secondaryContainer: r('secondaryContainer'),
      onSecondaryContainer: r('onSecondaryContainer'),
      secondaryFixed: r('secondaryFixed'),
      secondaryFixedDim: r('secondaryFixedDim'),
      onSecondaryFixed: r('onSecondaryFixed'),
      onSecondaryFixedVariant: r('onSecondaryFixedVariant'),
      tertiary: r('tertiary'),
      onTertiary: r('onTertiary'),
      tertiaryContainer: r('tertiaryContainer'),
      onTertiaryContainer: r('onTertiaryContainer'),
      tertiaryFixed: r('tertiaryFixed'),
      tertiaryFixedDim: r('tertiaryFixedDim'),
      onTertiaryFixed: r('onTertiaryFixed'),
      onTertiaryFixedVariant: r('onTertiaryFixedVariant'),
      error: r('error'),
      onError: r('onError'),
      errorContainer: r('errorContainer'),
      onErrorContainer: r('onErrorContainer'),
      surface: r('surface'),
      onSurface: r('onSurface'),
      surfaceDim: r('surfaceDim'),
      surfaceBright: r('surfaceBright'),
      surfaceContainerLowest: r('surfaceContainerLowest'),
      surfaceContainerLow: r('surfaceContainerLow'),
      surfaceContainer: r('surfaceContainer'),
      surfaceContainerHigh: r('surfaceContainerHigh'),
      surfaceContainerHighest: r('surfaceContainerHighest'),
      onSurfaceVariant: r('onSurfaceVariant'),
      outline: r('outline'),
      outlineVariant: r('outlineVariant'),
      shadow: r('shadow'),
      scrim: r('scrim'),
      inverseSurface: r('inverseSurface'),
      onInverseSurface: r('onInverseSurface'),
      inversePrimary: r('inversePrimary'),
      surfaceTint: Colors.transparent,
    );
  }

  /// The palettes' notices in Settings → About → License.
  static void registerLicenses() {
    LicenseRegistry.addLicense(() async* {
      yield LicenseEntryWithLineBreaks(
        const ['Gruvbox', 'Gruvbox Material'],
        await rootBundle.loadString('assets/licenses/Gruvbox-NOTICE.txt'),
      );
    });
  }

  GruvboxColors colors(Brightness brightness) =>
      GruvboxColors._(this, brightness);

  @override
  bool operator ==(Object other) =>
      other is GruvboxTheme &&
      other.family == family &&
      other.accent == accent &&
      other.contrast == contrast;

  @override
  int get hashCode => Object.hash(family, accent, contrast);

  @override
  String toString() =>
      'GruvboxTheme(${family.name}, ${accent.name}, ${contrast.name})';
}

/// The whole Gruvbox palette of one variant and brightness, by meaning.
///
/// The seven hues are the family's canonical anchors (readable as
/// indicators and chart marks on every surface of the family); text that has
/// to meet contrast on surfaces uses the [ColorScheme]'s corrected roles.
final class GruvboxColors {
  GruvboxColors._(this.theme, this.brightness)
    : _a = _anchors[(theme.family, brightness)]!,
      _scheme = theme.scheme(brightness);

  final GruvboxTheme theme;
  final Brightness brightness;
  final List<int> _a;
  final ColorScheme _scheme;

  Color get red => Color(_a[0]);
  Color get orange => Color(_a[1]);
  Color get yellow => Color(_a[2]);
  Color get green => Color(_a[3]);
  Color get aqua => Color(_a[4]);
  Color get blue => Color(_a[5]);
  Color get purple => Color(_a[6]);

  /// The chosen accent as a mark (chart line, indicator).
  Color get accent => theme.accent == GruvboxAccent.green ? green : blue;

  /// Gruvbox gray.
  Color get gray => brightness == Brightness.dark
      ? const Color(0xFFA89984)
      : const Color(0xFF7C6F64);

  Color get background => _scheme.surface;
  Color get foreground => _scheme.onSurface;
  Color get selection => _scheme.primaryContainer;

  // Semantic roles.
  Color get success => green;
  Color get warning => yellow;
  Color get danger => red;
  Color get idle => gray;
  Color get disconnected => gray;
  Color get cpu => accent;
  Color get memory => purple;
  Color get diskRead => aqua;
  Color get diskWrite => yellow;
  Color get netDownload => theme.accent == GruvboxAccent.green ? blue : green;
  Color get netUpload => orange;
}
