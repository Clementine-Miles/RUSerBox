#!/usr/bin/env bash
# Checks what an APK carries: native code for arm64-v8a only, every library
# an AArch64 ELF with 16 KiB-aligned segments, the archive aligned for 16 KiB
# pages, and nothing of Flutter.
#
#   scripts/release/inspect-apk.sh <apk>
set -euo pipefail

apk="$1"
: "${ANDROID_HOME:?ANDROID_HOME is not set}"
ndk="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/$(tr -d '[:space:]' < "$(dirname "$0")/../../android/ndk-version.txt")}"
readelf="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf"
build_tools="$ANDROID_HOME/build-tools/$(tr -d '[:space:]' < "$(dirname "$0")/../../android/build-tools-version.txt")"

libs=$(unzip -Z1 "$apk" | grep '^lib/' || true)
[ -n "$libs" ] || { echo "no native libraries" >&2; exit 1; }
if echo "$libs" | grep -v '^lib/arm64-v8a/'; then
  echo "native code for another ABI (above)" >&2
  exit 1
fi
if unzip -Z1 "$apk" | grep -iE 'libflutter|flutter_assets|libapp\.so'; then
  echo "Flutter in the APK (above)" >&2
  exit 1
fi

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
unzip -q "$apk" 'lib/*' -d "$dir"
for so in "$dir"/lib/arm64-v8a/*.so; do
  "$readelf" -h "$so" | grep -q 'Machine:.*AArch64' || { echo "$so is not AArch64" >&2; exit 1; }
  if "$readelf" -lW "$so" | awk '$1 == "LOAD" && $NF != "0x4000" && $NF != "0x10000" { bad = 1 } END { exit !bad }'; then
    echo "$so has a LOAD segment aligned below 16 KiB" >&2
    exit 1
  fi
  echo "ok: ${so#"$dir"/} ($(du -h "$so" | cut -f1))"
done
"$build_tools/zipalign" -c -P 16 4 "$apk"
echo "ok: $apk is aligned for 16 KiB pages and carries arm64-v8a only"
