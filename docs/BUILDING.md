# Building

## Toolchain (verified)

| Tool | Version |
| --- | --- |
| Flutter / Dart | 3.50.1 / 3.14.0 (`.github/actions/setup-flutter`) |
| JDK running Gradle | Temurin 27 (JDK 25 LTS also works); Java/Kotlin bytecode target 17 |
| Gradle / AGP / Kotlin | 9.8.0 / 9.4.1 / 2.3.21 |
| Android SDK | compile 36, build-tools 36.1.0, minSdk 24 (Flutter default) |
| NDK | 28.2.13676358 (`android/ndk-version.txt`) |
| Rust | 1.98.1 (`crates/sbm_ffi/rust-toolchain.toml`), targets `aarch64-linux-android`, `x86_64-unknown-linux-gnu` (tests) |
| Host tools | GNU awk (proot's build), Python 3 (palette generator) |

Point Flutter at the JDK with `flutter config --jdk-dir <jdk>` and check with
`flutter doctor -v`; Flutter does not necessarily use `JAVA_HOME`.

## Build

```sh
git submodule update --init          # packages/dartssh2, packages/xterm
flutter pub get
make build-android                   # scripts/release/build-android.sh
```

`build-android.sh` builds the arm64-v8a release APK
(`build/app/outputs/flutter-apk/<name>_v1.0.<build>_arm64.apk` and `.sha256`)
and runs `scripts/release/verify-apk.sh`. It signs with the release key from
1Password; for a local check set `ORG_GRADLE_PROJECT_allowDebugReleaseSigning=true`
(debug key) or `ORG_GRADLE_PROJECT_allowUnsignedRelease=true`. The build number
is `BuildData.build` (`lib/data/res/build_data.dart`), bumped with
`pubspec.yaml`'s version for a release; the version code is `build * 100 + 3`.

Any Android ABI other than arm64-v8a is refused (`hook/build.dart`), so an
x86_64 emulator cannot be targeted; use an arm64 device or emulator image.

Reproducible/F-Droid build: `scripts/release/prepare-fdroid.sh` then
`FDROID_OFFLINE=true scripts/release/build-fdroid.sh` (unsigned, offline);
`.github/workflows/android-reproducible.yml` builds twice from different paths
and compares.

## Test

```sh
flutter analyze lib test
flutter test --timeout 60s
cargo test --workspace
(cd android && ./gradlew :app:testDebugUnitTest -x :app:compileFlutterBuildDebug)
```

`flutter test` builds the Rust and C libraries for the Linux host into
`build/native_assets/linux/`. Opt-in end-to-end tests (real SSH hosts,
libvirt, PVE, WebDAV) are skipped unless their environment variables are set;
see the headers of `test/e2e/*` and `crates/*/tests/*_e2e.rs`.

## Code generation

```sh
flutter_rust_bridge_codegen generate   # after changing crates/sbm_ffi/src/api
dart run build_runner build            # freezed / json / riverpod / drift
flutter gen-l10n                       # after editing lib/l10n/*.arb
python3 tools/gen_gruvbox.py <ServerBox_Gruvbox_Theme checkout at the pinned revision>
```
