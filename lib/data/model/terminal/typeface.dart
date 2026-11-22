import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// The terminal fonts available without a download or a device-local file.
enum TerminalTypeface {
  hack('Hack Nerd Font', 'Hack-Nerd-Font'),
  jetBrainsMono('JetBrains Mono', 'JetBrains-Mono'),
  firaCode('Fira Code', 'Fira-Code'),
  iosevka('Iosevka', 'Iosevka'),
  cascadiaCode('Cascadia Code', 'Cascadia-Code');

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
