English | [简体中文](README_zh.md)

# RUSerBox

An Android (arm64-v8a) app for administering Linux servers: SSH and SFTP, a
full terminal with persistent Zellij sessions, a local shell and PRoot Linux
userlands on the device, and live server monitoring. Flutter draws the UI;
Rust owns the server commands, their parsers and the native work; Kotlin is the
thin Android layer.

RUSerBox is a fork of [ServerBox](https://github.com/lollipopkit/flutter_server_box)
reduced to an Android-only administration tool: no AI or MCP features, no
desktop/iOS/web builds, no Windows server backend, no RDP/VNC viewer, no
home-screen widgets and no theme store.

## Install

Download `RUSerBox…_arm64.apk` (or the ServerBox-named APK of a release) from
[Releases](https://github.com/Clementine-Miles/RUSerBox/releases). The app
keeps the application id `tech.lolli.toolbox`, its data and its signing key, so
it installs as an update over earlier arm64 builds.

Requires an Android device with a 64-bit ARM CPU (arm64-v8a, Armv8 or Armv9),
Android 7.0 (API 24) or later.

## Features

- **Connecting**: SSH directly, through jump hosts or a `ProxyCommand`, with
  host-key verification, keys (Ed25519, ECDSA, RSA, encrypted keys), ssh-agent
  and agent forwarding; or HTTP to an independently deployed
  [Monitor agent](docs/monitor-api.md); or both, with fallback.
- **Watching**: live and historical CPU, memory, swap, disks, disk I/O,
  network, sensors, GPUs, batteries and S.M.A.R.T.; Linux (glibc, musl/BusyBox,
  systemd or not) and BSD hosts of any CPU architecture.
- **Working**: terminal with virtual keys and hardware-keyboard shortcuts,
  Zellij session attach/reconnect, split-pane workspace; SFTP/SCP file
  browser and editor; Docker/Podman, processes, systemd, users, firewall,
  storage, shares, cron, port forwards, snippets, Wake-on-LAN, BMC/Redfish,
  libvirt and Proxmox VE (text consoles), YABS and iperf.
- **This device**: a local shell and PRoot Linux userlands (aarch64 rootfs,
  signed manifest, verified downloads).
- **Look**: built-in Gruvbox and Gruvbox Material (Hard/Medium/Soft), Green or
  Blue accent, Light/Dark/System; Hack or JetBrainsMono Nerd Font in the
  terminal. Works offline.
- **Data**: one encrypted SQLite database (SQLite3MultipleCiphers),
  biometric lock, age-encrypted backups and WebDAV sync.

## Documentation

- [docs/BUILDING.md](docs/BUILDING.md) — toolchain, build, test, release
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the code is laid out
- [docs/STORAGE.md](docs/STORAGE.md) — database, migrations, backups, retired data
- [docs/ANDROID.md](docs/ANDROID.md) — background sessions, PRoot, packaging
- [docs/monitor-api.md](docs/monitor-api.md) — the Monitor client contract
- [SECURITY.md](SECURITY.md) — reporting vulnerabilities

## Acknowledgements

Built on [ServerBox](https://github.com/lollipopkit/flutter_server_box) by
lollipopkit and contributors, [dartssh2](https://github.com/TerminalStudio/dartssh2)
and [xterm.dart](https://github.com/TerminalStudio/xterm.dart). Theme palettes:
[gruvbox](https://github.com/morhetz/gruvbox) and
[gruvbox-material](https://github.com/sainnhe/gruvbox-material) via
[ServerBox_Gruvbox_Theme](https://github.com/Clementine-Miles/ServerBox_Gruvbox_Theme);
see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and `assets/fonts/`.

## License

AGPL v3 — lollipopkit & all contributors. See [LICENSE](LICENSE).
