
import 'package:fl_lib/fl_lib.dart';

/// This app's LK Cloud client: the `serverbox` public client at the IdP.
abstract final class Lkc {
  /// Registered with the IdP; AppAuth's Android receiver takes it too
  /// (`appAuthRedirectScheme` in `android/app/build.gradle`).
  static const redirectUrl = 'serverbox://auth/callback';

  /// Homebase's name of this app, its sync files under it.
  static const app = 'serverbox';

  /// Whether the account shows in settings and the client runs at launch.
  // TODO: rm once the LK Cloud account ships in ServerBox.
  static const enabled = false;

  static final client = LkcClient(
    oidc: LkcOidc(clientId: 'serverbox', redirectUrl: redirectUrl),
  );

  static Future<void> init() async {
    try {
      await client.init();
    } catch (e, s) {
      Loggers.app.warning('LK Cloud init', e, s);
    }
  }
}
