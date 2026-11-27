import 'package:flutter/painting.dart';
import 'package:material_ui/material_ui.dart' show Brightness;
import 'package:server_box/data/res/chart_series.dart';
import 'package:server_box/data/res/gruvbox.dart';

/// The colours a reading is drawn in, wherever it is drawn: the active Gruvbox
/// variant's semantic hues (CPU in the accent, memory purple, disk reads aqua
/// and writes yellow, download green — blue under the green accent — and
/// upload orange), so a reading keeps its colour across every chart and card.
///
/// [resolve] is called by the app with the active variant before pages that
/// display readings are built.
abstract final class ChartPalette {
  static GruvboxColors _colors = const GruvboxTheme().colors(Brightness.dark);
  static SeriesPalette _series = _seriesOf(_colors);

  static SeriesPalette _seriesOf(GruvboxColors c) => SeriesPalette.fixed([
    c.cpu,
    c.memory,
    c.diskRead,
    c.diskWrite,
    c.netDownload,
    c.netUpload,
  ]);

  /// Takes the colours of [theme] at [brightness], if they changed.
  static void resolve(GruvboxTheme theme, {required bool dark}) {
    final brightness = dark ? Brightness.dark : Brightness.light;
    if (_colors.theme == theme && _colors.brightness == brightness) return;
    _colors = theme.colors(brightness);
    _series = _seriesOf(_colors);
    StatePalette._colors = _colors;
  }

  /// The six as they stand, for a caller that wants them in order.
  static SeriesPalette get series => _series;

  /// What a reading is drawn in where there is nothing to tell it apart from:
  /// the icon at the head of its row, its bar, the one line of its chart. The
  /// theme colour itself.
  ///
  /// Every reading, not only the one a card is watching. The rest were a tint
  /// barely off grey, on a hue moved 100° off the theme's so as not to read as
  /// a faded version of it — and read instead as a colour that had nothing to
  /// do with the theme: olive under a pink one, beside cards whose own icons
  /// were pink. Which row is the one drawn in full is said by its surface and
  /// its glyph, and what is compared across rows is lengths.
  ///
  /// The first of [series], so a line drawn in this and a stretch of a bar
  /// drawn in [cpu] are one colour rather than two near ones.
  static Color get accent => _series.cpu;

  /// The lines of one chart, in the order they are handed out.
  ///
  /// The first is [accent]: a chart with one line is drawn in the theme
  /// colour, and one with several starts there — the write rate beside the
  /// read, the first device of six. The rest are the other five of the fan,
  /// taken from the far side of the wheel first, so that two lines are
  /// opposite hues rather than neighbours and it is the sixth that ends up
  /// 60° from the first.
  ///
  /// By position rather than by device, and derived rather than fixed. They
  /// were six colours chosen at build time, on the argument that a line's
  /// colour is its identity; but the legend under the chart names every line
  /// beside its colour, and six fixed hues were six more colours on the page
  /// with nothing to do with the theme — two of them [StatePalette]'s own red
  /// and amber, on a line that was neither failing nor over anything.
  ///
  /// As long as the most lines a chart draws; one that would need a seventh
  /// says "+N more" instead.
  static List<Color> get lines {
    final all = _series.all;
    return [for (final i in const [0, 3, 1, 4, 2, 5]) all[i]];
  }

  /// Also [accent], which is the same colour asked for by what it is for.
  static Color get cpu => _series.cpu;

  /// The other half of the pair that shares an axis: an overlaid chart draws
  /// this under [cpu], so the two are a lightness step apart as well as 60°.
  static Color get mem => _series.mem;

  /// Never drawn beside the network, so it can have its colour.
  static Color get swap => _series.netRx;

  /// Disk *usage*, which is never drawn beside the two rates below.
  static Color get disk => _series.diskWrite;

  static Color get diskRead => _series.diskRead;
  static Color get diskWrite => _series.diskWrite;

  static Color get netRx => _series.netRx;
  static Color get netTx => _series.netTx;

  static Color get gpu => _series.diskRead;
  static Color get temp => _series.netTx;
  static Color get battery => _series.netRx;

  /// What a reading over `kServerAlertPercent` is drawn in — see
  /// [StatePalette.warn].
  static Color get warn => StatePalette.warn;
}

/// What a thing that is either working or not is drawn in: the same four
/// states wherever the question is asked (a server in a list, a systemd unit,
/// a process, a reading against its threshold), in the variant's own green,
/// yellow, red and gray.
abstract final class StatePalette {
  static GruvboxColors _colors = const GruvboxTheme().colors(Brightness.dark);

  static Color get running => _colors.success;

  /// Starting, stopping, or a reading that has crossed its line: something to
  /// look at rather than something that has failed.
  static Color get warn => _colors.warning;

  static Color get failed => _colors.danger;

  /// Present and doing nothing, which includes a server nobody has connected.
  static Color get idle => _colors.idle;
}
