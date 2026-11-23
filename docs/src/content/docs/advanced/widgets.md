---
title: Home Screen Widgets
description: Add server status widgets to your home screen
---

Install the [Monitor](/docs/advanced/monitor-agent/) on the server, then add
it to the App. The App passes its configured Monitor servers to the widgets.

## How widgets work

Widgets request data directly from the Monitor, so the App does not need to
be open. For each server configured with the Monitor, the App publishes its
name, address, and a short-lived read-only token. The token is stored in the
platform's secure credential storage.

Choose between two layouts: **Small** displays current readings, and
**Medium** displays charts for several metrics. During setup, select a server
and its leading metric. The widget gets its URL from the server configuration.

## Android widgets

### Setup

1. Long-press the Home Screen and tap **Widgets**.
2. Find “Server Box”, choose the Small or Medium type, and add it.
3. On the configuration page, select a server and leading metric.
4. Tap Save.
5. Tap the widget to refresh it manually when needed.

Every Android widget instance keeps its own server and metric selection, so
widgets can show different information. The App supplies the server list. If
it is empty, add a server configured with the Monitor first.

## Troubleshooting

### Widgets do not update

- Make sure the Monitor is running and reachable at the configured address.
- Check the Monitor password, certificate, and `allowInsecure` setting in the App.
- Tap an Android widget to refresh it. Open its configuration to confirm the selected server and metric.

### A widget shows an error or no servers

- Add at least one Monitor server in the App.
- Check the Monitor's HTTPS configuration, saved credentials, and network reachability.
- Widgets use the App's server list instead of manually entered `/status` URLs. If a one-time migration prompt appears, follow it to configure the server again.

## Security

- Prefer HTTPS.
- To use plaintext HTTP with a non-loopback address, enable **Allow insecure HTTP** for that server in the App. Widget endpoints (`/api/v1/metrics`, `/api/v1/metrics/history`, and `/api/v1/watch-token`) have no Monitor-side switch. The Monitor's `allow_insecure` options apply to `[remote_access.terminal]` and `[remote_access.fs]` only.
- Keep Monitor credentials and widget tokens out of public documentation and version control.
