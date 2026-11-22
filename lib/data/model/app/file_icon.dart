import 'package:icons_plus/icons_plus.dart';
import 'package:material_ui/material_ui.dart';

/// Built-in file browser glyphs resolved by file type or extension.
abstract final class FileIcon {
  static const dir = 'file.dir';
  static const link = 'file.link';
  static const fallback = 'file.default';

  static const _named = {
    'dockerfile': 'dockerfile',
    'containerfile': 'dockerfile',
    'makefile': 'makefile',
    'gemfile': 'rb',
    'rakefile': 'rb',
  };

  static const _alias = {
    'bash': 'sh',
    'zsh': 'sh',
    'pyi': 'py',
    'pyw': 'py',
    'mjs': 'js',
    'cjs': 'js',
    'jsx': 'js',
    'tsx': 'ts',
    'mts': 'ts',
    'cts': 'ts',
    'yaml': 'yml',
    'markdown': 'md',
    'mdx': 'md',
    'htm': 'html',
    'scss': 'css',
    'sass': 'css',
    'less': 'css',
    'cc': 'cpp',
    'cxx': 'cpp',
    'hpp': 'cpp',
    'hh': 'cpp',
    'h': 'c',
    'kts': 'kt',
    'exs': 'ex',
    'hrl': 'erl',
    'pm': 'pl',
    'gql': 'graphql',
    'gitattributes': 'gitignore',
    'gitmodules': 'gitignore',
    'crt': 'pem',
    'cer': 'pem',
    'key': 'pem',
  };

  static final _segment = RegExp(r'^[a-z][a-z0-9]*$');

  static String keyOf({
    required String name,
    required bool isDir,
    required bool isLink,
  }) {
    if (isDir) return dir;
    if (isLink) return link;
    return forName(name) ?? fallback;
  }

  /// Returns the glyph key for a recognized file name or extension.
  static String? forName(String name) {
    final base = name.split('/').last.toLowerCase();
    if (base.isEmpty || base == '.' || base == '..') return null;
    if (_named[base] case final mapped?) return 'file.$mapped';
    final dot = base.lastIndexOf('.');
    final String ext;
    if (dot == 0) {
      ext = base.substring(1);
    } else if (dot > 0 && dot < base.length - 1) {
      ext = base.substring(dot + 1);
    } else {
      return null;
    }
    if (!_segment.hasMatch(ext)) return null;
    return 'file.$ext';
  }

  /// A related extension that shares the same glyph.
  static String? aliasOf(String key) {
    if (!key.startsWith('file.')) return null;
    final to = _alias[key.substring(5)];
    if (to == null) return null;
    return 'file.$to';
  }

  /// The matching glyph and its optional brand color.
  static (IconData, Color?) builtIn(String key) {
    var k = key;
    if (!_builtIn.containsKey(k)) k = aliasOf(k) ?? k;
    return _builtIn[k] ?? _builtIn[fallback]!;
  }

  static const _builtIn = <String, (IconData, Color?)>{
    dir: (Icons.folder_open, null),
    link: (Icons.link, null),
    fallback: (Icons.insert_drive_file, null),
    'file.py': (FontAwesome.python_brand, Color(0xFF3776AB)),
    'file.rs': (FontAwesome.rust_brand, Color(0xFFDEA584)),
    'file.js': (FontAwesome.js_brand, Color(0xFFF7DF1E)),
    'file.ts': (FontAwesome.js_brand, Color(0xFF3178C6)),
    'file.go': (FontAwesome.golang_brand, Color(0xFF00ADD8)),
    'file.html': (FontAwesome.html5_brand, Color(0xFFE34F26)),
    'file.css': (FontAwesome.css3_brand, Color(0xFF1572B6)),
    'file.md': (FontAwesome.markdown_brand, Color(0xFF519ABA)),
    'file.sh': (FontAwesome.terminal_solid, Color(0xFF89E051)),
    'file.dockerfile': (FontAwesome.docker_brand, Color(0xFF2496ED)),
    'file.php': (FontAwesome.php_brand, Color(0xFF777BB4)),
    'file.java': (FontAwesome.java_brand, Color(0xFFEA2D2E)),
    'file.swift': (FontAwesome.swift_brand, Color(0xFFF05138)),
    'file.vue': (FontAwesome.vuejs_brand, Color(0xFF42B883)),
    'file.gitignore': (FontAwesome.git_brand, Color(0xFFF05032)),
    'file.json': (Icons.data_object, Color(0xFFF5DE19)),
    'file.yml': (Icons.data_object, Color(0xFFCB171E)),
    'file.xml': (Icons.code, Color(0xFFE37933)),
    'file.sql': (FontAwesome.database_solid, Color(0xFF336791)),
    'file.conf': (Icons.settings, Color(0xFF009900)),
    'file.toml': (Icons.settings, Color(0xFF9C4221)),
    'file.ini': (Icons.settings, null),
    'file.cfg': (Icons.settings, null),
    'file.lock': (FontAwesome.lock_solid, Color(0xFFE5C07B)),
    'file.pem': (FontAwesome.key_solid, Color(0xFFE5C07B)),
    'file.log': (FontAwesome.file_lines_solid, null),
    'file.txt': (FontAwesome.file_lines_solid, null),
    'file.png': (FontAwesome.file_image_solid, Color(0xFFC679DD)),
    'file.jpg': (FontAwesome.file_image_solid, Color(0xFFC679DD)),
    'file.jpeg': (FontAwesome.file_image_solid, Color(0xFFC679DD)),
    'file.gif': (FontAwesome.file_image_solid, Color(0xFFC679DD)),
    'file.webp': (FontAwesome.file_image_solid, Color(0xFFC679DD)),
    'file.svg': (FontAwesome.file_image_solid, Color(0xFFFFB13B)),
    'file.mp4': (FontAwesome.file_video_solid, Color(0xFFE06C75)),
    'file.mkv': (FontAwesome.file_video_solid, Color(0xFFE06C75)),
    'file.webm': (FontAwesome.file_video_solid, Color(0xFFE06C75)),
    'file.mp3': (FontAwesome.file_audio_solid, Color(0xFF98C379)),
    'file.wav': (FontAwesome.file_audio_solid, Color(0xFF98C379)),
    'file.flac': (FontAwesome.file_audio_solid, Color(0xFF98C379)),
    'file.zip': (FontAwesome.file_zipper_solid, Color(0xFFE5C07B)),
    'file.gz': (FontAwesome.file_zipper_solid, Color(0xFFE5C07B)),
    'file.tar': (FontAwesome.file_zipper_solid, Color(0xFFE5C07B)),
    'file.pdf': (FontAwesome.file_pdf_solid, Color(0xFFE5252A)),
    'file.rb': (Icons.code, Color(0xFFCC342D)),
    'file.dart': (Icons.code, Color(0xFF0175C2)),
    'file.kt': (Icons.code, Color(0xFF7F52FF)),
    'file.c': (Icons.code, Color(0xFFA8B9CC)),
    'file.cpp': (Icons.code, Color(0xFF00599C)),
    'file.lua': (Icons.code, Color(0xFF000080)),
  };
}
