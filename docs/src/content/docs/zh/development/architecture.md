---
title: 实现架构
description: RUSerBox 的实现方式：Android 上的原生 Rust、Vulkan 渲染和共享的服务器层
---

RUSerBox 是一个 Rust workspace。Android 应用和 Monitor agent 共享与服务器通信的那一层，各自拥有自己的状态和界面。

- `app/`：Android 应用。`platform/android.rs` 是 GameActivity 主循环（触摸、按键、输入法、surface 生命周期）；`gfx.rs` 用 wgpu 绘制 egui，**只编译 Vulkan 后端**；`ui/` 是各页面；`term/` 是终端仿真（alacritty_terminal）、按键编码与绘制；`core.rs` 管理每台服务器的连接、状态轮询和主机密钥确认。
- `crates/sbm_ssh`：基于 russh 的 SSH 客户端（exec、PTY、SFTP、跳板机、端口转发）。
- `crates/sbm_monitor_client`：Monitor 的 HTTP 与终端 WebSocket 客户端，TLS 使用 Web PKI 或固定的叶证书。
- `crates/sbm_store`：加密的 `store.db`（SQLite3 Multiple Ciphers）。
- `crates/sbm_parser`：所有发往服务器的命令、状态脚本和解析器，与 Monitor 共用。

## 渲染

界面是 egui，由 wgpu 的 Vulkan 后端绘制。实例只为 `Backends::VULKAN` 创建，并检查选中的适配器确实是 Vulkan；没有可用 Vulkan 驱动的设备会看到明确的错误，而不会退回到其他渲染方式。只有在需要时（输入、后台任务、动画）才绘制新帧，并以 FIFO 呈现节奏控制。`InitWindow` 时创建 surface，`TerminateWindow` 时释放；设备会在 surface 之外继续存在，丢失后会重建。

## Android 集成

`MainActivity`（Java）继承 GameActivity，不包含应用逻辑：Rust 通过 JNI 调用它的方法完成剪贴板、文件选择与保存、窗口 insets、用 Android Keystore 封装数据库密钥、迁移 Flutter 版本保存的密钥、后台保持会话的前台服务，以及向桌面小部件发布服务器列表。小部件本身（AppWidget/RemoteViews）仍是 Kotlin。

文字输入使用 GameActivity 的 GameTextInput：输入法的组合区域在终端光标处显示为预编辑文本，提交的文字只发送一次。

## 连接与存储

服务器可以通过 SSH、Monitor 或两者访问。SSH 主机密钥始终校验：未知或变化的密钥会交给用户确认，只有接受后才会保存。数据保存在 Flutter 版本使用的同一目录中的加密 `store.db`，更新后服务器、密钥、代码片段和设置都会保留；无法读取的版本会被拒绝而不是重置。
