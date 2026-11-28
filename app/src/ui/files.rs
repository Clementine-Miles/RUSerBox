//! A server's files: SFTP over SSH, or the Monitor's file API for a server
//! reached only through the agent. Uploads come from the platform's picker;
//! downloads go where the person saves them.

use std::sync::Arc;

use egui::{RichText, Ui};

use super::detail::Slot;
use super::{App, Pick, Platform};
use crate::core::lock;
use crate::i18n::t;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
    pub link: bool,
    pub size: Option<u64>,
    pub mode: Option<u32>,
}

#[derive(Clone)]
enum Fs {
    Sftp(Arc<sbm_ssh::SftpSession>),
    Monitor(Arc<sbm_monitor_client::Client>),
}

impl Fs {
    async fn list(&self, path: &str) -> Result<Vec<Entry>, String> {
        let mut v = match self {
            Fs::Sftp(s) => s
                .read_dir(path)
                .await
                .map_err(|e| e.to_string())?
                .map(|e| {
                    let m = e.metadata();
                    Entry { name: e.file_name(), dir: m.is_dir(), link: m.is_symlink(), size: m.size, mode: m.permissions.map(|p| p & 0o7777) }
                })
                .collect::<Vec<_>>(),
            Fs::Monitor(c) => c
                .fs_list(path)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|e| Entry { dir: e.kind == "dir", link: e.kind == "link", name: e.name, size: e.size, mode: e.mode })
                .collect(),
        };
        v.retain(|e| e.name != "." && e.name != "..");
        v.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(v)
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        match self {
            Fs::Sftp(s) => s.read(path).await.map_err(|e| e.to_string()),
            Fs::Monitor(c) => c.fs_read(path).await.map_err(|e| e.to_string()),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<(), String> {
        match self {
            Fs::Sftp(s) => s.write(path, &data).await.map_err(|e| e.to_string()),
            Fs::Monitor(c) => c.fs_write(path, data).await.map(drop).map_err(|e| e.to_string()),
        }
    }

    async fn mkdir(&self, path: &str) -> Result<(), String> {
        match self {
            Fs::Sftp(s) => s.create_dir(path).await.map_err(|e| e.to_string()),
            Fs::Monitor(c) => c.fs_mkdir(path).await.map_err(|e| e.to_string()),
        }
    }

    async fn remove(&self, path: &str, dir: bool) -> Result<(), String> {
        match self {
            Fs::Sftp(s) => if dir { s.remove_dir(path).await } else { s.remove_file(path).await }.map_err(|e| e.to_string()),
            Fs::Monitor(c) => c.fs_remove(path, false).await.map_err(|e| e.to_string()),
        }
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        match self {
            Fs::Sftp(s) => s.rename(from, to).await.map_err(|e| e.to_string()),
            Fs::Monitor(c) => c.fs_rename(from, to).await.map_err(|e| e.to_string()),
        }
    }

    async fn home(&self) -> String {
        match self {
            Fs::Sftp(s) => s.canonicalize(".").await.unwrap_or_else(|_| "/".into()),
            Fs::Monitor(c) => c.fs_roots().await.ok().and_then(|r| r.into_iter().next()).unwrap_or_else(|| "/".into()),
        }
    }
}

#[derive(Default)]
pub struct State {
    server: String,
    fs: Slot<(Fs, String)>,
    ready: Option<Fs>,
    path: String,
    listing: Slot<Vec<Entry>>,
    busy: Slot<String>,
    rename: Option<(String, String)>,
    new_dir: Option<String>,
}

/// `dir` joined with `name`, POSIX style.
pub fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') { format!("{dir}{name}") } else { format!("{dir}/{name}") }
}

/// The parent of a POSIX path (`/` stays `/`).
pub fn parent(path: &str) -> String {
    let t = path.trim_end_matches('/');
    match t.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => t[..i].to_owned(),
    }
}

fn refresh(app: &App) {
    let Some(fs) = app.files.ready.clone() else { return };
    let path = app.files.path.clone();
    *lock(&app.files.listing) = None;
    let slot = app.files.listing.clone();
    let ctx = app.core.ctx.clone();
    app.core.rt.spawn(async move {
        *lock(&slot) = Some(fs.list(&path).await);
        ctx.request_repaint();
    });
}

fn run(app: &App, fut: impl std::future::Future<Output = Result<String, String>> + Send + 'static) {
    *lock(&app.files.busy) = None;
    let slot = app.files.busy.clone();
    let ctx = app.core.ctx.clone();
    app.core.rt.spawn(async move {
        *lock(&slot) = Some(fut.await);
        ctx.request_repaint();
    });
}

pub fn show(app: &mut App, ui: &mut Ui, id: &str, platform: &dyn Platform) {
    let p = app.palette;
    let Some(server) = app.core.store.server(id).ok().flatten() else {
        app.back();
        return;
    };
    if app.files.server != id {
        app.files = State { server: id.to_owned(), ..Default::default() };
        // Open the file system: SFTP when SSH is configured, else the agent.
        let slot = app.files.fs.clone();
        let ctx = app.core.ctx.clone();
        if server.uses_ssh() {
            let fut = app.core.session(&server);
            app.core.rt.spawn(async move {
                let r = async {
                    let sess = fut.await?;
                    let sftp = Fs::Sftp(Arc::new(sess.sftp().await.map_err(|e| e.to_string())?));
                    let home = sftp.home().await;
                    Ok((sftp, home))
                }
                .await;
                *lock(&slot) = Some(r);
                ctx.request_repaint();
            });
        } else {
            let client = app.core.monitor_client(&server);
            app.core.rt.spawn(async move {
                let r = async {
                    let c = Arc::new(client?);
                    c.login().await.map_err(|e| e.to_string())?;
                    let fs = Fs::Monitor(c);
                    let home = fs.home().await;
                    Ok((fs, home))
                }
                .await;
                *lock(&slot) = Some(r);
                ctx.request_repaint();
            });
        }
    }
    let opened = lock(&app.files.fs).take();
    match opened {
        Some(Ok((fs, home))) => {
            app.files.ready = Some(fs);
            app.files.path = home;
            refresh(app);
        }
        Some(Err(e)) => *lock(&app.files.listing) = Some(Err(e)),
        None => {}
    }
    if let Some(done) = lock(&app.files.busy).take() {
        match done {
            Ok(msg) => {
                if !msg.is_empty() {
                    app.core.notice(msg);
                }
            }
            Err(e) => app.core.notice(e),
        }
        refresh(app);
    }

    ui.horizontal(|ui| {
        if ui.button("←").clicked() {
            app.back();
        }
        ui.heading(format!("{} · {}", t("files", "Files"), server.name));
    });
    if app.files.ready.is_none() {
        if let Some(Err(e)) = lock(&app.files.listing).as_ref() {
            ui.colored_label(p.red, e);
        } else {
            ui.spinner();
        }
        return;
    }
    let mut go: Option<String> = None;
    ui.horizontal(|ui| {
        if ui.button("\u{f062}").clicked() {
            go = Some(parent(&app.files.path));
        }
        let mut path = app.files.path.clone();
        let r = ui.add(egui::TextEdit::singleline(&mut path).desired_width(ui.available_width() - 160.0));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            go = Some(path);
        }
        if ui.button("\u{f021}").clicked() {
            go = Some(app.files.path.clone());
        }
        if ui.button(t("upload", "Upload")).clicked() {
            platform.pick_file(Pick::Upload { server: id.to_owned(), dir: app.files.path.clone() });
        }
        if ui.button("\u{f067} \u{f07b}").clicked() {
            app.files.new_dir = Some(String::new());
        }
    });
    if let Some(name) = &mut app.files.new_dir {
        let mut done = None;
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(name).hint_text(t("name", "Name")));
            if ui.button(t("ok", "OK")).clicked() {
                done = Some(true);
            }
            if ui.button(t("cancel", "Cancel")).clicked() {
                done = Some(false);
            }
        });
        if let Some(ok) = done {
            let name = app.files.new_dir.take().unwrap_or_default();
            if ok && !name.trim().is_empty() {
                let fs = app.files.ready.clone().expect("checked");
                let path = join(&app.files.path, name.trim());
                run(app, async move { fs.mkdir(&path).await.map(|_| String::new()) });
            }
        }
    }
    if let Some((from, to)) = &mut app.files.rename {
        let mut done = None;
        ui.horizontal(|ui| {
            ui.label(format!("{} →", from));
            ui.add(egui::TextEdit::singleline(to));
            if ui.button(t("ok", "OK")).clicked() {
                done = Some(true);
            }
            if ui.button(t("cancel", "Cancel")).clicked() {
                done = Some(false);
            }
        });
        if let Some(ok) = done {
            let (from, to) = app.files.rename.take().unwrap_or_default();
            if ok && !to.trim().is_empty() && to != from {
                let fs = app.files.ready.clone().expect("checked");
                let (a, b) = (join(&app.files.path, &from), join(&app.files.path, to.trim()));
                run(app, async move { fs.rename(&a, &b).await.map(|_| String::new()) });
            }
        }
    }
    ui.separator();
    let listing = lock(&app.files.listing).clone();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match listing {
        None => {
            ui.spinner();
        }
        Some(Err(e)) => {
            ui.colored_label(p.red, e);
        }
        Some(Ok(entries)) => {
            for e in entries {
                ui.horizontal(|ui| {
                    let icon = if e.dir { "\u{f07b}" } else if e.link { "\u{f0c1}" } else { "\u{f15b}" };
                    let r = ui.add(egui::Button::new(RichText::new(format!("{icon} {}", e.name))).frame(false));
                    if r.clicked() && (e.dir || e.link) {
                        go = Some(join(&app.files.path, &e.name));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("\u{f142}", |ui| {
                            if !e.dir && ui.button(t("download", "Download")).clicked() {
                                let fs = app.files.ready.clone().expect("checked");
                                let path = join(&app.files.path, &e.name);
                                let name = e.name.clone();
                                let slot: Slot<(String, Vec<u8>)> = Slot::default();
                                let out = slot.clone();
                                let ctx = app.core.ctx.clone();
                                app.core.rt.spawn(async move {
                                    *lock(&out) = Some(fs.read(&path).await.map(|d| (name, d)));
                                    ctx.request_repaint();
                                });
                                ui.memory_mut(|m| m.data.insert_temp(egui::Id::unique("download"), DownloadSlot(slot)));
                                ui.close();
                            }
                            if ui.button(t("rename", "Rename")).clicked() {
                                app.files.rename = Some((e.name.clone(), e.name.clone()));
                                ui.close();
                            }
                            if ui.button(RichText::new(t("delete", "Delete")).color(p.red)).clicked() {
                                let fs = app.files.ready.clone().expect("checked");
                                let path = join(&app.files.path, &e.name);
                                let dir = e.dir;
                                run(app, async move { fs.remove(&path, dir).await.map(|_| String::new()) });
                                ui.close();
                            }
                        });
                        if let Some(size) = e.size.filter(|_| !e.dir) {
                            ui.label(RichText::new(super::widgets::bytes(size)).color(p.muted).size(12.0));
                        }
                        if let Some(m) = e.mode {
                            ui.label(RichText::new(format!("{m:o}")).color(p.muted).monospace().size(11.0));
                        }
                    });
                });
            }
        }
    });
    // A finished download goes to the platform's save dialog.
    if let Some(DownloadSlot(slot)) = ui.memory(|m| m.data.get_temp::<DownloadSlot>(egui::Id::unique("download"))) {
        let done = lock(&slot).take();
        if let Some(r) = done {
            ui.memory_mut(|m| m.data.remove::<DownloadSlot>(egui::Id::unique("download")));
            match r {
                Ok((name, data)) => platform.save_file(&name, data),
                Err(e) => app.toast(e),
            }
        }
    }
    if let Some(path) = go {
        app.files.path = path;
        refresh(app);
    }
}

#[derive(Clone)]
struct DownloadSlot(Slot<(String, Vec<u8>)>);

/// Writes a picked file into [dir] on [server].
pub fn upload(app: &mut App, server: &str, dir: &str, name: String, data: Vec<u8>) {
    if app.files.server != server {
        return;
    }
    let Some(fs) = app.files.ready.clone() else { return };
    let path = join(dir, &name);
    let n = data.len() as u64;
    run(app, async move { fs.write(&path, data).await.map(|_| format!("{} ({})", t("uploaded", "Uploaded"), super::widgets::bytes(n))) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_paths() {
        assert_eq!(join("/home/a", "b"), "/home/a/b");
        assert_eq!(join("/", "etc"), "/etc");
        assert_eq!(parent("/home/a/"), "/home");
        assert_eq!(parent("/home"), "/");
        assert_eq!(parent("/"), "/");
    }
}
