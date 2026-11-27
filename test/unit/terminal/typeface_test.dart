import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/terminal/typeface.dart';

void main() {
  test('exactly two bundled families, Hack first', () {
    expect(TerminalTypeface.values, [TerminalTypeface.hack, TerminalTypeface.jetBrainsMono]);
    expect(TerminalTypeface.fromStored(null), TerminalTypeface.hack);
  });

  test('a retired font resolves to Hack; jetBrainsMono keeps its name', () {
    for (final retired in ['firaCode', 'iosevka', 'cascadiaCode', 'whatever', 3]) {
      expect(TerminalTypeface.fromStored(retired), TerminalTypeface.hack, reason: '$retired');
    }
    expect(TerminalTypeface.fromStored('jetBrainsMono'), TerminalTypeface.jetBrainsMono);
  });

  test('the bundled files are real fonts, not LFS pointers', () {
    for (final f in [
      'assets/fonts/HackNerdFontMono-Regular.ttf',
      'assets/fonts/JetBrainsMonoNLNerdFontMono-Regular.ttf',
    ]) {
      final bytes = File(f).readAsBytesSync();
      expect(bytes.length, greaterThan(1 << 20), reason: f);
      // TrueType: 0x00010000.
      expect(bytes.sublist(0, 4), [0, 1, 0, 0], reason: f);
    }
  });
}
