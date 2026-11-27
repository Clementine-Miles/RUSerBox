//! Snippets: saved scripts, run in a terminal on a chosen server.

use egui::{RichText, Ui};
use sbm_store::Snippet;

use super::widgets::{card, field, wide_button};
use super::{App, Page};
use crate::i18n::t;

#[derive(Default)]
pub struct State {
    editing: Option<Snippet>,
    tags: String,
    run: Option<String>,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = app.palette;
    let store = app.store();
    if let Some(s) = app.snippets.editing.as_mut() {
        let mut close = false;
        let mut save = false;
        let mut delete = false;
        ui.horizontal(|ui| {
            if ui.button("←").clicked() {
                close = true;
            }
            ui.heading(t("snippet", "Snippet"));
        });
        card(ui, &p, "", |ui| {
            field(ui, &t("name", "Name"), &mut s.name, false);
            field(ui, &t("tag", "Tags"), &mut app.snippets.tags, false);
            ui.label(t("content", "Script"));
            ui.add(egui::TextEdit::multiline(&mut s.script).code_editor().desired_rows(8).desired_width(f32::INFINITY));
            let mut note = s.note.clone().unwrap_or_default();
            field(ui, &t("note", "Note"), &mut note, false);
            s.note = Some(note).filter(|n| !n.trim().is_empty());
        });
        if wide_button(ui, t("save", "Save")).clicked() {
            save = true;
        }
        if wide_button(ui, RichText::new(t("delete", "Delete")).color(p.red)).clicked() {
            delete = true;
        }
        let mut error = None;
        if save {
            s.tags = app.snippets.tags.split(',').map(|x| x.trim().to_owned()).filter(|x| !x.is_empty()).collect();
            match store.put_snippet(s) {
                Ok(()) => close = true,
                Err(e) => error = Some(e.to_string()),
            }
        }
        if delete {
            let _ = store.delete_snippet(&s.id);
            close = true;
        }
        if close {
            app.snippets.editing = None;
        }
        if let Some(e) = error {
            app.toast(e);
        }
        return;
    }
    ui.horizontal(|ui| {
        ui.heading(t("snippet", "Snippets"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("\u{f067} {}", t("add", "Add"))).color(p.yellow)).clicked() {
                app.snippets.tags.clear();
                app.snippets.editing = Some(Snippet { id: sbm_store::new_id(), name: String::new(), script: String::new(), note: None, tags: vec![] });
            }
        });
    });
    let snippets = store.snippets().unwrap_or_default();
    let servers: Vec<_> = store.servers().unwrap_or_default().into_iter().filter(|s| s.uses_ssh()).collect();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for s in snippets {
            card(ui, &p, "", |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&s.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t("edit", "Edit")).clicked() {
                            app.snippets.tags = s.tags.join(", ");
                            app.snippets.editing = Some(s.clone());
                        }
                        if ui.button(format!("\u{f04b} {}", t("run", "Run"))).clicked() {
                            app.snippets.run = Some(s.id.clone());
                        }
                    });
                });
                ui.label(RichText::new(s.script.lines().take(3).collect::<Vec<_>>().join("\n")).monospace().color(p.muted).size(12.0));
                if app.snippets.run.as_deref() == Some(&s.id) {
                    ui.separator();
                    for server in &servers {
                        if ui.button(&server.name).clicked() {
                            app.terms.open_ssh(&mut app.core, server, None, &format!("{} · {}", s.name, server.name));
                            // Typed into the shell once it starts.
                            app.snippets.run = None;
                            app.terms.queue_last(s.script.clone());
                            app.push(Page::Terminal);
                        }
                    }
                }
            });
        }
    });
}
