import 'package:server_box/data/model/virt/virt.dart';
import 'package:server_box/data/model/virt/virt_detail.dart';

/// What opening a console needs, fetched by `VirtBackend.console`.
///
/// A handle, not a connection: a PVE ticket is good for a short while and
/// one use, so fetch it right before connecting.
sealed class VirtConsole {
  const VirtConsole();

  VirtConsoleKind get kind;
}

/// A PVE console: `termproxy`, reached through the node's
/// `vncwebsocket`, which `PveBackend.openConsole` opens over the same
/// transport and login as the API calls.
sealed class PveConsole extends VirtConsole {
  const PveConsole({
    required this.node,
    required this.guestKind,
    required this.vmid,
    required this.port,
    required this.ticket,
    required this.user,
  });

  final String node;
  final VirtGuestKind guestKind;
  final int vmid;

  /// The proxy's port on the node, passed back to `vncwebsocket`.
  final int port;

  /// The console ticket. Short-lived; never logged.
  final String ticket;

  /// The PVE user the ticket was issued to.
  final String user;

  @override
  String toString() =>
      '$runtimeType(node: $node, ${guestKind.name}/$vmid, port: $port)';
}

/// PVE `termproxy`: an xterm.js-protocol text console, spoken by
/// `sbm_virt::pve::console::Console`.
final class PveTermConsole extends PveConsole {
  const PveTermConsole({
    required super.node,
    required super.guestKind,
    required super.vmid,
    required super.port,
    required super.ticket,
    required super.user,
  });

  @override
  VirtConsoleKind get kind => VirtConsoleKind.text;
}

/// libvirt serial console: [command] run in a terminal session on the server
/// (it needs a PTY, so it is not an exec).
final class LibvirtSerialConsole extends VirtConsole {
  const LibvirtSerialConsole({required this.command, required this.needsRoot});

  /// `virsh ... console --force --domain '<uuid>'`. Single-quoted arguments
  /// only, so it runs the same under a POSIX shell and fish.
  final String command;

  /// This account reaches the daemon only through sudo: run it as
  /// `sudo <command>`, where the terminal can ask for the password.
  final bool needsRoot;

  @override
  VirtConsoleKind get kind => VirtConsoleKind.text;
}
