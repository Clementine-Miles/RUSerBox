//! What the interface drives: the store, one tokio runtime, and a connection
//! per server with its latest status.
//!
//! Background tasks update shared state and ask egui for a repaint; the
//! interface reads that state each frame. Nothing the UI thread does waits on
//! the network. Each server's poller is a task aborted on disconnect, and
//! every network call below has a timeout.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sbm_parser::SystemType;
use sbm_ssh::{Auth, BoxFuture, Hop, HostKeyPrompt, HostTrust, Options, Session, StatusRunner};
use sbm_store::{KvStore, Server, Store, Transport};
use tokio::sync::oneshot;

use crate::i18n::t;

/// A host key question waiting for the person.
pub struct PendingPrompt {
    pub prompt: HostKeyPrompt,
    pub server_name: String,
    reply: Option<oneshot::Sender<bool>>,
}

impl PendingPrompt {
    pub fn answer(&mut self, accept: bool) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(accept);
        }
    }
}

/// Host keys from the store; new ones asked about through the interface.
pub struct Trust {
    store: Arc<Store>,
    prompts: Arc<Mutex<VecDeque<PendingPrompt>>>,
    ctx: egui::Context,
    names: HashMap<String, String>,
}

impl HostTrust for Trust {
    fn known(&self, host_id: &str, key_type: &str) -> Option<String> {
        self.store.known_host(host_id, key_type).ok().flatten()
    }
    fn ask(&self, prompt: HostKeyPrompt) -> BoxFuture<'static, bool> {
        let (tx, rx) = oneshot::channel();
        let server_name = self.names.get(&prompt.host_id).cloned().unwrap_or_else(|| prompt.host.clone());
        lock(&self.prompts).push_back(PendingPrompt { prompt, server_name, reply: Some(tx) });
        self.ctx.request_repaint();
        // An unanswered prompt (the app closed) is a refusal.
        Box::pin(async move { rx.await.unwrap_or(false) })
    }
    fn remember(&self, host_id: &str, key_type: &str, fingerprint: &str) {
        if let Err(e) = self.store.trust_host_key(host_id, key_type, fingerprint) {
            log::error!("could not save a trusted host key: {e}");
        }
    }
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// One reading, from either transport, in the units the pages show.
#[derive(Debug, Clone, Default)]
pub struct StatusView {
    pub cpu_percent: f64,
    pub cores: Vec<f64>,
    pub cpu_brand: Option<String>,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disks: Vec<DiskView>,
    pub net_rx_rate: f64,
    pub net_tx_rate: f64,
    pub net_rx_total: u64,
    pub net_tx_total: u64,
    pub temperature: Option<f64>,
    pub uptime: Option<String>,
    pub sys: Option<String>,
    pub host: Option<String>,
    pub ips: Vec<String>,
    pub gpus: Vec<GpuView>,
    pub at: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
pub struct DiskView {
    pub mount: String,
    pub path: String,
    pub used: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GpuView {
    pub name: String,
    pub usage: Option<f64>,
    pub temperature: Option<i64>,
    pub mem_used: Option<i64>,
    pub mem_total: Option<i64>,
}

/// How many readings the charts keep.
pub const HISTORY: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnState {
    #[default]
    Idle,
    Connecting,
    Connected,
    Failed(String),
}

/// One server's live state.
#[derive(Default)]
pub struct Live {
    pub state: ConnState,
    pub status: Option<StatusView>,
    pub cpu_history: VecDeque<f64>,
    pub mem_history: VecDeque<f64>,
    pub rx_history: VecDeque<f64>,
    pub tx_history: VecDeque<f64>,
    pub ssh: Option<Arc<Session>>,
    pub monitor: Option<Arc<sbm_monitor_client::Client>>,
    pub system: Option<SystemType>,
    pub transport: Option<Transport>,
    prev_cpu: Vec<sbm_parser::types::CpuCore>,
    prev_net: Option<(u64, u64, Instant)>,
}

fn push(q: &mut VecDeque<f64>, v: f64) {
    if q.len() == HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}

impl Live {
    fn record(&mut self, mut v: StatusView) {
        let now = Instant::now();
        if let Some((rx, tx, at)) = self.prev_net {
            let dt = now.duration_since(at).as_secs_f64().max(0.001);
            v.net_rx_rate = v.net_rx_total.saturating_sub(rx) as f64 / dt;
            v.net_tx_rate = v.net_tx_total.saturating_sub(tx) as f64 / dt;
        }
        if v.net_rx_total > 0 || v.net_tx_total > 0 {
            self.prev_net = Some((v.net_rx_total, v.net_tx_total, now));
        }
        v.at = Some(now);
        push(&mut self.cpu_history, v.cpu_percent);
        push(&mut self.mem_history, if v.mem_total > 0 { v.mem_used as f64 / v.mem_total as f64 * 100.0 } else { 0.0 });
        push(&mut self.rx_history, v.net_rx_rate);
        push(&mut self.tx_history, v.net_tx_rate);
        self.status = Some(v);
    }
}

fn busy(c: &sbm_parser::types::CpuCore) -> (u64, u64) {
    let idle = c.idle + c.iowait;
    let total = c.user + c.sys + c.nice + c.idle + c.iowait + c.irq + c.softirq;
    (total - idle.min(total), total)
}

/// CPU use from two readings of the raw counters (`cpu` first: the total).
pub fn cpu_percent(prev: &[sbm_parser::types::CpuCore], now: &[sbm_parser::types::CpuCore]) -> Vec<f64> {
    now.iter()
        .map(|c| {
            let Some(p) = prev.iter().find(|p| p.id == c.id) else { return 0.0 };
            let (b1, t1) = busy(p);
            let (b2, t2) = busy(c);
            let dt = t2.saturating_sub(t1);
            if dt == 0 { 0.0 } else { (b2.saturating_sub(b1) as f64 / dt as f64 * 100.0).clamp(0.0, 100.0) }
        })
        .collect()
}

fn from_ssh(s: &sbm_parser::ServerStatus, prev_cpu: &[sbm_parser::types::CpuCore], count_network: bool) -> StatusView {
    let pct = cpu_percent(prev_cpu, &s.cpu);
    let (total, cores) = match s.cpu.first() {
        Some(c) if c.id == "cpu" => (pct.first().copied().unwrap_or(0.0), pct.get(1..).map(<[f64]>::to_vec).unwrap_or_default()),
        _ => (if pct.is_empty() { 0.0 } else { pct.iter().sum::<f64>() / pct.len() as f64 }, pct.clone()),
    };
    let (disk_used, disk_total) = sbm_parser::types::disk_usage(&s.disks, count_network);
    let (rx, tx) = s.net.iter().filter(|n| n.device != "lo").fold((0, 0), |(r, t), n| (r + n.rx_bytes, t + n.tx_bytes));
    StatusView {
        cpu_percent: total,
        cores,
        cpu_brand: s.cpu_brand.first().map(|(b, n)| if *n > 1 { format!("{b} ×{n}") } else { b.clone() }),
        mem_used: s.mem.as_ref().map_or(0, |m| m.total.saturating_sub(m.avail) * 1024),
        mem_total: s.mem.as_ref().map_or(0, |m| m.total * 1024),
        swap_used: s.swap.as_ref().map_or(0, |m| m.total.saturating_sub(m.free) * 1024),
        swap_total: s.swap.as_ref().map_or(0, |m| m.total * 1024),
        disk_used: disk_used * 1024,
        disk_total: disk_total * 1024,
        disks: s.disks.iter().map(|d| DiskView { mount: d.mount.clone(), path: d.path.clone(), used: d.used * 1024, size: d.size * 1024 }).collect(),
        net_rx_total: rx,
        net_tx_total: tx,
        temperature: s.temps.first(),
        uptime: s.uptime.clone(),
        sys: s.sys.clone(),
        host: s.host.clone(),
        ips: s.ips.clone(),
        gpus: s
            .gpus
            .iter()
            .map(|g| GpuView {
                name: g.name.clone(),
                usage: g.utilization,
                temperature: g.temperature,
                mem_used: g.memory.as_ref().map(|m| m.used),
                mem_total: g.memory.as_ref().map(|m| m.total),
            })
            .collect(),
        ..Default::default()
    }
}

fn from_monitor(m: &sbm_monitor_client::Metrics) -> StatusView {
    StatusView {
        cpu_percent: m.cpu_usage,
        cpu_brand: m.cpu_brand.clone(),
        mem_used: m.memory.used,
        mem_total: m.memory.total,
        swap_used: m.swap.used,
        swap_total: m.swap.total,
        disk_used: m.disk.used,
        disk_total: m.disk.total,
        net_rx_total: m.network.rx_bytes,
        net_tx_total: m.network.tx_bytes,
        temperature: m.temperature,
        uptime: m.uptime.clone(),
        sys: m.sys.clone(),
        host: Some(m.server_name.clone()).filter(|s| !s.is_empty()),
        ips: m.ips.clone(),
        gpus: m.gpus.iter().map(|g| GpuView { name: g.name.clone(), usage: g.usage_percent, temperature: g.temperature, mem_used: g.memory_used, mem_total: g.memory_total }).collect(),
        ..Default::default()
    }
}

/// Parses `user@host:port` (the server's alternative address).
pub fn parse_alter_url(url: &str) -> Option<(String, String, u16)> {
    let (user, rest) = url.split_once('@')?;
    let idx = rest.rfind(':')?;
    let port: u16 = rest[idx + 1..].parse().ok().filter(|p| *p > 0)?;
    Some((rest[..idx].to_owned(), user.to_owned(), port))
}

pub struct Core {
    pub store: Arc<Store>,
    pub rt: tokio::runtime::Runtime,
    pub ctx: egui::Context,
    pub prompts: Arc<Mutex<VecDeque<PendingPrompt>>>,
    pub live: HashMap<String, Arc<Mutex<Live>>>,
    pollers: HashMap<String, tokio::task::JoinHandle<()>>,
    /// Messages for a toast at the bottom of the screen.
    pub notices: Arc<Mutex<VecDeque<String>>>,
    /// The widget server list, ready for the platform.
    pub widget_payload: Arc<Mutex<Option<String>>>,
}

impl Core {
    pub fn new(store: Arc<Store>, ctx: egui::Context) -> Core {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().thread_name("ruserbox-net").build().expect("tokio runtime");
        Core { store, rt, ctx, prompts: Arc::default(), live: HashMap::new(), pollers: HashMap::new(), notices: Arc::default(), widget_payload: Arc::default() }
    }

    pub fn notice(&self, msg: impl Into<String>) {
        lock(&self.notices).push_back(msg.into());
        self.ctx.request_repaint();
    }

    pub fn live(&mut self, id: &str) -> Arc<Mutex<Live>> {
        self.live.entry(id.to_owned()).or_default().clone()
    }

    fn trust(&self) -> Arc<dyn HostTrust> {
        let names = self.store.servers().unwrap_or_default().into_iter().map(|s| (s.id, s.name)).collect();
        Arc::new(Trust { store: self.store.clone(), prompts: self.prompts.clone(), ctx: self.ctx.clone(), names })
    }

    pub fn options(&self) -> Options {
        let secs: u64 = self.store.kv(KvStore::Setting).get_or("timeOut", 5u64).clamp(2, 60);
        Options { connect_timeout: Duration::from_secs(secs * 2), keepalive: Some(Duration::from_secs(30)) }
    }

    /// The hop for [server] (its own credentials), at [host]/[user]/[port].
    fn hop(&self, s: &Server, host: Option<(String, String, u16)>) -> Option<Hop> {
        let ssh = s.ssh.as_ref()?;
        let mut auth = Auth { password: ssh.pwd.clone().filter(|p| !p.is_empty()), ..Default::default() };
        if let Some(kid) = &ssh.key_id
            && let Some(k) = self.store.private_keys().ok()?.into_iter().find(|k| &k.id == kid)
        {
            auth.private_key = Some(k.key);
            auth.passphrase = self.store.kv(KvStore::KeyPassphrase).get::<String>(kid);
        }
        let (h, u, p) = host.unwrap_or_else(|| (ssh.ip.clone(), ssh.user.clone(), ssh.port));
        Some(Hop { id: s.id.clone(), host: h, port: p, user: u, auth, legacy_algorithms: ssh.allow_legacy_algorithms, proxy_command: ssh.proxy_command.clone().filter(|c| !c.trim().is_empty()) })
    }

    /// Jump servers first, the server last; jumps of jumps resolved, with
    /// a loop refused.
    pub fn chain(&self, s: &Server, alter: bool) -> Result<Vec<Hop>, String> {
        let all = self.store.servers().map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut seen = vec![s.id.clone()];
        fn walk(all: &[Server], s: &Server, seen: &mut Vec<String>, out: &mut Vec<Server>) -> Result<(), String> {
            for j in s.ssh.as_ref().map(|x| x.jump_ids.clone()).unwrap_or_default() {
                if seen.contains(&j) {
                    return Err(t("jumpLoop", "The jump servers form a loop"));
                }
                seen.push(j.clone());
                let js = all.iter().find(|x| x.id == j).ok_or_else(|| t("noJumpServerAvailable", "A jump server is missing"))?;
                walk(all, js, seen, out)?;
                out.push(js.clone());
            }
            Ok(())
        }
        let mut jumps = Vec::new();
        walk(&all, s, &mut seen, &mut jumps)?;
        for j in &jumps {
            out.push(self.hop(j, None).ok_or_else(|| format!("{}: no SSH", j.name))?);
        }
        let alt = if alter { s.ssh.as_ref().and_then(|x| x.alter_url.as_deref()).and_then(parse_alter_url) } else { None };
        out.push(self.hop(s, alt).ok_or_else(|| t("noSsh", "This server has no SSH settings"))?);
        Ok(out)
    }

    /// Opens an SSH session to [s] (trying its alternative address when the
    /// first does not answer).
    pub fn connect_ssh(&self, s: &Server) -> impl std::future::Future<Output = Result<Session, String>> + Send + 'static {
        let chain = self.chain(s, false);
        let alt_chain = s.ssh.as_ref().and_then(|x| x.alter_url.as_ref()).map(|_| self.chain(s, true));
        let trust = self.trust();
        let opts = self.options();
        async move {
            let chain = chain?;
            match Session::connect(&chain, trust.clone(), opts).await {
                Ok(s) => Ok(s),
                Err(e @ (sbm_ssh::Error::Timeout(_) | sbm_ssh::Error::Io(_))) => match alt_chain {
                    Some(Ok(alt)) => Session::connect(&alt, trust, opts).await.map_err(|e2| format!("{e}; {e2}")),
                    _ => Err(e.to_string()),
                },
                Err(e) => Err(e.to_string()),
            }
        }
    }

    pub fn monitor_client(&self, s: &Server) -> Result<sbm_monitor_client::Client, String> {
        let m = s.monitor.as_ref().ok_or_else(|| t("noMonitor", "This server has no Monitor"))?;
        sbm_monitor_client::Client::new(sbm_monitor_client::Config {
            addr: m.addr.clone(),
            password: m.pwd.clone().unwrap_or_default(),
            cert_sha256: m.cert_sha256.clone(),
            device_name: "RUSerBox on Android".into(),
            device_token: None,
            timeout: self.options().connect_timeout,
        })
        .map_err(|e| e.to_string())
    }

    /// Starts polling [s] (no-op when already running).
    pub fn start(&mut self, s: &Server) {
        if self.pollers.get(&s.id).is_some_and(|h| !h.is_finished()) {
            return;
        }
        let live = self.live(&s.id);
        let interval = Duration::from_secs(self.store.kv(KvStore::Setting).get_or("serverStatusUpdateInterval", 3u64).clamp(1, 600));
        let ctx = self.ctx.clone();
        let server = s.clone();
        let ssh_fut = server.uses_ssh().then(|| self.connect_ssh(&server));
        let mon = server.uses_monitor().then(|| self.monitor_client(&server));
        let lead = server.lead_transport();
        let count_network = server.disk_counts_network;
        let temp_div = 1000.0;
        lock(&live).state = ConnState::Connecting;
        ctx.request_repaint();
        let handle = self.rt.spawn(async move {
            // Connect: the lead transport, falling back to the other.
            let mut order: Vec<Transport> = lead.into_iter().collect();
            if let Some(f) = server.fallback_transport() {
                order.push(f);
            }
            let mut ssh_fut = ssh_fut;
            let mut mon = mon;
            let mut errors = Vec::new();
            let mut chosen = None;
            for tr in order {
                match tr {
                    Transport::Ssh => {
                        if let Some(f) = ssh_fut.take() {
                            match f.await {
                                Ok(sess) => {
                                    lock(&live).ssh = Some(Arc::new(sess));
                                    chosen = Some(tr);
                                    break;
                                }
                                Err(e) => errors.push(format!("SSH: {e}")),
                            }
                        }
                    }
                    Transport::MonitorHttp => match mon.take() {
                        Some(Ok(c)) => match c.login().await {
                            Ok(_) => {
                                lock(&live).monitor = Some(Arc::new(c));
                                chosen = Some(tr);
                                break;
                            }
                            Err(e) => errors.push(format!("Monitor: {e}")),
                        },
                        Some(Err(e)) => errors.push(e),
                        None => {}
                    },
                }
            }
            let Some(tr) = chosen else {
                lock(&live).state = ConnState::Failed(errors.join("\n"));
                ctx.request_repaint();
                return;
            };
            {
                let mut l = lock(&live);
                l.state = ConnState::Connected;
                l.transport = Some(tr);
            }
            ctx.request_repaint();
            let mut runner: Option<StatusRunner> = None;
            loop {
                let started = Instant::now();
                let (ssh, monitor) = {
                    let l = lock(&live);
                    (l.ssh.clone(), l.monitor.clone())
                };
                let result: Result<(), String> = match (tr, ssh, monitor) {
                    (Transport::Ssh, Some(sess), _) => {
                        if sess.is_closed() {
                            Err(t("disconnected", "Disconnected"))
                        } else {
                            if runner.is_none() {
                                let sys = match server.system_type {
                                    Some(sbm_store::SystemKind::Linux) => SystemType::Linux,
                                    Some(sbm_store::SystemKind::Bsd) => SystemType::Bsd,
                                    Some(sbm_store::SystemKind::Windows) => SystemType::Windows,
                                    None => StatusRunner::detect(&sess).await.unwrap_or(SystemType::Linux),
                                };
                                lock(&live).system = Some(sys);
                                runner = Some(StatusRunner::new(sys, server.script_dir.clone(), server.disabled_cmds.clone()));
                            }
                            let r = runner.as_mut().expect("set above");
                            match r.status(&sess, temp_div).await {
                                Ok(st) => {
                                    let mut l = lock(&live);
                                    let v = from_ssh(&st, &l.prev_cpu, count_network);
                                    l.prev_cpu = st.cpu.clone();
                                    l.record(v);
                                    Ok(())
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        }
                    }
                    (Transport::MonitorHttp, _, Some(c)) => match c.metrics().await {
                        Ok(m) => {
                            lock(&live).record(from_monitor(&m));
                            Ok(())
                        }
                        Err(e) => Err(e.to_string()),
                    },
                    _ => Err(t("disconnected", "Disconnected")),
                };
                match result {
                    Ok(()) => lock(&live).state = ConnState::Connected,
                    Err(e) => {
                        let closed = lock(&live).ssh.as_ref().is_some_and(|s| s.is_closed());
                        lock(&live).state = ConnState::Failed(e);
                        if closed {
                            lock(&live).ssh = None;
                            ctx.request_repaint();
                            return;
                        }
                    }
                }
                ctx.request_repaint();
                tokio::time::sleep(interval.saturating_sub(started.elapsed()).max(Duration::from_millis(500))).await;
            }
        });
        self.pollers.insert(s.id.clone(), handle);
    }

    pub fn stop(&mut self, id: &str) {
        if let Some(h) = self.pollers.remove(id) {
            h.abort();
        }
        if let Some(l) = self.live.get(id) {
            let mut l = lock(l);
            if let Some(s) = l.ssh.take() {
                self.rt.spawn(async move { s.disconnect().await });
            }
            l.monitor = None;
            l.state = ConnState::Idle;
        }
    }

    /// An SSH session for a page (terminal, files): the poller's, or a new one.
    pub fn session(&mut self, s: &Server) -> impl std::future::Future<Output = Result<Arc<Session>, String>> + Send + 'static {
        let live = self.live(&s.id);
        let existing = lock(&live).ssh.clone().filter(|x| !x.is_closed());
        let fut = if existing.is_none() && s.uses_ssh() { Some(self.connect_ssh(s)) } else { None };
        async move {
            if let Some(e) = existing {
                return Ok(e);
            }
            let sess = Arc::new(fut.ok_or_else(|| t("noSsh", "This server has no SSH settings"))?.await?);
            lock(&live).ssh = Some(sess.clone());
            Ok(sess)
        }
    }

    /// Builds the widget configuration screen's server list (Monitor servers
    /// with a read-only token each) in the background; the platform takes it
    /// from [`Core::widget_payload`].
    pub fn publish_widgets(&self) {
        let servers: Vec<Server> = self.store.servers().unwrap_or_default().into_iter().filter(|s| s.monitor.is_some()).collect();
        let clients: Vec<(Server, Option<sbm_monitor_client::Client>)> = servers.into_iter().map(|s| {
            let c = self.monitor_client(&s).ok();
            (s, c)
        }).collect();
        let out = self.widget_payload.clone();
        self.rt.spawn(async move {
            let mut entries = Vec::new();
            let mut sorted = clients;
            sorted.sort_by_key(|(s, _)| s.name.to_lowercase());
            for (s, c) in sorted {
                let m = s.monitor.clone().expect("filtered");
                let token = match c {
                    Some(c) => c.issue_watch_token(&format!("widget:{}", s.id)).await.ok(),
                    None => None,
                };
                let mut e = serde_json::json!({ "id": s.id, "name": s.name, "addr": m.addr.trim_end_matches('/'), "expiresAt": token.as_ref().map_or(0, |t| t.1), "allowInsecure": m.allow_insecure.unwrap_or(false) });
                if let Some((t, _)) = token {
                    e["token"] = serde_json::Value::String(t);
                }
                if let Some(f) = m.fallback_addr.filter(|f| !f.trim().is_empty()) {
                    e["fallbackAddr"] = serde_json::Value::String(f.trim_end_matches('/').to_owned());
                }
                if let Some(c) = m.cert_sha256 {
                    e["certSha256"] = serde_json::Value::String(c);
                }
                entries.push(e);
            }
            *lock(&out) = Some(serde_json::json!({ "v": 1, "servers": entries }).to_string());
        });
    }

    /// Records a reading as if it had come from the server (previews, tests).
    #[doc(hidden)]
    pub fn preview_status(&mut self, id: &str, v: StatusView) {
        let live = self.live(id);
        let mut l = lock(&live);
        l.state = ConnState::Connected;
        l.record(v);
    }

    pub fn has_prompt(&self) -> bool {
        !lock(&self.prompts).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbm_parser::types::CpuCore;

    fn core(id: &str, user: u64, idle: u64) -> CpuCore {
        CpuCore { id: id.into(), user, sys: 0, nice: 0, idle, iowait: 0, irq: 0, softirq: 0 }
    }

    #[test]
    fn cpu_use_comes_from_counter_deltas() {
        let a = [core("cpu", 100, 900)];
        let b = [core("cpu", 150, 950)];
        assert_eq!(cpu_percent(&a, &b), [50.0]);
        assert_eq!(cpu_percent(&[], &b), [0.0], "no baseline yet");
    }

    #[test]
    fn alternative_addresses_parse_like_the_flutter_app() {
        assert_eq!(parse_alter_url("root@10.0.0.2:2222"), Some(("10.0.0.2".into(), "root".into(), 2222)));
        assert_eq!(parse_alter_url("u@[::1]:22"), Some(("[::1]".into(), "u".into(), 22)));
        assert_eq!(parse_alter_url("nouser:22"), None);
        assert_eq!(parse_alter_url("u@h:0"), None);
    }

    #[test]
    fn jump_chains_resolve_in_order_and_refuse_loops() {
        let store = Arc::new(Store::in_memory().unwrap());
        let ctx = egui::Context::default();
        let c = Core::new(store.clone(), ctx);
        let a = Server::new_ssh("a", "a.example", 22, "root");
        let mut b = Server::new_ssh("b", "b.example", 22, "root");
        b.ssh.as_mut().unwrap().jump_ids = vec![a.id.clone()];
        let mut target = Server::new_ssh("t", "t.internal", 22, "ops");
        target.ssh.as_mut().unwrap().jump_ids = vec![b.id.clone()];
        store.put_server(&a).unwrap();
        store.put_server(&b).unwrap();
        store.put_server(&target).unwrap();
        let hosts: Vec<String> = c.chain(&target, false).unwrap().into_iter().map(|h| h.host).collect();
        assert_eq!(hosts, ["a.example", "b.example", "t.internal"]);
        let mut a2 = a.clone();
        a2.ssh.as_mut().unwrap().jump_ids = vec![target.id.clone()];
        store.put_server(&a2).unwrap();
        assert!(c.chain(&target, false).is_err());
    }
}
