import 'dart:async';

import 'package:fl_lib/fl_lib.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_box/core/app_navigator.dart';
import 'package:server_box/core/extension/context/locale.dart';
import 'package:server_box/core/motion.dart';
import 'package:server_box/core/service/diagnostics_upload.dart';
import 'package:server_box/data/res/app_theme.dart';
import 'package:server_box/data/res/build_data.dart';
import 'package:server_box/data/res/chart_palette.dart';
import 'package:server_box/data/res/gruvbox.dart';
import 'package:server_box/data/res/url.dart';
import 'package:server_box/data/store/stores.dart';
import 'package:server_box/generated/l10n/l10n.dart';
import 'package:server_box/view/page/home.dart';
import 'package:server_box/view/page/server/full_screen.dart';
import 'package:server_box/view/page/setting/entry.dart';
import 'package:server_box/view/page/setting/platform/platform_pub.dart';
import 'package:server_box/view/page/setting/tiles.dart';
import 'package:server_box/view/page/ssh/widget/session_keep_alive_notice.dart';
import 'package:server_box/view/widget/column_inset.dart';
import 'package:server_box/view/widget/diagnostics_level_picker.dart';
import 'package:server_box/view/widget/theme_mode_tile.dart';

part 'intro.dart';


class MyApp extends StatefulWidget {
  const MyApp({super.key});

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  late final Future<List<IntroPageBuilder>> _introFuture = _IntroPage.builders;
  bool _transparentNavBarConfigured = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_transparentNavBarConfigured) return;
    _transparentNavBarConfigured = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      SystemUIs.setTransparentNavigationBar(context);
    });
  }

  @override
  Widget build(BuildContext context) {
    final settings = Stores.setting;
    return ListenableBuilder(
      listenable: Listenable.merge([
        RNodes.app,
        settings.appThemePreset.listenable(),
        settings.appThemeAccent.listenable(),
        settings.appThemeContrast.listenable(),
        settings.themeMode.listenable(),
        settings.locale.listenable(),
      ]),
      builder: (context, _) {
        final theme = settings.appTheme;
        return _buildApp(
          context,
          theme: theme,
          light: buildAppTheme(theme, Brightness.light),
          dark: buildAppTheme(theme, Brightness.dark),
        );
      },
    );
  }

  Widget _buildApp(
    BuildContext ctx, {
    required GruvboxTheme theme,
    required ThemeData light,
    required ThemeData dark,
  }) {
    final themeMode = appThemeMode(Stores.setting.themeMode.fetch());
    ServerFullScreenTheme.dark = dark;
    final locale = Stores.setting.locale.fetch().toLocale;

    return MaterialApp(
      key: ValueKey(locale),
      restorationScopeId: 'serverbox',
      navigatorKey: AppNavigator.key,
      // It sits over the top-right corner, which is where every page's app bar
      // keeps its actions — and it says nothing a debug build does not already
      // say everywhere else.
      debugShowCheckedModeBanner: false,
      // Outside the breakpoints builder: a toast is sized against the window,
      // not against the scaled layout the breakpoints hand to the pages.
      builder: (ctx, child) {
        final accent = Theme.of(ctx).colorScheme.primary;
        UIs.colorSeed = accent;
        UIs.primaryColor = accent;
        ChartPalette.resolve(theme, dark: ctx.isDark);
        // Above everything that reads the padding, toasts included — see
        // [WindowCornerInsets].
        final content = WindowCornerInsets(
          child: ToastHost(
            // Under the host, whose toasts it raises, and over every page:
            // a remote session closes as idle whichever page is showing.
            child: SessionKeepAliveNotices(
              child: ResponsivePoints.builder(ctx, child),
            ),
          ),
        );
        // TODO: remove once the dependencies below that still import
        // package:flutter/material.dart migrate to material_ui (#1591). It
        // hands them the theme and localizations, which they look up by the
        // legacy types.
        // ignore: deprecated_member_use
        return MaterialUiCompatibilityBridge(
          child: MotionScope(child: content),
        );
      },
      locale: locale,
      localizationsDelegates: appLocalizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      localeListResolutionCallback: LocaleUtil.resolve,
      navigatorObservers: [AppRouteObserver.instance],
      title: BuildData.name,
      themeMode: themeMode,
      theme: light,
      darkTheme: dark,
      home: FutureBuilder<List<IntroPageBuilder>>(
        future: _introFuture,
        builder: (context, snapshot) {
          context.setLibL10n();
          final appL10n = AppLocalizations.of(context);
          if (appL10n != null) l10n = appL10n;

          if (snapshot.connectionState == ConnectionState.waiting) {
            return const Scaffold(
              body: Center(child: CircularProgressIndicator()),
            );
          }
          final intros = snapshot.data ?? [];
          return intros.isEmpty ? const HomePage() : _IntroPage(intros);
        },
      ),
    );
  }
}
