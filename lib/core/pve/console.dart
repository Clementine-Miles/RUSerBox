import 'dart:async';
import 'dart:typed_data';

import 'package:server_box/src/rust/api/pve.dart';

/// An open PVE console, as the console views use it: bytes in and out, and a
/// terminal size. `sbm_virt::pve::console::Console` does the rest — the
/// websocket, termproxy's login, framing and keep-alive.
abstract interface class PveConsoleLink {
  /// The next bytes, or null once the console has ended.
  Future<Uint8List?> recv();

  Future<void> send(List<int> data);

  /// The text console's new size.
  Future<void> resize(int cols, int rows);

  Future<void> close();
}

/// [PveConsoleLink] over the FFI's [PveConsoleChannel].
final class PveConsoleChannelLink implements PveConsoleLink {
  PveConsoleChannelLink(this._channel);

  final PveConsoleChannel _channel;

  @override
  Future<Uint8List?> recv() => _channel.recv();

  @override
  Future<void> send(List<int> data) => _channel.send(data: data);

  @override
  Future<void> resize(int cols, int rows) =>
      _channel.resize(cols: cols, rows: rows);

  @override
  Future<void> close() => _channel.close();
}

/// Calls on a [PveConsoleLink] one after another, in the order they were
/// made: each is its own FFI call, and two in flight at once could reach the
/// console in either order.
///
/// Bytes queued behind a call in flight go out together in the next one, and
/// no more than [maxPending] of them wait: a console that has taken nothing
/// for that long has stopped, and is ended rather than buffered for.
final class PveConsoleWriter {
  PveConsoleWriter(this._link, {required this.onError});

  final PveConsoleLink _link;

  /// A write failed, or the queue overflowed: the console has gone.
  final void Function(Object error) onError;

  static const maxPending = 1 << 20;

  /// Each a byte run to send, or a size: `(cols, rows)`.
  final _queue = <Object>[];
  var _pending = 0;
  Completer<void>? _drained;
  var _failed = false;

  /// Completes once what is queued now has been sent.
  Future<void> send(List<int> data) {
    if (data.isEmpty) return _idle;
    final last = _queue.lastOrNull;
    if (last is BytesBuilder) {
      last.add(data);
    } else {
      _queue.add(BytesBuilder()..add(data));
    }
    _pending += data.length;
    if (_pending > maxPending) {
      _fail(StateError('The console is not taking input'));
    }
    return _run();
  }

  Future<void> resize(int cols, int rows) {
    _queue.add((cols, rows));
    return _run();
  }

  Future<void> get _idle => _drained?.future ?? Future<void>.value();

  Future<void> _run() {
    if (_failed) {
      _queue.clear();
      return Future<void>.value();
    }
    final running = _drained;
    if (running != null) return running.future;
    final drained = _drained = Completer<void>();
    () async {
      try {
        while (_queue.isNotEmpty && !_failed) {
          switch (_queue.removeAt(0)) {
            case final BytesBuilder bytes:
              _pending -= bytes.length;
              await _link.send(bytes.takeBytes());
            case (final int cols, final int rows):
              await _link.resize(cols, rows);
          }
        }
      } catch (e) {
        _fail(e);
      } finally {
        _drained = null;
        drained.complete();
      }
    }();
    return drained.future;
  }

  void _fail(Object error) {
    if (_failed) return;
    _failed = true;
    _queue.clear();
    _pending = 0;
    onError(error);
  }
}
