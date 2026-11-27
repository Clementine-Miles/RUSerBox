import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// The two bundled terminal fonts, both Nerd Fonts "Mono" builds (icons one
/// cell wide). Hack is the default. JetBrains Mono is its no-ligature (NL)
/// build, so ligatures never merge cells.
///
/// Stored by [name]. `jetBrainsMono` keeps the identifier earlier builds wrote
/// (then the unpatched face); any other stored font — Fira Code, Iosevka,
/// Cascadia Code, all retired — reads as [hack].
enum TerminalTypeface {
  hack('Hack Nerd Font', 'Hack-Nerd-Font'),
  jetBrainsMono('JetBrainsMono Nerd Font', 'JetBrainsMono-Nerd-Font');

  const TerminalTypeface(this.family, this._licenseName);

  final String family;
  final String _licenseName;

  static TerminalTypeface fromStored(Object? value) =>
      values.firstWhere((font) => font.name == value, orElse: () => hack);

  /// Makes the bundled fonts' notices available in Settings → About → License.
  static void registerLicenses() {
    LicenseRegistry.addLicense(() async* {
      for (final font in values) {
        final notice = await rootBundle.loadString(
          'assets/fonts/${font._licenseName}-LICENSE.txt',
        );
        yield LicenseEntryWithLineBreaks([font.family], notice);
      }
    });
  }
}
