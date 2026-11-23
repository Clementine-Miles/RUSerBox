---
title: Zellij 会话
description: 在 SSH 连接断开后保留远程终端会话
---

在 SSH 主机上[安装 Zellij](https://zellij.dev/documentation/installation)，并确保远程账户的 `PATH` 能找到它。Server Box 使用现有 SSH 连接管理会话，并通过 SSH 伪终端运行 Zellij 客户端。已验证 Zellij 0.41.2 和 0.46.0。

在终端工具栏或更多菜单中打开 **Zellij 会话**。也可以在 SSH 键盘设置中启用 Zellij 虚拟按键。

- 点击正在运行的会话即可附加，或选择**新建会话**并输入名称。
- 选择**分离**可以回到普通 SSH Shell，远程会话会继续运行。
- 确认**终止会话**后，所选会话及其中运行的程序将结束。
- Zellij 在终端内显示标签页和窗格，并处理相应的快捷键。

在 **设置 → SSH → Zellij 自动附加**中，可以启用自动附加并选择是否在连接时显示会话选择器。关闭选择器后，应用优先附加指定名称的会话，其次附加已有会话；没有运行中的会话时会创建指定名称的会话。默认名称为 `server_box`。重新连接时，应用会尝试恢复最后附加的会话。若 Zellij 或该会话不可用，则打开普通 SSH Shell。
