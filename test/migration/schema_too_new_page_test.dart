/// What a downgraded install is shown.
///
/// The refusal happens inside `_initApp`, before `runApp`, so until this page
/// existed the launch simply stopped: no window, no message. From the outside
/// that is a crash, and the one thing the user needed — *the data is intact,
/// reinstall the newer build* — was in a log file on the device.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart'
    show FlutterSecureStorage;
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/core/extension/context/locale.dart';
import 'package:server_box/data/store/schema.dart';
import 'package:server_box/view/page/rescue/rescue_app.dart';
import 'package:server_box/view/page/rescue/schema_too_new.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../helpers/deny_file_deletion.dart';
import '../helpers/test_db.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tmp;
  late Directory dbDir;

  // `Paths.doc` is `late` and refuses a second assignment, so it is set once
  // and its contents cleared per test instead.
  setUpAll(() async {
    tmp = await Directory.systemTemp.createTemp('schema-page-');
    Paths.doc = tmp.path;

    final rng = Random(11);
    FlutterSecureStorage.setMockInitialValues({
      'hivePwd': base64UrlEncode(
        Uint8List.fromList(List<int>.generate(32, (_) => rng.nextInt(256))),
      ),
    });
    SharedPreferences.setMockInitialValues({});
    await PrefStore.shared.init();
  });

  tearDownAll(() {
    if (tmp.existsSync()) tmp.deleteSync(recursive: true);
  });

  setUp(() async {
    for (final e in tmp.listSync()) {
      e.deleteSync(recursive: true);
    }
    // A real file, not `openTestDb`'s in-memory database: the export runs on a
    // second isolate that opens the store by path, and an in-memory one has
    // none. `Paths.doc` is where the page writes its copies, so the store goes
    // in a subdirectory to keep the two apart.
    dbDir = Directory('${tmp.path}/store')..createSync();
    await SqliteDb.open(dbDir.path);
  });

  tearDown(closeTestDb);

  late _FakeDirectoryPicker picker;
  final original = FilePickerPlatform.instance;

  setUp(() {
    final out = Directory('${tmp.path}/chosen')..createSync();
    picker = _FakeDirectoryPicker(out);
    FilePickerPlatform.instance = picker;
  });

  tearDown(() => FilePickerPlatform.instance = original);

  /// Lets real asynchronous work run for a moment.
  ///
  /// The export runs under `Isolate.run`, which a `testWidgets` fake-async zone
  /// does not drive on its own.
  Future<void> settle(WidgetTester tester) async {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 200)),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
  }

  Future<void> pump(WidgetTester tester) async {
    await tester.pumpWidget(
      const RescueApp(
        page: SchemaTooNewPage(
          err: SchemaTooNewException(stored: 23, supported: 21),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
  }

  testWidgets('it says which versions, and that nothing was touched', (
    tester,
  ) async {
    await pump(tester);

    expect(tester.takeException(), isNull);
    expect(find.text(l10n.schemaTooNewTitle), findsOneWidget);
    // Both numbers, because "newer" alone tells nobody which build to look for.
    expect(find.textContaining('23'), findsWidgets);
    expect(find.textContaining('21'), findsWidgets);
    // The answer that costs nothing is on screen, not just the destructive one.
    expect(find.text(l10n.schemaTooNewReinstall), findsOneWidget);
  });

  testWidgets('it offers a backup, an unencrypted one, and a wipe', (
    tester,
  ) async {
    await pump(tester);

    expect(find.text(libL10n.backup), findsOneWidget);
    expect(find.text(l10n.schemaTooNewExportPlain), findsOneWidget);
    expect(find.text(l10n.schemaTooNewWipe), findsOneWidget);
  });

  testWidgets('the unencrypted export warns before it writes anything', (
    tester,
  ) async {
    // The file holds every private key and password in the clear. Warning
    // after the fact is not a warning.
    await pump(tester);

    await tester.tap(find.text(l10n.schemaTooNewExportPlain));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));

    expect(find.text(l10n.schemaTooNewPlainWarn), findsOneWidget);
    expect(
      picker.asked,
      0,
      reason: 'the export began before the user answered',
    );
  });

  testWidgets('the wipe asks first, and says what is left afterwards', (
    tester,
  ) async {
    await pump(tester);

    await tester.tap(find.text(l10n.schemaTooNewWipe));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text(l10n.schemaTooNewWipeConfirm), findsOneWidget);

    await tester.tap(find.text(libL10n.ok).last);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    // The answer comes once the sheet is gone; the page redraws after it.
    await tester.pump();

    expect(tester.takeException(), isNull);
    // The controls are gone: there is nothing left to export.
    expect(find.text(l10n.schemaTooNewWipeDone), findsOneWidget);
    expect(find.text(l10n.schemaTooNewWipeFailed), findsNothing);
    expect(find.text(libL10n.backup), findsNothing);
    expect(find.text(l10n.schemaTooNewWipe), findsNothing);
    // And a way out, rather than an instruction the user cannot act on:
    // reopening resumes this same process and this same dead screen.
    expect(find.text(libL10n.exit), findsOneWidget);
  });

  testWidgets('a wipe that cannot delete still closes the export routes', (
    tester,
  ) async {
    // `wipe` closes the connection before it deletes. If a delete then fails,
    // offering Backup again is offering a button that can only report "the
    // database is not open" — and it was the user's last chance at the data.
    await pump(tester);
    // Exercise the real wipe failure path on every OS, including root.
    final denied = DenyFileDeletion(File(SqliteDb.path!));
    final previous = IOOverrides.current;
    IOOverrides.global = denied;
    addTearDown(() => IOOverrides.global = previous);

    await tester.tap(find.text(l10n.schemaTooNewWipe));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.text(libL10n.ok).last);
    await tester.pump();
    // Past the sheet's exit, which is when it answers.
    await tester.pump(const Duration(milliseconds: 400));
    await settle(tester);

    expect(find.text(libL10n.backup), findsNothing);
    expect(find.text(l10n.schemaTooNewExportPlain), findsNothing);
    // And it does not claim the data is gone. It is not: a delete that failed
    // leaves a database this build still cannot open, and saying otherwise is
    // wrong in both directions.
    expect(find.text(l10n.schemaTooNewWipeDone), findsNothing);
    expect(find.text(l10n.schemaTooNewWipeFailed), findsOneWidget);
    expect(denied.attempted, isTrue);
  });

  testWidgets('declining the wipe leaves everything alone', (tester) async {
    await pump(tester);

    await tester.tap(find.text(l10n.schemaTooNewWipe));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.text(libL10n.cancel).last);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));

    expect(find.text(l10n.schemaTooNewWipeDone), findsNothing);
    expect(find.text(libL10n.backup), findsOneWidget);
    expect(SqliteDb.isOpen, isTrue);
  });
}

/// Stands in for the platform's directory picker.
///
/// Replaces the platform rather than the page's call, so the page's own desktop
/// path runs: the picker answers, and the copy into that directory is the
/// page's.
class _FakeDirectoryPicker extends FilePickerPlatform {
  _FakeDirectoryPicker(this.dir);

  /// Where the user "chose" to save.
  final Directory dir;

  /// Answers as a user who dismissed the picker.
  bool cancel = false;

  /// How many times the picker was opened.
  int asked = 0;

  @override
  Future<String?> getDirectoryPath({
    String? dialogTitle,
    String? initialDirectory,
    AndroidOptions androidOptions = const AndroidOptions(),
    WindowsOptions windowsOptions = const WindowsOptions(),
    LinuxOptions linuxOptions = const LinuxOptions(),
    WebOptions webOptions = const WebOptions(),
  }) async {
    asked++;
    return cancel ? null : dir.path;
  }
}

