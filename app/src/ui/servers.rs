//! The server list (home) and the server editor.

use egui::{RichText, Ui};
use sbm_store::{MonitorCfg, Server, SshCfg, Transport};

use super::widgets::{self, bar, card, field, level};
use super::{App, Page};
use crate::core::{ConnState, lock};
use crate::i18n::t;

#[derive(Default)]
pub struct State {
    form: Option<Form>,
    filter: String,
    confirm_delete: bool,
}

/// The editor's fields as text, so a half-typed port is not an error yet.
#[derive(Clone, Default)]
struct Form {
    id: Option<String>,
    name: String,
    host: String,
    port: String,
    user: String,
    password: String,
    key_id: Option<String>,
    jumps: Vec<String>,
    alter_url: String,
    proxy_command: String,
    legacy: bool,
    monitor_addr: String,
    monitor_pwd: String,
    monitor_cert: String,
    prefer_monitor: bool,
    ssh_enabled: bool,
    monitor_enabled: bool,
    tags: String,
    script_dir: String,
    auto_connect: bool,
    base: Option<Server>,
}

impl Form {
    fn from(s: Option<&Server>) -> Form {
        let Some(s) = s else {
            return Form { port: "22".into(), user: "root".into(), ssh_enabled: true, monitor_enabled: true, auto_connect: true, ..Default::default() };
        };
        let ssh = s.ssh.clone().unwrap_or_default();
        let mon = s.monitor.clone().unwrap_or_default();
        Form {
            id: Some(s.id.clone()),
            name: s.name.clone(),
            host: ssh.ip.clone(),
            port: if s.ssh.is_some() { ssh.port.to_string() } else { "22".into() },
            user: if s.ssh.is_some() { ssh.user.clone() } else { "root".into() },
            password: ssh.pwd.clone().unwrap_or_default(),
            key_id: ssh.key_id.clone(),
            jumps: ssh.jump_ids.clone(),
            alter_url: ssh.alter_url.clone().unwrap_or_default(),
            proxy_command: ssh.proxy_command.clone().unwrap_or_default(),
            legacy: ssh.allow_legacy_algorithms,
            monitor_addr: mon.addr.clone(),
            monitor_pwd: mon.pwd.clone().unwrap_or_default(),
            monitor_cert: mon.cert_sha256.clone().unwrap_or_default(),
            prefer_monitor: s.preferred_transport == Some(Transport::MonitorHttp),
            ssh_enabled: s.ssh_enabled,
            monitor_enabled: s.monitor_enabled,
            tags: s.tags.join(", "),
            script_dir: s.script_dir.clone().unwrap_or_default(),
            auto_connect: s.auto_connect,
            base: Some(s.clone()),
        }
    }

    fn build(&self) -> Result<Server, String> {
        let opt = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        let mut s = match &self.base {
            Some(b) => b.clone(),
            None => {
                let mut s = Server::new_ssh("", "x", 22, "root");
                s.ssh = None;
                s
            }
        };
        s.name = self.name.trim().to_owned();
        if s.name.is_empty() {
            s.name = self.host.trim().to_owned();
        }
        s.ssh = if self.host.trim().is_empty() {
            None
        } else {
            let port: u16 = self.port.trim().parse().map_err(|_| t("invalidPort", "The port is 1 to 65535"))?;
            let old = s.ssh.clone().unwrap_or_default();
            Some(SshCfg {
                ip: self.host.trim().to_owned(),
                port,
                user: opt(&self.user).unwrap_or_else(|| "root".into()),
                pwd: opt(&self.password),
                key_id: self.key_id.clone(),
                jump_ids: self.jumps.clone(),
                alter_url: opt(&self.alter_url),
                proxy_command: opt(&self.proxy_command),
                allow_legacy_algorithms: self.legacy,
                ..old
            })
        };
        s.monitor = opt(&self.monitor_addr).map(|addr| {
            let addr = if addr.contains("://") { addr } else { format!("https://{addr}") };
            let old = s.monitor.clone().unwrap_or_default();
            MonitorCfg { addr, pwd: opt(&self.monitor_pwd), cert_sha256: opt(&self.monitor_cert).map(|c| c.to_lowercase().replace(':', "")), ..old }
        });
        s.preferred_transport = (s.ssh.is_some() && s.monitor.is_some()).then_some(if self.prefer_monitor { Transport::MonitorHttp } else { Transport::Ssh });
        s.ssh_enabled = self.ssh_enabled;
        s.monitor_enabled = self.monitor_enabled;
        s.tags = self.tags.split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect();
        s.script_dir = opt(&self.script_dir);
        s.auto_connect = self.auto_connect;
        Ok(s)
    }
}

pub fn list(app: &mut App, ui: &mut Ui) {
    let p = app.palette;
    let servers = app.core.store.servers().unwrap_or_default();
    // Servers set to connect on their own do so when the list first shows.
    for s in servers.iter().filter(|s| s.auto_connect) {
        if lock(&app.core.live(&s.id)).state == ConnState::Idle {
            app.core.start(s);
        }
    }
    ui.horizontal(|ui| {
        ui.heading(t("server", "Servers"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("\u{f067} {}", t("add", "Add"))).color(p.yellow)).clicked() {
                app.servers.form = None;
                app.push(Page::EditServer(None));
            }
        });
    });
    if servers.len() > 6 {
        ui.add(egui::TextEdit::singleline(&mut app.servers.filter).hint_text(t("search", "Search")).desired_width(f32::INFINITY));
    }
    if servers.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("serverTabEmpty", "No servers yet")).color(p.muted));
        });
        return;
    }
    let filter = app.servers.filter.to_lowercase();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for s in servers.iter().filter(|s| filter.is_empty() || s.name.to_lowercase().contains(&filter) || s.tags.iter().any(|t| t.to_lowercase().contains(&filter))) {
            let live = app.core.live(&s.id);
            let (state, status) = {
                let l = lock(&live);
                (l.state.clone(), l.status.clone())
            };
            let resp = egui::Frame::new().fill(p.surface).corner_radius(14).inner_margin(egui::Margin::same(14)).outer_margin(egui::Margin::symmetric(0, 5)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (dot, label) = match &state {
                        ConnState::Idle => (p.muted, t("disconnected", "Disconnected")),
                        ConnState::Connecting => (p.yellow, t("connecting", "Connecting")),
                        ConnState::Connected => (p.green, status.as_ref().and_then(|s| s.uptime.clone()).unwrap_or_default()),
                        ConnState::Failed(e) => (p.red, e.lines().next().unwrap_or_default().chars().take(60).collect()),
                    };
                    ui.label(RichText::new("\u{f111}").color(dot));
                    ui.label(RichText::new(&s.name).strong().size(17.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(label).color(p.muted).size(12.0));
                    });
                });
                if let Some(st) = &status {
                    ui.add_space(4.0);
                    let mem = if st.mem_total > 0 { st.mem_used as f64 / st.mem_total as f64 } else { 0.0 };
                    let disk = if st.disk_total > 0 { st.disk_used as f64 / st.disk_total as f64 } else { 0.0 };
                    ui.columns(3, |c| {
                        c[0].label(RichText::new(format!("CPU {:.0}%", st.cpu_percent)).size(12.0));
                        bar(&mut c[0], &p, st.cpu_percent / 100.0, level(&p, st.cpu_percent / 100.0));
                        c[1].label(RichText::new(format!("{} {:.0}%", t("mem", "Mem"), mem * 100.0)).size(12.0));
                        bar(&mut c[1], &p, mem, level(&p, mem));
                        c[2].label(RichText::new(format!("{} {:.0}%", t("disk", "Disk"), disk * 100.0)).size(12.0));
                        bar(&mut c[2], &p, disk, level(&p, disk));
                    });
                    ui.label(RichText::new(format!("↓ {}   ↑ {}", widgets::rate(st.net_rx_rate), widgets::rate(st.net_tx_rate))).color(p.muted).size(12.0));
                } else if !s.tags.is_empty() {
                    ui.label(RichText::new(s.tags.join(" · ")).color(p.muted).size(12.0));
                }
            });
            let r = resp.response.interact(egui::Sense::click());
            if r.clicked() {
                app.core.start(s);
                app.push(Page::Detail(s.id.clone()));
            }
        }
    });
}

pub fn edit(app: &mut App, ui: &mut Ui, id: Option<String>) {
    let p = app.palette;
    let store = app.store();
    if app.servers.form.as_ref().is_none_or(|f| f.id != id) {
        let server = id.as_deref().and_then(|i| store.server(i).ok().flatten());
        app.servers.form = Some(Form::from(server.as_ref()));
        app.servers.confirm_delete = false;
    }
    let keys = store.private_keys().unwrap_or_default();
    let others: Vec<Server> = store.servers().unwrap_or_default().into_iter().filter(|s| Some(&s.id) != id.as_ref() && s.ssh.is_some()).collect();
    let mut save = false;
    let mut delete = false;
    ui.horizontal(|ui| {
        if ui.button("←").clicked() {
            app.servers.form = None;
            app.back();
        }
        ui.heading(if id.is_some() { t("edit", "Edit") } else { t("add", "Add") });
    });
    let Some(f) = app.servers.form.as_mut() else { return };
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        card(ui, &p, "", |ui| {
            field(ui, &t("name", "Name"), &mut f.name, false);
            field(ui, &t("tag", "Tags"), &mut f.tags, false);
            ui.checkbox(&mut f.auto_connect, t("autoConnect", "Connect automatically"));
        });
        card(ui, &p, "SSH", |ui| {
            ui.checkbox(&mut f.ssh_enabled, t("enabled", "Enabled"));
            field(ui, &t("host", "Host"), &mut f.host, false);
            ui.columns(2, |c| {
                field(&mut c[0], &t("port", "Port"), &mut f.port, false);
                field(&mut c[1], &t("user", "User"), &mut f.user, false);
            });
            field(ui, &t("pwd", "Password"), &mut f.password, true);
            ui.label(t("privateKey", "Private Key"));
            egui::ComboBox::from_id_salt("key").width(ui.available_width()).selected_text(f.key_id.as_ref().and_then(|k| keys.iter().find(|x| &x.id == k)).map_or_else(|| t("none", "None"), |k| k.name.clone())).show_ui(ui, |ui| {
                ui.selectable_value(&mut f.key_id, None, t("none", "None"));
                for k in &keys {
                    ui.selectable_value(&mut f.key_id, Some(k.id.clone()), &k.name);
                }
            });
            ui.add_space(4.0);
            ui.label(t("jumpServer", "Jump servers"));
            for o in &others {
                let mut on = f.jumps.contains(&o.id);
                if ui.checkbox(&mut on, &o.name).changed() {
                    if on {
                        f.jumps.push(o.id.clone());
                    } else {
                        f.jumps.retain(|j| j != &o.id);
                    }
                }
            }
            ui.collapsing(t("more", "More"), |ui| {
                field(ui, &t("alterUrl", "Alternative address (user@host:port)"), &mut f.alter_url, false);
                field(ui, "ProxyCommand", &mut f.proxy_command, false);
                field(ui, &t("scriptDir", "Script directory"), &mut f.script_dir, false);
                ui.checkbox(&mut f.legacy, t("sshAllowLegacyAlgorithms", "Allow legacy algorithms (SHA-1)"));
            });
        });
        card(ui, &p, "Monitor", |ui| {
            ui.checkbox(&mut f.monitor_enabled, t("enabled", "Enabled"));
            field(ui, &t("addr", "Address"), &mut f.monitor_addr, false);
            field(ui, &t("pwd", "Password"), &mut f.monitor_pwd, true);
            field(ui, &t("certFingerprint", "Pinned certificate (SHA-256)"), &mut f.monitor_cert, false);
            if !f.host.trim().is_empty() && !f.monitor_addr.trim().is_empty() {
                ui.checkbox(&mut f.prefer_monitor, t("preferMonitor", "Prefer the Monitor over SSH"));
            }
        });
        ui.add_space(8.0);
        if widgets::wide_button(ui, RichText::new(t("save", "Save")).strong()).clicked() {
            save = true;
        }
        if id.is_some() {
            ui.add_space(8.0);
            if app.servers.confirm_delete {
                if widgets::wide_button(ui, RichText::new(t("delete", "Delete")).color(p.red).strong()).clicked() {
                    delete = true;
                }
            } else if widgets::wide_button(ui, RichText::new(t("delete", "Delete")).color(p.red)).clicked() {
                app.servers.confirm_delete = true;
            }
        }
        ui.add_space(24.0);
    });
    if save {
        let built = app.servers.form.as_ref().map(Form::build);
        match built {
            Some(Ok(s)) => match store.put_server(&s) {
                Ok(()) => {
                    app.core.stop(&s.id);
                    app.servers.form = None;
                    app.back();
                    app.toast(t("saved", "Saved"));
                }
                Err(e) => app.toast(e.to_string()),
            },
            Some(Err(e)) => app.toast(e),
            None => {}
        }
    }
    if delete && let Some(id) = &id {
        app.core.stop(id);
        match store.delete_server(id) {
            Ok(()) => {
                app.servers.form = None;
                app.stack.truncate(1);
            }
            Err(e) => app.toast(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_builds_a_server_and_keeps_what_it_does_not_edit() {
        let mut base = Server::new_ssh("a", "a.example", 22, "root");
        base.wol_mac = Some("aa:bb".into());
        let mut f = Form::from(Some(&base));
        f.port = "2222".into();
        f.monitor_addr = "a.example:3770".into();
        f.monitor_cert = "AB:CD".into();
        let s = f.build().unwrap();
        assert_eq!(s.ssh.unwrap().port, 2222);
        assert_eq!(s.monitor.as_ref().unwrap().addr, "https://a.example:3770");
        assert_eq!(s.monitor.unwrap().cert_sha256.as_deref(), Some("abcd"));
        assert_eq!(s.wol_mac.as_deref(), Some("aa:bb"), "untouched columns survive");
        assert_eq!(s.preferred_transport, Some(Transport::Ssh));
        f.port = "x".into();
        assert!(f.build().is_err());
    }
}
