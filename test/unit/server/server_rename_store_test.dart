
import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/server/port_forward.dart';
import 'package:server_box/data/model/server/server_private_info.dart';
import 'package:server_box/data/model/server/snippet.dart';
import 'package:server_box/data/model/server/ssh_credential.dart';
import 'package:server_box/data/store/port_forward.dart';
import 'package:server_box/data/store/server.dart';
import 'package:server_box/data/store/snippet.dart';

import '../../helpers/test_db.dart';

void main() {
  late ServerStore servers;
  late PortForwardStore forwards;
  late SnippetStore snippets;

  const original = Spi(
    id: 'server-old',
    name: 'production',
    ssh: SshCredential(ip: '10.0.0.1'),
  );
  const jumpOwner = Spi(
    id: 'jump-owner',
    name: 'through-production',
    ssh: SshCredential(ip: '10.0.0.2', jumpIds: ['server-old']),
  );
  const forward = PortForwardConfig(
    id: 'forward-1',
    serverId: 'server-old',
    name: 'postgres',
    type: PortForwardType.local,
    localPort: 15432,
  );
  const snippet = Snippet(
    id: 'snippet-1',
    name: 'deploy',
    script: 'deploy',
    autoRunOn: ['server-old'],
  );

  setUp(() async {
    await openTestDb();
    forwards = PortForwardStore();
    snippets = SnippetStore();
    servers = ServerStore(
      portForwards: forwards,
      snippets: snippets,
    );
    servers.put(original);
    servers.put(jumpOwner);
    forwards.put(forward);
    snippets.put(snippet);
    SqliteDb.instance.execute(
      'INSERT INTO known_host (server_id, key_type, fingerprint) VALUES (?, ?, ?);',
      [original.id, 'ssh-ed25519', 'SHA256:old'],
    );
    SqliteDb.instance.execute('INSERT INTO container_host VALUES (?, ?, ?);', [
      original.id,
      'docker',
      'tcp://docker:2375',
    ]);
    SqliteDb.instance.execute('INSERT INTO container_runtime VALUES (?, ?);', [
      original.id,
      'podman',
    ]);
    SqliteDb.instance.execute(
      'INSERT INTO server_dist (server_id, dist, updated_at) VALUES (?, ?, ?);',
      [original.id, 'ubuntu', 1],
    );
    SqliteDb.instance.execute(
      'INSERT INTO benchmark_run '
      '(id, server_id, started_at, status, options, run_dir) '
      'VALUES (?, ?, ?, ?, ?, ?);',
      ['bench-1', original.id, 1, 'running', '{}', '/tmp/yabs-1'],
    );
    SqliteDb.instance.execute(
      'INSERT INTO conn_stat '
      '(id, server_id, server_name, timestamp, result, duration_ms) '
      'VALUES (?, ?, ?, ?, ?, ?);',
      ['stat-1', original.id, original.name, 1, 'success', 5],
    );
  });

  tearDown(closeTestDb);

  test('renaming moves every dependent row in one committed state', () async {
    // Prime the caches that a raw foreign-key update used to leave stale.
    expect(forwards.fetch().single.serverId, original.id);
    expect(snippets.fetch().single.autoRunOn, [original.id]);
    final forwardChanged = forwards.watch().first;
    final snippetChanged = snippets.watch().first;

    final oldForwardRev =
        SqliteDb.instance.select('SELECT rev FROM port_forward WHERE id = ?;', [
              forward.id,
            ]).single['rev']
            as int;
    final oldSnippetRev =
        SqliteDb.instance.select('SELECT rev FROM snippet WHERE id = ?;', [
              snippet.id,
            ]).single['rev']
            as int;
    final oldOwnerRev =
        SqliteDb.instance.select('SELECT rev FROM server WHERE id = ?;', [
              jumpOwner.id,
            ]).single['rev']
            as int;

    final replacement = original.copyWith(id: 'server-new');
    servers.rename(original, replacement);
    await Future.wait([
      forwardChanged,
      snippetChanged,
    ]).timeout(const Duration(seconds: 1));

    expect(servers.fetchOneRaw(original.id), isNull);
    expect(servers.fetchOneRaw(replacement.id), replacement);
    expect(
      {
        for (final row in SqliteDb.instance.select(
          'SELECT key_type, fingerprint FROM known_host WHERE server_id = ?;',
          [replacement.id],
        ))
          row['key_type']: row['fingerprint'],
      },
      {'ssh-ed25519': 'SHA256:old'},
    );
    expect(forwards.fetch().single.serverId, replacement.id);
    expect(snippets.fetch().single.autoRunOn, [replacement.id]);
    expect(
      SqliteDb.instance
          .select('SELECT server_id FROM container_host;')
          .single['server_id'],
      replacement.id,
    );
    expect(
      SqliteDb.instance
          .select('SELECT server_id FROM container_runtime;')
          .single['server_id'],
      replacement.id,
    );
    expect(
      SqliteDb.instance
          .select('SELECT server_id FROM conn_stat;')
          .single['server_id'],
      replacement.id,
    );
    // Left out of the carried tables, these were cascaded away by the delete
    // that ends a rename: the recorded distribution, and the whole benchmark
    // history including a row naming a directory with a live run in it.
    expect(
      SqliteDb.instance
          .select('SELECT server_id FROM server_dist;')
          .single['server_id'],
      replacement.id,
    );
    expect(
      SqliteDb.instance
          .select('SELECT server_id, run_dir FROM benchmark_run;')
          .single['server_id'],
      replacement.id,
    );
    expect(servers.fetchOneRaw(jumpOwner.id)?.ssh?.jumpIds, [replacement.id]);

    expect(
      SqliteDb.instance.select('SELECT rev FROM port_forward WHERE id = ?;', [
        forward.id,
      ]).single['rev'],
      greaterThan(oldForwardRev),
    );
    expect(
      SqliteDb.instance.select('SELECT rev FROM snippet WHERE id = ?;', [
        snippet.id,
      ]).single['rev'],
      greaterThan(oldSnippetRev),
    );
    expect(
      SqliteDb.instance.select('SELECT rev FROM server WHERE id = ?;', [
        jumpOwner.id,
      ]).single['rev'],
      greaterThan(oldOwnerRev),
    );
    expect(
      SqliteDb.instance.select(
        "SELECT count(*) AS n FROM tombstone WHERE tbl = 'server' AND row_id = ?;",
        [original.id],
      ).single['n'],
      1,
    );
  });

  test('a failed replacement rolls the original graph back', () {
    // The snippet's link is moved after the rename's other writes, so a
    // refusal there has every earlier write to undo.
    SqliteDb.instance.execute('''
      CREATE TRIGGER refuse_link BEFORE UPDATE ON snippet_auto_run_on
      BEGIN SELECT RAISE(ABORT, 'refused'); END;
    ''');

    expect(
      () => servers.rename(original, original.copyWith(id: 'server-new')),
      throwsA(anything),
    );

    servers.dropCache();
    expect(servers.fetchOneRaw(original.id), original);
    expect(forwards.fetchForServer(original.id), [forward]);
    expect(snippets.fetch().single.autoRunOn, [original.id]);
    expect({
      for (final row in SqliteDb.instance.select(
        'SELECT key_type, fingerprint FROM known_host WHERE server_id = ?;',
        [original.id],
      ))
        row['key_type']: row['fingerprint'],
    }, isNotEmpty);
    expect(
      SqliteDb.instance.select(
        'SELECT count(*) AS n FROM server WHERE id = ?;',
        ['server-new'],
      ).single['n'],
      0,
    );
  });

  test(
    'direct deletion invalidates child caches and stamps removed links',
    () async {
      expect(forwards.fetch(), [forward]);
      expect(snippets.fetch().single.autoRunOn, [original.id]);
      final forwardChanged = forwards.watch().first;
      final snippetChanged = snippets.watch().first;
      final oldSnippetRev =
          SqliteDb.instance.select('SELECT rev FROM snippet WHERE id = ?;', [
                snippet.id,
              ]).single['rev']
              as int;

      servers.deleteById(original.id);
      await Future.wait([
        forwardChanged,
        snippetChanged,
      ]).timeout(const Duration(seconds: 1));

      expect(forwards.fetch(), isEmpty);
      expect(snippets.fetch().single.autoRunOn, anyOf(isNull, isEmpty));
      expect(
        SqliteDb.instance.select('SELECT rev FROM snippet WHERE id = ?;', [
          snippet.id,
        ]).single['rev'],
        greaterThan(oldSnippetRev),
      );
      expect(
        servers.fetchOneRaw(jumpOwner.id)?.ssh?.jumpIds,
        anyOf(isNull, isEmpty),
      );
    },
  );

  test('a column this build does not know moves with the row', () {
    // As a newer build's step leaves it: `SchemaVersion.readsAhead`.
    final db = SqliteDb.instance;
    db.execute("ALTER TABLE server ADD COLUMN future TEXT NOT NULL DEFAULT '';");
    db.execute("UPDATE server SET future = 'kept', ssh_control_path = 'auto' WHERE id = ?;", [original.id]);

    servers.rename(original, original.copyWith(id: 'server-new', name: 'renamed'));

    final rows = db.select('SELECT id, name, future FROM server WHERE id IN (?, ?);', [original.id, 'server-new']);
    expect(rows.map((r) => (r['id'], r['name'], r['future'])), [('server-new', 'renamed', 'kept')]);
  });
}
