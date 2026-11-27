import 'dart:convert';

import 'package:fl_lib/fl_lib.dart';

/// Decodes SSH output: UTF-8, malformed sequences replaced.
///
/// Every host this app administers speaks UTF-8; the GBK fallback that only a
/// Windows host needed went with Windows support.
class SSHDecoder {
  static String decode(List<int> bytes, {String? context}) {
    if (bytes.isEmpty) return '';
    try {
      return utf8.decode(bytes, allowMalformed: true);
    } catch (e) {
      final contextInfo = context != null ? ' [$context]' : '';
      Loggers.app.warning('UTF-8 decode failed$contextInfo: $e');
      return '';
    }
  }
}
