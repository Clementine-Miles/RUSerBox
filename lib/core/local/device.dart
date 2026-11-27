/// This device, where ServerBox names machines: fl_tty's `LocalExec` runs
/// the commands, this says what the device is called.
abstract final class LocalDevice {
  /// What names this machine where a server id is expected.
  ///
  /// Reserved rather than generated: server ids come from `ShortId`, whose
  /// alphabet has no `#`, so nothing the user configures can collide with it.
  /// One spelling, shared with the terminal's `LocalSource`.
  static const id = '#local';
}
