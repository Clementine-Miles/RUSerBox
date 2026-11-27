import 'dart:async';
import 'dart:io';

import 'package:fl_lib/fl_lib.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/server/monitor/monitor_http_credential.dart';
import 'package:server_box/data/model/server/server_private_info.dart';
import 'package:server_box/data/provider/server/all.dart';
import 'package:server_box/data/store/stores.dart';

import '../../helpers/rust_lib_helper.dart';

/// Saving or deleting a server must not wait on the agent it is leaving: the
/// commonest reason to move away from an address is that it stopped
/// answering, and anything awaited there runs into `MonitorHttpClient`'s
/// ten-second connect timeout between the Save button and the editor closing.
/// The agent here accepts requests and never answers them.
void main() {
  // `MonitorHttpClient` names its protocol from Rust.
  setUpAll(initRustLibForTest);

  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;

  setUpAll(() async {
    tempDir = await Directory.systemTemp.createTemp('sbm-save-agent-wait-');
    Paths.doc = tempDir.path;
  });

  tearDownAll(() async => tempDir.delete(recursive: true));

  setUp(() async {
    SqliteDb.openInMemory();
    await Stores.init();
  });

  tearDown(() async {
    await getIt.reset();
    await SqliteDb.close();
  });

  const agentAddr = 'http://127.0.0.1:3770';
  const movedAddr = 'http://127.0.0.1:3771';

  Spi monitorServer(String id, String addr) => Spi(
    id: id,
    name: id,
    monitorHttp: MonitorHttpCredential(addr: addr, pwd: 'x'),
  );

  /// Runs [body] with every `HttpClient` replaced by one that never answers.
  ///
  /// A real socket would not do it: `TestWidgetsFlutterBinding` installs an
  /// `HttpOverrides` of its own that answers 400 without opening anything, so
  /// a test written against a silent listener measures nothing and passes
  /// whether or not the wait is still there.
  Future<void> withSilentAgent(
    Future<void> Function(_SilentHttpClient client) body,
  ) {
    final client = _SilentHttpClient();
    return HttpOverrides.runZoned(
      () => body(client),
      createHttpClient: (_) => client,
    );
  }

  test('changing a monitor address does not wait for the old agent', () async {
    await withSilentAgent((client) async {
      final before = monitorServer('moved-server', agentAddr);
      final after = before.copyWith(
        monitorHttp: const MonitorHttpCredential(
          addr: movedAddr,
          pwd: 'x',
        ),
      );

      Stores.server.put(before);
      final container = ProviderContainer();
      addTearDown(container.dispose);
      container.read(serversProvider);

      // Times out rather than hangs the suite if the wait comes back: an
      // unanswered request has no deadline of its own here.
      await container
          .read(serversProvider.notifier)
          .updateServer(before, after)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => fail('the save waited on the old agent'),
          );

      expect(
        container.read(serversProvider).servers['moved-server']?.monitor?.addr,
        movedAddr,
        reason: 'the save should have gone through',
      );
    });
  });

  test('deleting a server does not wait for its agent either', () async {
    await withSilentAgent((client) async {
      final doomed = monitorServer('doomed-server', agentAddr);
      Stores.server.put(doomed);
      Stores.selfAddr.put(doomed.id, InternetAddress('8.8.8.8'));
      final container = ProviderContainer();
      addTearDown(container.dispose);
      container.read(serversProvider);

      await container
          .read(serversProvider.notifier)
          .delServer(doomed.id)
          .timeout(
            const Duration(seconds: 5),
            onTimeout: () => fail('the delete waited on the agent'),
          );

      expect(
        container.read(serversProvider).servers,
        isNot(contains(doomed.id)),
      );
      expect(Stores.selfAddr.probedAt(doomed.id), isNull);
    });
  });

  test('renaming a server moves its self-reported address record', () async {
    final before = monitorServer('old-id', agentAddr);
    final after = before.copyWith(id: 'new-id', name: 'new-id');
    Stores.server.put(before);
    Stores.selfAddr.put(before.id, InternetAddress('8.8.8.8'));
    final probedAt = Stores.selfAddr.probedAt(before.id);

    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(serversProvider);

    await container.read(serversProvider.notifier).updateServer(before, after);

    expect(Stores.selfAddr.probedAt(before.id), isNull);
    expect(Stores.selfAddr.probedAt(after.id), probedAt);
    expect(Stores.selfAddr.addrOf(after.id)?.address, '8.8.8.8');
  });
}

/// An `HttpClient` that accepts a request and never answers it.
///
/// Everything but [openUrl] is left to `noSuchMethod`: the interface is wide,
/// and a client that never gets past opening a request never reaches the rest
/// of it.
class _SilentHttpClient implements HttpClient {
  final requested = <Uri>[];
  final firstRequest = Completer<void>();

  @override
  Future<HttpClientRequest> openUrl(String method, Uri url) {
    requested.add(url);
    if (!firstRequest.isCompleted) firstRequest.complete();
    // Never completes, which is the whole point.
    return Completer<HttpClientRequest>().future;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => null;
}
