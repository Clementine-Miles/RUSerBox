# Security Policy

## Supported versions

Fixes go into the latest release of the Android app only. Before reporting, check that the issue still happens on the [latest release](https://github.com/Clementine-Miles/RUSerBox/releases/latest).

## Reporting a vulnerability

Report it privately through GitHub: **[Report a vulnerability](https://github.com/Clementine-Miles/RUSerBox/security/advisories/new)** (the repository's Security tab). Do not open a public issue, pull request or discussion for it.

Include:

- the affected component: the app's Dart, Kotlin or Rust code;
- the version, and how the server is reached (SSH, Monitor agent or both);
- steps to reproduce, and what an attacker gains;
- a proof of concept, if you have one.

The report stays private until a fix is released. Credit is given in the advisory unless you ask otherwise.

## Scope

In scope: this repository's Android app and its Rust crates. The Monitor agent is a separate project.

Out of scope:

- vulnerabilities in the servers you manage, or in third-party software the app talks to (OpenSSH, Docker, libvirt, Proxmox VE, a BMC);
- dependencies with a published advisory that do not affect RUSerBox. Report those upstream; Dependabot tracks them here;
- attacks that need an already unlocked device or an already compromised server account.
