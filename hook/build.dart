import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

import 'rust_build_environment.dart';

/// Builds `crates/sbm_ffi` and hands the result to the Flutter SDK as a code
/// asset.
///
/// The app ships for Android `arm64-v8a` only (Rust target
/// `aarch64-linux-android`). Any other Android architecture is refused here,
/// so an emulator or 32-bit build cannot quietly produce an APK. Linux is the
/// host `flutter test` loads the library on and is built host-native.
void main(List<String> args) async {
  await build(args, (input, output) async {
    if (input.config.buildCodeAssets) {
      final code = input.config.code;
      if (code.targetOS == OS.android &&
          code.targetArchitecture != Architecture.arm64) {
        throw UnsupportedError(
          'RUSerBox ships for Android arm64-v8a only; '
          'got ${code.targetArchitecture}. Build with '
          '--target-platform android-arm64.',
        );
      }
    }
    await FlutterRustBridgeNativeAssetsBuilder(
      cratePath: 'crates/sbm_ffi',
      extraCargoEnvironmentVariables: reproducibleCargoEnvironment(
        environment: Platform.environment,
        packageRoot: Directory.fromUri(input.packageRoot).path,
        isWindows: Platform.isWindows,
      ),
    ).run(input: input, output: output);
  });
}
