part of '../entry.dart';

extension _SSH on _AppSettingsPageState {
  List<SettingsGroup> _buildSSH() {
    return [
      SettingsGroup(libL10n.general, [
        _buildLetterCache(),
        _buildSSHWakeLock(),
        _buildSSHVirtualKeyAutoOff(),
        _buildProgramStatusAlerts(),
        _buildTermMetricsBar(),
        _buildHostCertAuthorities(),
        _buildRememberKeyPassphrase(),
      ]),
      SettingsGroup(libL10n.theme, [
        _buildTermTheme(),
        _buildFont(),
        _buildTermFontSize(),
        _buildTermLineHeight(),
      ]),
      SettingsGroup(l10n.zellijAutoAttach, [
        _buildZellijAutoToggle(),
        _buildZellijShowSelector(),
        _buildZellijSessionName(),
      ]),
    ];
  }

  SettingsRow _buildSSHVirtualKeyAutoOff() {
    final label = l10n.sshVirtualKeyAutoOff;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.hotkey_fill),
        title: Text(label),
        subtitle: const Text('Ctrl & Alt', style: UIs.textGrey),
        trailing: StoreSwitch(prop: _setting.sshVirtualKeyAutoOff),
      ),
      keywords: 'Ctrl Alt',
    );
  }

  SettingsRow _buildFont() {
    final label = libL10n.font;
    final prop = _setting.terminalTypeface;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.font_fill),
        title: Text(label),
        trailing: prop.listenable().listenVal(
          (font) => Text(font.family, style: UIs.text15),
        ),
        onTap: () async {
          final font = await context.showPickSingleSheet(
            title: label,
            items: TerminalTypeface.values,
            display: (font) => font.family,
            initial: prop.fetch(),
          );
          if (font == null || font == prop.fetch()) return;
          prop.put(font);
          unawaited(RNodes.app.notify());
        },
      ),
      keywords: TerminalTypeface.values.map((font) => font.family).join(' '),
    );
  }

  SettingsRow _buildTermFontSize() {
    final label = libL10n.fontSize;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.font_size_line),
        title: TipText(label, l10n.termFontSizeTip),
        trailing: ValBuilder(
          listenable: _setting.termFontSize.listenable(),
          builder: (val) => Text(val.toString(), style: UIs.text15),
        ),
        onTap: () => _showFontSizeDialog(_setting.termFontSize),
      ),
      keywords: l10n.termFontSizeTip,
    );
  }

  SettingsRow _buildTermLineHeight() {
    final label = l10n.termLineHeight;
    final ctrl = TextEditingController(
      text: _setting.termLineHeight.fetch().toString(),
    );
    void onSave() {
      final val = double.tryParse(ctrl.text);
      if (val == null) {
        Toast.error('${libL10n.invalid}: ${ctrl.text}');
        return;
      }
      _setting.termLineHeight.put(
        val.clamp(TerminalLook.minLineHeight, TerminalLook.maxLineHeight),
      );
      context.popSheet();
    }

    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Icons.format_line_spacing),
        title: TipText(label, l10n.termLineHeightTip),
        trailing: ValBuilder(
          listenable: _setting.termLineHeight.listenable(),
          builder: (val) => Text(val.toString(), style: UIs.text15),
        ),
        onTap: () {
          ctrl.text = _setting.termLineHeight.fetch().toString();
          context.showSheet(
            title: label,
            child: Input(
              controller: ctrl,
              autoFocus: true,
              type: const TextInputType.numberWithOptions(decimal: true),
              hint: '1.2',
              icon: Icons.format_line_spacing,
              suggestion: false,
              onSubmitted: (_) => onSave(),
            ),
            actions: Btn.ok(onTap: onSave).toList,
          );
        },
      ),
      keywords: l10n.termLineHeightTip,
    );
  }

  SettingsRow _buildTermTheme() {
    String index2Str(int index) {
      switch (index) {
        case 0:
          return libL10n.auto;
        case 1:
          return libL10n.bright;
        case 2:
          return libL10n.dark;
        default:
          return libL10n.error;
      }
    }

    return SettingsRow(
      libL10n.theme,
      () => ListTile(
        leading: const Icon(MingCute.moon_stars_fill),
        title: Text(libL10n.theme),
        trailing: ValBuilder(
          listenable: _setting.termTheme.listenable(),
          builder: (val) => Text(index2Str(val), style: UIs.text15),
        ),
        onTap: () async {
          final selected = await context.showPickSingleSheet(
            title: libL10n.theme,
            items: List.generate(3, (index) => index),
            display: (p0) => index2Str(p0),
            initial: _setting.termTheme.fetch(),
          );
          if (selected != null) {
            _setting.termTheme.put(selected);
          }
        },
      ),
    );
  }

  SettingsRow _buildSSHWakeLock() {
    final label = l10n.wakeLock;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.lock_fill),
        title: Text(label),
        trailing: StoreSwitch(prop: _setting.sshWakeLock),
      ),
    );
  }

  SettingsRow _buildRememberKeyPassphrase() {
    final label = l10n.rememberKeyPassphrase;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Icons.key),
        title: TipText(label, l10n.rememberKeyPassphraseTip),
        trailing: StoreSwitch(
          prop: _setting.rememberKeyPassphrase,
          // Off means none kept, not merely none added from now on. Off
          // before clearing, so a sync running now takes none after it.
          callback: (on) {
            if (on) return;
            _setting.rememberKeyPassphrase.put(false);
            KeyPassphrases.clear();
          },
        ),
      ),
    );
  }

  SettingsRow _buildProgramStatusAlerts() {
    final label = l10n.programStatusAlerts;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.notification_line),
        title: TipText(label, l10n.programStatusAlertsTip),
        trailing: StoreSwitch(prop: _setting.programStatusAlerts),
      ),
      keywords: l10n.programStatusAlertsTip,
    );
  }

  SettingsRow _buildTermMetricsBar() {
    final label = l10n.termMetricsBar;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Icons.speed_outlined),
        title: TipText(label, l10n.termMetricsBarTip),
        trailing: StoreSwitch(prop: _setting.termMetricsBar),
      ),
      keywords: l10n.termMetricsBarTip,
    );
  }

  SettingsRow _buildLetterCache() {
    final label = l10n.letterCache;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Bootstrap.alphabet),
        title: TipText(label, '${l10n.letterCacheTip}\n${l10n.needRestart}'),
        trailing: StoreSwitch(prop: _setting.letterCache),
      ),
      keywords: l10n.letterCacheTip,
    );
  }

  SettingsRow _buildZellijAutoToggle() {
    final label = l10n.zellijAuto;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Icons.terminal),
        title: Text(label),
        subtitle: Text(l10n.zellijAutoTip, style: UIs.textGrey),
        trailing: StoreSwitch(prop: _setting.zellijAuto),
      ),
      keywords: 'zellij ${l10n.zellijAutoTip}',
    );
  }

  SettingsRow _buildZellijShowSelector() {
    final label = l10n.zellijSessionSelector;
    return SettingsRow(
      label,
      () => _setting.zellijAuto.listenable().listenVal((autoEnabled) {
        return IgnorePointer(
          ignoring: !autoEnabled,
          child: Opacity(
            opacity: autoEnabled ? 1.0 : 0.5,
            child: ListTile(
              leading: const Icon(Icons.list_alt),
              title: Text(label),
              subtitle: Text(l10n.zellijSessionSelectorTip, style: UIs.textGrey),
              trailing: StoreSwitch(prop: _setting.zellijShowSelector),
            ),
          ),
        );
      }),
      keywords: 'zellij ${l10n.zellijSessionSelectorTip}',
    );
  }

  SettingsRow _buildZellijSessionName() {
    final label = l10n.zellijDefaultSessionName;
    return SettingsRow(
      label,
      () => _setting.zellijAuto.listenable().listenVal((autoEnabled) {
        return _setting.zellijSessionName.listenable().listenVal((name) {
          final displayName = name.isEmpty ? 'server_box' : name;
          return IgnorePointer(
            ignoring: !autoEnabled,
            child: Opacity(
              opacity: autoEnabled ? 1.0 : 0.5,
              child: ListTile(
                leading: const Icon(Icons.badge_outlined),
                title: Text(label),
                trailing: Text(displayName, style: UIs.text15),
                onTap: () => _showZellijSessionNameDialog(name),
              ),
            ),
          );
        });
      }),
      keywords: 'zellij',
    );
  }

  Future<void> _showZellijSessionNameDialog(String current) async {
    withTextFieldController((ctrl) async {
      ctrl.text = current;
      void onSave() {
        final name = ctrl.text.trim();
        if (name.isNotEmpty && !ZellijSessions.validName(name)) {
          Toast.error(l10n.zellijInvalidName);
          return;
        }
        _setting.zellijSessionName.put(name);
        // `popSheet`: `context` here is the settings page's, and the dialog
        // is on the root navigator.
        context.popSheet();
      }

      await context.showSheet<bool>(
        title: l10n.zellijSessionName,
        child: Input(
          controller: ctrl,
          autoFocus: true,
          hint: 'server_box',
          suggestion: false,
          onSubmitted: (_) => onSave(),
        ),
        actions: Btn.ok(onTap: onSave).toList,
      );
    });
  }

  SettingsRow _buildHostCertAuthorities() {
    final label = l10n.hostCertAuthorities;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(Icons.verified_user_outlined),
        title: TipText(label, l10n.hostCertAuthoritiesTip),
        trailing: _setting.sshHostCertAuthorities.listenable().listenVal(
          (list) => Text('${list.length}', style: UIs.text15),
        ),
        onTap: _showHostCertAuthorities,
      ),
      keywords: 'ssh certificate ca cert-authority known_hosts',
    );
  }

  Future<void> _showHostCertAuthorities() async {
    final prop = _setting.sshHostCertAuthorities;
    await context.showSheet(
      title: l10n.hostCertAuthorities,
      child: SizedBox(
        width: 480,
        child: prop.listenable().listenVal((list) {
          if (list.isEmpty) {
            return Text(libL10n.empty, style: UIs.textGrey);
          }
          return ListView(
            shrinkWrap: true,
            children: [
              for (final (index, ca) in list.indexed)
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(_hostCertAuthorityLabel(ca)),
                  subtitle: Text(ca.hosts, style: UIs.textGrey),
                  trailing: IconButton(
                    tooltip: libL10n.delete,
                    icon: const Icon(Icons.delete_outline),
                    onPressed: () => _removeHostCertAuthority(index, ca),
                  ),
                ),
            ],
          );
        }),
      ),
      actions: [
        Btn.text(text: libL10n.add, onTap: _addHostCertAuthority),
        Btn.ok(),
      ],
    );
  }

  /// The key's type and comment, which is how a person tells two apart.
  String _hostCertAuthorityLabel(HostCertAuthority ca) =>
      [ca.type, if (ca.comment.isNotEmpty) ca.comment].join(' ');

  /// Asked first, as forgetting a host key is: every host it vouches for
  /// goes back to the first-use prompt.
  Future<void> _removeHostCertAuthority(int index, HostCertAuthority ca) async {
    final ok = await context.showSheet<bool>(
      title: libL10n.attention,
      child: Text(
        libL10n.askContinue('${libL10n.delete} ${_hostCertAuthorityLabel(ca)}'),
      ),
      actions: Btnx.cancelRedOk,
    );
    if (ok != true) return;
    final prop = _setting.sshHostCertAuthorities;
    final list = [...prop.get()];
    // By position: the list may have changed while this was asked.
    if (index < list.length && list[index] == ca) {
      list.removeAt(index);
      prop.put(list);
    }
  }

  void _addHostCertAuthority() {
    withTextFieldController((keyCtrl) async {
      // Not a second `withTextFieldController`: it returns nothing to await,
      // so this callback would end at once and [keyCtrl] be disposed under an
      // open dialog.
      final hostsCtrl = TextEditingController(text: '*');
      try {
        // The dialog's own state, so nothing outlives it: a notifier disposed
        // when this returns would still be listened to through its exit
        // animation.
        String? keyError;
        String? hostsError;
        late StateSetter setErrors;
        void onSave() {
          final ca = HostCertAuthority.parse(
            keyCtrl.text,
            hosts: hostsCtrl.text,
          );
          final prop = _setting.sshHostCertAuthorities;
          final list = prop.get();
          setErrors(() {
            keyError = ca.key == null
                ? l10n.hostCertAuthorityInvalid
                : list.any(ca.sameAs)
                ? l10n.hostCertAuthorityExists
                : null;
            hostsError = ca.hasHosts ? null : l10n.hostCertAuthorityNoHosts;
          });
          if (keyError != null || hostsError != null) return;
          prop.put([...list, ca]);
          context.popSheet();
        }

        await context.showSheet(
          title: libL10n.add,
          child: StatefulBuilder(
            builder: (context, setState) {
              setErrors = setState;
              return Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Input(
                    controller: keyCtrl,
                    autoFocus: true,
                    label: libL10n.key,
                    hint: 'ssh-ed25519 AAAA... ca@example.com',
                    maxLines: 3,
                    suggestion: false,
                    errorText: keyError,
                  ),
                  Input(
                    controller: hostsCtrl,
                    label: libL10n.host,
                    hint: '*.example.com,!bastion.example.com',
                    suggestion: false,
                    errorText: hostsError,
                    onSubmitted: (_) => onSave(),
                  ),
                ],
              );
            },
          ),
          actions: Btn.ok(onTap: onSave).toList,
        );
      } finally {
        _disposeAfterExit(hostsCtrl);
      }
    });
  }
}
