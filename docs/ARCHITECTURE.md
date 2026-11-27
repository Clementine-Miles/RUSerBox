# Architecture

RUSerBox is one Android app: Flutter for the UI, Rust for what is said to a
server and for native work, Kotlin for the Android integration that needs the
JVM.

## Layers

| Layer | Where | Owns |
| --- | --- | --- |
| Rust | `crates/sbm_parser` | every server command, script and output parser (pure, no I/O; raw counters, rates stay with the caller); process kill, users, firewall, storage, cron, shares, Zellij, benchmarks |
| | `crates/sbm_virt` | the virtualization model: libvirt commands and the Proxmox VE API client (sessions, TOTP, tickets, certificate pinning, text consoles) |
| | `crates/sbm_redfish` | the BMC (Redfish) client; also decides PVE certificate pins |
| | `crates/sbm_ffi` | flutter_rust_bridge bindings (`src/api/*`), SSH record-layer and signature crypto (`docs/notes/ssh-crypto.md`) |
| | `packages/fl_lib/rust` | encrypted sync engine, LK Cloud client (disabled at build: `Lkc.enabled = false`), age encryption |
| Dart | `lib/` | UI, navigation, Riverpod state, stores (`lib/data/store`), transports (dartssh2, Monitor HTTP), terminal (xterm) |
| | `packages/fl_lib` | shared widgets, sheets, storage base (`SqliteStore`), sync engine host |
| | `packages/fl_tty` | the shell on this device: PTY (`src/flutter_pty.c`), PRoot, rootfs manifest and installer |
| Kotlin | `android/app/src/main/kotlin` | foreground service and session notification, transfer notification, quick-settings tile, launcher shortcuts, incoming shares/drops, Linux DocumentsProvider, keyboard shortcut list, per-app language |

Rules that keep the split honest:

- **Rust owns what is said to a server.** Dart never builds a command or
  parses its output; it calls `sbm_ffi`. New server features start in Rust.
  (HTTP JSON clients such as the PVE API live in `sbm_virt` too.)
- Generated code is regenerated, never edited: `flutter_rust_bridge_codegen
  generate` (output `lib/src/rust`, never run `integrate`), `dart run
  build_runner build`, `flutter gen-l10n`, `python3 tools/gen_gruvbox.py <ref>`.
- `flutter_rust_bridge` in `pubspec.yaml`, `crates/sbm_ffi/Cargo.toml` and the
  installed codegen must be the same version.
- `ScriptConstants.version` and the `v<N>` in `ScriptConstants.scriptFile` stay
  equal.

## Connections

A server is reached over SSH, an independently deployed Monitor agent (HTTP,
`docs/monitor-api.md`), both, or is this device (`Spi.local`).

- `ServerNotifier.ensureExec()` is the one place a command reaches a server,
  falling back to the other transport. `ensureShellClient()` is SSH-only.
- `ServerTcpDialer` is the one place a TCP connection from the server's side
  is made (SSH direct-tcpip, the agent's `/stream/ws`, or local).
- Ask `ServerCapabilities`, never the transport.
- SSH: direct, jump hosts or `ProxyCommand`; host keys are always verified.
- A Windows host (detected by `ver`) is shown as unsupported and never sent a
  script or status command.

## Theme

`lib/data/res/gruvbox.dart` is the only palette: two families, eight variants
(classic Gruvbox Medium; Gruvbox Material Hard/Medium/Soft; each Green or Blue)
from the generated table `gruvbox_palettes.g.dart`. `ColorScheme`
(`app_theme.dart`), chart and status colours (`chart_palette.dart`) and the
terminal ANSI table (`terminal.dart`) all derive from it.

## UI rules

- **Pages**: in `lib/`, a page is `AppRoute.go`, `AppPageRoute` or `AppPage`
  (fl_lib), never a bare `MaterialPageRoute`/`MaterialPage`, so Android's
  predictive back pops only the page on screen. A pushed page keeps a way back
  at every width, decided by `ModalRoute.canPopOf`, never by the width
  (`test/widget/pushed_page_way_back_test.dart`).
- **Tabs**: a narrow tab bar is a `SwitcherBar`; a tab's single column is the
  subject and the list moves behind a bar button. `AppTab` is stored by name.
- **Dialog rules — no dialogs**: a question is a bottom sheet
  (`context.showSheet`, `showInputSheet`, `showPickSheet`, …). Close a sheet
  with `context.popSheet()` or a returning `Btn.ok`/`Btnx.cancelOk`, never
  `context.pop()`. No sheet stands over a sheet: what is raised from one is
  its next page (`SheetStack`). `showResizableSheet(standalone: true)` is the
  one way to a second sheet, argued in a comment.
- No interface copy that narrates the interface.
- GetIt for stores and services; split UI into build/actions/utils with
  `extension on`.

## Storage

See [STORAGE.md](STORAGE.md): one encrypted SQLite file (SQLite3MultipleCiphers),
Drift for DDL only, hand-written synchronous queries, ids as keys, enums by
name, three-edit schema steps with permanent tests, age-encrypted backups.

## Rust migration status

| Area | Today | Next step |
| --- | --- | --- |
| Command building, parsing | Rust (`sbm_parser`) | — |
| Virtualization, Redfish, PVE | Rust | — |
| SSH record layer, signatures, KDF | Rust (`sbm_ffi`) | — |
| Backup crypto (age), sync engine | Rust (fl_lib) | — |
| SSH/SFTP protocol | Dart (dartssh2 fork) | evaluate `russh` behind the same `SSHClient` surface; keep dartssh2 until host keys, agent forwarding, jump hosts, ProxyCommand and SFTP pass the same tests |
| Terminal emulation | Dart (xterm fork) | evaluate a Rust VT parser (e.g. `vte`) feeding the Dart renderer |
| Local PTY | C (`fl_tty`) | small and stable; replace only with a tested Rust `rustix`/`nix` PTY |
| Monitor HTTP client | Dart (dio) | candidate for a Rust client sharing `sbm_parser::protocol` |
