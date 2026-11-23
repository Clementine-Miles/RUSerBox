---
title: Monitor 设置备份
description: 把 Monitor 的设置备份到 WebDAV 文件夹，用你的口令加密，并从备份恢复
---

Monitor 可以将账号密码、代码片段、远程桌面配置、BMC 与 PVE 连接、Agent 设置、推送渠道和监控规则备份到 WebDAV 文件夹。指标历史、审计日志和会话保留在服务器上。

## 设置

通过需要鉴权的 `/api/v1/monitor-backup` API 设置备份：

1. 用 `POST /api/v1/monitor-backup/test` 测试目标文件夹。
2. 用 `PUT /api/v1/monitor-backup` 保存设置，并提供当前 Monitor 密码。
3. 用 `PUT /api/v1/monitor-backup/key` 设置备份口令。

为非本机的 WebDAV 地址使用 HTTPS。允许明文 HTTP 时，备份仍然加密，但 WebDAV 密码会以明文发送。

## 备份方式

- 每 10 分钟检查一次设置。设置有变化时备份；只有自动变化的值变化时（计划任务的上次运行时间、刷新的登录令牌），每天备份一次。
- 保留最新的 10 份（可通过 API 的 `keep` 参数修改）。单份备份最大 100 MB。每台服务器的文件名带有自己的 id（`<server>`），多台服务器共用一个文件夹时不会删除彼此的备份。
- 备份失败时，Monitor 会记录失败并通过配置的推送渠道发送告警。

## 加密

每份备份是加密到一个密钥的 [age](https://age-encryption.org) 文件。密钥的私钥也存放在文件夹中，即 `serverbox-monitor-key-<server>-<id>.age`，用你的口令加密。Monitor 只保留公钥，因此无需口令即可备份，也无法读取自己的备份。

**口令丢失后备份无法恢复。** 新口令只用于之后的备份，之前的备份仍使用当时的口令。

这些文件是标准的 age 格式，不依赖 ServerBox 也能打开：

```sh
age -d serverbox-monitor-key-<server>-<id>.age > key.txt   # 会询问口令
age -d -i key.txt serverbox-monitor-<server>-<time>-<id>.tar.gz.age | tar -xz
```

压缩包内有 `manifest.json`、`config.toml`、`settings.db`（SQLite）以及设置引用的文件（`blobs/`）。

## 恢复

恢复备份时需提供其口令。设置会被替换；这台服务器的历史记录、监听地址、TLS 文件、数据库位置和备份位置保留。账号密码会变为备份中的密码，所有设备需要重新登录。作为 systemd 服务（安装脚本的默认方式）或在 OpenRC 下运行的 Monitor 会自动重启；其他情况（包括容器）请手动重启。

在新机器上，可以在 Monitor 的工作目录中用命令行恢复（安装前后均可）：

```sh
server_box_monitor backup restore --url https://dav.example.com/serverbox/web-1/ --user me
```

它会询问 WebDAV 密码和口令（也可以通过 `--password-env`、`--passphrase-env` 指定的环境变量读取），恢复最新的备份（`--name` 可指定其他；多台服务器共用的文件夹必须指定），并以那台服务器的身份继续备份到同一个文件夹。`server_box_monitor backup list` 列出备份，`backup run` 立即备份一次。
