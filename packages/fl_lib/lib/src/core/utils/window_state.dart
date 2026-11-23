import 'package:material_ui/material_ui.dart';
import 'package:json_annotation/json_annotation.dart';

part 'window_state.g.dart';

@JsonSerializable()
class WindowState {
  const WindowState(this.size, this.position);

  factory WindowState.fromJson(Map<String, dynamic> json) => _$WindowStateFromJson(json);

  @_SizeJsonConverter()
  final Size size;

  @_OffsetJsonConverter()
  final Offset position;

  Map<String, dynamic> toJson() => _$WindowStateToJson(this);
}

class _SizeJsonConverter implements JsonConverter<Size, Map<String, dynamic>> {
  const _SizeJsonConverter();

  @override
  Size fromJson(Map<String, dynamic> json) {
    return Size(json['width'] as double, json['height'] as double);
  }

  @override
  Map<String, dynamic> toJson(Size object) {
    return {
      'width': object.width,
      'height': object.height,
    };
  }
}

class _OffsetJsonConverter implements JsonConverter<Offset, Map<String, dynamic>> {
  const _OffsetJsonConverter();

  @override
  Offset fromJson(Map<String, dynamic> json) {
    return Offset(json['dx'] as double, json['dy'] as double);
  }

  @override
  Map<String, dynamic> toJson(Offset object) {
    return {
      'dx': object.dx,
      'dy': object.dy,
    };
  }
}
