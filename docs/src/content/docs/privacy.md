---
title: Privacy Policy
description: What RUSerBox stores, what it sends, and to whom
---

Last updated: 2026-11-27.

RUSerBox is a client for the servers you configure. It has no user account,
and the developers run no service that relays SSH, SFTP or Monitor traffic:
connections go straight from the app to the addresses you set.

## What is stored on the device

| Data | Where |
|---|---|
| Servers (addresses, users, SSH passwords, Monitor passwords and pinned certificates), private keys and their passphrases, snippets, port forwards, trusted host keys, connection history, settings | One encrypted SQLite database (`store.db`, ChaCha20-Poly1305). Its key is sealed by a key held in the Android Keystore, which never leaves it. |
| The home-screen widgets' server list and read-only Monitor tokens | App-private preferences; the tokens encrypted with a Keystore key. |

Android's cloud backup and device transfer are switched off for the app: the
database could not be opened on another device without the Keystore key.

## What is sent, and to whom

| Connection | Destination | When |
|---|---|---|
| SSH (terminal, status, files, forwards) | The servers and jump servers you configured | When you connect, or when a server is set to connect automatically |
| Monitor API | The Monitor addresses you configured | When you connect to a server that uses the Monitor, and when the widgets refresh |

Nothing else: no analytics, no crash reports, no update checks, and no
requests to the developers. The app has no AI features.

## Removing data

Deleting a server removes it, its trusted host keys and its connection history.
Uninstalling the app removes everything it stored.
