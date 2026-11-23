import 'package:server_box/data/model/server/server_private_info.dart';
import 'package:server_box/data/model/terminal/virtual_key.dart';
import 'package:server_box/data/terminal/terminal_source.dart';

extension VirtualKeySupport on VirtualKey {
  /// Whether a terminal on [source] can do what this key does.
  ///
  /// Some of these keys act on a *server*, and on a shell that is not on one
  /// they returned without a word: the strip drew them, they took a tap, and
  /// nothing happened. The strip now draws them disabled. Answered here beside
  /// the rest of what a key is, rather
  /// than in the page that draws them, so the toolbar and the strip cannot
  /// come to different conclusions about the same button.
  bool worksOn(TerminalSource source, {bool shellUsesAgent = false}) {
    final spi = switch (source) {
      ServerSource(:final spi) => spi,
      LocalSource() || ConsoleSource() => null,
    };
    return switch (this) {
      // Opens the files of the server this shell is on. This device has its own
      // browser, in the files tab.
      VirtualKey.sftp => spi != null,
      // Inserts the password stored for this server. There is none for a shell
      // that is not on one, none worth inserting for a session already root, and
      // none at all on a server reached only through its monitor agent — `Spi.ssh`
      // is where the password lives, and a monitor server carries no
      // `SshCredential`. `isRoot` alone answered false for those, so the key was
      // drawn on a terminal where tapping it could only show an empty result.
      VirtualKey.sudo => spi?.ssh != null && !spi!.isRoot,
      // The session manager uses SSH exec beside the interactive SSH PTY.
      VirtualKey.zellij => spi?.sshOn != null && !shellUsesAgent,
      // Everything else is the terminal's own — keys, modifiers,
      // the IME, and snippets, which are a script typed into whatever is there.
      _ => true,
    };
  }
}
