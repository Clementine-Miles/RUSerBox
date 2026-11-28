[English](README.md) | 简体中文

<h2 align="center">RUSerBox</h2>

<p align="center">
用 Rust 编写的 Android 服务器管理工具：Linux、Unix、Windows 服务器状态图表、SSH 终端、文件与运维工具，使用 Vulkan 原生绘制。
<br>
<a href="https://github.com/lollipopkit/flutter_server_box">Flutter Server Box</a> 的 Rust 重写版本。
</p>

## 安装

从 [GitHub Releases](https://github.com/Clementine-Miles/RUSerBox/releases) 下载 Android APK（arm64-v8a，Android 9+，Vulkan 1.1）。它可以覆盖安装 Flutter Server Box 1.0.x，并保留其中的服务器、密钥、代码片段和设置。

## 功能

- **连接**：SSH 直连、跳板机或 `ProxyCommand`，可设置备用地址；主机密钥始终校验，新的或变化的密钥会先交给你确认。也可以通过 [Monitor](monitor/) 的 HTTPS（固定证书或 Web PKI）连接，同一台服务器可同时使用两种方式并自动回退。
- **监控**：CPU（每核）、内存、交换、磁盘、网络、温度和 GPU 的实时图表；由 Monitor 提供数据的桌面小部件。
- **操作**：兼容 xterm 的终端（备用屏幕、256 色与真彩色、鼠标上报、括号粘贴、中日韩与宽字符、Nerd Font 图标、回滚、选择复制、输入法组合输入、硬件键盘和屏幕按键栏），SSH 或 Monitor 终端会话；Zellij 会话管理；SFTP（或 Monitor 文件 API）文件管理；进程、Docker/Podman 容器、systemd 服务、电源操作、代码片段、私钥（导入或生成 Ed25519）。
- **外观**：Gruvbox Material 与 Gruvbox，浅色和深色；五种内置终端字体；17 种语言。

**Rust 版本尚未提供**（Flutter Server Box 1.0.x 中存在）：远程桌面页面（RDP/VNC 客户端保留在 `crates/sbm_remote_desktop`）、虚拟化、BMC、防火墙、用户、计划任务、存储管理、共享、基准测试、端口转发管理、分屏工作区、地球仪、备份与同步、生物识别锁。已永久移除：AI Agent 和设备本机终端（请使用 Termux）。

## 构建

```sh
make test                 # Rust workspace 测试
make build-android-debug  # 需要 Android SDK/NDK、cargo-ndk 和 Gradle 9.8
make monitor-dev          # 在 :3770 运行 Monitor API
```

结构说明见 [CLAUDE.md](CLAUDE.md)，详细内容见[开发文档](docs/src/content/docs/zh/development/)。不需要 Dart 或 Flutter SDK。

## 许可证

`AGPL v3 lollipopkit & all contributors`
