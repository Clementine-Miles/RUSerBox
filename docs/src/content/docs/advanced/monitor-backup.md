---
title: Monitor Settings Backup
description: Back up the Monitor's settings to a WebDAV folder, encrypted with your passphrase, and restore them
---

The Monitor can back up its account password, snippets, remote desktop
profiles, BMC and PVE connections, Agent settings, push channels, and
monitoring rules to a WebDAV folder. Metrics history, audit logs, and
sessions stay on the server.

## Set it up

Use the authenticated `/api/v1/monitor-backup` API:

1. Test the destination with `POST /api/v1/monitor-backup/test`.
2. Save the destination with `PUT /api/v1/monitor-backup`. Include the current
   Monitor password.
3. Set the backup passphrase with `PUT /api/v1/monitor-backup/key`.

Use HTTPS for non-loopback WebDAV destinations. If you enable plain HTTP,
the backup remains encrypted and the WebDAV password travels unencrypted.

## How backups are made

- The settings are checked every 10 minutes. A backup is made when they
  changed, and once a day when only values that change on their own did (the
  last run of a schedule, refreshed sign-in tokens).
- The newest 10 are kept by default. The API accepts a `keep` setting. A backup can be at
  most 100 MB. Each server's files carry its own id (`<server>`), so servers
  sharing a folder never remove each other's backups.
- The Monitor records backup failures and sends an alert through the
  configured push channels.

## Encryption

Each backup is a [age](https://age-encryption.org) file encrypted to a key. The
key's secret half is in the folder too, as `serverbox-monitor-key-<server>-<id>.age`,
encrypted with your passphrase. The Monitor keeps only the public half, so it
backs up without the passphrase and cannot read its own backups.

**If you lose the passphrase, the backups cannot be restored.** A new
passphrase applies to backups from then on; older backups keep the passphrase
they were made with.

The files are plain age, so you can open them without ServerBox:

```sh
age -d serverbox-monitor-key-<server>-<id>.age > key.txt   # asks the passphrase
age -d -i key.txt serverbox-monitor-<server>-<time>-<id>.tar.gz.age | tar -xz
```

The archive holds `manifest.json`, `config.toml`, `settings.db` (SQLite) and
the files the settings name (`blobs/`).

## Restore

Restore with the command below and enter the backup passphrase. The settings are replaced; the server's history, its
listen address, TLS files, database location and where backups go stay. The
account password becomes the one in the backup, so every device signs in
again. A Monitor run as a systemd service (as the installer sets it up) or under
OpenRC restarts by itself; otherwise, in a container too, restart it.

On a new machine, before or after installing the Monitor, restore from the
command line in its working directory:

```sh
server_box_monitor backup restore --url https://dav.example.com/serverbox/web-1/ --user me
```

It asks the WebDAV password and the passphrase (or reads them from the
variables `--password-env` and `--passphrase-env` name), restores the newest
backup (`--name` picks another; a folder several servers back up to needs it),
and keeps backing up to the same folder as that server.
`server_box_monitor backup list` lists the backups and `backup run` makes one
now.
