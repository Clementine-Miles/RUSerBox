# Native client migration gates

## Rollback point

On 2026-11-28, the result of `git ls-remote origin refs/heads/main` and the local HEAD were both `bb15c850b1400ae0c4948e06920fb83750dea02c`. The tree diff was empty. This commit identifies the Flutter client and all its migration fixtures. Retrieve it by commit and run `git submodule update --init` to reproduce the source baseline. A source rollback cannot downgrade an installed Android database. Test upgrades on a dedicated device with a recoverable encrypted export.

| Property | Baseline |
| --- | --- |
| Client | Flutter/Riverpod, Kotlin, FRB 2.13.0, `server_box` 1.0.1925+1925 |
| Android | `tech.lolli.toolbox`, arm64-v8a, minimum API 24, release versionCode 192503 |
| Source dependencies | dartssh2 `d3cbea7cf1946d6b520e2e2b1553793e021138d1`, xterm.dart `dbb85324999bb6e339e364de1f8d2b03af0318e7` |
| Native workspaces | sbm_parser, sbm_virt, sbm_redfish and sbm_ffi in the root. The backup and sync workspace lives under `packages/fl_lib/rust`. |
| User data | SQLite3MultipleCiphers, schema 44 in `lib/data/store/schema.dart`. Drift manages DDL. Hive supplies historical input. |
| Monitor | External HTTP/WebSocket service. Protocol 2, minimum agent 2, minimum client 1. The historical commit `dfb7bede78cda1cff9923e24f309d41b171aead3` contains both `monitor/` and `crates/sbm_native/`. |

The build, architecture, storage and Monitor API documents describe the shipping baseline. The native APK is a development target. A signed build alone establishes neither upgrade compatibility nor runtime behavior.

## Retained behavior inventory

Paths below identify existing tests or implementation contracts at the rollback point. They give the port a regression target. A passing parser fixture exercises a narrower claim than operating a real host.

| Behavior to preserve | Evidence or device check |
| --- | --- |
| SSH trust, Host CA, password and key auth, encrypted keys, SSH agent, identity isolation | `test/unit/ssh/{host_key_verify,host_cert_authority,ssh_combined_auth,ssh_agent,ssh_key_unlock}_test.dart`. Exercise host-key changes and rejected prompts on each hop. |
| Jump/failover, SOCKS5, ProxyCommand, local and remote forwarding | `test/unit/ssh/{jump_chain,jump_failover,socks5,proxy_command,port_forward_provider,ssh_local_tunnel}_test.dart`. Existing ProxyCommand profiles need a visible migration boundary when local execution retires. |
| Dual transport, capability checks, SSH shell, Monitor PTY and TCP relay | `docs/ARCHITECTURE.md`, `docs/monitor-api.md`, `test/unit/ssh/{server_exec,server_tcp}_test.dart`, `test/unit/terminal/monitor_terminal_ws_test.dart`. Exercise SSH-only, agent-only and fallback paths separately. |
| Linux glibc, musl/BusyBox and BSD monitoring | `crates/sbm_parser/tests/{alpine_e2e,field_output,disk_sources,proc_compat}.rs` and their fixtures. Keep the unsupported Windows profile and its credentials. |
| CPU, memory, swap, disks, SMART, I/O, interfaces, GPU, battery, sensors, history, retention and alerts | `docs/monitor-api.md`, `test/unit/server/{disk_smart,status_history}_test.dart`, `test/e2e/monitor_overview_test.dart`. Distinguish missing device detail from a measured zero. |
| Remote terminal, independent tabs, resize, keyboard modifiers and Zellij | `test/unit/terminal/{session_manager,zellij,virt_key_semantics}_test.dart`, `test/unit/ssh/terminal_session_resize_test.dart`. Verify attach/detach, Unicode IME and reconnect without command replay on a device. |
| SFTP/SCP, replacement and privileged operations | `test/unit/file/{sftp_replace,sftp_file_backend,sftp_sudo,scp_protocol,scp_close}_test.dart`. Test SAF cancellation and interrupted large transfers before replacing the backend. |
| Docker/Podman, processes, systemd, users, firewall, storage, shares, cron and snippets | `test/unit/server/container_test.dart`, `crates/sbm_parser/tests/{proc_compat,service_compat,user_compat,firewall_change,cron_compat,snippet_compat}.rs` and the tests in `crates/sbm_parser/src/{storage,shares}.rs`. |
| Wake-on-LAN, YABS, iperf | Preserve configured MAC, destination and optional SecureOn password. Confirm receipt on a test LAN. Use `crates/sbm_parser/tests/bench_script.rs` and `crates/sbm_parser/src/iperf.rs` for benchmark launch, cancellation and results. |
| BMC/Redfish, libvirt/PVE management and text consoles | `crates/{sbm_redfish,sbm_virt}/tests/`, `test/unit/virt/{pve_termproxy,virt_text_consoles}_test.dart` and `test/e2e/virt_real_test.dart`. |
| Gruvbox families, Light/Dark/System, contrast, accents and both terminal fonts | `test/unit/app/gruvbox_test.dart`, `lib/data/res/gruvbox_palettes.g.dart` and `assets/fonts/`. Compare real-device captures at small sizes and check keyboard, chart and terminal colours. |
| Encrypted DB, biometric lock, historical import and unsupported data | `docs/STORAGE.md`, `test/widget/bio_auth_lock_test.dart`, `test/migration/hive_release_migration_test.dart`, `test/fixtures/hive_v{1466,1480,1491}/`. Verify key cancellation, invalidation and interrupted import against preserved originals. |
| age export/import, WebDAV, revisions, tombstones, categories and pass-through | `packages/fl_lib/rust`, `test/unit/store/{backup_v2,tombstone,segment_sync}_test.dart`, `test/e2e/{backup_sync,segment_sync}_test.dart`. Preserve IDs, child records, enum names and device-local timestamps. Test against an old peer. |

Local shell, PTY and PRoot still ship in this baseline. Their eventual removal is separate from remote PTY, remote BusyBox compatibility, SSH agent support and SAF. The retired AI, RDP/VNC, widget and theme-store names in migrations and fixtures carry historical meaning.

## Upstream references

* **foot 1.28.0** at `ab33c9a19d626f8d0ac0bd1adfdba21946ada948`. The full upstream export in Debian's `foot_1.28.0.orig.tar.xz` has SHA-256 `17621ad2e38a83688202117599857212a65434ae0562752a38167537a6c35cbd`. The hash matches the Sources index authenticated by Debian's signed sid InRelease. The commit comes from `xz -dc foot_1.28.0.orig.tar.xz | git get-tar-commit-id`. Direct Codeberg Git access returned HTTP 403. The top-level MIT license credits Daniel Eklöf (2019). The Unicode data carries a separate Unicode notice. Preserve those notices if porting or copying their respective material.
* Before porting, trace the state transitions in `vt.c`, dispatch in `csi.c`, `osc.c` and `dcs.c`, and the relationships between `grid.c`, `terminal.h`, sixel data and selection. Wayland, process spawning, fcft and SHM stay outside the host core. A complete source export is available for that review. Differential results and parser coverage are pending.
* **Conduit** at `9cd2ff9174e4ee8e9f0679d1da6aa4aa0edcf742`. Fetched the exact Git object and checked its Apache-2.0 LICENSE and THIRD_PARTY_NOTICES. The connection factory, attempt generation in `terminal_session_controller.dart` and prompt coordinator are design references. Conduit's third-party local-shell binaries have separate licenses.

## Android responsibilities at cutover

| Baseline component | Native disposition |
| --- | --- |
| MainActivity | Reimplement Framework callbacks in Java/JNI, including validated intents, lifecycle, privacy cover, insets and IME. |
| AppLocale, AppShortcuts, KeyboardShortcuts | Reimplement the Android locale, launcher and physical keyboard integration. Preserve host IDs in shortcuts. |
| ForegroundService, SessionsTileService, TransferNotification | Reimplement the foreground session notification, session tile and transfer progress. Keep stop and cancellation explicit. |
| IncomingActivity, IncomingFiles, DropListener | Reimplement share/drop boundaries with validated content URIs, grants and bounded FD streaming. |
| RetiredWidgetData | Reimplement the one-time preference and Keystore cleanup for direct upgrades. Pair it with the remote retired-token revocation behavior. |
| ImpellerCompatibility | Retire with Flutter after the Vulkan replacement passes device gates. |
| GuestPath, LinuxDocumentsProvider | Retire with the on-device Linux userland after replacement acceptance. Keep the SFTP document picker. |

## Evidence and remaining gates

The baseline host commands were `tini -s -- cargo test --locked --workspace --no-fail-fast` and `tini -s -- cargo test --locked --workspace --manifest-path packages/fl_lib/rust/Cargo.toml --no-fail-fast`. On this host the root suite passed 1,342 tests with 13 ignored. The separate workspace passed 120 with one ignored. Tini reaps the detached benchmark processes inside the cloud container.

Record the device model, Android/API version, GPU driver, actual display refresh rate, thermal state, signing identity and workload alongside every device run. Compare the baseline and candidate on the same device for cold start, input-to-display percentiles, output throughput, memory, idle and sustained power. For the candidate, also capture atlas hits/misses, GPU timestamp queries, database growth and write amplification. No device measurements have been recorded.
