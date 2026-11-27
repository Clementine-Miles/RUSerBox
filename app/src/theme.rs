//! Gruvbox and Gruvbox Material, light and dark: the app's only appearance.
//!
//! The palettes are the Flutter app's (`lib/data/res/gruvbox.dart`), and the
//! settings keep its names: `appThemePreset` (`material` | `original`) and
//! `themeMode` (0 system, 1 light, 2 dark).

use egui::{Color32, CornerRadius, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    #[default]
    Material,
    Original,
}

impl Preset {
    pub fn label(self) -> &'static str {
        match self {
            Preset::Material => "Gruvbox Material",
            Preset::Original => "Gruvbox",
        }
    }
    pub fn from_setting(s: Option<&str>) -> Self {
        if s == Some("original") { Preset::Original } else { Preset::Material }
    }
    pub fn name(self) -> &'static str {
        match self {
            Preset::Material => "material",
            Preset::Original => "original",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub foreground: Color32,
    pub muted: Color32,
    pub red: Color32,
    pub green: Color32,
    pub yellow: Color32,
    pub blue: Color32,
    pub purple: Color32,
    pub aqua: Color32,
    /// The dim ANSI colors (red … aqua) where the palette has its own.
    pub ansi: Option<[Color32; 6]>,
    pub dark: bool,
}

const fn c(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

const ORIGINAL_ANSI: [Color32; 6] = [c(0xCC241D), c(0x98971A), c(0xD79921), c(0x458588), c(0xB16286), c(0x689D6A)];

impl Palette {
    pub fn of(preset: Preset, dark: bool) -> Palette {
        match (preset, dark) {
            (Preset::Material, true) => Palette {
                background: c(0x282828),
                surface: c(0x32302F),
                raised: c(0x45403D),
                foreground: c(0xD4BE98),
                muted: c(0xA89984),
                red: c(0xEA6962),
                green: c(0xA9B665),
                yellow: c(0xD8A657),
                blue: c(0x7DAEA3),
                purple: c(0xD3869B),
                aqua: c(0x89B482),
                ansi: None,
                dark,
            },
            (Preset::Material, false) => Palette {
                background: c(0xFBF1C7),
                surface: c(0xF2E5BC),
                raised: c(0xE6D5AE),
                foreground: c(0x654735),
                muted: c(0x7C6F64),
                red: c(0xC14A4A),
                green: c(0x6C782E),
                yellow: c(0xA96B2C),
                blue: c(0x45707A),
                purple: c(0x945E80),
                aqua: c(0x4C7A5D),
                ansi: None,
                dark,
            },
            (Preset::Original, true) => Palette {
                background: c(0x282828),
                surface: c(0x3C3836),
                raised: c(0x504945),
                foreground: c(0xEBDBB2),
                muted: c(0xA89984),
                red: c(0xFB4934),
                green: c(0xB8BB26),
                yellow: c(0xFABD2F),
                blue: c(0x83A598),
                purple: c(0xD3869B),
                aqua: c(0x8EC07C),
                ansi: Some(ORIGINAL_ANSI),
                dark,
            },
            (Preset::Original, false) => Palette {
                background: c(0xFBF1C7),
                surface: c(0xEBDBB2),
                raised: c(0xD5C4A1),
                foreground: c(0x3C3836),
                muted: c(0x665C54),
                red: c(0x9D0006),
                green: c(0x79740E),
                yellow: c(0xB57614),
                blue: c(0x076678),
                purple: c(0x8F3F71),
                aqua: c(0x427B58),
                ansi: Some(ORIGINAL_ANSI),
                dark,
            },
        }
    }

    /// The 16 ANSI colors for the terminal: 0–7 normal, 8–15 bright.
    pub fn ansi16(&self) -> [Color32; 16] {
        let bright = [self.red, self.green, self.yellow, self.blue, self.purple, self.aqua];
        let normal = self.ansi.unwrap_or(bright);
        let (black, bright_black) = if self.dark { (self.surface, self.muted) } else { (self.raised, self.muted) };
        let (white, bright_white) = (self.muted, self.foreground);
        [
            black, normal[0], normal[1], normal[2], normal[3], normal[4], normal[5], white, bright_black, bright[0], bright[1], bright[2], bright[3], bright[4], bright[5], bright_white,
        ]
    }

    pub fn visuals(&self) -> Visuals {
        let mut v = if self.dark { Visuals::dark() } else { Visuals::light() };
        v.dark_mode = self.dark;
        v.override_text_color = Some(self.foreground);
        v.panel_fill = self.background;
        v.window_fill = self.surface;
        v.extreme_bg_color = if self.dark { c(0x1D2021) } else { c(0xF9F5D7) };
        v.faint_bg_color = self.surface;
        v.code_bg_color = self.surface;
        v.hyperlink_color = self.blue;
        v.warn_fg_color = self.yellow;
        v.error_fg_color = self.red;
        v.selection.bg_fill = self.blue.gamma_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0, self.foreground);
        let r = CornerRadius::same(10);
        v.window_corner_radius = CornerRadius::same(14);
        v.window_stroke = Stroke::new(1.0, self.raised);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = r;
            w.fg_stroke = Stroke::new(1.0, self.foreground);
        }
        v.widgets.noninteractive.bg_fill = self.surface;
        v.widgets.noninteractive.weak_bg_fill = self.surface;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.raised);
        v.widgets.inactive.bg_fill = self.raised;
        v.widgets.inactive.weak_bg_fill = self.surface;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.hovered.bg_fill = self.raised;
        v.widgets.hovered.weak_bg_fill = self.raised;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, self.muted);
        v.widgets.active.bg_fill = self.blue.gamma_multiply(0.6);
        v.widgets.active.weak_bg_fill = self.raised;
        v.widgets.active.bg_stroke = Stroke::new(1.0, self.blue);
        v.widgets.open.bg_fill = self.raised;
        v.widgets.open.weak_bg_fill = self.raised;
        v
    }
}

/// `themeMode`: follows the system unless set.
pub fn is_dark(mode: i64, system_dark: bool) -> bool {
    match mode {
        1 => false,
        2 => true,
        _ => system_dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_keeps_text_readable() {
        for preset in [Preset::Material, Preset::Original] {
            for dark in [false, true] {
                let p = Palette::of(preset, dark);
                let lum = |c: Color32| 0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b());
                assert!((lum(p.foreground) - lum(p.background)).abs() > 80.0, "{preset:?} dark={dark}");
                assert_eq!(p.ansi16().len(), 16);
            }
        }
        assert_eq!(Preset::from_setting(Some("original")), Preset::Original);
        assert_eq!(Preset::from_setting(None), Preset::Material);
        assert!(is_dark(2, false) && !is_dark(1, true) && is_dark(0, true));
    }
}
