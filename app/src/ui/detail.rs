//! One server: its status, and the tools that act on it.
//!
//! Commands come from `sbm_parser` and run over whichever transport the
//! server is connected by ([`Exec`]); anything that may ask for a sudo
//! password runs in a terminal, where the person can type it.

use std::sync::{Arc, Mutex};

use egui::{RichText, Ui};
use sbm_parser::SystemType;
use sbm_parser::container::{self, ContainerAction, ContainerCmd, ContainerType};
use sbm_parser::proc::{ProcSignal, ProcSortMode, PsResult};
use sbm_parser::script::{self, ScriptOptions, ShellFunc};
use sbm_store::Server;

use super::widgets::{self, bar, card, level, sparkline};
use super::{App, Page};
use crate::core::{ConnState, Live, lock};
use crate::i18n::t;

/// A result filled in by a background task.
pub type Slot<T> = Arc<Mutex<Option<Result<T, String>>>>;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Status,
    Processes,
    Containers,
    Zellij,
    Services,
}

#[derive(Default)]
pub struct State {
    tab: Tab,
    server: String,
    procs: Slot<PsResult>,
    containers: Slot<(ContainerType, Vec<container::Container>)>,
    zellij: Slot<(String, Vec<String>)>,
    services: Slot<Vec<sbm_parser::service::ServiceUnit>>,
    service_manager: Option<sbm_parser::service::ServiceManagerType>,
    new_session: String,
    confirm: Option<(String, String)>,
}

/// Runs a command over the server's connection.
#[derive(Clone)]
pub enum Exec {
    Ssh(Arc<sbm_ssh::Session>),
    Monitor(Arc<sbm_monitor_client::Client>),
}

/// stdout, stderr, exit code.
pub type Output = (String, String, i32);

impl Exec {
    pub fn of(live: &Live) -> Option<Exec> {
        if let Some(s) = live.ssh.clone().filter(|s| !s.is_closed()) {
            return Some(Exec::Ssh(s));
        }
        live.monitor.clone().map(Exec::Monitor)
    }

    pub async fn run(&self, cmd: &str, stdin: Option<&str>) -> Result<Output, String> {
        match self {
            Exec::Ssh(s) => {
                let o = s.exec(cmd, stdin.map(str::as_bytes), std::time::Duration::from_secs(60)).await.map_err(|e| e.to_string())?;
                Ok((o.stdout_str(), o.stderr_str(), o.exit_code.unwrap_or(-1)))
            }
            Exec::Monitor(c) => {
                let o = c.exec(cmd, stdin).await.map_err(|e| e.to_string())?;
                if o.timed_out {
                    return Err(t("timeout", "Timed out"));
                }
                Ok((o.stdout, o.stderr, o.exit_code.unwrap_or(-1)))
            }
        }
    }

    /// Installs the status script and runs one of its functions.
    pub async fn script(&self, system: SystemType, custom_dir: Option<&str>, func: ShellFunc) -> Result<String, String> {
        if let Exec::Ssh(s) = self {
            let mut r = sbm_ssh::StatusRunner::new(system, custom_dir.map(str::to_owned), vec![]);
            return r.run(s, func).await.map_err(|e| e.to_string());
        }
        let paths = sbm_ssh::ScriptPaths::new(system, custom_dir, false);
        let content = script::install_payload(system, &script::build_script(system, &ScriptOptions { disabled: vec![], build_number: sbm_ssh::SCRIPT_VERSION.to_string() }));
        let (_, err, code) = self.run(&script::install_command(system, &paths.dir, &paths.path), Some(&content)).await?;
        if code != 0 {
            return Err(err);
        }
        Ok(self.run(&script::exec_command(system, &paths.path, func), None).await?.0)
    }
}

fn spawn<T: Send + 'static>(app: &App, slot: &Slot<T>, fut: impl std::future::Future<Output = Result<T, String>> + Send + 'static) {
    *lock(slot) = None;
    let slot = slot.clone();
    let ctx = app.core.ctx.clone();
    app.core.rt.spawn(async move {
        let r = fut.await;
        *lock(&slot) = Some(r);
        ctx.request_repaint();
    });
}

pub fn show(app: &mut App, ui: &mut Ui, id: &str) {
    let p = app.palette;
    let Some(server) = app.core.store.server(id).ok().flatten() else {
        app.back();
        return;
    };
    if app.detail.server != id {
        app.detail = State { server: id.to_owned(), ..Default::default() };
    }
    let live = app.core.live(id);
    let (state, exec, system) = {
        let l = lock(&live);
        (l.state.clone(), Exec::of(&l), l.system.unwrap_or(SystemType::Linux))
    };
    ui.horizontal(|ui| {
        if ui.button("←").clicked() {
            app.back();
        }
        ui.add(egui::Label::new(RichText::new(&server.name).heading()).truncate());
    });
    ui.horizontal_wrapped(|ui| {
        match state {
            ConnState::Connected | ConnState::Connecting => {
                if ui.button(t("disconnect", "Disconnect")).clicked() {
                    app.core.stop(id);
                }
            }
            _ => {
                if ui.button(t("connect", "Connect")).clicked() {
                    app.core.stop(id);
                    app.core.start(&server);
                }
            }
        }
        if server.uses_ssh() && ui.button(format!("\u{f120} {}", t("terminal", "Terminal"))).clicked() {
            app.terms.open_ssh(&mut app.core, &server, None, &server.name);
            app.push(Page::Terminal);
        }
        if server.uses_monitor() && !server.uses_ssh() && ui.button(format!("\u{f120} {}", t("terminal", "Terminal"))).clicked() {
            app.terms.open_monitor(&mut app.core, &server, None);
            app.push(Page::Terminal);
        }
        if ui.button(format!("\u{f07b} {}", t("files", "Files"))).clicked() {
            app.push(Page::Files(id.to_owned()));
        }
        if ui.button(format!("\u{f044} {}", t("edit", "Edit"))).clicked() {
            app.push(Page::EditServer(Some(id.to_owned())));
        }
    });
    egui::ScrollArea::horizontal().id_salt("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (tab, label) in [(Tab::Status, t("status", "Status")), (Tab::Processes, t("process", "Processes")), (Tab::Containers, t("container", "Containers")), (Tab::Zellij, "Zellij".to_owned()), (Tab::Services, t("service", "Services"))] {
                if ui.selectable_label(app.detail.tab == tab, label).clicked() {
                    app.detail.tab = tab;
                }
            }
        });
    });
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match app.detail.tab {
        Tab::Status => status(app, ui, &live, &server, exec.clone(), system),
        Tab::Processes => processes(app, ui, exec.clone(), system, &server),
        Tab::Containers => containers(app, ui, exec.clone(), &server),
        Tab::Zellij => zellij(app, ui, exec.clone(), &server),
        Tab::Services => services(app, ui, exec.clone(), &server),
    });
    let _ = p;
}

fn need_connection(app: &App, ui: &mut Ui) {
    ui.label(RichText::new(t("waitConnection", "Waiting for the connection…")).color(app.palette.muted));
}

fn status(app: &mut App, ui: &mut Ui, live: &Arc<Mutex<Live>>, server: &Server, exec: Option<Exec>, system: SystemType) {
    let p = app.palette;
    let l = lock(live);
    if let ConnState::Failed(e) = &l.state {
        card(ui, &p, &t("error", "Error"), |ui| {
            ui.colored_label(p.red, e);
        });
    }
    let Some(st) = l.status.clone() else {
        if l.state == ConnState::Connecting {
            ui.spinner();
        }
        return;
    };
    let (cpu, mem, rx, tx): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = (l.cpu_history.iter().copied().collect(), l.mem_history.iter().copied().collect(), l.rx_history.iter().copied().collect(), l.tx_history.iter().copied().collect());
    drop(l);
    card(ui, &p, "", |ui| {
        if let Some(sys) = &st.sys {
            ui.label(RichText::new(sys).strong());
        }
        if let Some(u) = &st.uptime {
            ui.label(RichText::new(format!("{} {u}", t("uptime", "Uptime"))).color(p.muted));
        }
        if !st.ips.is_empty() {
            ui.label(RichText::new(st.ips.join("  ")).color(p.muted).size(12.0));
        }
    });
    card(ui, &p, &format!("CPU  {:.1}%", st.cpu_percent), |ui| {
        if let Some(b) = &st.cpu_brand {
            ui.label(RichText::new(b).color(p.muted).size(12.0));
        }
        sparkline(ui, &p, &cpu, Some(100.0), p.blue, 64.0);
        if !st.cores.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (i, c) in st.cores.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(44.0);
                        ui.label(RichText::new(format!("{i}: {c:.0}%")).size(11.0));
                        bar(ui, &p, c / 100.0, level(&p, c / 100.0));
                    });
                }
            });
        }
        if let Some(temp) = st.temperature {
            ui.label(format!("{temp:.1} °C"));
        }
    });
    let mem_frac = if st.mem_total > 0 { st.mem_used as f64 / st.mem_total as f64 } else { 0.0 };
    card(ui, &p, &format!("{}  {} / {}", t("mem", "Memory"), widgets::bytes(st.mem_used), widgets::bytes(st.mem_total)), |ui| {
        sparkline(ui, &p, &mem, Some(100.0), p.purple, 48.0);
        bar(ui, &p, mem_frac, level(&p, mem_frac));
        if st.swap_total > 0 {
            let f = st.swap_used as f64 / st.swap_total as f64;
            ui.label(RichText::new(format!("Swap  {} / {}", widgets::bytes(st.swap_used), widgets::bytes(st.swap_total))).size(12.0));
            bar(ui, &p, f, level(&p, f));
        }
    });
    card(ui, &p, &format!("{}  ↓ {}  ↑ {}", t("net", "Network"), widgets::rate(st.net_rx_rate), widgets::rate(st.net_tx_rate)), |ui| {
        sparkline(ui, &p, &rx, None, p.green, 40.0);
        sparkline(ui, &p, &tx, None, p.yellow, 40.0);
    });
    card(ui, &p, &format!("{}  {} / {}", t("disk", "Disk"), widgets::bytes(st.disk_used), widgets::bytes(st.disk_total)), |ui| {
        for d in &st.disks {
            if d.size == 0 {
                continue;
            }
            let f = d.used as f64 / d.size as f64;
            ui.label(RichText::new(format!("{}  {} / {}", d.mount, widgets::bytes(d.used), widgets::bytes(d.size))).size(12.0));
            bar(ui, &p, f, level(&p, f));
        }
    });
    if !st.gpus.is_empty() {
        card(ui, &p, "GPU", |ui| {
            for g in &st.gpus {
                ui.label(RichText::new(&g.name).strong());
                if let Some(u) = g.usage {
                    bar(ui, &p, u / 100.0, level(&p, u / 100.0));
                }
                let mut parts = vec![];
                if let Some(u) = g.usage {
                    parts.push(format!("{u:.0}%"));
                }
                if let Some(tmp) = g.temperature {
                    parts.push(format!("{tmp} °C"));
                }
                if let (Some(u), Some(tot)) = (g.mem_used, g.mem_total) {
                    parts.push(format!("{u} / {tot} MiB"));
                }
                ui.label(RichText::new(parts.join("  ")).color(p.muted).size(12.0));
            }
        });
    }
    // Power: through a terminal, which can take a sudo password.
    if exec.is_some() && server.uses_ssh() {
        card(ui, &p, &t("power", "Power"), |ui| {
            ui.horizontal(|ui| {
                for (func, label) in [(ShellFunc::Reboot, t("reboot", "Reboot")), (ShellFunc::Shutdown, t("shutdown", "Shut down")), (ShellFunc::Suspend, t("suspend", "Suspend"))] {
                    if ui.button(&label).clicked() {
                        app.detail.confirm = Some((label.clone(), func.flag().to_owned()));
                    }
                }
            });
            if let Some((label, flag)) = app.detail.confirm.clone() {
                ui.colored_label(p.red, format!("{label}?"));
                ui.horizontal(|ui| {
                    if ui.button(t("cancel", "Cancel")).clicked() {
                        app.detail.confirm = None;
                    }
                    if ui.button(RichText::new(t("confirm", "Confirm")).color(p.red)).clicked() {
                        let func = ShellFunc::ALL.into_iter().find(|f| f.flag() == flag).unwrap_or(ShellFunc::Reboot);
                        let paths = sbm_ssh::ScriptPaths::new(system, server.script_dir.as_deref(), false);
                        let cmd = script::exec_command(system, &paths.path, func);
                        app.terms.open_ssh(&mut app.core, server, Some(cmd), &label);
                        app.detail.confirm = None;
                        app.push(Page::Terminal);
                    }
                });
            }
        });
    }
}

fn processes(app: &mut App, ui: &mut Ui, exec: Option<Exec>, system: SystemType, server: &Server) {
    let key = egui::Id::unique(("procs-started", server.id.as_str()));
    let p = app.palette;
    let Some(exec) = exec else { return need_connection(app, ui) };
    let started = ui.memory(|m| m.data.get_temp::<bool>(key).is_some());
    if ui.button(t("refresh", "Refresh")).clicked() || !started {
        ui.memory_mut(|m| m.data.insert_temp(key, true));
        let e = exec.clone();
        let dir = server.script_dir.clone();
        spawn(app, &app.detail.procs, async move {
            let raw = e.script(system, dir.as_deref(), ShellFunc::Process).await?;
            Ok(PsResult::parse(&raw, ProcSortMode::Cpu, None, None, sbm_store::now_ms()))
        });
    }
    let result = lock(&app.detail.procs).clone();
    match result {
        None => {
            ui.spinner();
        }
        Some(Err(e)) => {
            ui.colored_label(p.red, e);
        }
        Some(Ok(ps)) => {
            if let Some(load) = &ps.load {
                ui.label(RichText::new(format!("load {:.2} {:.2} {:.2}", load.one, load.five, load.fifteen)).color(p.muted));
            }
            for pr in ps.procs.iter().take(200) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:>7}", pr.pid)).monospace().size(12.0));
                    ui.label(RichText::new(format!("{:>5.1}%", pr.cpu.unwrap_or(0.0))).monospace().size(12.0).color(p.blue));
                    ui.label(RichText::new(format!("{:>5.1}%", pr.mem.unwrap_or(0.0))).monospace().size(12.0).color(p.purple));
                    ui.label(RichText::new(pr.command.chars().take(48).collect::<String>()).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(cmd) = sbm_parser::proc::kill_command(pr.pid, pr.start_id.as_deref(), system, ProcSignal::Term)
                            && ui.small_button("\u{f00d}").clicked()
                        {
                            let e = exec.clone();
                            let ctx = app.core.ctx.clone();
                            let notices = app.core.notices.clone();
                            let slot = app.detail.procs.clone();
                            app.core.rt.spawn(async move {
                                let msg = match e.run("sh", Some(&cmd)).await {
                                    Ok((out, _, _)) => format!("{:?}", sbm_parser::proc::kill_outcome(&out)),
                                    Err(err) => err,
                                };
                                lock(&notices).push_back(msg);
                                *lock(&slot) = None;
                                ctx.request_repaint();
                            });
                            ui.memory_mut(|m| m.data.remove::<bool>(key));
                        }
                    });
                });
            }
        }
    }
}

fn containers(app: &mut App, ui: &mut Ui, exec: Option<Exec>, server: &Server) {
    let key = egui::Id::unique(("ctr-started", server.id.as_str()));
    let p = app.palette;
    let Some(exec) = exec else { return need_connection(app, ui) };
    let started = ui.memory(|m| m.data.get_temp::<bool>(key).is_some());
    if ui.button(t("refresh", "Refresh")).clicked() || !started {
        ui.memory_mut(|m| m.data.insert_temp(key, true));
        let e = exec.clone();
        spawn(app, &app.detail.containers, async move {
            let mut last = String::new();
            for ty in [ContainerType::Docker, ContainerType::Podman] {
                let cmd = container::build_runtime_command(&ContainerCmd::Ps.exec(ty), ty, None, false);
                let (out, err, code) = e.run(&cmd, None).await?;
                if container::is_not_installed(ty, &out, &err, code) {
                    last = err;
                    continue;
                }
                if code != 0 {
                    return Err(err.trim().to_owned());
                }
                let list = match ty {
                    ContainerType::Docker => container::parse_docker_ps(&out),
                    ContainerType::Podman => container::parse_podman_ps(&out),
                };
                return Ok((ty, list));
            }
            Err(if last.trim().is_empty() { t("containerNoRuntime", "Neither Docker nor Podman is installed") } else { last })
        });
    }
    let result = lock(&app.detail.containers).clone();
    match result {
        None => {
            ui.spinner();
        }
        Some(Err(e)) => {
            ui.colored_label(p.red, e);
        }
        Some(Ok((ty, list))) => {
            if list.is_empty() {
                ui.label(RichText::new(t("empty", "Empty")).color(p.muted));
            }
            for c in &list {
                card(ui, &p, "", |ui| {
                    ui.label(RichText::new(c.name.clone().unwrap_or_default()).strong());
                    ui.label(RichText::new(format!("{}  {}", c.image.clone().unwrap_or_default(), c.raw_status.clone().unwrap_or_default())).color(p.muted).size(12.0));
                    if let Some(ports) = &c.ports {
                        ui.label(RichText::new(ports).size(12.0));
                    }
                    let Some(id) = c.id.clone() else { return };
                    ui.horizontal_wrapped(|ui| {
                        let act = |ui: &mut Ui, label: String, a: ContainerAction| {
                            if ui.small_button(label).clicked()
                                && let Ok(cmd) = a.exec(ty)
                            {
                                let e = exec.clone();
                                let notices = app.core.notices.clone();
                                let ctx = app.core.ctx.clone();
                                app.core.rt.spawn(async move {
                                    let msg = match e.run(&container::build_runtime_command(&cmd, ty, None, false), None).await {
                                        Ok((_, err, 0)) => {
                                            let _ = err;
                                            t("success", "Done")
                                        }
                                        Ok((_, err, _)) => err,
                                        Err(err) => err,
                                    };
                                    lock(&notices).push_back(msg);
                                    ctx.request_repaint();
                                });
                                ui.memory_mut(|m| m.data.remove::<bool>(key));
                            }
                        };
                        act(ui, t("start", "Start"), ContainerAction::Start { id: id.clone() });
                        act(ui, t("stop", "Stop"), ContainerAction::Stop { id: id.clone() });
                        act(ui, t("restart", "Restart"), ContainerAction::Restart { id: id.clone() });
                        if server.uses_ssh() {
                            if ui.small_button(t("log", "Logs")).clicked() {
                                app.terms.open_ssh(&mut app.core, server, Some(container::logs_command(ty, &id)), &format!("{} logs", c.name.clone().unwrap_or_default()));
                                app.push(Page::Terminal);
                            }
                            if ui.small_button("Shell").clicked() {
                                app.terms.open_ssh(&mut app.core, server, Some(container::shell_command(ty, &id)), &c.name.clone().unwrap_or_default());
                                app.push(Page::Terminal);
                            }
                        }
                    });
                });
            }
        }
    }
}

fn zellij(app: &mut App, ui: &mut Ui, exec: Option<Exec>, server: &Server) {
    let key = egui::Id::unique(("zj-started", server.id.as_str()));
    let p = app.palette;
    let Some(exec) = exec else { return need_connection(app, ui) };
    if !server.uses_ssh() {
        ui.label(RichText::new(t("zellijNeedsSsh", "Zellij sessions are managed over SSH.")).color(p.muted));
        return;
    }
    let started = ui.memory(|m| m.data.get_temp::<bool>(key).is_some());
    if ui.button(t("refresh", "Refresh")).clicked() || !started {
        ui.memory_mut(|m| m.data.insert_temp(key, true));
        let e = exec.clone();
        spawn(app, &app.detail.zellij, async move {
            let (out, _, _) = e.run(sbm_parser::zellij::FIND_COMMAND, None).await?;
            let bin = sbm_parser::zellij::parse_find(&out).ok_or_else(|| t("zellijNotAvailable", "Zellij is not installed on this server"))?;
            let (out, err, code) = e.run(&sbm_parser::zellij::list_sessions_command(&bin), None).await?;
            let list = sbm_parser::zellij::read_sessions(&out, &err, code).ok_or_else(|| err.trim().to_owned())?;
            Ok((bin, list))
        });
    }
    let result = lock(&app.detail.zellij).clone();
    match result {
        None => {
            ui.spinner();
        }
        Some(Err(e)) => {
            ui.colored_label(p.red, e);
        }
        Some(Ok((bin, list))) => {
            for name in &list {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(t("delete", "Kill")).clicked()
                            && let Some(cmd) = sbm_parser::zellij::kill_session_command(&bin, name)
                        {
                            let e = exec.clone();
                            let slot = app.detail.zellij.clone();
                            let ctx = app.core.ctx.clone();
                            app.core.rt.spawn(async move {
                                let _ = e.run(&cmd, None).await;
                                *lock(&slot) = None;
                                ctx.request_repaint();
                            });
                            ui.memory_mut(|m| m.data.remove::<bool>(key));
                        }
                        if ui.small_button(t("attach", "Attach")).clicked()
                            && let Some(cmd) = sbm_parser::zellij::attach_command(&bin, name, false)
                        {
                            app.terms.open_ssh(&mut app.core, server, Some(cmd), &format!("zellij: {name}"));
                            app.push(Page::Terminal);
                        }
                    });
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.detail.new_session).hint_text(t("name", "Name")).desired_width(180.0));
                let name = app.detail.new_session.trim().to_owned();
                if ui.add_enabled(sbm_parser::zellij::valid_name(&name), egui::Button::new(t("create", "Create"))).clicked()
                    && let Some(cmd) = sbm_parser::zellij::attach_command(&bin, &name, true)
                {
                    app.terms.open_ssh(&mut app.core, server, Some(cmd), &format!("zellij: {name}"));
                    app.detail.new_session.clear();
                    ui.memory_mut(|m| m.data.remove::<bool>(key));
                    app.push(Page::Terminal);
                }
            });
        }
    }
}

fn services(app: &mut App, ui: &mut Ui, exec: Option<Exec>, server: &Server) {
    let key = egui::Id::unique(("svc-started", server.id.as_str()));
    use sbm_parser::service::{self as svc, ServiceManagerType};
    let p = app.palette;
    let Some(exec) = exec else { return need_connection(app, ui) };
    let started = ui.memory(|m| m.data.get_temp::<bool>(key).is_some());
    if ui.button(t("refresh", "Refresh")).clicked() || !started {
        ui.memory_mut(|m| m.data.insert_temp(key, true));
        let e = exec.clone();
        spawn(app, &app.detail.services, async move {
            let (out, _, _) = e.run(svc::DETECT_SCRIPT, None).await?;
            let probe = svc::parse_probe(&out);
            if probe.manager_type != Some(ServiceManagerType::Systemd) {
                return Err(t("serviceListFailed", "Only systemd is listed here; this server uses ") + &probe.detected_name);
            }
            let mut units = Vec::new();
            for scope in [svc::ServiceScope::System, svc::ServiceScope::User] {
                if let Some(cmd) = ServiceManagerType::Systemd.list_command(scope) {
                    let (out, _, code) = e.run(&cmd, None).await?;
                    if code == 0 {
                        units.extend(svc::parse_list_units(&out, scope));
                    }
                }
            }
            units.sort_by(svc::compare_services);
            Ok(units)
        });
        app.detail.service_manager = Some(ServiceManagerType::Systemd);
    }
    let result = lock(&app.detail.services).clone();
    match result {
        None => {
            ui.spinner();
        }
        Some(Err(e)) => {
            ui.colored_label(p.red, e);
        }
        Some(Ok(units)) => {
            for u in units.iter().take(300) {
                ui.horizontal(|ui| {
                    let color = match u.state {
                        svc::ServiceState::Running => p.green,
                        svc::ServiceState::Failed => p.red,
                        _ => p.muted,
                    };
                    ui.label(RichText::new("\u{f111}").color(color));
                    ui.label(RichText::new(&u.name).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for a in svc::service_actions(u.state, u.enabled).into_iter().take(3) {
                            if ui.small_button(format!("{a:?}")).clicked() && server.uses_ssh() {
                                let cmd = ServiceManagerType::Systemd.command_for(u, a);
                                let line = svc::terminal_command(&cmd, u.scope == svc::ServiceScope::System, false);
                                app.terms.open_ssh(&mut app.core, server, Some(line), &u.name);
                                app.push(Page::Terminal);
                            }
                        }
                    });
                });
            }
        }
    }
}
