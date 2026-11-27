---
title: Building
description: Build RUSerBox for Android (arm64-v8a) and the Monitor
---

## Requirements

- Rust: the toolchain in `rust-toolchain.toml` (rustup installs it, with the
  `aarch64-linux-android` target).
- Android SDK, build tools and NDK at the versions in
  `android/compile-sdk-version.txt`, `build-tools-version.txt` and
  `ndk-version.txt`; `ANDROID_HOME` set.
- `cargo install cargo-ndk`, Gradle 9.8 and JDK 17 or newer.

No Dart or Flutter SDK is involved.

## Android

```sh
make android-check          # cargo check for aarch64-linux-android
make build-android-debug    # android/app/build/outputs/apk/debug/
make build-android          # release, signed through scripts/release/android-signing.sh
scripts/release/inspect-apk.sh android/app/build/outputs/apk/release/app-release.apk
```

Gradle's `cargoBuild` task runs `cargo ndk -t arm64-v8a` and packages
`libruserbox.so` as the only native library. The APK targets arm64-v8a only,
requires Android 9 (API 28) and Vulkan 1.1, and stores the library uncompressed
with 16 KiB-aligned segments. For a local release check without the release
key, add `-PallowDebugReleaseSigning=true`.

## Development window

`make run-desktop` runs the same interface in a desktop window (Linux,
Vulkan), with data in `./ruserbox-data`. It is a development aid, not a
shipped target: its store key is not sealed.

## Monitor

```sh
make monitor-dev
cargo build -p server_box_monitor --release
```

The Monitor keeps its own Linux (x86_64, arm64), macOS and Windows builds
(`.github/workflows/monitor-release.yml`).
