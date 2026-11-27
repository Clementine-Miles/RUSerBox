import 'dart:io';

import 'package:xterm/core.dart';

/// Which keyboard the person at the terminal is typing on: the **host's**
/// platform, not the server's, since this decides how key presses are turned
/// into bytes.
///
/// Android on a device. The Linux test host answers `linux`; anything else
/// is never reached by this build.
TerminalTargetPlatform get hostTerminalPlatform {
  if (Platform.isAndroid) return TerminalTargetPlatform.android;
  if (Platform.isLinux) return TerminalTargetPlatform.linux;
  return TerminalTargetPlatform.unknown;
}
