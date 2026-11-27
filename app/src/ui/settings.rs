//! Settings, kept under the Flutter app's keys in `kv('setting', …)`.

use egui::{RichText, Ui};
use sbm_store::KvStore;

use super::App;
use super::widgets::card;
use crate::fonts::Typeface;
use crate::i18n::{self, t};
use crate::theme::Preset;

#[derive(Default)]
pub struct State {
    licenses: bool,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = app.palette;
    let store = app.store();
    let kv = store.kv(KvStore::Setting);
    ui.heading(t("setting", "Settings"));
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        card(ui, &p, &t("theme", "Appearance"), |ui| {
            ui.horizontal(|ui| {
                for preset in [Preset::Material, Preset::Original] {
                    if ui.selectable_label(app.preset == preset, preset.label()).clicked() {
                        app.preset = preset;
                        let _ = kv.set("appThemePreset", preset.name());
                    }
                }
            });
            ui.horizontal(|ui| {
                for (mode, label) in [(0, t("followSystem", "System")), (1, t("light", "Light")), (2, t("dark", "Dark"))] {
                    if ui.selectable_label(app.theme_mode == mode, label).clicked() {
                        app.theme_mode = mode;
                        let _ = kv.set("themeMode", &mode);
                    }
                }
            });
        });
        card(ui, &p, &t("terminal", "Terminal"), |ui| {
            egui::ComboBox::from_label(t("font", "Font")).selected_text(app.typeface.label()).show_ui(ui, |ui| {
                for f in Typeface::ALL {
                    if ui.selectable_label(app.typeface == f, f.label()).clicked() {
                        app.typeface = f;
                        let _ = kv.set("terminalTypeface", f.name());
                    }
                }
            });
            let mut size = app.font_size;
            if ui.add(egui::Slider::new(&mut size, 8.0..=32.0).step_by(1.0).text(t("fontSize", "Font size"))).changed() {
                app.font_size = size;
                let _ = kv.set("termFontSize", &f64::from(size));
            }
        });
        card(ui, &p, &t("server", "Servers"), |ui| {
            let mut interval: i64 = kv.get_or("serverStatusUpdateInterval", 3);
            if ui.add(egui::Slider::new(&mut interval, 1..=60).text(t("updateInterval", "Status interval (s)"))).changed() {
                let _ = kv.set("serverStatusUpdateInterval", &interval);
            }
            let mut timeout: i64 = kv.get_or("timeOut", 5);
            if ui.add(egui::Slider::new(&mut timeout, 2..=60).text(t("timeout", "Timeout (s)"))).changed() {
                let _ = kv.set("timeOut", &timeout);
            }
        });
        card(ui, &p, &t("language", "Language"), |ui| {
            let current = i18n::locale();
            egui::ComboBox::from_id_salt("lang").selected_text(&current).show_ui(ui, |ui| {
                for l in i18n::languages() {
                    if ui.selectable_label(current == l, l).clicked() {
                        i18n::set_locale(l);
                        let _ = kv.set("locale", l);
                    }
                }
            });
        });
        card(ui, &p, &t("about", "About"), |ui| {
            ui.label(RichText::new(format!("RUSerBox {}", env!("CARGO_PKG_VERSION"))).strong());
            ui.label(RichText::new("AGPL-3.0-or-later").color(p.muted));
            if ui.button(t("license", "Licenses")).clicked() {
                app.settings.licenses = !app.settings.licenses;
            }
            if app.settings.licenses {
                for (name, text) in LICENSES {
                    ui.collapsing(*name, |ui| {
                        ui.label(RichText::new(*text).monospace().size(10.0));
                    });
                }
            }
        });
    });
}

const LICENSES: &[(&str, &str)] = &[
    ("Hack Nerd Font", include_str!("../../../assets/fonts/Hack-Nerd-Font-LICENSE.txt")),
    ("JetBrains Mono", include_str!("../../../assets/fonts/JetBrains-Mono-LICENSE.txt")),
    ("Fira Code", include_str!("../../../assets/fonts/Fira-Code-LICENSE.txt")),
    ("Iosevka", include_str!("../../../assets/fonts/Iosevka-LICENSE.txt")),
    ("Cascadia Code", include_str!("../../../assets/fonts/Cascadia-Code-LICENSE.txt")),
];
