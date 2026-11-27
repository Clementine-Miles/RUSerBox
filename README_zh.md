[English](README.md) | 简体中文

# RUSerBox

面向 Android（arm64-v8a）的 Linux 服务器管理应用：SSH 与 SFTP、带 Zellij
持久会话的完整终端、本机 Shell 与 PRoot Linux 环境，以及实时服务器监控。
界面由 Flutter 绘制；服务器命令、解析与原生逻辑由 Rust 负责；Kotlin 只做
必要的 Android 集成。

RUSerBox 由 [ServerBox](https://github.com/lollipopkit/flutter_server_box)
精简而来：没有 AI/MCP 功能、桌面/iOS/Web 版本、Windows 服务器后端、
RDP/VNC 查看器、桌面小部件和主题商店。

- 安装：从 [Releases](https://github.com/Clementine-Miles/RUSerBox/releases)
  下载 arm64 APK。应用 ID、数据与签名保持不变，可直接覆盖升级。
- 要求：64 位 ARM（arm64-v8a，Armv8/Armv9）设备，Android 7.0 及以上。
- 主题：内置 Gruvbox 与 Gruvbox Material（硬/中/软对比度），绿色或蓝色强调色，
  浅色/深色/跟随系统。终端字体：Hack 或 JetBrainsMono Nerd Font。

文档见 [docs/](docs/)。许可证：AGPL v3。
