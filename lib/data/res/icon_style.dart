enum IconStyle {
  classic,
  mingcute;

  static IconStyle? parse(Object? value) =>
      value == mingcute.name ? mingcute : classic;
}
