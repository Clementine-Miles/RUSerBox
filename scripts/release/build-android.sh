#!/usr/bin/env bash
# The one Android release build: an arm64-v8a APK, verified.
#
#   scripts/release/build-android.sh            # signed with the release key (1Password)
#   ORG_GRADLE_PROJECT_allowDebugReleaseSigning=true scripts/release/build-android.sh
#   ORG_GRADLE_PROJECT_allowUnsignedRelease=true scripts/release/build-android.sh
#
# The build number is BuildData.build (lib/data/res/build_data.dart), which a
# release bumps together with pubspec.yaml's version; Gradle turns it into the
# version code build * 100 + 3, the arm64 code published releases use.
# Writes build/app/outputs/flutter-apk/<app>_v1.0.<build>_arm64.apk and its
# .sha256, and runs scripts/release/verify-apk.sh on it.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

build="$(sed -n -E 's/.*static const int build = ([0-9]+);/\1/p' lib/data/res/build_data.dart)"
name="$(sed -n -E 's/.*static const String name = "([^"]+)";/\1/p' lib/data/res/build_data.dart)"
[ -n "$build" ] && [ -n "$name" ] || { echo "BuildData not readable" >&2; exit 1; }

out_dir=build/app/outputs/flutter-apk
rm -f "$out_dir"/*.apk "$out_dir"/*.sha256

# As build-fdroid.sh: refresh the plugin metadata, then drop dev-only plugins
# (integration_test) so the release registrant does not name them.
flutter pub get --enforce-lockfile
scripts/release/patch-jni-build-id.sh
flutter build apk --no-pub --release --config-only
dart --packages="$REPO_ROOT/scripts/release/empty-package-config.json" \
  "$REPO_ROOT/scripts/release/prune_android_dev_plugins.dart"
dart --packages="$REPO_ROOT/scripts/release/empty-package-config.json" \
  "$REPO_ROOT/scripts/release/map_plugin_registrant_package.dart"

scripts/release/android-signing.sh flutter build apk \
  --no-pub \
  --release \
  --target-platform android-arm64 \
  --build-number="$build" \
  --build-name="1.0.$build" \
  "$@"

apk="$out_dir/${name}_v1.0.${build}_arm64.apk"
cp "$out_dir/app-release.apk" "$apk"
(cd "$out_dir" && sha256sum "$(basename "$apk")" > "$(basename "$apk").sha256")
scripts/release/verify-apk.sh "$apk"
echo "$apk"
