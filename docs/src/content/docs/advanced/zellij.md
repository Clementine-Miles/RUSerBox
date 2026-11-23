---
title: Zellij sessions
description: Keep a remote terminal session running across SSH connections
---

Install [Zellij](https://zellij.dev/documentation/installation) on the SSH host and make it available on the remote account's `PATH`. Server Box uses the existing SSH connection for session management and opens the Zellij client on an SSH PTY. The integration was tested with Zellij 0.41.2 and 0.46.0.

Open **Zellij sessions** from the terminal toolbar or overflow menu. You can also enable the Zellij virtual key in the SSH keyboard settings.

- Tap a running session to attach, or use **New session** to enter a name and attach to it.
- **Detach** returns to an SSH shell while the remote session keeps running.
- **Terminate session** ends the selected remote session and its running programs after confirmation.
- Zellij draws its own tabs and panes and handles their keyboard shortcuts inside the terminal.

Under **Settings → SSH → Zellij auto-attach**, you can enable auto-attach and choose whether to open the picker when connecting. With the picker off, Server Box prefers the configured name, then an existing session, then creates the configured session. The default is `server_box`. On reconnect, Server Box tries the last attached session. It opens a regular SSH shell if Zellij or the previous session is unavailable.
