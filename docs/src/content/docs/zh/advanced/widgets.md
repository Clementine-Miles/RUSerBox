---
title: 主屏幕小组件
description: 在主屏幕添加服务器状态小组件
---

先在服务器上安装 [Monitor](/docs/zh/advanced/monitor-agent/)，再将服务器添加到 App。小组件会使用 App 中配置的 Monitor 服务器。

## 工作方式

小组件直接向 Monitor 请求数据，因此 App 无需保持打开。对于每台已配置 Monitor 的服务器，App 会向小组件提供名称、地址和短期 read-only token。token 保存在平台的安全凭据存储中。

小组件有两种固定布局：**Small** 显示当前读数，**Medium** 显示多个指标的图表。设置时选择服务器和主指标；URL 从服务器配置中读取，无需手动填写。

## Android 小组件

### 添加步骤

1. 长按主屏幕，点击 **小组件**。
2. 找到 “Server Box”，选择 Small 或 Medium 类型并添加。
3. 在配置页面选择服务器和主指标。
4. 点击保存。
5. 点击主屏幕上的小组件可以手动刷新。

每个 Android 小组件实例都会单独保存服务器和指标选择，因此多个小组件可以显示不同内容。服务器列表由 App 提供；如果列表为空，请先添加一台配置了 Monitor 的服务器。

## 故障排除

### 小组件不更新

- 确认 Monitor 正在运行，且配置地址可访问。
- 检查 App 中保存的 Monitor 密码、证书和 `allowInsecure` 设置。
- 点击 Android 小组件可手动刷新；打开其配置页确认服务器和指标选择。

### 小组件显示错误或没有服务器

- 在 App 中至少添加一台配置了 Monitor 的服务器。
- 检查 Monitor 的 HTTPS 地址、登录凭据和网络连接。
- 小组件直接使用 App 同步的服务器列表，不再读取旧版手动填写的 `/status` URL。如遇迁移提示，请按引导重新配置服务器。

## 安全性

- 优先使用 HTTPS。
- 非 loopback 地址使用明文 HTTP 时，必须在 App 中为该服务器开启**允许不安全 HTTP**。小组件 endpoint（`/api/v1/metrics`、`/api/v1/metrics/history`、`/api/v1/watch-token`）没有对应的 Monitor 开关；Monitor 的 `allow_insecure` 仅适用于 `[remote_access.terminal]` 和 `[remote_access.fs]`。
- 不要将 Monitor 凭据或小组件 token 写入公开文档或提交到版本控制。
