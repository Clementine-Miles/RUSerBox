import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/app_theme.dart';
import 'package:server_box/data/store/stores.dart';

class ThemeModeTile extends StatelessWidget {
  const ThemeModeTile({super.key});

  static String _label(ThemeMode mode) => switch (mode) {
    ThemeMode.light => libL10n.bright,
    ThemeMode.dark => libL10n.dark,
    ThemeMode.system => libL10n.auto,
  };

  @override
  Widget build(BuildContext context) {
    final setting = Stores.setting.themeMode;
    final title = libL10n.themeMode;
    return ValBuilder(
      listenable: setting.listenable(),
      builder: (value) => ListTile(
        leading: const Icon(Icons.dark_mode),
        title: Text(title),
        trailing: Text(_label(appThemeMode(value)), style: UIs.text15),
        onTap: () async {
          final selected = await context.showPickSingleSheet(
            title: title,
            items: ThemeMode.values,
            display: _label,
            initial: appThemeMode(value),
          );
          if (selected != null) setting.put(selected.index);
        },
      ),
    );
  }
}
