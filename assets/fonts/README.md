# Terminal fonts

Exactly two families are bundled, one regular face each. Settings → Terminal →
Font picks one; Hack Nerd Font Mono is the default. Both are the official Nerd
Fonts "Mono" builds, so Powerline and Nerd Font icons take one terminal cell.
Characters neither face covers (CJK, emoji, many scripts) fall back to the
Android system fonts.

| Font asset | Source | License |
| --- | --- | --- |
| `HackNerdFontMono-Regular.ttf` | [Nerd Fonts v3.5.1](https://github.com/ryanoasis/nerd-fonts/releases/tag/v3.5.1), `Hack.tar.xz` | [Hack and Nerd Fonts notices](Hack-Nerd-Font-LICENSE.txt) |
| `JetBrainsMonoNLNerdFontMono-Regular.ttf` | [Nerd Fonts v3.5.1](https://github.com/ryanoasis/nerd-fonts/releases/tag/v3.5.1), `JetBrainsMono.tar.xz` (sha256 `04d5e8f9…10cf`) | [JetBrains Mono and Nerd Fonts notices](JetBrainsMono-Nerd-Font-LICENSE.txt) |

The JetBrains face is the **NL** (no-ligature) build: it has no `GSUB` table,
so `->` or `!=` never merge into one glyph spanning two cells. Both files are
byte-for-byte copies of the release files (sha256 of the JetBrains face:
`d9a80146bbf2ff23b187316a3e82ee2a2ceab8220267fe09ac150ca46ab2c3bd`). License
notices are also shown in Settings → About → License.

A stored choice of a retired font (Fira Code, Iosevka, Cascadia Code) reads as
Hack; the old `jetBrainsMono` choice now selects the Nerd Font build.
