//! Fonts: the bundled terminal typefaces (Hack Nerd Font is always loaded,
//! for Nerd Font glyphs and as the fallback), egui's own for the interface,
//! and the system's CJK font, read at startup rather than shipped.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};
use serde::{Deserialize, Serialize};

/// `terminalTypeface`, by the Flutter app's names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Typeface {
    #[default]
    Hack,
    JetBrainsMono,
    FiraCode,
    Iosevka,
    CascadiaCode,
}

impl Typeface {
    pub const ALL: [Typeface; 5] = [Typeface::Hack, Typeface::JetBrainsMono, Typeface::FiraCode, Typeface::Iosevka, Typeface::CascadiaCode];

    pub fn label(self) -> &'static str {
        match self {
            Typeface::Hack => "Hack Nerd Font",
            Typeface::JetBrainsMono => "JetBrains Mono",
            Typeface::FiraCode => "Fira Code",
            Typeface::Iosevka => "Iosevka",
            Typeface::CascadiaCode => "Cascadia Code",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Typeface::Hack => "hack",
            Typeface::JetBrainsMono => "jetBrainsMono",
            Typeface::FiraCode => "firaCode",
            Typeface::Iosevka => "iosevka",
            Typeface::CascadiaCode => "cascadiaCode",
        }
    }

    pub fn from_setting(s: Option<&str>) -> Self {
        Typeface::ALL.into_iter().find(|t| Some(t.name()) == s).unwrap_or_default()
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Typeface::Hack => include_bytes!("../../assets/fonts/HackNerdFontMono-Regular.ttf"),
            Typeface::JetBrainsMono => include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
            Typeface::FiraCode => include_bytes!("../../assets/fonts/FiraCode-Regular.ttf"),
            Typeface::Iosevka => include_bytes!("../../assets/fonts/Iosevka-Terminal-Regular.ttf"),
            Typeface::CascadiaCode => include_bytes!("../../assets/fonts/CascadiaCode-Regular.ttf"),
        }
    }
}

/// System fonts with CJK coverage, first found wins (Android, then Linux).
const CJK_CANDIDATES: &[(&str, u32)] = &[
    ("/system/fonts/NotoSansCJK-Regular.ttc", 0),
    ("/system/fonts/NotoSansSC-Regular.otf", 0),
    ("/system/fonts/DroidSansFallback.ttf", 0),
    ("/system/fonts/DroidSansFallbackFull.ttf", 0),
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", 0),
];

fn system_cjk() -> Option<FontData> {
    CJK_CANDIDATES.iter().find_map(|(p, idx)| {
        let bytes = std::fs::read(p).ok()?;
        let mut d = FontData::from_owned(bytes);
        d.index = *idx;
        Some(d)
    })
}

/// The family the terminal draws with.
pub const TERM: &str = "term";

pub fn definitions(face: Typeface) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("hack".into(), Arc::new(FontData::from_static(Typeface::Hack.bytes())));
    if face != Typeface::Hack {
        defs.font_data.insert(face.name().into(), Arc::new(FontData::from_static(face.bytes())));
    }
    let cjk = system_cjk().map(|d| {
        defs.font_data.insert("cjk".into(), Arc::new(d));
        "cjk".to_owned()
    });
    let emoji: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default().into_iter().filter(|n| n.contains("emoji") || n.contains("Emoji")).collect();

    let mut term = vec![face.name().to_owned()];
    if face != Typeface::Hack {
        term.push("hack".into());
    }
    term.extend(cjk.clone());
    term.extend(emoji.iter().cloned());
    defs.families.insert(FontFamily::Name(TERM.into()), term);

    let mono = defs.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "hack".into());
    mono.extend(cjk.clone());
    let prop = defs.families.entry(FontFamily::Proportional).or_default();
    prop.push("hack".into());
    prop.extend(cjk);
    defs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terminal_family_falls_back_to_hack() {
        let d = definitions(Typeface::FiraCode);
        let term = &d.families[&FontFamily::Name(TERM.into())];
        assert_eq!(term[0], "firaCode");
        assert_eq!(term[1], "hack");
        assert_eq!(Typeface::from_setting(Some("iosevka")), Typeface::Iosevka);
        assert_eq!(Typeface::from_setting(Some("bogus")), Typeface::Hack);
    }
}
