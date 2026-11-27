//! The interface: one egui frame at a time over [`Core`].
//!
//! Pages are a stack ([`Page`]); the system back gesture pops it. Questions
//! (a host key, a confirmation) are modal sheets drawn over the page.

mod detail;
mod files;
mod keys;
mod servers;
mod settings;
mod snippets;
mod terminal;
pub mod widgets;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{Align, Context, Layout, RichText};
use sbm_store::{KvStore, Store};

use crate::core::{Core, lock};
use crate::fonts::Typeface;
use crate::i18n::t;
use crate::theme::{Palette, Preset};

/// What the platform does for the interface.
pub trait Platform {
    fn set_clipboard(&self, text: &str);
    fn clipboard(&self) -> Option<String>;
    /// Opens a file picker; the result comes back through [`App::file_picked`].
    fn pick_file(&self, purpose: Pick);
    /// Saves [data] where the person chooses.
    fn save_file(&self, name: &str, data: Vec<u8>);
    /// Keeps the process alive in the background while sessions are open.
    fn keep_alive(&self, on: bool);
    /// Whether the system is in dark mode.
    fn system_dark(&self) -> bool;
}

/// Why a file is being picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    ImportKey,
    /// Upload into this remote directory of this server.
    Upload { server: String, dir: String },
}

/// A picked file, its display name and its bytes.
pub struct Picked {
    pub purpose: Pick,
    pub name: String,
    pub data: Vec<u8>,
}

/// Picked files arrive here from the platform (on another thread).
pub type Inbox = Arc<Mutex<VecDeque<Picked>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Servers,
    EditServer(Option<String>),
    Detail(String),
    Terminal,
    Files(String),
    Snippets,
    Keys,
    Settings,
}

pub struct App {
    pub core: Core,
    pub stack: Vec<Page>,
    pub palette: Palette,
    pub preset: Preset,
    pub theme_mode: i64,
    pub typeface: Typeface,
    pub font_size: f32,
    pub inbox: Inbox,
    servers: servers::State,
    detail: detail::State,
    pub terms: terminal::Sessions,
    files: files::State,
    snippets: snippets::State,
    keys: keys::State,
    settings: settings::State,
    toast: Option<(String, Instant)>,
    fonts_for: Option<Typeface>,
    applied_dark: Option<bool>,
}

impl App {
    pub fn new(store: Arc<Store>, ctx: Context) -> App {
        let kv = store.kv(KvStore::Setting);
        let preset = Preset::from_setting(kv.get::<String>("appThemePreset").as_deref());
        let theme_mode = kv.get_or("themeMode", 0i64);
        let typeface = Typeface::from_setting(kv.get::<String>("terminalTypeface").as_deref());
        let font_size = kv.get_or("termFontSize", 13.0f64) as f32;
        let locale: String = kv.get_or("locale", String::new());
        if !locale.is_empty() {
            crate::i18n::set_locale(&locale);
        }
        let core = Core::new(store, ctx);
        App {
            core,
            stack: vec![Page::Servers],
            palette: Palette::of(preset, true),
            preset,
            theme_mode,
            typeface,
            font_size: font_size.clamp(8.0, 32.0),
            inbox: Arc::default(),
            servers: Default::default(),
            detail: Default::default(),
            terms: Default::default(),
            files: Default::default(),
            snippets: Default::default(),
            keys: Default::default(),
            settings: Default::default(),
            toast: None,
            fonts_for: None,
            applied_dark: None,
        }
    }

    pub fn page(&self) -> &Page {
        self.stack.last().unwrap_or(&Page::Servers)
    }

    pub fn push(&mut self, p: Page) {
        if self.page() != &p {
            self.stack.push(p);
        }
    }

    /// The system back action; false when there is nowhere to go back to.
    pub fn back(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn store(&self) -> Arc<Store> {
        self.core.store.clone()
    }

    fn apply_style(&mut self, ctx: &Context, system_dark: bool) {
        let dark = crate::theme::is_dark(self.theme_mode, system_dark);
        if self.applied_dark != Some(dark) || self.palette != Palette::of(self.preset, dark) {
            self.palette = Palette::of(self.preset, dark);
            ctx.set_visuals(self.palette.visuals());
            self.applied_dark = Some(dark);
        }
        if self.fonts_for != Some(self.typeface) {
            ctx.set_fonts(crate::fonts::definitions(self.typeface));
            self.fonts_for = Some(self.typeface);
        }
    }

    /// One frame.
    pub fn update(&mut self, root: &mut egui::Ui, platform: &dyn Platform) {
        let ctx = &root.ctx().clone();
        self.apply_style(ctx, platform.system_dark());
        ctx.global_style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(12.0, 8.0);
            s.spacing.interact_size.y = 36.0;
            s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
            s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
        });

        // Files picked since the last frame.
        let picked: Vec<Picked> = lock(&self.inbox).drain(..).collect();
        for p in picked {
            self.file_picked(p);
        }
        let notices: Vec<String> = lock(&self.core.notices).drain(..).collect();
        for n in notices {
            self.toast(n);
        }
        platform.keep_alive(self.terms.any_open());

        let p = self.palette;
        let page = self.page().clone();
        if page != Page::Terminal {
            egui::Panel::bottom("nav").frame(egui::Frame::new().fill(p.surface).inner_margin(egui::Margin::symmetric(8, 6))).show(root, |ui| self.nav(ui));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.background).inner_margin(egui::Margin::same(if page == Page::Terminal { 0 } else { 12 }))).show(root, |ui| match page {
            Page::Servers => servers::list(self, ui),
            Page::EditServer(id) => servers::edit(self, ui, id),
            Page::Detail(id) => detail::show(self, ui, &id),
            Page::Terminal => terminal::show(self, ui, platform),
            Page::Files(id) => files::show(self, ui, &id, platform),
            Page::Snippets => snippets::show(self, ui),
            Page::Keys => keys::show(self, ui, platform),
            Page::Settings => settings::show(self, ui),
        });

        self.host_key_prompt(ctx);
        self.draw_toast(ctx);
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        let items = [
            (Page::Servers, "\u{f233}", t("server", "Servers")),
            (Page::Terminal, "\u{f120}", t("terminal", "Terminal")),
            (Page::Snippets, "\u{f121}", t("snippet", "Snippets")),
            (Page::Keys, "\u{f084}", t("privateKey", "Keys")),
            (Page::Settings, "\u{f013}", t("setting", "Settings")),
        ];
        ui.columns(items.len(), |cols| {
            for (i, (page, icon, label)) in items.into_iter().enumerate() {
                let here = std::mem::discriminant(self.page()) == std::mem::discriminant(&page);
                let color = if here { p.yellow } else { p.muted };
                let resp = cols[i].with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.add(egui::Button::new(RichText::new(format!("{icon}\n{label}")).color(color).size(13.0)).frame(false).min_size(egui::vec2(56.0, 44.0)))
                });
                if resp.inner.clicked() {
                    self.stack.truncate(1);
                    if page != Page::Servers {
                        self.stack.push(page);
                    }
                }
            }
        });
    }

    fn host_key_prompt(&mut self, ctx: &Context) {
        let p = self.palette;
        let mut prompts = lock(&self.core.prompts);
        let Some(front) = prompts.front_mut() else { return };
        let mut answered = None;
        egui::Modal::new(egui::Id::unique("hostkey")).show(ctx, |ui| {
            ui.set_max_width(520.0);
            let mismatch = front.prompt.previous.is_some();
            ui.heading(if mismatch { t("hostKeyChanged", "Host key changed") } else { t("unknownHostKey", "Unknown host key") });
            ui.add_space(6.0);
            ui.label(format!("{} ({})", front.server_name, front.prompt.host));
            ui.label(RichText::new(&front.prompt.key_type).color(p.muted));
            ui.add(egui::Label::new(RichText::new(&front.prompt.fingerprint).monospace()).wrap());
            if let Some(prev) = &front.prompt.previous {
                ui.add_space(6.0);
                ui.colored_label(p.red, t("hostKeyMismatchTip", "The server presented a different key than the one trusted before. This can mean someone is intercepting the connection."));
                ui.label(RichText::new(t("previous", "Previous")).color(p.muted));
                ui.add(egui::Label::new(RichText::new(prev).monospace()).wrap());
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(t("cancel", "Cancel")).clicked() {
                    answered = Some(false);
                }
                let accept = if mismatch { RichText::new(t("trustNewKey", "Trust the new key")).color(p.red) } else { RichText::new(t("trust", "Trust")) };
                if ui.button(accept).clicked() {
                    answered = Some(true);
                }
            });
        });
        if let Some(a) = answered {
            front.answer(a);
            prompts.pop_front();
        }
    }

    fn draw_toast(&mut self, ctx: &Context) {
        let Some((msg, at)) = &self.toast else { return };
        if at.elapsed() > Duration::from_secs(4) {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(500));
        let p = self.palette;
        egui::Area::new(egui::Id::unique("toast")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -90.0)).order(egui::Order::Tooltip).show(ctx, |ui| {
            egui::Frame::new().fill(p.raised).corner_radius(12).inner_margin(egui::Margin::symmetric(14, 10)).show(ui, |ui| {
                ui.set_max_width(ctx.content_rect().width() - 48.0);
                ui.label(RichText::new(msg).color(p.foreground));
            });
        });
    }

    fn file_picked(&mut self, f: Picked) {
        match f.purpose.clone() {
            Pick::ImportKey => keys::import(self, &f.name, &f.data),
            Pick::Upload { server, dir } => files::upload(self, &server, &dir, f.name, f.data),
        }
    }
}
