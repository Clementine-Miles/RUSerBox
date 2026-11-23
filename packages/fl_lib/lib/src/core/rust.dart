import 'package:fl_lib/src/rust/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;

/// Loads the LK Cloud and sync bridge once.
abstract final class FlRust {
  static Future<void>? _init;

  /// Loads the library; [library] is for tests, which load the one cargo built.
  static Future<void> ensureInitialized({ExternalLibrary? library}) =>
      _init ??= RustLib.init(externalLibrary: library);

  static bool get ready => RustLib.instance.initialized;
}
