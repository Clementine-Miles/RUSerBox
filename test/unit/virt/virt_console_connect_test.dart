/// Which command a libvirt serial console runs. The terminal page's arguments per
/// host are asserted in `test/widget/virt_tab_test.dart`, over the tab's
/// providers.
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/virt/virt_console.dart';
import 'package:server_box/view/page/virt/console_connect.dart';

void main() {
  const command =
      "virsh --connect qemu:///system console --force --domain 'a b'";

  test('the serial console runs as this account unless libvirt needs root', () {
    expect(
      VirtConsoleConnect.serialCommand(
        const LibvirtSerialConsole(command: command, needsRoot: false),
      ),
      command,
    );
    expect(
      VirtConsoleConnect.serialCommand(
        const LibvirtSerialConsole(command: command, needsRoot: true),
      ),
      'sudo $command',
    );
  });

  test("Disconnect sends virsh console's escape, Ctrl+]", () {
    expect(VirtConsoleConnect.serialEscape, [0x1d]);
  });
}
