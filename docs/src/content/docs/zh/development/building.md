---
title: 构建
description: 为 Android（arm64-v8a）构建 RUSerBox，以及构建 Monitor
---

## 需要

- `rust-toolchain.toml` 中的 Rust 工具链（rustup 会连同 `aarch64-linux-android` 目标一起安装）。
- `android/compile-sdk-version.txt`、`build-tools-version.txt`、`ndk-version.txt` 指定版本的 Android SDK、构建工具和 NDK，并设置 `ANDROID_HOME`。
- `cargo install cargo-ndk`、Gradle 9.8 和 JDK 17 或更新版本。

不需要 Dart 或 Flutter SDK。

## Android

```sh
make android-check          # 为 aarch64-linux-android 执行 cargo check
make build-android-debug    # android/app/build/outputs/apk/debug/
make build-android          # 发布版，通过 scripts/release/android-signing.sh 签名
scripts/release/inspect-apk.sh android/app/build/outputs/apk/release/app-release.apk
```

Gradle 的 `cargoBuild` 任务运行 `cargo ndk -t arm64-v8a`，并把 `libruserbox.so` 作为唯一的原生库打包。APK 只面向 arm64-v8a，需要 Android 9（API 28）和 Vulkan 1.1，原生库不压缩存储并按 16 KiB 对齐。没有发布密钥时，本地检查发布版可以加上 `-PallowDebugReleaseSigning=true`。

## 开发窗口

`make run-desktop` 在桌面窗口（Linux、Vulkan）中运行同一界面，数据位于 `./ruserbox-data`。它只是开发工具，不是发布目标：其数据库密钥未经封装。

## Monitor

```sh
make monitor-dev
cargo build -p server_box_monitor --release
```

Monitor 仍保留 Linux（x86_64、arm64）、macOS 和 Windows 构建（`.github/workflows/monitor-release.yml`）。
