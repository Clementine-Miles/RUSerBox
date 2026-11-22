import 'package:fl_lib/fl_lib.dart'
    show MotionX, SwipeBackPageTransitionsBuilder;
import 'package:material_ui/material_ui.dart';

/// The app's route transitions, adjusted for display insets and reduced motion.
abstract final class AppPageTransitions {
  static final plain = PageTransitionsTheme(
    builders: {
      for (final MapEntry(:key, :value) in _platform.entries)
        key: _SafeSides(_MotionAware(value)),
    },
  );

  static final paneSlide = PageTransitionsTheme(
    builders: {
      for (final platform in TargetPlatform.values)
        platform: const _SafeSides(_MotionAware(_PaneSlide())),
    },
  );

  static final _platform = {
    ...const PageTransitionsTheme().builders,
    TargetPlatform.iOS: const SwipeBackPageTransitionsBuilder(),
    TargetPlatform.macOS: const SwipeBackPageTransitionsBuilder(),
  };
}

/// The arriving page slides in from the end edge, and the one below moves a
/// third of the way out towards the start.
final class _PaneSlide extends PageTransitionsBuilder {
  const _PaneSlide();

  static final _incoming = Tween<Offset>(
    begin: const Offset(1, 0),
    end: Offset.zero,
  ).chain(CurveTween(curve: Curves.fastEaseInToSlowEaseOut));
  static final _outgoing = Tween<Offset>(
    begin: Offset.zero,
    end: const Offset(-1 / 3, 0),
  ).chain(CurveTween(curve: Curves.fastEaseInToSlowEaseOut));

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    final direction = Directionality.of(context);
    return SlideTransition(
      position: secondaryAnimation.drive(_outgoing),
      textDirection: direction,
      child: SlideTransition(
        position: animation.drive(_incoming),
        textDirection: direction,
        child: child,
      ),
    );
  }
}

/// A pushed page kept clear of the window's insets on its sides.
///
/// The root page manages its own insets. Pushed routes respect side insets
/// around their content.
final class _SafeSides extends PageTransitionsBuilder {
  const _SafeSides(this._inner);

  final PageTransitionsBuilder _inner;

  @override
  Duration get transitionDuration => _inner.transitionDuration;

  @override
  Duration get reverseTransitionDuration => _inner.reverseTransitionDuration;

  @override
  DelegatedTransitionBuilder? get delegatedTransition =>
      _inner.delegatedTransition;

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    // One shape either way: a page that becomes the first (the one under it
    // replaced) keeps its state rather than being built under a new parent.
    final first = route.isFirst;
    return _inner.buildTransitions(
      route,
      context,
      animation,
      secondaryAnimation,
      ColoredBox(
        color: first
            ? Colors.transparent
            : Theme.of(context).scaffoldBackgroundColor,
        child: SafeArea(
          top: false,
          bottom: false,
          left: !first,
          right: !first,
          child: child,
        ),
      ),
    );
  }
}

/// The platform's transition, or a fade in its place when the app moves less.
///
/// The platform's builder is still the one that builds the page, only held
/// still: it is also what carries the back gesture, and a swipe back is how a
/// page is left on iOS whether or not it slides. Its gesture drives the route's
/// own animation, which is what the fade reads, so the page fades under the
/// finger instead of following it.
final class _MotionAware extends PageTransitionsBuilder {
  const _MotionAware(this._inner);

  final PageTransitionsBuilder _inner;

  @override
  Duration get transitionDuration => _inner.transitionDuration;

  @override
  Duration get reverseTransitionDuration => _inner.reverseTransitionDuration;

  @override
  DelegatedTransitionBuilder? get delegatedTransition =>
      _inner.delegatedTransition == null ? null : _delegated;

  /// The page below moving aside for the one arriving, which it does not when
  /// the app moves less. A tear-off, so every read of [delegatedTransition]
  /// is the same value — the navigator compares two routes' by equality.
  Widget? _delegated(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    bool allowSnapshotting,
    Widget? child,
  ) {
    if (context.reduceMotion) return child;
    return _inner.delegatedTransition!(
      context,
      animation,
      secondaryAnimation,
      allowSnapshotting,
      child,
    );
  }

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    if (!context.reduceMotion) {
      return _inner.buildTransitions(
        route,
        context,
        animation,
        secondaryAnimation,
        child,
      );
    }
    return FadeTransition(
      opacity: animation,
      child: _inner.buildTransitions(
        route,
        context,
        kAlwaysCompleteAnimation,
        kAlwaysDismissedAnimation,
        child,
      ),
    );
  }
}
