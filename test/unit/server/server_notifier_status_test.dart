import 'dart:convert';
import 'dart:io';

import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/app/error.dart';
import 'package:server_box/data/model/server/monitor/monitor_http_credential.dart';
import 'package:server_box/data/model/server/server.dart';
import 'package:server_box/data/model/server/server_private_info.dart';
import 'package:server_box/data/model/server/try_limiter.dart';
import 'package:server_box/data/provider/server/all.dart';
import 'package:server_box/data/provider/server/single.dart';
import 'package:server_box/data/store/stores.dart';

import '../../helpers/fake_agent_health.dart';
import '../../helpers/rust_lib_helper.dart';

/// What [ServerNotifier] does across a refresh, over a scripted [CommandExec]
/// (this device) or a loopback agent: what it runs, what it caches and what it
/// counts.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;

  setUpAll(() async {
    await initRustLibForTest();
    tempDir = await Directory.systemTemp.createTemp('sbm-notifier-status-');
    Paths.doc = tempDir.path;
  });

  tearDownAll(() async => tempDir.delete(recursive: true));

  setUp(() async {
    SqliteDb.openInMemory();
    await Stores.init();
    TryLimiter.clear();
  });

  tearDown(() async {
    await getIt.reset();
    await SqliteDb.close();
  });

  test('an imported local profile cannot sample this device', () async {
    const spi = Spi(name: 'me', id: 'me', local: true);
    Stores.server.put(spi);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(serversProvider);
    final notifier = container.read(serverProvider(spi.id).notifier);
    await notifier.refresh();
    final state = container.read(serverProvider(spi.id));
    expect(state.conn, ServerConn.failed);
    expect(
      state.status.err,
      isA<LocalServerErr>().having(
        (e) => e.type,
        'type',
        LocalServerErrType.unsupported,
      ),
    );
  });

  group('history', () {
    late HttpServer agent;
    late ProviderContainer container;
    var historyAsks = 0;
    var historyFails = false;
    late Spi spi;

    setUp(() async {
      // The test binding answers every request 400.
      HttpOverrides.global = null;
      historyAsks = 0;
      historyFails = false;
      agent = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      agent.listen((request) async {
        if (await answerAgentHealth(request)) return;
        await request.drain<void>();
        final response = request.response
          ..headers.contentType = ContentType.json;
        switch (request.uri.path) {
          case '/api/v1/login':
            response.write(jsonEncode({'token': 'token'}));
          case '/api/v1/metrics/history':
            historyAsks++;
            if (historyFails) {
              response.statusCode = HttpStatus.internalServerError;
            } else {
              response.write('[]');
            }
          default:
            response.statusCode = HttpStatus.notFound;
        }
        await response.close();
      });
      spi = Spi(
        name: 'agent',
        id: 'agent',
        monitorHttp: MonitorHttpCredential(
          addr: 'http://${agent.address.address}:${agent.port}',
          pwd: 'password',
        ),
      );
      Stores.server.put(spi);
      container = ProviderContainer();
      container.read(serversProvider);
    });

    tearDown(() async {
      container.dispose();
      await agent.close(force: true);
    });

    ServerNotifier notifier() => container.read(serverProvider(spi.id).notifier);

    test('is seeded once per connection', () async {
      await notifier().seedHistory();
      await notifier().seedHistory();
      expect(historyAsks, 1);

      notifier().closeConnection();
      await notifier().seedHistory();
      await notifier().seedHistory();
      expect(historyAsks, 2);
    });

    test('a failed seed is asked again', () async {
      historyFails = true;
      await notifier().seedHistory();
      expect(historyAsks, greaterThanOrEqualTo(1));
      final failedAsks = historyAsks;

      historyFails = false;
      await notifier().seedHistory();
      expect(historyAsks, failedAsks + 1);
      await notifier().seedHistory();
      expect(historyAsks, failedAsks + 1);
    });
  });
}
