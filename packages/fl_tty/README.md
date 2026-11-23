# fl_tty

ServerBox vendors fl_tty from lollipopkit/fl_tty at revision
`e573bac3a3235b7c18c12d7c0712c3c942af2ca1`. See the [upstream revision](https://github.com/lollipopkit/fl_tty/tree/e573bac3a3235b7c18c12d7c0712c3c942af2ca1).

The Android app opens local pseudo-terminals and uses proot for arm64 Linux
userlands. The native library also builds on Linux for host-side Flutter tests.
The Dart API includes `LocalShellBackend`, `LocalExec` and `Rootfs`.

The native build hook compiles the PTY code. Builds that package their own
proot pair through Gradle set `hooks.user_defines.fl_tty.proot: false`. To
build a pair ahead of time, run `scripts/build-proot-android.sh`.

The package is AGPL-3.0. The flutter_pty sources retain their MIT notice in
[LICENSE.flutter_pty](LICENSE.flutter_pty).

Dart embedding headers retain their [BSD license](LICENSE.dart-sdk).
