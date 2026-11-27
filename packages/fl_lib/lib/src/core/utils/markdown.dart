import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:markdown/markdown.dart' as md;

abstract final class MarkdownUtils {
  static void onLinkTap(String? text, String? href, String title) {
    if (href == null) return;
    href.launchUrl();
  }

  static final extensionSet = md.ExtensionSet(
    [
      ...md.ExtensionSet.gitHubFlavored.blockSyntaxes,
    ],
    [md.EmojiSyntax(), ...md.ExtensionSet.gitHubFlavored.inlineSyntaxes],
  );

  static final extensionSetWithoutCode = md.ExtensionSet(
    md.ExtensionSet.gitHubFlavored.blockSyntaxes,
    [md.EmojiSyntax(), ...md.ExtensionSet.gitHubFlavored.inlineSyntaxes],
  );

  static final defaultGrey = MarkdownStyleSheet(p: UIs.textGrey);
}
