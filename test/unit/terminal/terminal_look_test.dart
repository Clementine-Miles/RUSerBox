import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/terminal/typeface.dart';

void main() {
  test('reads the stored font and falls back to Hack', () {
    for (final font in TerminalTypeface.values) {
      expect(TerminalTypeface.fromStored(font.name), font);
    }
    expect(TerminalTypeface.fromStored(null), TerminalTypeface.hack);
    expect(TerminalTypeface.fromStored('retired'), TerminalTypeface.hack);
  });
}
