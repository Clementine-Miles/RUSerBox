import 'package:dartssh2/dartssh2.dart';
import 'package:fl_lib/fl_lib.dart';
import 'package:server_box/core/extension/context/locale.dart';
import 'package:server_box/core/ssh/config.dart';
import 'package:server_box/data/model/app/error.dart';
import 'package:server_box/data/model/server/ssh_credential.dart';

/// Whether this build can reach an ssh-agent: a Unix socket, such as one an
/// `ssh-agent` in the local Linux userland listens on. Always true on
/// Android; kept as a getter so a caller states the question it asks.
bool get sshAgentSupported => true;

/// [ssh]'s agent, connected, for an [SSHClient] to bind and ask for keys;
/// null when it names none.
///
/// An agent that cannot be reached, or one this build cannot reach at all (a
/// server set up on a desktop, synced to a phone), is an error only when it
/// was the one way in: with [hasOtherAuth] (a password, a key) the connection
/// goes on without it, as OpenSSH's does.
Future<SSHAgentClient?> openSshAgent(
  SshCredential ssh, {
  required bool hasOtherAuth,
  String? originalHost,
}) async {
  final name = ssh.identityAgent;
  if (name == null) return null;
  if (!sshAgentSupported) {
    // Nothing to warn about every connection: the server says which agent it
    // uses on the machine it was set up on.
    if (hasOtherAuth) return null;
    throw SSHErr(type: SSHErrType.agent, message: l10n.sshAgentUnsupported);
  }
  final at = _resolve(ssh, name, originalHost);
  final path = at.path;
  if (path == null) return _unavailable(at.problem!, null, hasOtherAuth);
  try {
    return await _connect(path);
  } catch (e) {
    return _unavailable(l10n.sshAgentUnreachable(path, '$e'), e, hasOtherAuth);
  }
}

/// A new connection to [ssh]'s agent at each call, for each channel a server
/// forwards it on; null when it names none this build can reach. Whether it
/// answered at sign-in is not asked: one started since is still forwarded.
Future<SSHAgentClient> Function()? sshAgentConnector(
  SshCredential ssh, {
  String? originalHost,
}) {
  final name = ssh.identityAgent;
  if (name == null || !sshAgentSupported) return null;
  final path = _resolve(ssh, name, originalHost).path;
  if (path == null) return null;
  return () => _connect(path);
}

/// The socket or pipe [name] names for [ssh], or why there is none.
({String? path, String? problem}) _resolve(
  SshCredential ssh,
  String name,
  String? originalHost,
) {
  final path =
      SSHConfig.expandIdentityAgent(
        name,
        hostname: ssh.ip,
        remoteUser: ssh.user,
        originalHost: originalHost,
        port: ssh.port,
      );
  if (path == null) return (path: null, problem: l10n.sshAgentUnset(name));
  return (path: path, problem: null);
}

Future<SSHAgentClient> _connect(String path) async {
  const timeout = Duration(seconds: 5);
  return SSHAgentClient(await SSHSocket.connectUnix(path, timeout: timeout));
}

Null _unavailable(String message, Object? cause, bool hasOtherAuth) {
  if (!hasOtherAuth) {
    throw SSHErr(type: SSHErrType.agent, message: message, cause: cause);
  }
  Loggers.app.warning('$message; trying the other methods', cause);
  return null;
}
