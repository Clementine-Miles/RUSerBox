import 'package:material_ui/material_ui.dart';

enum Gruvbox {
  material('Gruvbox Material'),
  original('Gruvbox');

  const Gruvbox(this.label);

  final String label;

  static Gruvbox fromSetting(Object? value) =>
      value == original.name ? original : material;

  GruvboxPalette palette(Brightness brightness) => switch ((this, brightness)) {
    (material, Brightness.dark) => const GruvboxPalette(
      background: 0xFF282828,
      surface: 0xFF32302F,
      raised: 0xFF45403D,
      foreground: 0xFFD4BE98,
      muted: 0xFFA89984,
      red: 0xFFEA6962,
      green: 0xFFA9B665,
      yellow: 0xFFD8A657,
      blue: 0xFF7DAEA3,
      purple: 0xFFD3869B,
      aqua: 0xFF89B482,
    ),
    (material, Brightness.light) => const GruvboxPalette(
      background: 0xFFFBF1C7,
      surface: 0xFFF2E5BC,
      raised: 0xFFE6D5AE,
      foreground: 0xFF654735,
      muted: 0xFF7C6F64,
      red: 0xFFC14A4A,
      green: 0xFF6C782E,
      yellow: 0xFFA96B2C,
      blue: 0xFF45707A,
      purple: 0xFF945E80,
      aqua: 0xFF4C7A5D,
    ),
    (original, Brightness.dark) => const GruvboxPalette(
      background: 0xFF282828,
      surface: 0xFF3C3836,
      raised: 0xFF504945,
      foreground: 0xFFEBDBB2,
      muted: 0xFFA89984,
      red: 0xFFFB4934,
      green: 0xFFB8BB26,
      yellow: 0xFFFABD2F,
      blue: 0xFF83A598,
      purple: 0xFFD3869B,
      aqua: 0xFF8EC07C,
      ansi: [
        0xFFCC241D,
        0xFF98971A,
        0xFFD79921,
        0xFF458588,
        0xFFB16286,
        0xFF689D6A,
      ],
    ),
    (original, Brightness.light) => const GruvboxPalette(
      background: 0xFFFBF1C7,
      surface: 0xFFEBDBB2,
      raised: 0xFFD5C4A1,
      foreground: 0xFF3C3836,
      muted: 0xFF665C54,
      red: 0xFF9D0006,
      green: 0xFF79740E,
      yellow: 0xFFB57614,
      blue: 0xFF076678,
      purple: 0xFF8F3F71,
      aqua: 0xFF427B58,
      ansi: [
        0xFFCC241D,
        0xFF98971A,
        0xFFD79921,
        0xFF458588,
        0xFFB16286,
        0xFF689D6A,
      ],
    ),
  };
}

final class GruvboxPalette {
  const GruvboxPalette({
    required this.background,
    required this.surface,
    required this.raised,
    required this.foreground,
    required this.muted,
    required this.red,
    required this.green,
    required this.yellow,
    required this.blue,
    required this.purple,
    required this.aqua,
    this.ansi,
  });

  final int background, surface, raised, foreground, muted;
  final int red, green, yellow, blue, purple, aqua;

  /// Original Gruvbox has a second set of ANSI colors.
  final List<int>? ansi;
}
