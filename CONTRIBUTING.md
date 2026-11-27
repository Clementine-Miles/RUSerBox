# Contributing

- Read [docs/BUILDING.md](docs/BUILDING.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- Server commands and their parsers belong in Rust (`crates/sbm_parser`), exposed
  through `crates/sbm_ffi`; Dart never builds a command or parses output.
- A storage change follows [docs/STORAGE.md](docs/STORAGE.md): a schema step,
  `SchemaVersion.current`, `kSchemaMigrations`, and a permanent test.
- Generated code (`lib/src/rust`, `*.g.dart`, `*.freezed.dart`,
  `lib/generated/l10n`, `lib/data/res/gruvbox_palettes.g.dart`) is regenerated,
  never edited.
- UI strings go in `lib/l10n/app_en.arb` (and the other locales), then
  `flutter gen-l10n`.
- Before a pull request: `flutter analyze lib test`, `flutter test`,
  `cargo test --workspace`, and for Android changes
  `(cd android && ./gradlew :app:testDebugUnitTest -x :app:compileFlutterBuildDebug)`.
