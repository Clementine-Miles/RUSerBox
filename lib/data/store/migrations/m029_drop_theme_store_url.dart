import 'package:fl_lib/fl_lib.dart';
import 'package:server_box/data/store/schema.dart';

/// Removes the retired catalog URL from existing encrypted settings.
class DropThemeStoreUrlMigration extends SchemaMigration {
  const DropThemeStoreUrlMigration();

  @override
  int get from => 29;

  @override
  Future<void> apply() async {
    SqliteDb.instance.execute(
      "DELETE FROM kv WHERE store = 'setting' AND key = 'themeStoreUrl';",
    );
  }
}
