import 'dart:async';

import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

abstract final class SystemUIs {
  /// Enables edge-to-edge mode with a transparent navigation bar on Android.
  static void setTransparentNavigationBar(BuildContext context) {
    if (isAndroid) {
      SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
      SystemChrome.setSystemUIOverlayStyle(
        const SystemUiOverlayStyle(
          systemNavigationBarColor: Colors.transparent,
          systemNavigationBarContrastEnforced: false,
        ),
      );
    }
  }

  /// Shows or hides the system status bar.
  ///
  /// - [hide] if true, hides the status bar using immersive sticky mode.
  /// If false, shows the status bar using edge-to-edge mode.
  static void switchStatusBar({required bool hide}) {
    if (hide) {
      SystemChrome.setEnabledSystemUIMode(
        SystemUiMode.immersiveSticky,
        overlays: [],
      );
    } else {
      SystemChrome.setEnabledSystemUIMode(
        SystemUiMode.edgeToEdge,
        overlays: [SystemUiOverlay.top, SystemUiOverlay.bottom],
      );
    }
  }
}

void withTextFieldController(Future<void> Function(TextEditingController) callback) async {
  final controller = TextEditingController();
  try {
    await callback(controller);
    // Wait a moment to ensure any UI updates are processed
    await Future.delayed(const Duration(seconds: 3));
  } finally {
    controller.dispose();
  }
}
