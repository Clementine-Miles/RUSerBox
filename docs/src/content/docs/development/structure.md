---
title: Project Structure
description: Where things are in the RUSerBox repository
---

| Path | Contents |
|---|---|
| `app/` | The Android app (`ruserbox`). `src/core.rs`, `src/ui/`, `src/term/`, `src/gfx.rs`, `src/platform/`; `l10n/` holds the interface strings; `tests/render_vulkan.rs` draws real pages through Vulkan. |
| `crates/sbm_parser` | Commands, the status script and parsers, shared with the Monitor. |
| `crates/sbm_ssh` | The SSH client (russh). |
| `crates/sbm_store` | The encrypted store and its schema. |
| `crates/sbm_monitor_client` | The Monitor API client. |
| `crates/sbm_redfish`, `crates/sbm_virt`, `crates/sbm_remote_desktop` | BMC, virtualization, RDP/VNC. |
| `crates/sbm_native`, `crates/fl_lkc` | The Monitor's local sampler and LK Cloud client. |
| `monitor/` | The Monitor agent. |
| `android/` | Gradle packaging, the Java adapter and the Kotlin widgets. |
| `assets/` | Bundled terminal fonts, the benchmark script. |
| `third_party/ironrdp` | IronRDP fork (submodule). |
| `scripts/` | Release, deployment and end-to-end helpers. |
| `docs/`, `website/`, `packages/webui` | This site, the project website, shared Svelte components. |
