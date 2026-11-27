import 'package:flutter/widgets.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:server_box/data/model/app/keybinding.dart';
import 'package:server_box/data/store/stores.dart';

part 'keybindings.g.dart';

/// The chords in force on this device: the defaults, and what was changed.
final class KbState {
  const KbState({required this.overrides});

  /// The changes, by command id. An empty chord is a command
  /// whose keys were removed.
  final Map<String, String> overrides;

  bool isModified(KbCommand c) => overrides.containsKey(c.id);

  /// The chord [c] runs on; empty when it has none.
  String chordOf(KbCommand c) => overrides[c.id] ?? c.chord;

  SingleActivator? activatorOf(KbCommand c) => KbChord.activator(chordOf(c));

  /// [c]'s chord as a tooltip says it; empty when it has none.
  String labelOf(KbCommand c) => KbChord.label(chordOf(c));

  /// `name Ctrl+P`, or the name alone without a chord.
  String tip(String name, KbCommand c) {
    final keys = labelOf(c);
    return keys.isEmpty ? name : '$name $keys';
  }

  /// Every changeable command of [scopes] with a chord, for a [Shortcuts].
  Map<ShortcutActivator, Intent> shortcutsFor(Iterable<KbScope> scopes) => {
    for (final c in KbCommand.values)
      if (!c.fixed && scopes.contains(c.scope))
        ?activatorOf(c): KbIntent.of(c),
  };

  /// The commands that would answer [chord] wherever [command] does.
  List<KbCommand> conflictsOf(KbCommand command, String chord) => [
    for (final c in KbCommand.values)
      if (c != command &&
          !c.fixed &&
          chord.isNotEmpty &&
          chordOf(c) == chord &&
          c.sharesScopeWith(command))
        c,
  ];
}

/// The keyboard shortcuts, as changed on the settings page.
@Riverpod(keepAlive: true)
class Keybindings extends _$Keybindings {
  @override
  KbState build() => KbState(overrides: _read());

  /// Overrides are stored as `pc:<id>`: the key the PC-style keyboard (every
  /// Android hardware keyboard) always used. A `mac:` row an older build
  /// synced from an Apple device is left as it is.
  static const _prefix = 'pc:';

  static Map<String, String> _read() {
    const prefix = _prefix;
    return {
      for (final e in Stores.setting.keybindings.fetch().entries)
        // One no chord reads — written by hand, or by a build that took more
        // spellings — leaves its command at the default.
        if (e.key.startsWith(prefix) &&
            (e.value.isEmpty || KbChord.activator(e.value) != null))
          e.key.substring(prefix.length): e.value,
    };
  }

  /// Writes the changes, leaving any other row alone.
  void _write(Map<String, String> overrides) {
    final all = {
      for (final e in Stores.setting.keybindings.fetch().entries)
        if (!e.key.startsWith(_prefix)) e.key: e.value,
      for (final e in overrides.entries) '$_prefix${e.key}': e.value,
    };
    Stores.setting.keybindings.put(all);
    state = KbState(overrides: overrides);
  }

  /// [command] bound to [chord], taken from any command it conflicts with.
  /// A chord [KbChord.activator] cannot read is refused.
  void bind(KbCommand command, String chord) {
    if (command.fixed) return;
    if (chord.isNotEmpty && KbChord.activator(chord) == null) return;
    final next = {...state.overrides};
    for (final c in state.conflictsOf(command, chord)) {
      next[c.id] = '';
    }
    next[command.id] = chord;
    // Bound back to its default is not a change.
    if (chord == command.chord) next.remove(command.id);
    _write(next);
  }

  /// [command] with no keys at all.
  void unbind(KbCommand command) => bind(command, '');

  void reset(KbCommand command) =>
      _write({...state.overrides}..remove(command.id));

  void resetAll() => _write(const {});
}
