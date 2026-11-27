import 'package:fl_lib/fl_lib.dart';
import 'package:server_box/data/store/schema.dart';

/// Retires the active data of features this build no longer has.
///
/// - **The Agent** (fl_pi_llm): its kv stores (`chats`, `llm`, `tool`,
///   `memory`, `llmCredential`, `mcpSecret`), its legacy session chunks
///   (`pi_chunks`), and the sync stamps of those stores and of its skills.
/// - **Remote desktop** (RDP/VNC): the `remote_desktop_profile` table, its
///   tombstones and sync stamps. Its rows held saved desktop passwords.
///
/// Deleted here rather than left for a reader that no longer exists: provider
/// keys and MCP headers are secrets, and nothing would ever clear them again.
/// The removal is local. No tombstone is written and no op is sent, so a peer
/// still on an older build keeps its own copy; what such a peer sends is not
/// kept by this build (`SyncRecords.keeps`), and a restored backup's AI
/// sections are ignored because they are not in `SyncedStores.byName`.
///
/// Idempotent: every statement is a delete or an `IF EXISTS`.
class RetireRemovedFeaturesMigration extends SchemaMigration {
  const RetireRemovedFeaturesMigration();

  /// The AI stores' names, as fl_pi_llm_ui's `SqliteStore`s were called.
  static const aiStores = ['chats', 'llm', 'tool', 'memory', 'llmCredential', 'mcpSecret'];

  /// The op log's collection of the Agent's skills.
  static const skillCollection = 'skill';

  @override
  int get from => 43;

  @override
  Future<void> apply() async {
    final db = SqliteDb.instance;
    final marks = List.filled(aiStores.length, '?').join(', ');
    db.execute('DELETE FROM kv WHERE store IN ($marks);', aiStores);
    db.execute('DROP TABLE IF EXISTS pi_chunks;');
    db.execute('DROP INDEX IF EXISTS idx_remote_desktop_profile_server;');
    db.execute('DROP TABLE IF EXISTS remote_desktop_profile;');
    db.execute("DELETE FROM tombstone WHERE tbl = 'remote_desktop_profile';");

    final collections = [for (final s in aiStores) 'kv:$s', skillCollection, 'remote_desktop_profile'];
    final cMarks = List.filled(collections.length, '?').join(', ');
    for (final table in const ['sync_record', 'sync_passed']) {
      if (!_hasTable(table)) continue;
      db.execute('DELETE FROM $table WHERE c IN ($cMarks);', collections);
    }
  }

  static bool _hasTable(String name) => SqliteDb.instance
      .select("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?;", [name])
      .isNotEmpty;
}
