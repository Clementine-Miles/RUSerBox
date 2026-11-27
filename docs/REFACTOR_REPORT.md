# Android arm64 refactor — engineering report

Baseline: `dfb7bed` ("Prune native, Dart and platform dependencies").
Result: this branch. Every step is its own commit; `git log dfb7bed..` lists them.

## What changed

| Area | Result |
| --- | --- |
| AI / LLM / MCP / agent | Removed: `fl_pi_llm` (submodule), `sbm_mcp`, the agent UI, stores, providers and models, Ask-AI, `.claude/`, every `CLAUDE.md` (their engineering notes moved to `docs/`). SSH agent support and the non-AI Monitor client stay. Storage step `m043_retire_removed_features` drops the AI stores, `pi_chunks`, the remote-desktop table and their sync bookkeeping. |
| ABI | arm64-v8a only: Gradle `abiFilters`, the native-assets hook (refuses other Android arches), Rust toolchain targets, release/F-Droid scripts, CI. Version code `build * 100 + 3` (the arm64 code published releases already used), so upgrades stay monotonic. Linux host target kept for tests. |
| Platforms | Desktop/Apple/web branches removed from the app and fl_lib; Android keyboard and wide-layout behaviour kept. |
| Windows servers | Script/status/process back end removed from Rust and Dart. A host detected as Windows is shown as unsupported (`windowsServerUnsupported`), never sent a command; stored server records and credentials are untouched. |
| RDP / VNC | Removed (Rust `ironrdp`/`vnc-rs`, viewer, profiles). SSH tunnels and text consoles stay. |
| Widgets | Home-screen widgets removed; on first start the app deletes their preferences and Keystore key and revokes their Monitor watch tokens once. |
| Monitor server, website, docs site, web UI | Removed from the repository; the app's Monitor client and `docs/monitor-api.md` stay. |
| Theme | Built-in, offline Gruvbox: classic Gruvbox Medium (Green/Blue) and Gruvbox Material Hard/Medium/Soft (Green/Blue) = 8 variants × Light/Dark/System. Generated from the reference generator at a pinned revision (`tools/gen_gruvbox.py`). One palette feeds `ColorScheme`, chart/status colours and the terminal's ANSI table. |
| Fonts | Exactly two terminal fonts: Hack Nerd Font Mono (default) and JetBrainsMono NL Nerd Font Mono (Nerd Fonts v3.5.1); old font settings migrate. |
| Dependencies | Removed `fl_build`, `computer`, LaTeX markdown, `flutter_gbk2utf8`, `flutter_native_splash`, Rust `tokio`/`serde`/`ironrdp`/`vnc-rs`/`jpeg`/`x509-cert` from sbm_ffi, and others. |
| Build | Gradle runs on JDK 27 (Temurin 27.0.1); Java/Kotlin bytecode target 17; core library desugaring kept. `scripts/release/build-android.sh` + `verify-apk.sh`. |
| CI | Android arm64 build/verify/release, Rust tests + clippy (Linux), Flutter analyze/tests, Kotlin unit tests, reproducibility check (arm64). Monitor, website, docs-site, Windows and fl_pi_llm jobs removed. |
| l10n | 176 strings no code reads removed from all 17 languages. |

## Metrics

| | Baseline `dfb7bed` | Now |
| --- | --- | --- |
| Tracked files | 2808 | 2258 |
| Dart lines in `lib/` (hand-written) | 191 350 | 175 795 |
| Rust lines (excl. FRB output) | 170 121 | 97 326 |
| Git submodules | 5 | 2 (dartssh2, xterm) |
| Cargo.lock packages | 678 | 278 |
| pubspec.lock packages | 278 | 249 |
| English strings | 2034 (at fa01ec2) | 1836 |
| Diff | — | 888 files, +5 471 / −157 643 |

Release APK (arm64-v8a, debug-key signed for the check): 41.0 MB, 13 native
libraries, all AArch64. `libsbm_ffi.so` 14.3 MB, `libfl_lib_ffi.so` 7.4 MB,
`libapp.so` 36.6 MB (uncompressed sizes).

## Verification run

| Check | Result |
| --- | --- |
| `flutter analyze lib test`, `flutter analyze` (fl_lib) | no issues |
| `flutter test` (all non-e2e files) | 3618 passed, 11 skipped, 11 failed on the full run; 7 of those were tests of removed desktop/Windows behaviour, now removed or moved to the Android path. The 4 left: `local_file_backend_test` (expects a permission error, but the sandbox runs as root), `pve_backend_test` upload (the sandbox's HTTP proxy rewrites the multipart body), `app_locale_test` (timing; passes on rerun), `terminal_status_sheet_test` (code and test unchanged since baseline) |
| `cargo test --workspace` | all pass except `sbm_virt::probe_finds_pve_and_containers`, which fails only inside a Docker container (the probe correctly sees `/.dockerenv`); the crate was not changed |
| `cargo test` fl_lib Rust workspace | pass (120 tests) |
| `cargo clippy --workspace --all-targets -D warnings` | clean |
| Kotlin unit tests (`:app:testDebugUnitTest`) | 17/17 pass |
| `scripts/release/build-android.sh` on JDK 27 | APK built; `verify-apk.sh` OK |
| `verify-apk.sh` | only `lib/arm64-v8a/`, every `.so` AArch64, LOAD segments aligned ≥ 16 KB (0x4000/0x10000), no glibc dependency, no removed-feature libraries, `zipalign -c -P 16 4` OK, signature verifies, versionCode 192503 |
| `aapt2 dump badging` | `native-code: 'arm64-v8a'`, target/compile SDK 36, application ID `tech.lolli.toolbox` unchanged |

## Not tested (no device or service here)

- Running on a device or emulator: install, upgrade over an existing install
  (data migration on real data), 16 KB-page runtime loading, PRoot userland,
  foreground service, notifications, quick-settings tile, shares.
- Real SSH / Monitor / libvirt / PVE / Redfish / WebDAV servers (the e2e
  suites skip without them).
- The release signing path (1Password); the check used the debug key.
- The CI workflows themselves (they run on GitHub; YAML validated locally).

## Decisions and follow-ups

- `cargo fmt --check` is not a CI gate: the tree was never rustfmt-formatted,
  and reformatting ~140 files would bury the real changes. Do it as its own
  commit if wanted.
- Kept on purpose: `sentry` (crash reports, opt-in), `desktop_drop`
  (drag-and-drop from other Android apps; it still applies the Kotlin Gradle
  plugin, which Flutter will stop accepting), LK Cloud / `flutter_appauth`
  (compiled in, disabled with `Lkc.enabled = false`; removal is a candidate).
- Candidates: drop the unused icon-font packages behind `icons_plus` (tree-shaken
  but still downloaded), the iOS remnants in fl_lib's path helpers, LK Cloud.
- Rust-first next steps are listed in `docs/ARCHITECTURE.md` (SSH protocol via
  `russh`, VT parsing, Monitor HTTP client); none were attempted here because
  each needs device-level testing.
