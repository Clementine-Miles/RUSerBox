---
title: Building
description: Build Server Box for Android and build the Monitor service
---

Build the Flutter App with the repository's `fl_build` tool. The Monitor
is a separate Rust service and has its own build steps.

## Prerequisites

- Flutter SDK on the stable channel
- Android SDK, NDK, JDK 21, and GNU awk (`gawk`) for building PRoot
- Rust toolchain: the App builds `crates/sbm_ffi` using a Dart build hook, `flutter_rust_bridge_hooks`, and native assets

Initialize the Git submodules before fetching Dart packages:

```bash
git submodule update --init
```

## Run a development build

```bash
# Run on the default device
flutter run

# Select a device explicitly
flutter run -d <device-id>
```

## Release build

```bash
make build-android
```

Requires the Android SDK, a JDK, and the 1Password CLI: release builds are
signed with the key that `scripts/release/android-signing.sh` reads from
1Password. For local
verification only, pass `-PallowDebugReleaseSigning=true` explicitly to use
debug signing. Reproducible and F-Droid builds use
`-PallowUnsignedRelease=true`, which omits signing configuration;
`scripts/release/android-build-env.sh` exports this option.

## Build the Monitor

The Monitor is a standalone server binary. Build it separately from the
Flutter App:

```bash
# From the repository root
cargo build -p server_box_monitor --release

```

Run `make monitor-dev` to start the API on port 3770.

The `monitor-release.yml` workflow builds release artifacts and currently
runs only through `workflow_dispatch`. Monitor `monitor-v*` tags are separate
from App releases. For a Docker build, see `monitor/Dockerfile`.

## Build hooks

On every build, `fl_build` derives the build number from Git history and
regenerates `lib/data/res/build_data.dart`. Set the App name in the `fl_build:` section of `pubspec.yaml`.

## Troubleshooting

### Clean build

```bash
flutter clean
flutter pub get
dart run build_runner build
```

### Dependency version conflict

Check package compatibility before upgrading dependencies:

```bash
flutter pub upgrade
```

After changing an annotated model, run code generation as well; see [Code Generation](/docs/development/codegen/).

## Release checklist

1. Update the version in `pubspec.yaml`.
2. Run code generation and localization generation.
3. Run Dart, Flutter, and Rust tests.
4. Build the Android APKs.
5. Verify key features on real devices.
6. Create the GitHub release.
