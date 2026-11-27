#!/usr/bin/env bash
# Checks a built APK is what this project ships, and fails loudly if not:
#
# - lib/arm64-v8a is the only ABI directory;
# - every packaged ELF object (libraries and the proot helpers) is AArch64,
#   has LOAD segments aligned for 16 KB pages, and links no glibc;
# - none of the removed features' libraries survived (AI runtime, RDP/VNC);
# - zipalign passes; a signed APK verifies with apksigner;
# - the version code is build * 100 + 3.
#
#   scripts/release/verify-apk.sh path/to/app.apk
#
# Needs ANDROID_HOME (build-tools) and an NDK (llvm-readelf).

set -euo pipefail

apk="${1:?usage: $0 <apk>}"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
[ -n "$sdk" ] || { echo "ANDROID_HOME is not set" >&2; exit 1; }
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bt="$sdk/build-tools/$(tr -d '[:space:]' < "$REPO_ROOT/android/build-tools-version.txt")"
ndk="$sdk/ndk/$(tr -d '[:space:]' < "$REPO_ROOT/android/ndk-version.txt")"
readelf="$(ls "$ndk"/toolchains/llvm/prebuilt/*/bin/llvm-readelf | head -n1)"
fail=0
err() { echo "FAIL: $*" >&2; fail=1; }

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
unzip -q "$apk" 'lib/*' -d "$work" || true

abis="$(ls "$work/lib" 2>/dev/null | tr '\n' ' ')"
[ "$abis" = "arm64-v8a " ] || err "ABI directories: '$abis' (expected only arm64-v8a)"

for so in "$work"/lib/*/*; do
  base="$(basename "$so")"
  head -c4 "$so" | grep -q $'\x7fELF' || { err "$base is not ELF"; continue; }
  machine="$("$readelf" -h "$so" | sed -n 's/^ *Machine: *//p')"
  [ "$machine" = AArch64 ] || err "$base: machine $machine"
  while read -r align; do
    [ $((align)) -ge 16384 ] || err "$base: LOAD alignment $align < 16 KB"
  done < <("$readelf" -lW "$so" | awk '$1=="LOAD"{print $NF}')
  if "$readelf" -d "$so" 2>/dev/null | grep -qE 'NEEDED.*(libc\.so\.6|ld-linux)'; then
    err "$base links glibc"
  fi
  case "$base" in
    libfl_pi_llm.so|*quickjs*|*ironrdp*|*vnc*) err "$base belongs to a removed feature" ;;
  esac
  printf '  %-34s %8d bytes  %s\n' "$base" "$(stat -c %s "$so")" "$machine"
done

"$bt/zipalign" -c -P 16 4 "$apk" >/dev/null || err "zipalign -c -P 16 failed"

if unzip -l "$apk" | grep -q 'META-INF/.*\.\(RSA\|EC\|DSA\)$' || "$bt/apksigner" verify "$apk" >/dev/null 2>&1; then
  "$bt/apksigner" verify --print-certs "$apk" >/dev/null || err "apksigner verify failed"
else
  echo "  (unsigned APK: signature not checked)"
fi

build="$(sed -n -E 's/.*static const int build = ([0-9]+);/\1/p' "$REPO_ROOT/lib/data/res/build_data.dart")"
code="$("$bt/aapt2" dump badging "$apk" | sed -n "s/.*versionCode='\([0-9]*\)'.*/\1/p")"
[ "$code" = "$((build * 100 + 3))" ] || err "versionCode $code, expected $((build * 100 + 3))"

[ "$fail" = 0 ] && echo "OK: $apk (versionCode $code)"
exit "$fail"
