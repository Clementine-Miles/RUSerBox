import 'package:server_box/data/store/schema.dart';
import 'package:server_box/data/store/setting.dart';

/// Moves the last tab out of the five-button home bar used by older builds.
///
/// The setting used to contain every tab available to the home page. The
/// current setting contains only the tabs shown in the bar, with the rest
/// reached through "more", so the old five-item default has to be converted
/// once. A custom order is preserved; only the old set of five tabs is
/// recognized, which keeps this migration from changing a newer arrangement.
class HomeTabsBarMigration extends SettingsMigration {
  const HomeTabsBarMigration();


  static const appliedAt = 21;
  static const key = 'homeTabs';
  /// By name: `agent` is a retired tab this build no longer parses, and the
  /// step must still recognise the bar it was written for.
  static const legacyTabs = {'server', 'ssh', 'file', 'snippet', 'agent'};

  @override
  int get from => appliedAt;

  @override
  Future<void> apply() async => applySync();

  @override
  void applySync() {
    final store = SettingStore.instance;
    final raw = store.get<Object>(key);
    if (raw is! List) return;

    final tabs = [for (final e in raw) _name(e)];
    if (tabs.length != legacyTabs.length ||
        !tabs.toSet().containsAll(legacyTabs)) {
      return;
    }

    final ok = store.set(
      key,
      tabs.take(tabs.length - 1).toList(),
      updateLastUpdateTsOnSet: false,
    );
    if (!ok) throw StateError('m021: writing "$key" failed');
  }

  /// A stored element as a tab name; an integer by the positions the builds
  /// that wrote integers used.
  static String? _name(Object? e) => switch (e) {
    final String s => s,
    final int i when i >= 0 && i < _legacyIndexNames.length => _legacyIndexNames[i],
    _ => null,
  };

  static const _legacyIndexNames = ['server', 'ssh', 'file', 'snippet', 'agent'];
}
