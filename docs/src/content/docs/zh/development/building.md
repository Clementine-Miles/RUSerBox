---
title: 构建指南
description: 构建 Android 版 Server Box 和 Monitor
---

Flutter App 使用仓库提供的 `fl_build` 工具构建。Monitor 是独立的 Rust 服务，需要单独构建。

## 前置条件

- stable channel 的 Flutter SDK
- Android SDK、NDK 和 JDK
- Rust toolchain：App 通过 Dart build hook、`flutter_rust_bridge_hooks` 和 native assets 构建 `crates/sbm_ffi`

获取 Dart packages 前先初始化 Git submodule：

```bash
git submodule update --init --recursive
```

## 运行开发版

```bash
# 在默认设备上运行
flutter run

# 明确指定设备
flutter run -d <device-id>
```

## 发布版构建

```bash
make build-android
```

需要 Android SDK、JDK 和 1Password CLI：release build 使用 `scripts/release/android-signing.sh` 从 1Password 读取的签名 key。仅本地验证时，才显式传入 `-PallowDebugReleaseSigning=true` 使用 debug signing。可重现构建和 F-Droid 构建使用 `-PallowUnsignedRelease=true`，不配置任何签名；`scripts/release/android-build-env.sh` 会导出此选项。

## 构建 Monitor

Monitor 是独立的 server binary，构建步骤与 Flutter App 分开：

```bash
# 在仓库根目录执行
cargo build -p server_box_monitor --release
```

运行 `make monitor-dev` 可在 3770 端口启动 Monitor API。

release 产物由 `monitor-release.yml` workflow 构建，目前只能通过 `workflow_dispatch` 运行。Monitor 的 `monitor-v*` tag 与 App release 分开管理。Docker 构建方法见 `monitor/Dockerfile`。

## 构建前后处理

每次构建时，`fl_build` 都会重新生成 `lib/data/res/build_data.dart`，根据 Git 历史计算 build number。App 名称在 `pubspec.yaml` 的 `fl_build:` 段中设置。

## 故障排除

### Clean build

```bash
flutter clean
flutter pub get
dart run build_runner build
```

### 依赖版本不匹配

升级依赖前先确认版本兼容：

```bash
flutter pub upgrade
```

修改带有注解的 model 后，还需要运行代码生成；详见[代码生成](/docs/zh/development/codegen/)。

## 发布清单

1. 更新 `pubspec.yaml` 中的版本号。
2. 运行代码生成和本地化生成。
3. 运行 Dart、Flutter 和 Rust 测试。
4. 构建 Android APK。
5. 在真实设备上验证关键功能。
6. 创建 GitHub Release。
