---
title: Testing
description: How RUSerBox is tested
---

```sh
make test         # cargo test --workspace
make lint         # clippy, warnings denied
make test-render  # the app drawn through Vulkan (lavapipe without a GPU)
gradle -p android :app:testDebugUnitTest   # the widgets' JVM tests
```

- `crates/sbm_ssh/tests/sshd_e2e.rs` starts a throwaway OpenSSH server on
  loopback (when `/usr/sbin/sshd` exists) and covers host key accept, refusal
  and mismatch, exec, a PTY shell and resize, SFTP, the status script, a local
  forward and a jump chain.
- `crates/sbm_monitor_client/tests/monitor_e2e.rs` starts the workspace's
  Monitor binary (after `cargo build -p server_box_monitor`) and covers sign-in,
  metrics, exec and the terminal WebSocket.
- `crates/sbm_store` tests the encrypted file, key rejection and every schema
  step against a file in the previous shape.
- `app/tests/render_vulkan.rs` renders the server list, a status page, a
  terminal and the settings page offscreen through the Vulkan renderer and
  checks the pixels (`RUSERBOX_SHOTS=<dir>` writes them as PNG).
- Opt-in end-to-end tests against real machines: `SBM_E2E_SSH_HOST` (see
  `crates/sbm_parser/tests/ssh_e2e.rs`) and the RDP variables in
  `crates/sbm_remote_desktop`.
