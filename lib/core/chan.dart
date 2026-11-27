import 'dart:convert';

import 'package:fl_lib/fl_lib.dart';
import 'package:flutter/services.dart';
import 'package:server_box/core/utils/drop_zones.dart';
import 'package:server_box/data/res/misc.dart';
import 'package:server_box/data/store/stores.dart';

abstract final class MethodChans {
  static const _channel = MethodChannel('${Miscs.pkgName}/main_chan');

  /// Tells the system file picker the installed Linux systems changed, so the
  /// roots listed by `LinuxDocumentsProvider` follow.
  static Future<void> linuxSystemsChanged() async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('linuxSystemsChanged');
    } catch (e, s) {
      Loggers.app.warning('linuxSystemsChanged', e, s);
    }
  }

  /// Stops Android's terminal foreground service after its queued starts have
  /// been handled.
  static Future<void> stopService() async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('stopService');
    } catch (e, s) {
      Loggers.app.warning('Failed to stop Android terminal service', e, s);
    }
  }

  /// Whether leaving the app should start the keep-alive service. Started by
  /// the activity's own `onPause`, since a request from here can arrive after
  /// the app is background and be refused.
  static Future<void> setKeepAliveInBackground(bool value) async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('setKeepAliveInBackground', value);
    } catch (e, s) {
      Loggers.app.warning('Failed to set keep-alive', e, s);
    }
  }

  /// Hide the app's content from the Android app switcher.
  static Future<bool> setPrivacyBlur(bool enabled) async {
    if (!isAndroid) return true;
    try {
      await _channel.invokeMethod('setPrivacyBlur', enabled);
      return true;
    } catch (e, s) {
      Loggers.app.warning('Failed to set privacy blur', e, s);
      return false;
    }
  }

  /// Hold the cover in place after the app comes forward, until Flutter has
  /// decided whether a biometric lock is coming.
  ///
  /// Without it the cover comes off the moment the app is frontmost, and the
  /// real UI is on screen for however many frames it takes Flutter to hear
  /// about the lifecycle change at all.
  static bool? _privacyBlurLocked;

  static Future<void> setPrivacyBlurLocked(bool locked) async {
    if (!isAndroid) return;
    if (_privacyBlurLocked == locked) return;
    _privacyBlurLocked = locked;
    try {
      await _channel.invokeMethod('setPrivacyBlurLocked', locked);
    } catch (e, s) {
      _privacyBlurLocked = null;
      Loggers.app.warning('Failed to set privacy blur lock', e, s);
    }
  }

  /// Starts or updates Android's terminal foreground service with [payload].
  ///
  /// The native side also owns the notification permission request, so one
  /// call describes the desired state instead of racing a separate start.
  static Future<void> updateSessions(String payload) async {
    if (!isAndroid) return;
    try {
      Loggers.app.info('Updating Android sessions: $payload');
      await _channel.invokeMethod('updateSessions', payload);
    } on PlatformException catch (e, s) {
      // A denied notification permission is a supported state: Android cannot
      // run the foreground service, and the settings page explains how to
      // enable it. Do not turn every ordinary session sync into a warning.
      if (e.code == 'NOTIFICATION_PERMISSION_DENIED') return;
      Loggers.app.warning('Failed to update Android sessions', e, s);
    } catch (e, s) {
      Loggers.app.warning('Failed to update Android sessions', e, s);
    }
  }

  /// Whether Android will let this app post notifications.
  ///
  /// Read by the settings page rather than acted on: without the permission
  /// there is no foreground service, and without that the system freezes the
  /// process as soon as it is backgrounded — so `bgRun` is a switch that cannot
  /// keep its promise, and saying nothing about it leaves the user with a
  /// connection that drops for no reason they can see (#1287).
  ///
  /// True off Android, where the question does not arise.
  static Future<bool> notificationsAllowed() async {
    if (!isAndroid) return true;
    try {
      return await _channel.invokeMethod('notificationsAllowed') == true;
    } catch (e, s) {
      Loggers.app.warning('Failed to read the notification permission', e, s);
      // Assumed granted: a failure here is this app's, and reporting it as the
      // user's problem would send them to a settings page to fix nothing.
      return true;
    }
  }

  /// Opens this app's notification settings, for the case above.
  static Future<void> openNotificationSettings() async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('openNotificationSettings');
    } catch (e, s) {
      Loggers.app.warning('Failed to open the notification settings', e, s);
    }
  }

  /// Query whether the Android foreground service is currently running.
  /// How the process ended last time, as Android recorded it.
  ///
  /// The only way this app can see a native crash: one takes the process with
  /// it, so nothing in Dart runs afterwards and nothing is written. The system
  /// keeps the record instead, and this reads it on the next launch.
  ///
  /// Null below API 30, and null when there is nothing recorded. Keys:
  /// `reason` (a name — `crash_native`, `anr`, `low_memory`,
  /// `user_requested`, …), `timestamp` (ms, and the only thing distinguishing
  /// one record from another), `description`, `status`, `importance`, and one
  /// of two traces: `trace`, a `String`, for an ANR, or `traceProto`, the
  /// bytes of a `Tombstone` protocol buffer, for a native crash on API 31+.
  /// Both are absent for every other reason, and either can be absent anyway —
  /// the traces live in a global circular buffer that another app's crash can
  /// evict.
  static Future<Map<String, Object?>?> lastExitInfo() async {
    if (!isAndroid) return null;
    try {
      final res = await _channel.invokeMapMethod<String, Object?>(
        'lastExitInfo',
      );
      return res;
    } catch (e, s) {
      Loggers.app.warning('Failed to read the last exit info', e, s);
      return null;
    }
  }

  static Future<bool> isServiceRunning() async {
    if (!isAndroid) return false;
    try {
      final res = await _channel.invokeMethod('isServiceRunning');
      return res == true;
    } catch (e, s) {
      Loggers.app.warning(
        'Failed to check if Android service is running',
        e,
        s,
      );
      return false;
    }
  }

  /// Reads and clears the `serverbox://` link held after an Android intent.
  static Future<String?> takeOpenedLink() async {
    if (!isAndroid) return null;
    try {
      return await _channel.invokeMethod<String>('takeOpenedLink');
    } catch (e, s) {
      Loggers.app.warning('Failed to read the opened link', e, s);
      return null;
    }
  }

  /// What another app shared, or sent from its text selection, as the JSON
  /// `{"kind":"text","text"}` or `{"kind":"files","paths"}`. Android only;
  /// held natively and pulled like [takeOpenedLink]. Cleared by the read.
  static Future<Map<String, Object?>?> takeIncoming() async {
    if (!isAndroid) return null;
    try {
      final raw = await _channel.invokeMethod<String>('takeIncoming');
      if (raw == null) return null;
      return (jsonDecode(raw) as Map).cast<String, Object?>();
    } catch (e, s) {
      Loggers.app.warning('Failed to read incoming content', e, s);
      return null;
    }
  }

  /// Deletes the batches holding [paths], which `IncomingFiles` copied in.
  /// The native side decides what is a batch; anything else is left alone.
  static Future<void> discardIncoming(List<String> paths) async {
    if (!isAndroid || paths.isEmpty) return;
    try {
      await _channel.invokeMethod('discardIncoming', paths);
    } catch (e, s) {
      Loggers.app.warning('Failed to discard incoming files', e, s);
    }
  }

  /// The launcher's long-press list: `[{"id","label","uri"}]`, in order.
  static Future<void> setShortcuts(String payload) async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('setShortcuts', payload);
    } catch (e, s) {
      Loggers.app.warning('Failed to set launcher shortcuts', e, s);
    }
  }

  /// Asks the launcher to pin a shortcut; false when it cannot.
  static Future<bool> pinShortcut({
    required String id,
    required String label,
    required String uri,
  }) async {
    if (!isAndroid) return false;
    try {
      return await _channel.invokeMethod<bool>('pinShortcut', {
            'id': id,
            'label': label,
            'uri': uri,
          }) ??
          false;
    } catch (e, s) {
      Loggers.app.warning('Failed to pin a shortcut', e, s);
      return false;
    }
  }

  /// The transfer notification: `{"title","text","progress","chip"}`, or null
  /// to take it down.
  static Future<void> updateTransfers(String? payload) async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('updateTransfers', payload);
    } catch (e, s) {
      Loggers.app.warning('Failed to update the transfer notification', e, s);
    }
  }

  /// The keys listed by the system's shortcut list (Meta+/):
  /// `[{"label","items":[{"label","chord"}]}]`.
  static Future<void> setKeyboardShortcuts(String payload) async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('setKeyboardShortcuts', payload);
    } catch (e, s) {
      Loggers.app.warning('Failed to set keyboard shortcuts', e, s);
    }
  }

  /// The per-app language of Android 13+: `current`, the system's tags
  /// (empty for the system language), and `agreed`, what it held right after
  /// the app last set it (null before the first time). Null below 13.
  static Future<({String current, String? agreed})?> appLocale() async {
    if (!isAndroid) return null;
    try {
      final r = await _channel.invokeMapMethod<String, Object?>('appLocale');
      if (r == null) return null;
      return (current: r['current'] as String? ?? '', agreed: r['agreed'] as String?);
    } catch (e, s) {
      Loggers.app.warning('Failed to read the app locale', e, s);
      return null;
    }
  }

  /// Sets the per-app language, and records what the system then holds as
  /// `agreed`.
  static Future<void> setAppLocale(String tag) async {
    if (!isAndroid) return;
    try {
      await _channel.invokeMethod('setAppLocale', tag);
    } catch (e, s) {
      Loggers.app.warning('Failed to set the app locale', e, s);
    }
  }

  /// Its own channel, deliberately.
  ///
  /// A channel has exactly one handler and [registerHandler] already owns the
  /// main one, so a second `setMethodCallHandler` on that name would silently
  /// replace it. Separate names cannot collide however either side grows.
  static const _shareChannel = MethodChannel(
    '${Miscs.pkgName}/incoming_share',
  );

  /// Runs [onOpened] when the platform hands this app a share or a link while
  /// it is **already frontmost**.
  static void onOpened(VoidCallback onOpened) {
    if (!isAndroid) return;
    _shareChannel.setMethodCallHandler((call) async {
      if (call.method == 'opened') onOpened();
    });
  }

  /// Register a handler for native -> Flutter callbacks.
  /// Currently handles:
  /// - `stopAllConnections` with no arguments
  /// - `notificationPermissionGranted` with no arguments
  /// - `activityPaused` / `activityResumed` with no arguments
  static void registerHandler({
    VoidCallback? onStopAll,
    VoidCallback? onNotificationPermissionGranted,
    void Function(bool backgrounded)? onBackgrounded,
  }) {
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'stopAllConnections':
          onStopAll?.call();
          return null;
        // Asked before a drop from another app is copied in: see [DropZones].
        case 'acceptsDrop':
          return DropZones.accepting;
        // Android asks for the permission asynchronously, so the call that
        // triggered the prompt has already been refused by the time the user
        // answers it. This is the only edge that says the answer was yes, and
        // without acting on it the foreground service stays stopped until
        // something else happens to sync — see [updateSessions].
        case 'notificationPermissionGranted':
          onNotificationPermissionGranted?.call();
          return null;
        case 'activityPaused':
          onBackgrounded?.call(true);
          return null;
        case 'activityResumed':
          onBackgrounded?.call(false);
          return null;
        default:
          return null;
      }
    });
  }
}
