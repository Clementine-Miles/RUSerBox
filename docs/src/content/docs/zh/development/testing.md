---
title: 测试
description: RUSerBox 的测试方式
---

```sh
make test         # cargo test --workspace
make lint         # clippy，警告视为错误
make test-render  # 通过 Vulkan 绘制应用（没有 GPU 时使用 lavapipe）
gradle -p android :app:testDebugUnitTest   # 小部件的 JVM 测试
```

- `crates/sbm_ssh/tests/sshd_e2e.rs` 在本机回环地址上启动临时 OpenSSH 服务器（存在 `/usr/sbin/sshd` 时），覆盖主机密钥的接受、拒绝和不匹配、exec、PTY 与调整大小、SFTP、状态脚本、本地转发和跳板机。
- `crates/sbm_monitor_client/tests/monitor_e2e.rs` 启动 workspace 中的 Monitor（先执行 `cargo build -p server_box_monitor`），覆盖登录、指标、exec 和终端 WebSocket。
- `crates/sbm_store` 测试加密文件、错误密钥，以及每个 schema 步骤。
- `app/tests/render_vulkan.rs` 通过 Vulkan 离屏绘制服务器列表、状态页、终端和设置页并检查像素（设置 `RUSERBOX_SHOTS=<目录>` 可输出 PNG）。
- 针对真实机器的可选端到端测试：`SBM_E2E_SSH_HOST`（见 `crates/sbm_parser/tests/ssh_e2e.rs`）以及 `crates/sbm_remote_desktop` 中的 RDP 变量。
