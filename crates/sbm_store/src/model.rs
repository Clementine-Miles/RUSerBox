//! Typed records over the v43 tables.
//!
//! A write names its data columns (`ON CONFLICT DO UPDATE`), bumps `rev` and
//! stamps `updated_at`; children are replaced with their parent; a delete
//! leaves a `tombstone`. Columns this build does not edit are carried in the
//! record unchanged, so saving never clears what another build wrote.

use serde::{Deserialize, Serialize};

use crate::sqlite::{Connection, Row, Value};
use crate::{Error, Result, Store, new_id, now_ms};

/// The platform a server runs, as `server.system_type` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemKind {
    Linux,
    Bsd,
    Windows,
}

impl SystemKind {
    pub fn name(self) -> &'static str {
        match self {
            SystemKind::Linux => "linux",
            SystemKind::Bsd => "bsd",
            SystemKind::Windows => "windows",
        }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "linux" => Some(Self::Linux),
            "bsd" => Some(Self::Bsd),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }
}

/// `server.preferred_transport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transport {
    Ssh,
    MonitorHttp,
}

impl Transport {
    pub fn name(self) -> &'static str {
        match self {
            Transport::Ssh => "ssh",
            Transport::MonitorHttp => "monitorHttp",
        }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "ssh" => Some(Self::Ssh),
            "monitorHttp" => Some(Self::MonitorHttp),
            _ => None,
        }
    }
}

/// How an SSH server is reached.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SshCfg {
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub pwd: Option<String>,
    /// `private_key.id`.
    pub key_id: Option<String>,
    pub key_path: Option<String>,
    pub alter_url: Option<String>,
    pub proxy_command: Option<String>,
    /// `sftp` or `scp`, by name; `None` reads as `sftp`.
    pub file_transport: Option<String>,
    pub allow_legacy_algorithms: bool,
    pub identity_agent: Option<String>,
    pub forward_agent: bool,
    pub control_path: Option<String>,
    /// Jump servers, nearest first (`server_jump` in `ord` order).
    pub jump_ids: Vec<String>,
    pub identity_files: Vec<String>,
}

/// How the Monitor agent is reached.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MonitorCfg {
    pub addr: String,
    pub fallback_addr: Option<String>,
    pub legacy_user: Option<String>,
    pub pwd: Option<String>,
    /// A pinned leaf certificate (SHA-256, lowercase hex).
    pub cert_sha256: Option<String>,
    pub allow_insecure: Option<bool>,
}

/// One server (`server` and its children).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub auto_connect: bool,
    pub system_type: Option<SystemKind>,
    pub ssh: Option<SshCfg>,
    pub monitor: Option<MonitorCfg>,
    pub preferred_transport: Option<Transport>,
    pub ssh_enabled: bool,
    pub monitor_enabled: bool,
    pub is_local: bool,
    pub tags: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub disabled_cmds: Vec<String>,
    // Carried unchanged.
    pub wol_mac: Option<String>,
    pub wol_ip: Option<String>,
    pub wol_pwd: Option<String>,
    pub bmc_addr: Option<String>,
    pub bmc_cert_sha256: Option<String>,
    pub bmc_cred_id: Option<String>,
    pub prefer_temp_dev: Option<String>,
    pub temp_is_celsius: bool,
    pub logo_url: Option<String>,
    pub net_dev: Option<String>,
    pub script_dir: Option<String>,
    pub geo_lat: Option<f64>,
    pub geo_lon: Option<f64>,
    pub disk_counts_network: bool,
    pub updated_at: i64,
    pub rev: i64,
}

impl Server {
    /// A new SSH server.
    pub fn new_ssh(name: &str, ip: &str, port: u16, user: &str) -> Self {
        Server {
            id: new_id(),
            name: name.into(),
            auto_connect: true,
            system_type: None,
            ssh: Some(SshCfg { ip: ip.into(), port, user: user.into(), ..Default::default() }),
            monitor: None,
            preferred_transport: None,
            ssh_enabled: true,
            monitor_enabled: true,
            is_local: false,
            tags: vec![],
            envs: vec![],
            disabled_cmds: vec![],
            wol_mac: None,
            wol_ip: None,
            wol_pwd: None,
            bmc_addr: None,
            bmc_cert_sha256: None,
            bmc_cred_id: None,
            prefer_temp_dev: None,
            temp_is_celsius: true,
            logo_url: None,
            net_dev: None,
            script_dir: None,
            geo_lat: None,
            geo_lon: None,
            disk_counts_network: false,
            updated_at: 0,
            rev: 0,
        }
    }

    /// Whether SSH is configured and switched on.
    pub fn uses_ssh(&self) -> bool {
        self.ssh.is_some() && self.ssh_enabled
    }

    /// Whether the Monitor is configured and switched on.
    pub fn uses_monitor(&self) -> bool {
        self.monitor.is_some() && self.monitor_enabled
    }

    /// The transport tried first: SSH when both, unless preferred otherwise.
    pub fn lead_transport(&self) -> Option<Transport> {
        match (self.uses_ssh(), self.uses_monitor()) {
            (true, true) => Some(self.preferred_transport.unwrap_or(Transport::Ssh)),
            (true, false) => Some(Transport::Ssh),
            (false, true) => Some(Transport::MonitorHttp),
            (false, false) => None,
        }
    }

    /// The other transport, when both are configured.
    pub fn fallback_transport(&self) -> Option<Transport> {
        if !(self.uses_ssh() && self.uses_monitor()) {
            return None;
        }
        match self.lead_transport()? {
            Transport::Ssh => Some(Transport::MonitorHttp),
            Transport::MonitorHttp => Some(Transport::Ssh),
        }
    }

    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("a server needs a name".into()));
        }
        if self.ssh.is_none() && self.monitor.is_none() && !self.is_local {
            return Err(Error::Invalid("a server needs SSH or a Monitor address".into()));
        }
        if let Some(ssh) = &self.ssh {
            if ssh.ip.trim().is_empty() {
                return Err(Error::Invalid("an SSH server needs a host".into()));
            }
            if ssh.port == 0 {
                return Err(Error::Invalid("the SSH port is 1 to 65535".into()));
            }
            if ssh.jump_ids.iter().any(|j| j == &self.id) {
                return Err(Error::Invalid("a server cannot jump through itself".into()));
            }
        }
        Ok(())
    }
}

const SERVER_COLS: &str = "id, name, auto_connect, system_type, ssh_ip, ssh_port, ssh_user, ssh_pwd, ssh_key_id, ssh_key_path, \
    ssh_alter_url, ssh_proxy_command, ssh_file_transport, ssh_allow_legacy_algorithms, preferred_transport, ssh_enabled, \
    monitor_enabled, is_local, monitor_addr, monitor_user, monitor_pwd, monitor_cert_sha256, monitor_allow_insecure, \
    wol_mac, wol_ip, wol_pwd, bmc_addr, bmc_cert_sha256, bmc_cred_id, prefer_temp_dev, temp_is_celsius, logo_url, net_dev, \
    script_dir, geo_lat, geo_lon, disk_counts_network, ssh_identity_agent, ssh_forward_agent, ssh_control_path, \
    monitor_fallback_addr, updated_at, rev";

fn server_of(r: &Row) -> Server {
    let ssh = r.text(4).map(|ip| SshCfg {
        ip,
        port: r.int(5).and_then(|p| u16::try_from(p).ok()).unwrap_or(22),
        user: r.text(6).unwrap_or_else(|| "root".into()),
        pwd: r.text(7),
        key_id: r.text(8),
        key_path: r.text(9),
        alter_url: r.text(10),
        proxy_command: r.text(11),
        file_transport: r.text(12),
        allow_legacy_algorithms: r.bool(13),
        identity_agent: r.text(37),
        forward_agent: r.bool(38),
        control_path: r.text(39),
        jump_ids: vec![],
        identity_files: vec![],
    });
    let monitor = r.text(18).map(|addr| MonitorCfg {
        addr,
        fallback_addr: r.text(40),
        legacy_user: r.text(19),
        pwd: r.text(20),
        cert_sha256: r.text(21),
        allow_insecure: r.int(22).map(|v| v != 0),
    });
    Server {
        id: r.text_or_default(0),
        name: r.text_or_default(1),
        auto_connect: r.bool(2),
        system_type: r.text(3).as_deref().and_then(SystemKind::from_name),
        ssh,
        monitor,
        preferred_transport: r.text(14).as_deref().and_then(Transport::from_name),
        ssh_enabled: r.int(15).unwrap_or(1) != 0,
        monitor_enabled: r.int(16).unwrap_or(1) != 0,
        is_local: r.bool(17),
        tags: vec![],
        envs: vec![],
        disabled_cmds: vec![],
        wol_mac: r.text(23),
        wol_ip: r.text(24),
        wol_pwd: r.text(25),
        bmc_addr: r.text(26),
        bmc_cert_sha256: r.text(27),
        bmc_cred_id: r.text(28),
        prefer_temp_dev: r.text(29),
        temp_is_celsius: r.int(30).unwrap_or(1) != 0,
        logo_url: r.text(31),
        net_dev: r.text(32),
        script_dir: r.text(33),
        geo_lat: r.real(34),
        geo_lon: r.real(35),
        disk_counts_network: r.bool(36),
        updated_at: r.int(41).unwrap_or(0),
        rev: r.int(42).unwrap_or(0),
    }
}

fn server_values(s: &Server, now: i64, rev: i64) -> Vec<Value> {
    let ssh = s.ssh.as_ref();
    let mon = s.monitor.as_ref();
    vec![
        s.id.clone().into(),
        s.name.clone().into(),
        s.auto_connect.into(),
        s.system_type.map(|t| t.name()).into(),
        ssh.map(|x| x.ip.clone()).into(),
        ssh.map(|x| i64::from(x.port)).into(),
        ssh.map(|x| x.user.clone()).into(),
        ssh.and_then(|x| x.pwd.clone()).into(),
        ssh.and_then(|x| x.key_id.clone()).into(),
        ssh.and_then(|x| x.key_path.clone()).into(),
        ssh.and_then(|x| x.alter_url.clone()).into(),
        ssh.and_then(|x| x.proxy_command.clone()).into(),
        ssh.and_then(|x| x.file_transport.clone()).into(),
        ssh.is_some_and(|x| x.allow_legacy_algorithms).into(),
        s.preferred_transport.map(|t| t.name()).into(),
        s.ssh_enabled.into(),
        s.monitor_enabled.into(),
        s.is_local.into(),
        mon.map(|m| m.addr.clone()).into(),
        mon.and_then(|m| m.legacy_user.clone()).into(),
        mon.and_then(|m| m.pwd.clone()).into(),
        mon.and_then(|m| m.cert_sha256.clone()).into(),
        mon.and_then(|m| m.allow_insecure).into(),
        s.wol_mac.clone().into(),
        s.wol_ip.clone().into(),
        s.wol_pwd.clone().into(),
        s.bmc_addr.clone().into(),
        s.bmc_cert_sha256.clone().into(),
        s.bmc_cred_id.clone().into(),
        s.prefer_temp_dev.clone().into(),
        s.temp_is_celsius.into(),
        s.logo_url.clone().into(),
        s.net_dev.clone().into(),
        s.script_dir.clone().into(),
        s.geo_lat.into(),
        s.geo_lon.into(),
        s.disk_counts_network.into(),
        ssh.and_then(|x| x.identity_agent.clone()).into(),
        ssh.is_some_and(|x| x.forward_agent).into(),
        ssh.and_then(|x| x.control_path.clone()).into(),
        mon.and_then(|m| m.fallback_addr.clone()).into(),
        Value::Int(now),
        Value::Int(rev),
    ]
}

fn tombstone(c: &Connection, tbl: &str, id: &str) -> Result<()> {
    c.execute(
        "INSERT INTO tombstone (tbl, row_id, deleted_at) VALUES (?, ?, ?) \
         ON CONFLICT (tbl, row_id) DO UPDATE SET deleted_at = excluded.deleted_at",
        &[tbl.into(), id.into(), Value::Int(now_ms())],
    )?;
    Ok(())
}

fn clear_tombstone(c: &Connection, tbl: &str, id: &str) -> Result<()> {
    c.execute("DELETE FROM tombstone WHERE tbl = ? AND row_id = ?", &[tbl.into(), id.into()])?;
    Ok(())
}

/// The server order the Flutter app kept (`setting.serverOrder`).
const SERVER_ORDER: &str = "serverOrder";

impl Store {
    /// Every server, in the saved order, with its children.
    pub fn servers(&self) -> Result<Vec<Server>> {
        let c = self.conn();
        let mut list = c.query(&format!("SELECT {SERVER_COLS} FROM server"), &[], |r| Ok(server_of(r)))?;
        let group = |sql: &str| -> Result<Vec<(String, String, String)>> {
            c.query(sql, &[], |r| Ok((r.text_or_default(0), r.text_or_default(1), r.text_or_default(2))))
        };
        let tags = group("SELECT server_id, tag, '' FROM server_tag ORDER BY tag")?;
        let envs = group("SELECT server_id, key, value FROM server_env ORDER BY key")?;
        let jumps = group("SELECT server_id, jump_id, '' FROM server_jump ORDER BY ord")?;
        let files = group("SELECT server_id, path, '' FROM server_identity_file ORDER BY ord")?;
        let disabled = group("SELECT server_id, cmd_type, '' FROM server_disabled_cmd ORDER BY cmd_type")?;
        drop(c);
        for s in &mut list {
            s.tags = tags.iter().filter(|t| t.0 == s.id).map(|t| t.1.clone()).collect();
            s.envs = envs.iter().filter(|t| t.0 == s.id).map(|t| (t.1.clone(), t.2.clone())).collect();
            s.disabled_cmds = disabled.iter().filter(|t| t.0 == s.id).map(|t| t.1.clone()).collect();
            if let Some(ssh) = &mut s.ssh {
                ssh.jump_ids = jumps.iter().filter(|t| t.0 == s.id).map(|t| t.1.clone()).collect();
                ssh.identity_files = files.iter().filter(|t| t.0 == s.id).map(|t| t.1.clone()).collect();
            }
        }
        let order: Vec<String> = self.kv(crate::KvStore::Setting).get_or(SERVER_ORDER, vec![]);
        list.sort_by_key(|s| (order.iter().position(|o| o == &s.id).unwrap_or(usize::MAX), s.name.to_lowercase()));
        Ok(list)
    }

    pub fn server(&self, id: &str) -> Result<Option<Server>> {
        Ok(self.servers()?.into_iter().find(|s| s.id == id))
    }

    /// Inserts or updates [server] and replaces its children.
    pub fn put_server(&self, server: &Server) -> Result<()> {
        server.validate()?;
        let c = self.conn();
        c.transaction(|c| {
            let rev = c.query_one("SELECT rev FROM server WHERE id = ?", &[server.id.clone().into()], |r| Ok(r.int(0).unwrap_or(0)))?;
            let vals = server_values(server, now_ms(), rev.map_or(0, |r| r + 1));
            let cols: Vec<&str> = SERVER_COLS.split(',').map(str::trim).collect();
            let marks = vec!["?"; cols.len()].join(", ");
            let updates: Vec<String> = cols.iter().filter(|c| **c != "id").map(|c| format!("{c} = excluded.{c}")).collect();
            c.execute(
                &format!("INSERT INTO server ({SERVER_COLS}) VALUES ({marks}) ON CONFLICT (id) DO UPDATE SET {}", updates.join(", ")),
                &vals,
            )?;
            let id: Value = server.id.clone().into();
            for t in ["server_tag", "server_env", "server_jump", "server_identity_file", "server_disabled_cmd"] {
                c.execute(&format!("DELETE FROM {t} WHERE server_id = ?"), std::slice::from_ref(&id))?;
            }
            for tag in &server.tags {
                c.execute("INSERT OR IGNORE INTO server_tag (server_id, tag) VALUES (?, ?)", &[id.clone(), tag.into()])?;
            }
            for (k, v) in &server.envs {
                c.execute("INSERT OR REPLACE INTO server_env (server_id, key, value) VALUES (?, ?, ?)", &[id.clone(), k.into(), v.into()])?;
            }
            for d in &server.disabled_cmds {
                c.execute("INSERT OR IGNORE INTO server_disabled_cmd (server_id, cmd_type) VALUES (?, ?)", &[id.clone(), d.into()])?;
            }
            if let Some(ssh) = &server.ssh {
                for (i, j) in ssh.jump_ids.iter().enumerate() {
                    c.execute("INSERT INTO server_jump (server_id, ord, jump_id) VALUES (?, ?, ?)", &[id.clone(), Value::Int(i as i64), j.into()])?;
                }
                for (i, p) in ssh.identity_files.iter().enumerate() {
                    c.execute("INSERT INTO server_identity_file (server_id, ord, path) VALUES (?, ?, ?)", &[id.clone(), Value::Int(i as i64), p.into()])?;
                }
            }
            clear_tombstone(c, "server", &server.id)
        })
    }

    /// Deletes a server; its children and known host keys go with it.
    pub fn delete_server(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.transaction(|c| {
            c.execute("DELETE FROM server WHERE id = ?", &[id.into()])?;
            tombstone(c, "server", id)
        })
    }

    pub fn set_server_order(&self, ids: &[String]) -> Result<()> {
        self.kv(crate::KvStore::Setting).set(SERVER_ORDER, ids)
    }
}

/// A stored private key (`private_key`), PEM/OpenSSH text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivateKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub comment: Option<String>,
}

impl Store {
    pub fn private_keys(&self) -> Result<Vec<PrivateKey>> {
        self.conn().query("SELECT id, name, key, comment FROM private_key ORDER BY name COLLATE NOCASE", &[], |r| {
            Ok(PrivateKey { id: r.text_or_default(0), name: r.text_or_default(1), key: r.text_or_default(2), comment: r.text(3) })
        })
    }

    pub fn put_private_key(&self, k: &PrivateKey) -> Result<()> {
        let c = self.conn();
        let taken = c.query_one("SELECT id FROM private_key WHERE name = ? AND id != ?", &[k.name.clone().into(), k.id.clone().into()], |r| Ok(r.text_or_default(0)))?;
        if taken.is_some() {
            return Err(Error::DuplicateName(k.name.clone()));
        }
        c.transaction(|c| {
            c.execute(
                "INSERT INTO private_key (id, name, key, comment, updated_at, rev) VALUES (?, ?, ?, ?, ?, 0) \
                 ON CONFLICT (id) DO UPDATE SET name = excluded.name, key = excluded.key, comment = excluded.comment, \
                 updated_at = excluded.updated_at, rev = private_key.rev + 1",
                &[k.id.clone().into(), k.name.clone().into(), k.key.clone().into(), k.comment.clone().into(), Value::Int(now_ms())],
            )?;
            clear_tombstone(c, "private_key", &k.id)
        })
    }

    /// Servers using it keep their row; `ssh_key_id` becomes null.
    pub fn delete_private_key(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.transaction(|c| {
            c.execute("DELETE FROM private_key WHERE id = ?", &[id.into()])?;
            tombstone(c, "private_key", id)
        })
    }
}

/// A snippet (`snippet`, `snippet_tag`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub script: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

impl Store {
    pub fn snippets(&self) -> Result<Vec<Snippet>> {
        let c = self.conn();
        let mut list = c.query("SELECT id, name, script, note FROM snippet ORDER BY name COLLATE NOCASE", &[], |r| {
            Ok(Snippet { id: r.text_or_default(0), name: r.text_or_default(1), script: r.text_or_default(2), note: r.text(3), tags: vec![] })
        })?;
        let tags = c.query("SELECT snippet_id, tag FROM snippet_tag ORDER BY tag", &[], |r| Ok((r.text_or_default(0), r.text_or_default(1))))?;
        for s in &mut list {
            s.tags = tags.iter().filter(|t| t.0 == s.id).map(|t| t.1.clone()).collect();
        }
        Ok(list)
    }

    pub fn put_snippet(&self, s: &Snippet) -> Result<()> {
        let c = self.conn();
        let taken = c.query_one("SELECT id FROM snippet WHERE name = ? AND id != ?", &[s.name.clone().into(), s.id.clone().into()], |r| Ok(r.text_or_default(0)))?;
        if taken.is_some() {
            return Err(Error::DuplicateName(s.name.clone()));
        }
        c.transaction(|c| {
            c.execute(
                "INSERT INTO snippet (id, name, script, note, updated_at, rev) VALUES (?, ?, ?, ?, ?, 0) \
                 ON CONFLICT (id) DO UPDATE SET name = excluded.name, script = excluded.script, note = excluded.note, \
                 updated_at = excluded.updated_at, rev = snippet.rev + 1",
                &[s.id.clone().into(), s.name.clone().into(), s.script.clone().into(), s.note.clone().into(), Value::Int(now_ms())],
            )?;
            c.execute("DELETE FROM snippet_tag WHERE snippet_id = ?", &[s.id.clone().into()])?;
            for t in &s.tags {
                c.execute("INSERT OR IGNORE INTO snippet_tag (snippet_id, tag) VALUES (?, ?)", &[s.id.clone().into(), t.into()])?;
            }
            clear_tombstone(c, "snippet", &s.id)
        })
    }

    pub fn delete_snippet(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.transaction(|c| {
            c.execute("DELETE FROM snippet WHERE id = ?", &[id.into()])?;
            tombstone(c, "snippet", id)
        })
    }
}

/// `port_forward.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForwardKind {
    Local,
    Remote,
    Dynamic,
}

impl ForwardKind {
    pub fn name(self) -> &'static str {
        match self {
            ForwardKind::Local => "local",
            ForwardKind::Remote => "remote",
            ForwardKind::Dynamic => "dynamic",
        }
    }
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "local" => Some(Self::Local),
            "remote" => Some(Self::Remote),
            "dynamic" => Some(Self::Dynamic),
            _ => None,
        }
    }
}

/// A saved port forward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortForward {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub kind: ForwardKind,
    pub local_host: Option<String>,
    pub local_port: u16,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
}

impl Store {
    pub fn port_forwards(&self, server_id: &str) -> Result<Vec<PortForward>> {
        self.conn().query(
            "SELECT id, server_id, name, type, local_host, local_port, remote_host, remote_port FROM port_forward WHERE server_id = ? ORDER BY name",
            &[server_id.into()],
            |r| {
                Ok(PortForward {
                    id: r.text_or_default(0),
                    server_id: r.text_or_default(1),
                    name: r.text_or_default(2),
                    kind: r.text(3).as_deref().and_then(ForwardKind::from_name).unwrap_or(ForwardKind::Local),
                    local_host: r.text(4),
                    local_port: r.int(5).and_then(|p| u16::try_from(p).ok()).unwrap_or(0),
                    remote_host: r.text(6),
                    remote_port: r.int(7).and_then(|p| u16::try_from(p).ok()),
                })
            },
        )
    }

    pub fn put_port_forward(&self, f: &PortForward) -> Result<()> {
        if f.kind != ForwardKind::Dynamic && (f.remote_port.is_none() || f.remote_host.as_deref().unwrap_or("").is_empty()) {
            return Err(Error::Invalid("a local or remote forward needs a destination host and port".into()));
        }
        let c = self.conn();
        c.transaction(|c| {
            c.execute(
                "INSERT INTO port_forward (id, server_id, name, type, local_host, local_port, remote_host, remote_port, updated_at, rev) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0) ON CONFLICT (id) DO UPDATE SET server_id = excluded.server_id, name = excluded.name, \
                 type = excluded.type, local_host = excluded.local_host, local_port = excluded.local_port, remote_host = excluded.remote_host, \
                 remote_port = excluded.remote_port, updated_at = excluded.updated_at, rev = port_forward.rev + 1",
                &[
                    f.id.clone().into(),
                    f.server_id.clone().into(),
                    f.name.clone().into(),
                    f.kind.name().into(),
                    f.local_host.clone().into(),
                    f.local_port.into(),
                    f.remote_host.clone().into(),
                    f.remote_port.into(),
                    Value::Int(now_ms()),
                ],
            )?;
            clear_tombstone(c, "port_forward", &f.id)
        })
    }

    pub fn delete_port_forward(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.transaction(|c| {
            c.execute("DELETE FROM port_forward WHERE id = ?", &[id.into()])?;
            tombstone(c, "port_forward", id)
        })
    }
}

/// Trusted host keys (`known_host`): one fingerprint per server and key type,
/// `SHA256:<base64 without padding>` as OpenSSH prints it.
impl Store {
    pub fn known_host(&self, server_id: &str, key_type: &str) -> Result<Option<String>> {
        self.conn().query_one(
            "SELECT fingerprint FROM known_host WHERE server_id = ? AND key_type = ?",
            &[server_id.into(), key_type.into()],
            |r| Ok(r.text_or_default(0)),
        )
    }

    /// Every trusted key type of a server, for the algorithm preference.
    pub fn known_host_types(&self, server_id: &str) -> Result<Vec<(String, String)>> {
        self.conn().query("SELECT key_type, fingerprint FROM known_host WHERE server_id = ? ORDER BY key_type", &[server_id.into()], |r| {
            Ok((r.text_or_default(0), r.text_or_default(1)))
        })
    }

    /// Records a fingerprint the user accepted. Never called for a refusal.
    pub fn trust_host_key(&self, server_id: &str, key_type: &str, fingerprint: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO known_host (server_id, key_type, fingerprint) VALUES (?, ?, ?) \
             ON CONFLICT (server_id, key_type) DO UPDATE SET fingerprint = excluded.fingerprint",
            &[server_id.into(), key_type.into(), fingerprint.into()],
        )?;
        Ok(())
    }

    pub fn forget_host_keys(&self, server_id: &str) -> Result<()> {
        self.conn().execute("DELETE FROM known_host WHERE server_id = ?", &[server_id.into()])?;
        Ok(())
    }
}

/// A connection attempt (`conn_stat`), kept for the connection history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnStat {
    pub server_id: String,
    pub server_name: String,
    pub timestamp: i64,
    /// `success`, `timeout`, `authFailed`, `networkError`, `unknownError`.
    pub result: String,
    pub error_message: String,
    pub duration_ms: i64,
}

impl Store {
    pub fn record_conn(&self, s: &ConnStat) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO conn_stat (id, server_id, server_name, timestamp, result, error_message, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                new_id().into(),
                s.server_id.clone().into(),
                s.server_name.clone().into(),
                Value::Int(s.timestamp),
                s.result.clone().into(),
                s.error_message.clone().into(),
                Value::Int(s.duration_ms),
            ],
        )?;
        // The Flutter app kept a month; so does this.
        c.execute("DELETE FROM conn_stat WHERE timestamp < ?", &[Value::Int(now_ms() - 30 * 86_400_000)])?;
        Ok(())
    }

    pub fn conn_stats(&self, server_id: &str, limit: i64) -> Result<Vec<ConnStat>> {
        self.conn().query(
            "SELECT server_id, server_name, timestamp, result, error_message, duration_ms FROM conn_stat WHERE server_id = ? ORDER BY timestamp DESC LIMIT ?",
            &[server_id.into(), Value::Int(limit)],
            |r| {
                Ok(ConnStat {
                    server_id: r.text_or_default(0),
                    server_name: r.text_or_default(1),
                    timestamp: r.int(2).unwrap_or(0),
                    result: r.text_or_default(3),
                    error_message: r.text_or_default(4),
                    duration_ms: r.int(5).unwrap_or(0),
                })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_round_trips_with_its_children() {
        let st = Store::in_memory().unwrap();
        let jump = Server::new_ssh("jump", "10.0.0.1", 22, "root");
        st.put_server(&jump).unwrap();
        let mut s = Server::new_ssh("web", "example.com", 2222, "deploy");
        s.tags = vec!["prod".into(), "eu".into()];
        s.envs = vec![("LANG".into(), "C.UTF-8".into())];
        s.ssh.as_mut().unwrap().jump_ids = vec![jump.id.clone()];
        s.monitor = Some(MonitorCfg { addr: "https://example.com:3770".into(), ..Default::default() });
        st.put_server(&s).unwrap();
        let back = st.server(&s.id).unwrap().unwrap();
        assert_eq!(back.name, "web");
        assert_eq!(back.ssh.as_ref().unwrap().port, 2222);
        assert_eq!(back.ssh.as_ref().unwrap().jump_ids, std::slice::from_ref(&jump.id));
        assert_eq!(back.tags, ["eu", "prod"]);
        assert_eq!(back.monitor.as_ref().unwrap().addr, "https://example.com:3770");
        assert_eq!(back.lead_transport(), Some(Transport::Ssh));
        assert_eq!(back.fallback_transport(), Some(Transport::MonitorHttp));
        assert_eq!(back.rev, 0);

        st.put_server(&back).unwrap();
        assert_eq!(st.server(&s.id).unwrap().unwrap().rev, 1, "an edit bumps rev");

        st.trust_host_key(&s.id, "ssh-ed25519", "SHA256:abc").unwrap();
        st.delete_server(&s.id).unwrap();
        assert!(st.server(&s.id).unwrap().is_none());
        assert_eq!(st.known_host(&s.id, "ssh-ed25519").unwrap(), None, "keys go with the server");
        let stones: Vec<String> = st.conn().query("SELECT row_id FROM tombstone WHERE tbl = 'server'", &[], |r| Ok(r.text_or_default(0))).unwrap();
        assert_eq!(stones, [s.id.clone()]);
    }

    #[test]
    fn deleting_a_key_detaches_servers_without_deleting_them() {
        let st = Store::in_memory().unwrap();
        let k = PrivateKey { id: new_id(), name: "laptop".into(), key: "-----BEGIN OPENSSH PRIVATE KEY-----".into(), comment: None };
        st.put_private_key(&k).unwrap();
        let mut s = Server::new_ssh("a", "a", 22, "root");
        s.ssh.as_mut().unwrap().key_id = Some(k.id.clone());
        st.put_server(&s).unwrap();
        st.delete_private_key(&k.id).unwrap();
        let back = st.server(&s.id).unwrap().unwrap();
        assert_eq!(back.ssh.unwrap().key_id, None);
    }

    #[test]
    fn names_stay_unique_and_invalid_records_are_refused() {
        let st = Store::in_memory().unwrap();
        let a = Snippet { id: new_id(), name: "uptime".into(), script: "uptime".into(), note: None, tags: vec!["sys".into()] };
        st.put_snippet(&a).unwrap();
        let b = Snippet { id: new_id(), ..a.clone() };
        assert!(matches!(st.put_snippet(&b), Err(Error::DuplicateName(_))));
        let mut s = Server::new_ssh("x", "", 22, "root");
        assert!(st.put_server(&s).is_err());
        s.ssh = None;
        assert!(st.put_server(&s).is_err(), "neither SSH nor Monitor");
        assert_eq!(st.snippets().unwrap()[0].tags, ["sys"]);
    }

    #[test]
    fn the_saved_order_is_kept() {
        let st = Store::in_memory().unwrap();
        let a = Server::new_ssh("a", "a", 22, "root");
        let b = Server::new_ssh("b", "b", 22, "root");
        st.put_server(&a).unwrap();
        st.put_server(&b).unwrap();
        st.set_server_order(&[b.id.clone(), a.id.clone()]).unwrap();
        let names: Vec<String> = st.servers().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["b", "a"]);
    }
}
