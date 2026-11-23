import 'dart:async';
import 'dart:typed_data';

import 'package:dartssh2/dartssh2.dart';

/// Releases a forwarded channel and the jump SSH client carrying it together.
class OwningSocket implements SSHSocket {
  OwningSocket(this._inner, this._release);

  final SSHSocket _inner;
  final void Function() _release;
  var _released = false;

  void _releaseOnce() {
    if (_released) return;
    _released = true;
    _release();
  }

  @override
  Stream<Uint8List> get stream => _inner.stream;

  @override
  StreamSink<List<int>> get sink => _inner.sink;

  @override
  Future<void> get done => _inner.done;

  @override
  Future<void> close() async {
    try {
      await _inner.close();
    } finally {
      _releaseOnce();
    }
  }

  @override
  void destroy() {
    try {
      _inner.destroy();
    } finally {
      _releaseOnce();
    }
  }

  @override
  Future<void> flush() => _inner.flush();

  @override
  String toString() => 'OwningSocket($_inner)';
}
