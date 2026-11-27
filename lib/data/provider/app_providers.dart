import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The app's single [ProviderContainer], made in `main` so code outside any
/// widget (store reloads after a restore or a sync) reads the same providers
/// the pages do.
abstract final class AppProviders {
  static ProviderContainer? _container;

  /// Set once, before `runApp`.
  static set container(ProviderContainer c) {
    assert(_container == null, 'set once');
    _container = c;
  }

  /// The container, or null before `main` set it (tests, rescue screens).
  static ProviderContainer? get maybeContainer => _container;
}
