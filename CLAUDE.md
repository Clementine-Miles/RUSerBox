# CLAUDE.md

RUSerBox manages remote servers from Android. It is a Rust workspace: the app
is native Rust drawn with Vulkan, and the Monitor is a separately deployed Rust
agent. There is no Dart or Flutter anywhere; keep it that way.

## Layout

| Path | What it is |
|---|---|
| `app/` | The Android app (`ruserbox`, a `cdylib`): `core` (connections, status polling), `ui` (egui pages), `term` (terminal emulator), `gfx` (wgpu, Vulkan only), `platform/android.rs` (GameActivity loop, input, IME), `platform/jni.rs` (calls into `MainActivity`). `--features desktop` gives a development window. |
| `crates/sbm_parser` | Every command sent to a server and the parser of its output, shared by the app and the Monitor. |
| `crates/sbm_ssh` | russh client: host key trust, jump chains, ProxyCommand, exec, PTY shells, SFTP, forwards, the status script runner. |
| `crates/sbm_store` | The encrypted `store.db` (SQLite3 Multiple Ciphers), schema-compatible with the last Flutter release (v43). |
| `crates/sbm_monitor_client` | The app's client for the Monitor API (HTTP + terminal WebSocket). |
| `crates/sbm_redfish`, `crates/sbm_virt`, `crates/sbm_remote_desktop` | BMC/Redfish and certificate pinning, libvirt/PVE, RDP/VNC sessions. |
| `crates/sbm_native` | The Monitor's local sampler. `crates/fl_lkc`: LK Cloud for the Monitor. |
| `monitor/` | The agent (see `monitor/CLAUDE.md`). |
| `android/` | Gradle packaging only: `MainActivity`, `KeyVault`, `LegacyVault`, `SessionService` (Java), the home-screen widgets (Kotlin). |
| `third_party/ironrdp` | IronRDP fork (submodule) used by `sbm_remote_desktop`. |
| `docs/`, `website/`, `packages/webui` | Documentation site, project website and its Svelte components. |

## Commands

`make help`. The usual ones: `make test`, `make lint`, `make test-render`
(Vulkan via lavapipe when there is no GPU), `make android-check`,
`make build-android-debug`, `make run-desktop`, `make monitor-dev`.

- Android builds need the SDK/NDK pinned in `android/*-version.txt`,
  `cargo-ndk`, and Gradle 9.8. `android/app/build.gradle` runs
  `cargo ndk -t arm64-v8a` itself.
- `sbm_ssh`'s e2e test starts `/usr/sbin/sshd` when present;
  `sbm_monitor_client`'s starts `target/debug/server_box_monitor` when built.

## Rules

- **Rust owns what is said to a server.** Commands and parsers live in
  `sbm_parser`; the app and the Monitor call them. New server features start
  there.
- **Host keys are always verified.** Unknown and changed keys are asked about
  (`HostTrust::ask`) and stored only on an explicit accept. Never accept a key,
  a certificate or a TLS failure on the user's behalf.
- **Vulkan only.** `gfx` compiles no other wgpu backend and checks the adapter
  is Vulkan. A device without it gets `GfxError::NoVulkan`, not a fallback.
- **Android ABI is arm64-v8a only** (`abiFilters`, `rust-toolchain.toml`,
  `inspect-apk.sh`). The Monitor keeps its x86_64/arm64/macOS/Windows builds.
- **Storage**: a schema step is appended to `sbm_store::schema::MIGRATIONS`
  with a test fed by a file in the previous shape. A file the build cannot read
  is refused, never reset. The store key is sealed by the Android Keystore; the
  Flutter build's key is migrated once by `LegacyVault` (read only).
- **Monitor migrations are append-only** (`monitor/migrations`), and their
  checksums are pinned in `monitor/tests/migration_upgrade.rs`.
- Platform code (`android/`) holds no app logic: each `MainActivity` method is
  one framework call that Rust cannot make directly.
- Interface strings come from `app/l10n/<lang>.json` through `i18n::t(key,
  fallback)`; add the key to `en.json` with the fallback text.
- No AI agent, LLM, MCP or local-terminal functionality: these were removed
  deliberately.
- Never run formatters over code you did not change.
