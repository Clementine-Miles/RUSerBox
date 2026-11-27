import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/store/migrations/all.dart';
import 'package:server_box/data/store/migrations/m024_remote_desktop_profiles.dart';
import 'package:server_box/data/store/migrations/m043_retire_removed_features.dart';
import 'package:server_box/data/store/schema.dart';
import 'package:server_box/data/store/tables.dart';
import '../helpers/table_names.dart';

List<(String, String, bool, int)> _columns() => [
  for (final row in SqliteDb.instance.select(
    'PRAGMA table_info(remote_desktop_profile);',
  ))
    (
      row['name'] as String,
      row['type'] as String,
      (row['notnull'] as int) == 1,
      row['pk'] as int,
    ),
];

void main() {
  setUp(() => SqliteDb.openInMemory());

  tearDown(() async {
    await closeTables();
    await SqliteDb.close();
  });

  test('is registered at its own step', () {
    // The chain's last step is asserted where that step lives; what matters
    // here is that this one is in the list at the version it claims, since a
    // step missing from `kSchemaMigrations` is a launch that throws on a real
    // install and nothing in this file's other tests would notice.
    expect(const RemoteDesktopProfilesMigration().from, 24);
    expect(
      kSchemaMigrations.whereType<RemoteDesktopProfilesMigration>(),
      hasLength(1),
    );
    expect(SchemaVersion.current, greaterThan(24));
  });

  // The table is retired (m043): a fresh database has none, and the columns
  // below are what this released step wrote.
  test('creates the table its release used, and m043 drops it', () async {
    await createTables(SqliteDb.instance);
    expect(_columns(), isEmpty, reason: 'not in a fresh database');
    expect(Tables.syncRoots, isNot(contains('remote_desktop_profile')));

    await const RemoteDesktopProfilesMigration().apply();
    expect(_columns().map((c) => c.$1), [
      'updated_at', 'rev', 'id', 'server_id', 'name', 'protocol', 'host', //
      'port', 'username', 'password', 'domain', 'view_only', 'shared',
      'trusted_cert_sha256',
    ]);

    await const RetireRemovedFeaturesMigration().apply();
    expect(_columns(), isEmpty);
  });

  test('is safe to resume after the table was already created', () async {
    await createTables(SqliteDb.instance);
    await const RemoteDesktopProfilesMigration().apply();
    final before = _columns();
    await const RemoteDesktopProfilesMigration().apply();
    expect(_columns(), before);
  });
}
