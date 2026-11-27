import 'dart:async';
import 'dart:io';

import 'package:fl_lib/fl_lib.dart' show FlRust;
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart' show ExternalLibrary;
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/core/service/monitor_login_devices.dart';

/// Runs before every test file.
///
/// Loads the shared Rust libraries from the test build's native assets.
Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  // The platform Keychain/Keystore is not there under `flutter test`, and
  // what one test's agent handed out is not the next test's.
  MonitorLoginDevices.store = MemoryDeviceTokenStore();
  setUp(MonitorLoginDevices.resetForTest);
  if (_nativeAsset('fl_lib_ffi') case final lib?) {
    await FlRust.ensureInitialized(library: lib);
  }
  await testMain();
}

ExternalLibrary? _nativeAsset(String name) {
  // Tests run on a Linux host; the app itself ships for Android only.
  final file = File('build/native_assets/linux/lib$name.so');
  return file.existsSync() ? ExternalLibrary.open(file.path) : null;
}
