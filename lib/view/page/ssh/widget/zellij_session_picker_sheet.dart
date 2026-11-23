import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/core/extension/context/locale.dart';
import 'package:server_box/data/terminal/zellij.dart';

enum ZellijAction { attach, create, detach, terminate }

typedef ZellijChoice = ({ZellijAction action, String name});

Future<ZellijChoice?> showZellijSessionPickerSheet(
  BuildContext context, {
  required List<String> sessions,
  required String defaultSessionName,
  String? currentSessionName,
  bool allowTerminate = true,
}) {
  final l10n = context.l10n;
  final existingNames = sessions.toSet();
  return showResizableSheet<ZellijChoice>(
    context,
    builder: (sheetContext) => ListView(
      shrinkWrap: true,
      padding: const EdgeInsets.only(bottom: 17),
      children: [
        _ZellijNewSessionRow(
          suggestedSessionName: _uniqueName(defaultSessionName, existingNames),
          existingNames: existingNames,
        ),
        if (sessions.isNotEmpty) const Divider(height: 1),
        for (final name in sessions)
          ListTile(
            selected: name == currentSessionName,
            leading: Icon(
              name == currentSessionName ? Icons.link : Icons.link_outlined,
            ),
            title: Text(name),
            onTap: () => Navigator.of(
              sheetContext,
            ).pop((action: ZellijAction.attach, name: name)),
            trailing: !allowTerminate
                ? null
                : IconButton(
                    tooltip: l10n.zellijTerminate,
                    icon: const Icon(Icons.stop_circle_outlined),
                    onPressed: () => Navigator.of(
                      sheetContext,
                    ).pop((action: ZellijAction.terminate, name: name)),
                  ),
          ),
        if (currentSessionName != null) ...[
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.link_off_outlined),
            title: Text(l10n.zellijDetach),
            onTap: () => Navigator.of(
              sheetContext,
            ).pop((action: ZellijAction.detach, name: currentSessionName)),
          ),
        ],
      ],
    ),
  );
}

final class _ZellijNewSessionRow extends StatelessWidget {
  final String suggestedSessionName;
  final Set<String> existingNames;

  const _ZellijNewSessionRow({
    required this.suggestedSessionName,
    required this.existingNames,
  });

  @override
  Widget build(BuildContext context) {
    final l10n = context.l10n;
    return ListTile(
      leading: const Icon(Icons.add),
      title: Text(l10n.zellijNewSession),
      subtitle: Text(
        suggestedSessionName.isEmpty
            ? l10n.zellijSessionName
            : suggestedSessionName,
        style: UIs.text11Grey,
      ),
      onTap: () => _showNameDialog(context),
    );
  }

  Future<void> _showNameDialog(BuildContext sheetContext) async {
    // The text field can still be attached while the dialog route animates out,
    // so let the route dispose the controller and the name notifier together.
    final controller = TextEditingController(text: suggestedSessionName);
    final name = ValueNotifier<String>(suggestedSessionName);

    bool isValid(String value) {
      final trimmed = value.trim();
      return ZellijSessions.validName(trimmed) &&
          !existingNames.contains(trimmed);
    }

    final result = await sheetContext.showSheet<String>(
      title: sheetContext.l10n.zellijNewSession,
      childBuilder: (dialogContext) => DisposeWith(
        notifiers: [controller, name],
        child: ValueListenableBuilder<String>(
          valueListenable: name,
          builder: (context, value, _) {
            final trimmed = value.trim();
            return Input(
              controller: controller,
              hint: dialogContext.l10n.zellijSessionName,
              label: dialogContext.l10n.zellijNewSession,
              action: TextInputAction.done,
              suggestion: false,
              autoFocus: true,
              errorText: existingNames.contains(trimmed)
                  ? context.l10n.nameAlreadyExistsFmt(trimmed)
                  : !ZellijSessions.validName(trimmed)
                  ? context.l10n.zellijInvalidName
                  : null,
              onChanged: (text) => name.value = text,
              onSubmitted: (text) {
                name.value = text;
                if (isValid(text)) {
                  Navigator.of(dialogContext).pop(text.trim());
                }
              },
            );
          },
        ),
      ),
      actionsBuilder: (dialogContext) => [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(),
          child: Text(libL10n.cancel),
        ),
        ValueListenableBuilder<String>(
          valueListenable: name,
          builder: (context, value, _) => TextButton(
            onPressed: isValid(value)
                ? () => Navigator.of(dialogContext).pop(value.trim())
                : null,
            child: Text(libL10n.ok),
          ),
        ),
      ],
    );
    // The dialog is on the root navigator and can outlive the sheet.
    if (result == null || result.isEmpty || !sheetContext.mounted) return;
    Navigator.of(sheetContext).pop((action: ZellijAction.create, name: result));
  }
}

String _uniqueName(String preferred, Set<String> existingNames) {
  final name = preferred.trim();
  if (name.isEmpty || !existingNames.contains(name)) return name;

  for (var suffix = 2; ; suffix++) {
    final candidate = '$name-$suffix';
    if (!existingNames.contains(candidate)) return candidate;
  }
}
