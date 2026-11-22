part of '../entry.dart';

extension _Editor on _AppSettingsPageState {
  List<SettingsGroup> _buildEditor() {
    return [
      SettingsGroup(libL10n.general, [
        _buildEditorWrap(),
        _buildEditorHighlight(),
        _buildEditorCloseAfterEdit(),
      ]),
      SettingsGroup(libL10n.font, [
        _buildEditorFontFamily(),
        _buildEditorFontSize(),
      ]),
    ];
  }

  SettingsRow _buildEditorCloseAfterEdit() {
    final label = l10n.closeAfterSave;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.edit_fill),
        title: Text(label),
        trailing: StoreSwitch(prop: _setting.closeAfterSave),
      ),
    );
  }

  SettingsRow _buildEditorHighlight() {
    final label = libL10n.highlight;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.code_line),
        title: TipText(label, l10n.editorHighlightTip),
        trailing: StoreSwitch(prop: _setting.editorHighlight),
      ),
      keywords: l10n.editorHighlightTip,
    );
  }

  SettingsRow _buildEditorWrap() {
    final label = libL10n.softWrap;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.align_center_line),
        title: Text(label),
        trailing: StoreSwitch(prop: _setting.editorSoftWrap),
      ),
    );
  }

  SettingsRow _buildEditorFontSize() {
    final label = libL10n.fontSize;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.font_size_line),
        title: Text(label),
        trailing: ValBuilder(
          listenable: _setting.editorFontSize.listenable(),
          builder: (val) => Text(val.toString(), style: UIs.text15),
        ),
        onTap: () => _showFontSizeDialog(_setting.editorFontSize),
      ),
    );
  }

  SettingsRow _buildEditorFontFamily() {
    final label = libL10n.font;
    return SettingsRow(
      label,
      () => ListTile(
        leading: const Icon(MingCute.font_fill),
        title: Text(label),
        trailing: ValBuilder(
          listenable: _setting.editorFontFamily.listenable(),
          builder: (val) => Text(
            val.isEmpty ? libL10n.auto.toLowerCase() : val,
            style: UIs.text15,
          ),
        ),
        onTap: () => _showFontFamilyDialog(_setting.editorFontFamily),
      ),
    );
  }

  void _showFontFamilyDialog(SqliteProp<String> property) {
    showTextSettingDialog(
      title: libL10n.font,
      initialValue: property.fetch() ?? '',
      label: libL10n.font,
      hint: 'monospace / Consolas / Fira Code ...',
      icon: Icons.font_download,
      onSave: property.put,
    );
  }

  void _showFontSizeDialog(SqliteProp<double> property) {
    final ctrl = TextEditingController(text: property.fetch().toString());
    void onSave() {
      context.popSheet();
      final fontSize = double.tryParse(ctrl.text);
      if (fontSize == null) {
        context.showSheet(
          title: libL10n.fail,
          child: Text('${libL10n.invalid}: ${ctrl.text}'),
        );
        return;
      }
      property.set(fontSize);
    }

    context.showSheet(
      title: libL10n.fontSize,
      child: Input(
        controller: ctrl,
        autoFocus: true,
        type: TextInputType.number,
        icon: Icons.font_download,
        suggestion: false,
        onSubmitted: (_) => onSave(),
      ),
      actions: Btn.ok(onTap: onSave).toList,
    );
  }
}
