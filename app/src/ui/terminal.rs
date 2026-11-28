//! Terminal sessions: remote shells over SSH (a PTY channel) or through the
//! Monitor's terminal WebSocket, each drawn by [`crate::term::view`].

use std::sync::Arc;

use egui::{RichText, Ui};
use sbm_monitor_client::{MonitorTerminal, TerminalEvent};
use sbm_ssh::{Shell, ShellEvent};
use sbm_store::Server;

use super::detail::Slot;
use super::{App, Platform};
use crate::core::{Core, lock};
use crate::i18n::t;
use crate::term::keys::{self, Key, Mods};
use crate::term::view::{self, Sticky, ViewState};
use crate::term::Terminal;

enum Backend {
    Ssh(Shell),
    Monitor(MonitorTerminal),
}

impl Backend {
    fn write(&self, b: Vec<u8>) {
        match self {
            Backend::Ssh(s) => s.write(b),
            Backend::Monitor(m) => m.write(b),
        }
    }
    fn resize(&self, cols: usize, rows: usize) {
        match self {
            Backend::Ssh(s) => s.resize(cols as u32, rows as u32),
            Backend::Monitor(m) => m.resize(cols as u16, rows as u16),
        }
    }
}

enum Conn {
    Pending(Slot<Backend>),
    Live(Backend),
    Ended(String),
}

pub struct Session {
    pub title: String,
    pub server_id: String,
    term: Terminal,
    conn: Conn,
    view: ViewState,
    /// Typed into a Monitor shell once it is ready (it takes no command).
    queued: Option<String>,
}

#[derive(Default)]
pub struct Sessions {
    list: Vec<Session>,
    current: usize,
    sticky: Sticky,
}

impl Sessions {
    pub fn any_open(&self) -> bool {
        self.list.iter().any(|s| !matches!(s.conn, Conn::Ended(_)))
    }

    /// A session showing [bytes] with nothing behind it (previews, tests).
    #[doc(hidden)]
    pub fn preview(&mut self, title: &str, cols: usize, rows: usize, bytes: &[u8]) {
        let mut term = Terminal::new(cols, rows);
        term.feed(bytes);
        self.add(Session { title: title.to_owned(), server_id: String::new(), term, conn: Conn::Pending(Slot::default()), view: ViewState::default(), queued: None });
    }

    /// Types [text] into the newest session once its shell answers.
    pub fn queue_last(&mut self, text: String) {
        if let Some(s) = self.list.last_mut() {
            s.queued = Some(text);
        }
    }

    fn add(&mut self, s: Session) {
        self.list.push(s);
        self.current = self.list.len() - 1;
    }

    /// A shell on [server], or [command] in a PTY.
    pub fn open_ssh(&mut self, core: &mut Core, server: &Server, command: Option<String>, title: &str) {
        let slot: Slot<Backend> = Slot::default();
        let fut = core.session(server);
        let ctx = core.ctx.clone();
        let out = slot.clone();
        let (cols, rows) = (80, 24);
        core.rt.spawn(async move {
            let r = async {
                let sess = fut.await?;
                let shell = sess.shell("xterm-256color", cols, rows, command.as_deref()).await.map_err(|e| e.to_string())?;
                let c = ctx.clone();
                shell.set_waker(move || c.request_repaint());
                Ok(Backend::Ssh(shell))
            }
            .await;
            *lock(&out) = Some(r);
            ctx.request_repaint();
        });
        self.add(Session { title: title.to_owned(), server_id: server.id.clone(), term: Terminal::new(cols as usize, rows as usize), conn: Conn::Pending(slot), view: ViewState::default(), queued: None });
    }

    /// A shell on the Monitor's machine, as the agent's user.
    pub fn open_monitor(&mut self, core: &mut Core, server: &Server, command: Option<String>) {
        let slot: Slot<Backend> = Slot::default();
        let client = core.monitor_client(server);
        let live = core.live(&server.id);
        let existing = lock(&live).monitor.clone();
        let ctx = core.ctx.clone();
        let out = slot.clone();
        core.rt.spawn(async move {
            let r = async {
                let c = match existing {
                    Some(c) => c,
                    None => {
                        let c = Arc::new(client?);
                        c.login().await.map_err(|e| e.to_string())?;
                        c
                    }
                };
                let term = c.terminal(80, 24).await.map_err(|e| e.to_string())?;
                let cx = ctx.clone();
                term.set_waker(move || cx.request_repaint());
                Ok(Backend::Monitor(term))
            }
            .await;
            *lock(&out) = Some(r);
            ctx.request_repaint();
        });
        self.add(Session { title: format!("{} (Monitor)", server.name), server_id: server.id.clone(), term: Terminal::new(80, 24), conn: Conn::Pending(slot), view: ViewState::default(), queued: command });
    }

    /// Moves bytes between each session and its terminal.
    fn pump(&mut self) {
        for s in &mut self.list {
            if let Conn::Pending(slot) = &s.conn {
                let taken = lock(slot).take();
                match taken {
                    Some(Ok(b)) => {
                        let size = s.term.size();
                        b.resize(size.cols, size.rows);
                        s.conn = Conn::Live(b);
                    }
                    Some(Err(e)) => s.conn = Conn::Ended(e),
                    None => {}
                }
            }
            let mut ended = None;
            if let Conn::Live(b) = &mut s.conn {
                loop {
                    let ev = match b {
                        Backend::Ssh(sh) => sh.try_recv().map(|e| match e {
                            ShellEvent::Data(d) => Ok(d),
                            ShellEvent::Exit(code, sig) => Err(format!("{} {}", t("exit", "Exited"), code.map(|c| c.to_string()).or(sig).unwrap_or_default())),
                            ShellEvent::Closed => Err(t("disconnected", "Disconnected")),
                        }),
                        Backend::Monitor(m) => m.try_recv().map(|e| match e {
                            TerminalEvent::Data(d) => Ok(d),
                            TerminalEvent::Ready { .. } => Ok(vec![]),
                            TerminalEvent::Exit(code) => Err(format!("{} {}", t("exit", "Exited"), code.map(|c| c.to_string()).unwrap_or_default())),
                            TerminalEvent::Error(e) => Err(e),
                            TerminalEvent::Closed => Err(t("disconnected", "Disconnected")),
                        }),
                    };
                    match ev {
                        Some(Ok(d)) => {
                            s.term.feed(&d);
                            if let Some(q) = s.queued.take() {
                                b.write(format!("{q}\n").into_bytes());
                            }
                        }
                        Some(Err(e)) => {
                            ended = Some(e);
                            break;
                        }
                        None => break,
                    }
                }
                let reply = s.term.take_output();
                if !reply.is_empty() {
                    b.write(reply);
                }
            }
            if let Some(e) = ended {
                s.term.feed(format!("\r\n\x1b[2m[{e}]\x1b[0m\r\n").as_bytes());
                s.conn = Conn::Ended(e);
            }
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui, platform: &dyn Platform) {
    let p = app.palette;
    app.terms.pump();
    if app.terms.list.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("noSession", "No terminal sessions. Open one from a server.")).color(p.muted));
            if ui.button(t("server", "Servers")).clicked() {
                app.stack.truncate(1);
            }
        });
        return;
    }
    app.terms.current = app.terms.current.min(app.terms.list.len() - 1);
    // Tabs.
    let mut close = None;
    egui::Frame::new().fill(p.surface).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("←").clicked() {
                app.back();
            }
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, s) in app.terms.list.iter().enumerate() {
                        let title = s.term.title().unwrap_or_else(|| s.title.clone());
                        let dim = matches!(s.conn, Conn::Ended(_));
                        let text = RichText::new(title.chars().take(24).collect::<String>()).color(if dim { p.muted } else { p.foreground });
                        let tab = ui.selectable_label(i == app.terms.current, text);
                        let tab = if let Conn::Ended(why) = &s.conn { tab.on_hover_text(why) } else { tab };
                        if tab.clicked() {
                            app.terms.current = i;
                        }
                        if ui.small_button("\u{f00d}").clicked() {
                            close = Some(i);
                        }
                    }
                });
            });
        });
    });
    if let Some(i) = close {
        if let Conn::Live(Backend::Ssh(s)) = &app.terms.list[i].conn {
            s.close();
        }
        app.terms.list.remove(i);
        if app.terms.list.is_empty() {
            return;
        }
        app.terms.current = app.terms.current.min(app.terms.list.len() - 1);
    }
    let font = app.font_size;
    let cur = app.terms.current;
    // The key bar under the terminal.
    let bar_h = 46.0;
    let avail = ui.available_size();
    let term_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(avail.x, (avail.y - bar_h).max(40.0)));
    let id = egui::Id::unique(("term", cur));
    let out = {
        let Sessions { list, sticky, .. } = &mut app.terms;
        let s = &mut list[cur];
        if matches!(s.conn, Conn::Pending(_)) && !s.server_id.is_empty() {
            ui.painter().text(term_rect.center(), egui::Align2::CENTER_CENTER, t("connecting", "Connecting"), egui::FontId::proportional(16.0), p.muted);
        }
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(term_rect).id_salt(id));
        view::show(&mut child, id, &mut s.term, &p, font, sticky, &mut s.view)
    };
    ui.advance_cursor_after_rect(term_rect);
    let mut bytes = out.bytes;
    if let Some(text) = out.copy {
        platform.set_clipboard(&text);
        app.toast(t("copied", "Copied"));
    }
    // Key bar.
    let mode = app.terms.list[cur].term.mode();
    egui::Frame::new().fill(p.surface).inner_margin(egui::Margin::symmetric(4, 4)).show(ui, |ui| {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                let sticky = &mut app.terms.sticky;
                let mut key = |ui: &mut Ui, label: &str, k: Key| {
                    if ui.add(egui::Button::new(RichText::new(label).monospace()).min_size(egui::vec2(40.0, 36.0))).clicked() {
                        let m = Mods { ctrl: sticky.ctrl, alt: sticky.alt, shift: false };
                        bytes.extend(keys::encode(k, m, mode).unwrap_or_default());
                        *sticky = Sticky::default();
                    }
                };
                key(ui, "Esc", Key::Escape);
                key(ui, "Tab", Key::Tab);
                let ctrl = if app.terms.sticky.ctrl { RichText::new("Ctrl").color(p.yellow).strong() } else { RichText::new("Ctrl") };
                if ui.add(egui::Button::new(ctrl).min_size(egui::vec2(44.0, 36.0))).clicked() {
                    app.terms.sticky.ctrl = !app.terms.sticky.ctrl;
                }
                let alt = if app.terms.sticky.alt { RichText::new("Alt").color(p.yellow).strong() } else { RichText::new("Alt") };
                if ui.add(egui::Button::new(alt).min_size(egui::vec2(44.0, 36.0))).clicked() {
                    app.terms.sticky.alt = !app.terms.sticky.alt;
                }
                let sticky = &mut app.terms.sticky;
                let mut key = |ui: &mut Ui, label: &str, k: Key| {
                    if ui.add(egui::Button::new(RichText::new(label).monospace()).min_size(egui::vec2(40.0, 36.0))).clicked() {
                        let m = Mods { ctrl: sticky.ctrl, alt: sticky.alt, shift: false };
                        bytes.extend(keys::encode(k, m, mode).unwrap_or_default());
                        *sticky = Sticky::default();
                    }
                };
                key(ui, "←", Key::Left);
                key(ui, "↓", Key::Down);
                key(ui, "↑", Key::Up);
                key(ui, "→", Key::Right);
                key(ui, "Home", Key::Home);
                key(ui, "End", Key::End);
                key(ui, "PgUp", Key::PageUp);
                key(ui, "PgDn", Key::PageDown);
                for c in ["|", "/", "-", "~", ":"] {
                    if ui.add(egui::Button::new(RichText::new(c).monospace()).min_size(egui::vec2(36.0, 36.0))).clicked() {
                        bytes.extend(c.as_bytes());
                    }
                }
                if ui.add(egui::Button::new(t("paste", "Paste")).min_size(egui::vec2(52.0, 36.0))).clicked()
                    && let Some(text) = platform.clipboard()
                {
                    bytes.extend(keys::encode_paste(&text, mode));
                }
                if ui.add(egui::Button::new(t("copy", "Copy")).min_size(egui::vec2(52.0, 36.0))).clicked() {
                    let s = &app.terms.list[cur].term;
                    let text = s.selection_text().unwrap_or_else(|| s.screen_text());
                    platform.set_clipboard(&text);
                }
                if ui.add(egui::Button::new("A−").min_size(egui::vec2(36.0, 36.0))).clicked() {
                    app.font_size = (app.font_size - 1.0).max(8.0);
                    let _ = app.core.store.kv(sbm_store::KvStore::Setting).set("termFontSize", &(app.font_size as f64));
                }
                if ui.add(egui::Button::new("A+").min_size(egui::vec2(36.0, 36.0))).clicked() {
                    app.font_size = (app.font_size + 1.0).min(32.0);
                    let _ = app.core.store.kv(sbm_store::KvStore::Setting).set("termFontSize", &(app.font_size as f64));
                }
            });
        });
    });
    if out.tapped {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let s = &mut app.terms.list[cur];
    if let Some(text) = s.term.take_clipboard() {
        platform.set_clipboard(&text);
    }
    if let Conn::Live(b) = &s.conn {
        if let Some((c, r)) = out.resized {
            b.resize(c, r);
        }
        if !bytes.is_empty() {
            b.write(bytes);
        }
    } else if let Conn::Ended(_) = s.conn
        && !bytes.is_empty()
    {
        // Enter on an ended session reconnects it.
        let server = app.core.store.server(&s.server_id).ok().flatten();
        if bytes.contains(&b'\r')
            && let Some(server) = server
        {
            let title = s.title.clone();
            app.terms.list.remove(cur);
            app.terms.open_ssh(&mut app.core, &server, None, &title);
        }
    }
}
