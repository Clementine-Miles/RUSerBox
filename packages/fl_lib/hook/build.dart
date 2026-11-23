import 'dart:io';

import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

import 'rust_build_environment.dart';

/// Builds the LK Cloud and sync bridge as a Dart native asset.
void main(List<String> args) async {
  await build(args, (input, output) async {
    await FlutterRustBridgeNativeAssetsBuilder(
      cratePath: 'rust/ffi',
      extraCargoEnvironmentVariables: reproducibleCargoEnvironment(
        environment: Platform.environment,
        packageRoot: Directory.fromUri(input.packageRoot).path,
        isWindows: Platform.isWindows,
      ),
    ).run(input: input, output: output);
  });
}
