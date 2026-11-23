import 'package:device_info_plus/device_info_plus.dart';
import 'package:fl_lib/fl_lib.dart';
import 'package:sentry/sentry.dart' as sentry;

/// Adds the Android hardware model, architecture and OS release to reports.
abstract final class DiagnosticsPlatform {
  /// Attaches the contexts to the current Sentry scope.
  ///
  /// Best effort by construction. Every platform channel here can fail on a
  /// device that is unusual in some way, and a report with no device section
  /// is worth incomparably more than no report — so a failure is logged and
  /// swallowed rather than propagated into the caller, which is the code path
  /// that has just finished starting crash reporting.
  ///
  /// Awaited by its caller rather than fired off, so that an error arriving
  /// moments after launch is already described. It is one platform call.
  static Future<void> describe() async {
    try {
      if (!isAndroid) return;
      final (device, os) = await _android(DeviceInfoPlugin());
      await sentry.Sentry.configureScope((scope) {
        scope.setContexts(sentry.SentryDevice.type, device);
        scope.setContexts(sentry.SentryOperatingSystem.type, os);
      });
    } catch (e, s) {
      Loggers.app.warning('DiagnosticsPlatform.describe', e, s);
    }
  }

  static Future<(sentry.SentryDevice, sentry.SentryOperatingSystem)> _android(
    DeviceInfoPlugin plugin,
  ) async {
    final info = await plugin.androidInfo;
    return (
      sentry.SentryDevice(
        manufacturer: info.manufacturer,
        brand: info.brand,
        model: info.model,
        // `supportedAbis` is ordered best-first, so the head is what this
        // process is actually running as — which is the question a native
        // stack frame raises.
        arch: info.supportedAbis.firstOrNull,
        simulator: !info.isPhysicalDevice,
        lowMemory: info.isLowRamDevice,
      ),
      sentry.SentryOperatingSystem(
        name: 'Android',
        version: info.version.release,
        // The API level, which is what a platform behaviour is keyed on and
        // what `ApplicationExitInfo` availability follows.
        build: '${info.version.sdkInt}',
      ),
    );
  }
}
