import 'package:fl_lib/fl_lib.dart';
import 'package:server_box/core/monitor/http.dart';
import 'package:server_box/data/model/server/server_private_info.dart';
import 'package:server_box/data/store/stores.dart';

/// Revokes the read-only Monitor tokens the retired home-screen widget had
/// minted (`client_id` `widget:<server id>`), once per install.
///
/// The widget is gone, so nothing renews or uses them, but each stays valid on
/// its agent for up to ninety days. Best effort: an agent that cannot be
/// reached keeps its token until it expires, and is not retried, since
/// retrying at every launch would mean a request to every Monitor on every
/// start for the rest of the install's life. The device-side copies are
/// deleted by the Android side (`RetiredWidgetData`).
abstract final class RetiredWidgetTokens {
  static const _doneKey = 'retiredWidgetTokensRevoked';

  static Future<void> revokeOnce() async {
    final store = Stores.setting;
    if (store.get<bool>(_doneKey) == true) return;
    // Whether this install ever had a widget is not recorded anywhere this
    // build reads, so every Monitor server is asked; an agent with no such
    // token answers the delete as a no-op.
    final servers = Stores.server.fetch().where((spi) => spi.monitorOn != null);
    for (final spi in servers) {
      await _revoke(spi);
    }
    store.set(_doneKey, true, updateLastUpdateTsOnSet: false);
  }

  static Future<void> _revoke(Spi spi) async {
    final monitor = spi.monitorOn;
    if (monitor == null) return;
    MonitorHttpClient? client;
    try {
      client = MonitorHttpClient(monitor);
      await client.revokeWatchToken('widget:${spi.id}');
    } catch (e) {
      Loggers.app.info('Revoking the retired widget token of ${spi.id}: $e');
    } finally {
      client?.dispose();
    }
  }
}
