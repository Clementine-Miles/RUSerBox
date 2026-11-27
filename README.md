English | [简体中文](README_zh.md)

<h2 align="center">RUSerBox</h2>

<p align="center">
Server management for Android, written in Rust: status charts for Linux, Unix and Windows servers, an SSH terminal, files and tools — drawn natively with Vulkan.
<br>
A Rust rewrite of <a href="https://github.com/lollipopkit/flutter_server_box">Flutter Server Box</a>.
</p>

## Installation

Download the Android APK (arm64-v8a, Android 9+, Vulkan 1.1) from
[GitHub Releases](https://github.com/Clementine-Miles/RUSerBox/releases). It
installs over Flutter Server Box 1.0.x and keeps its servers, keys, snippets
and settings.

## Features

**Connecting**

- SSH directly, through jump servers or a `ProxyCommand`, with an alternative
  address to fall back to. Host keys are always verified; a new or changed key
  is shown to you before anything is sent.
- The [Monitor](monitor/) over HTTPS (a pinned certificate or the web PKI) for
  hosts whose SSH port you do not want to expose. A server can use both, with
  automatic fallback.

**Watching**

- Live charts for CPU (per core), memory, swap, disks, network, temperatures
  and GPUs, from the shared status script or the Monitor.
- Home-screen widgets fed by the Monitor; the Monitor's alerts and history.

**Working on a server**

- A terminal: xterm-compatible (alternate screen, 256 and true color, mouse
  reporting, bracketed paste), CJK, wide and combining characters, Nerd Font
  glyphs, scrollback, selection and copy, IME composition, hardware keyboards
  and an on-screen key bar. Sessions over SSH or the Monitor's terminal.
- Zellij sessions: list, attach, create, kill.
- Files over SFTP (or the Monitor's file API): browse, upload, download,
  rename, delete, create folders.
- Processes (and stopping them), Docker/Podman containers (start, stop,
  restart, logs, shell), systemd services, power actions, snippets, private
  keys (import or generate Ed25519).

**Making it yours**

- Gruvbox Material and Gruvbox, light and dark; five bundled terminal fonts;
  17 languages.

**Not yet in the Rust app** (present in Flutter Server Box 1.0.x): remote
desktop pages (the RDP/VNC client is kept as `crates/sbm_remote_desktop`),
virtualization, BMC, firewall, users, scheduled tasks, storage administration,
shares, benchmarks, port-forward management, workspaces with split panes, the
globe, backups and sync, biometric lock. Removed for good: the AI Agent and the
terminal on the device itself (use Termux).

## Building

```sh
make test                 # Rust workspace tests
make build-android-debug  # needs the Android SDK/NDK, cargo-ndk and Gradle 9.8
make monitor-dev          # the Monitor API on :3770
```

See [CLAUDE.md](CLAUDE.md) for the layout and the
[development docs](docs/src/content/docs/development/) for details. No Dart or
Flutter SDK is needed.

## Monitor

[ServerBox Monitor](monitor/) runs on your servers. It is required for
features that work while the app is closed (push alerts, widgets), and gives
charts history from before the app's first connection.

## Contributions

[CONTRIBUTING.md](CONTRIBUTING.md) covers the development setup and the checks
to run. Contributors sign the [CLA](CLA.md) once on their first pull request.

## License

`AGPL v3 lollipopkit & all contributors`
