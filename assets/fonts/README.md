# Terminal fonts

ServerBox bundles one regular face per family. Settings → SSH → Font selects
which family the terminal uses. Hack Nerd Font Mono is the default and keeps
Nerd Font icons the width of a terminal cell. The terminal falls back to system
fonts for characters absent from the selected face.

| Font asset | Source | License |
| --- | --- | --- |
| `HackNerdFontMono-Regular.ttf` | [Nerd Fonts v3.5.1](https://github.com/ryanoasis/nerd-fonts/releases/tag/v3.5.1), `Hack.tar.xz` | [Hack and Nerd Fonts notices](Hack-Nerd-Font-LICENSE.txt) |
| `JetBrainsMono-Regular.ttf` | [JetBrains Mono v2.304](https://github.com/JetBrains/JetBrainsMono/tree/v2.304/fonts/ttf) | [SIL OFL 1.1](JetBrains-Mono-LICENSE.txt) |
| `FiraCode-Regular.ttf` | [Fira Code 6.2](https://github.com/tonsky/FiraCode/releases/tag/6.2), `ttf/` in the release ZIP | [SIL OFL 1.1](Fira-Code-LICENSE.txt) |
| `Iosevka-Terminal-Regular.ttf` | [Iosevka v34.9.1](https://github.com/be5invis/Iosevka/releases/tag/v34.9.1), `Iosevka-Regular.ttf` in `PkgTTF-Iosevka-34.9.1.zip` | [SIL OFL 1.1](Iosevka-LICENSE.txt) |
| `CascadiaCode-Regular.ttf` | [Cascadia Code v2407.24](https://github.com/microsoft/cascadia-code/releases/tag/v2407.24), `ttf/static/` in the release ZIP | [SIL OFL 1.1](Cascadia-Code-LICENSE.txt) |

Iosevka was reduced from 11.6 MB to 1.4 MB by removing optional OpenType style
variants. All 7,761 mapped characters, hinting, font names, default ligatures
and mark positioning are retained. The other four faces are byte-for-byte
copies of the source fonts. License notices are available in Settings → About
→ License.

To reproduce the Iosevka asset with FontTools 4.61.1:

```sh
pyftsubset Iosevka-Regular.ttf --output-file=Iosevka-Terminal-Regular.ttf \
  --unicodes='*' --layout-features='calt,ccmp,locl,mark,mkmk' \
  --name-IDs='*' --name-legacy --name-languages='*'
```
