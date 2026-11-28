---
title: 项目结构
description: RUSerBox 仓库中各部分的位置
---

| 路径 | 内容 |
|---|---|
| `app/` | Android 应用（`ruserbox`）：`src/core.rs`、`src/ui/`、`src/term/`、`src/gfx.rs`、`src/platform/`；`l10n/` 是界面文字；`tests/render_vulkan.rs` 通过 Vulkan 绘制真实页面。 |
| `crates/sbm_parser` | 命令、状态脚本和解析器，与 Monitor 共用。 |
| `crates/sbm_ssh` | SSH 客户端（russh）。 |
| `crates/sbm_store` | 加密存储及其 schema。 |
| `crates/sbm_monitor_client` | Monitor API 客户端。 |
| `crates/sbm_redfish`、`crates/sbm_virt`、`crates/sbm_remote_desktop` | BMC、虚拟化、RDP/VNC。 |
| `crates/sbm_native`、`crates/fl_lkc` | Monitor 的本地采样与 LK Cloud 客户端。 |
| `monitor/` | Monitor agent。 |
| `android/` | Gradle 打包、Java 适配层和 Kotlin 小部件。 |
| `assets/` | 内置终端字体、基准测试脚本。 |
| `third_party/ironrdp` | IronRDP 分支（子模块）。 |
| `scripts/` | 发布、部署和端到端测试脚本。 |
| `docs/`、`website/`、`packages/webui` | 文档站、项目网站和共享的 Svelte 组件。 |
