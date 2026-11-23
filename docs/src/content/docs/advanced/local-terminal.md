---
title: Terminal on This Device
description: Open a shell on the device running Server Box
---

When supported by the current build, the terminal tab shows local execution
options before your remote servers: a shell on **this device** and a Linux
userland provided by Server Box.

These entries need no server connection or credentials.

## Shell on this device

The shell runs in the Server Box Android app sandbox.

## Linux userland

If the platform has no shell or its shell is limited, Server Box can install
an independent Linux userland. Alpine 3.22.5 is currently the default; the
release list may also include other distributions and versions, such as Debian
or Ubuntu.

The terminal tab labels it **<distribution> <version>** and lists it beside
the local shell. Choose the distribution and version when installing. Updating
keeps the existing profile and does not offer a different choice.

Arm64 builds download a Linux rootfs, verify it against a pinned digest,
and enter the unpacked filesystem with `proot`.

### Use cases

- Use `curl`, `dig`, `ssh`, or `jq` on a phone that does not include them.
- Perform temporary work that you do not want to run directly on a production server.
- Give Agent an execution target isolated from the device filesystem; see [Agent](/docs/advanced/agent/).

Each userland follows its distribution's standard layout and package manager.
For example, install packages with `apk add` on Alpine or `apt install` on
Debian and Ubuntu.

### Filesystem isolation

The Alpine userland has a separate filesystem and cannot read phone storage,
App data, private keys, or user files. **Run commands on this device** therefore
uses the isolated userland.

## Differences from a server terminal

The local terminal uses the same terminal emulator, virtual keyboard, and tabs
as a server session. Only the source of terminal input and output changes.

Because it runs locally, this shell has no server host key to verify and does
not reconnect as a server session would. It is not added to the server list
and has no status charts.
