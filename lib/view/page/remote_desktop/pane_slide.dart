import 'package:material_ui/material_ui.dart';
import 'package:server_box/view/widget/page_transitions.dart';

class RemoteDesktopPaneSlide extends StatelessWidget {
  const RemoteDesktopPaneSlide({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Theme(
      data: theme.copyWith(pageTransitionsTheme: AppPageTransitions.paneSlide),
      child: child,
    );
  }
}
