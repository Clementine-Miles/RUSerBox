import 'package:fl_lib/fl_lib.dart';
import 'package:icons_plus/icons_plus.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/data/res/icon_style.dart';
import 'package:server_box/data/store/stores.dart';

/// A navigation icon from the selected built-in icon family.
class AppIcon extends StatelessWidget {
  const AppIcon(this.icon, {super.key, this.size, this.color});

  final IconData icon;
  final double? size;
  final Color? color;

  @override
  Widget build(BuildContext context) => ValBuilder(
    listenable: Stores.setting.appIconStyle.listenable(),
    builder: (style) => Icon(
      style == IconStyle.mingcute ? _mingcuteIcons[icon] ?? icon : icon,
      size: size,
      color: color,
    ),
  );
}

final _mingcuteIcons = <IconData, IconData>{
  Icons.more_horiz: MingCute.more_1_line,
  Icons.more_vert: MingCute.more_2_line,
  Icons.settings_outlined: MingCute.settings_2_line,
  Icons.settings: MingCute.settings_2_fill,
  Icons.tune: MingCute.settings_2_line,
  Icons.privacy_tip_outlined: MingCute.shield_line,
  Icons.auto_awesome_outlined: MingCute.magic_2_line,
  Icons.tab_outlined: MingCute.dashboard_line,
  Icons.dns_outlined: MingCute.server_line,
  Icons.sort: MingCute.sort_ascending_line,
  Icons.terminal: MingCute.terminal_line,
  Icons.folder_outlined: MingCute.folder_line,
  Icons.cloud_outlined: MingCute.cloud_line,
  Icons.edit_note: MingCute.code_line,
  Icons.inbox_outlined: MingCute.inbox_line,
  Icons.key_outlined: MingCute.key_2_line,
  Icons.info_outline: MingCute.information_line,
  Icons.file_download_outlined: MingCute.download_line,
  Icons.desktop_windows_outlined: MingCute.computer_line,
};
