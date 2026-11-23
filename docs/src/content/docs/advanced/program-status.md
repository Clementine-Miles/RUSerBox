---
title: Program Status
description: See when a program in a terminal is waiting for you, has finished or has failed
---

A program running in a terminal can tell Server Box what it is doing: running,
waiting for you, finished or failed. Server Box shows that next to the
terminal's name and, when the terminal is out of sight, as a notification.

No server-side software is required. The program writes a short escape
sequence to its output, which reaches the App through SSH, a Monitor
terminal, or a terminal on this device, alongside the rest of the output.

## What Server Box reads

| Sequence | Sent by | What it says |
|---|---|---|
| OSC 7501 | Programs that support the [Program Status Protocol](https://gist.github.com/mitchellh/7acae3abd8355c1c00287d67e96c913a) | `idle`, `working` (with progress), `done`, `blocked` (waiting for an approval, an answer or a sign-in) or `error`, with a title and a message |
| OSC 9;4 | systemd, winget and other programs that show a progress bar | Progress, failed, paused |
| OSC 133 | Shells with shell integration, such as fish 4 | A command started, or finished with an exit code |

## Where it is shown

- **A dot before the terminal's name**, in the terminal tab's switcher, its
  list and the side bar: orange while a program waits for you, red when one
  failed, green when one finished, the accent colour while one runs (a ring
  when it reports progress). A program that is only idle shows nothing.
- **The status button** in the terminal's bar lists the reports the terminal
  receives. A finished or failed report stays until the program replaces it or
  you clear it there.
- **Notifications**: when a terminal is not on screen — another tab, or the
  App in the background — a program starting to wait, finishing or failing
  shows a notification, and so does a command that ran for 30 seconds or more
  ending. Tapping it opens that terminal. Turn it off in **Settings → SSH →
  Notify when a program needs you**.
- **The ongoing Android notification** shows
  what the session's programs are doing — waiting, finished, failed or running —
  instead of the address. It can be read without unlocking, so they carry
  only that state, never a program's own title or message, and nothing while
  the notification switch above is off.
- **The Agent** reads the reports along with the screen, so it can tell that a
  program is waiting for an answer.

## Reporting from your own scripts

A report is `ESC ] 7501 ; key=value:key=value ESC \`, with `msg` and `title`
encoded in base64:

```sh
status() {
  printf '\e]7501;state=%s:msg=%s\e\\' "$1" "$(printf '%s' "$2" | base64 | tr -d '\n')"
}

status working 'Backing up /srv'
# ...
status done 'Backup finished'
```

`state=clear` removes the report. A `working`, `blocked` or `idle` report is removed
when the program's shell shows its next prompt (with shell integration) or the
session ends; `done` and `error` stay. The full rules, such as `id` for
several reports from one program, are in the
[specification](https://gist.github.com/mitchellh/7acae3abd8355c1c00287d67e96c913a).
