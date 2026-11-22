import 'package:flutter_test/flutter_test.dart';
import 'package:server_box/data/model/app/file_icon.dart';

void main() {
  test('maps a name to file.<ext>', () {
    expect(FileIcon.forName('app.py'), 'file.py');
    expect(FileIcon.forName('Lib.rs'), 'file.rs');
    expect(FileIcon.forName('.gitignore'), 'file.gitignore');
    expect(FileIcon.forName('.env'), 'file.env');
    expect(FileIcon.forName('archive.tar.gz'), 'file.gz');
    expect(FileIcon.forName('Dockerfile'), 'file.dockerfile');
    expect(FileIcon.forName('Makefile'), 'file.makefile');
    expect(FileIcon.forName('Gemfile'), 'file.rb');
    expect(FileIcon.forName('style.scss'), 'file.scss');
    expect(FileIcon.forName('README'), isNull);
    expect(FileIcon.forName('archive.7z'), isNull);
  });

  test('aliases related extensions onto a glyph the package may share', () {
    expect(FileIcon.aliasOf('file.scss'), 'file.css');
    expect(FileIcon.aliasOf('file.bash'), 'file.sh');
    expect(FileIcon.aliasOf('file.py'), isNull);
  });

  test('built-in glyphs differ by extension', () {
    expect(
      FileIcon.builtIn('file.py').$1,
      isNot(FileIcon.builtIn('file.rs').$1),
    );
    expect(FileIcon.builtIn('file.scss').$1, FileIcon.builtIn('file.css').$1);
    expect(
      FileIcon.builtIn('file.jpg').$1,
      isNot(FileIcon.builtIn('file.pdf').$1),
    );
  });

  test('dirs and links ignore the name', () {
    expect(
      FileIcon.keyOf(name: 'src', isDir: true, isLink: false),
      FileIcon.dir,
    );
    expect(
      FileIcon.keyOf(name: 'app.py', isDir: false, isLink: true),
      FileIcon.link,
    );
    expect(
      FileIcon.keyOf(name: 'app.py', isDir: false, isLink: false),
      'file.py',
    );
    expect(
      FileIcon.keyOf(name: 'README', isDir: false, isLink: false),
      FileIcon.fallback,
    );
  });
}
