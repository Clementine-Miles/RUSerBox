import 'dart:async';
import 'dart:io';

import 'package:fl_lib/fl_lib.dart' show FlRust;
import 'package:fl_pi_llm/fl_pi_llm.dart';
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
  if (_nativeAsset('fl_pi_llm') case final lib?) {
    await FlPiLlm.loadLibrary(externalLibrary: lib);
  }
  await testMain();
}

ExternalLibrary? _nativeAsset(String name) {
  final file = File(switch (Platform.operatingSystem) {
    'macos' => 'build/native_assets/macos/lib$name.dylib',
    'windows' => 'build/native_assets/windows/$name.dll',
    _ => 'build/native_assets/linux/lib$name.so',
  });
  return file.existsSync() ? ExternalLibrary.open(file.path) : null;
}
