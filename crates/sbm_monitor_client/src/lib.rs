//! The app's client for a ServerBox Monitor agent.
//!
//! - **TLS**: an `https` address is verified against the web PKI unless a
//!   certificate was reviewed and pinned (`cert_sha256`), in which case that
//!   leaf and nothing else is accepted. A certificate that fails both is
//!   reported as [`Error::CertUnconfirmed`] with what to review
//!   ([`review_certificate`]); nothing is trusted on first use. Plain `http`
//!   is allowed, as the agent allows it, and the agent itself restricts what
//!   a plaintext link may do.
//! - **Protocol**: `/health` is read before signing in, and an agent this
//!   client cannot talk to is refused (`sbm_parser::protocol::compat`).
//! - **Session**: one sign-in, its token reused; a 401 signs in again once.
//!
//! Every request has a timeout.

mod terminal;

use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

pub use sbm_redfish::cert::CertInfo;
pub use terminal::{MonitorTerminal, TerminalEvent};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the Monitor's certificate is not trusted; review it to continue")]
    CertUnconfirmed,
    #[error("the Monitor's certificate changed since it was reviewed")]
    CertChanged,
    #[error("the password was not accepted")]
    LoginFailed,
    #[error("this Monitor is older than the app supports; update the Monitor")]
    AgentTooOld,
    #[error("this Monitor needs a newer app")]
    ClientTooOld,
    #[error("the Monitor refuses this over the current connection")]
    NotGranted,
    #[error("the Monitor has no such endpoint")]
    NotFound,
    #[error("the Monitor answered {status}: {body}")]
    Status { status: u16, body: String },
    #[error("network: {0}")]
    Http(String),
    #[error("invalid response: {0}")]
    Invalid(String),
    #[error("invalid address: {0}")]
    Address(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to reach one agent.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// `https://host:3770` or `http://…`.
    pub addr: String,
    pub password: String,
    /// The reviewed leaf certificate, SHA-256 lowercase hex.
    pub cert_sha256: Option<String>,
    /// The name this device is listed under on the agent.
    pub device_name: String,
    /// A device token an earlier sign-in returned.
    pub device_token: Option<String>,
    pub timeout: Duration,
}

/// `POST /exec`'s answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ExecOutput {
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub timed_out: bool,
}

/// One entry of `/fs/list`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FsEntry {
    pub name: String,
    /// `file`, `dir`, `link` or `other`.
    pub kind: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub modified: Option<i64>,
    #[serde(default)]
    pub mode: Option<u32>,
    #[serde(default)]
    pub link_target: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// What `/metrics` reports, tolerant of agents older and newer than this
/// build: every field defaults, and unknown ones are ignored.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Metrics {
    pub timestamp: String,
    pub server_name: String,
    pub cpu_usage: f64,
    pub cpu_brand: Option<String>,
    pub memory: Usage,
    pub swap: Usage,
    pub disk: Usage,
    pub network: Network,
    pub temperature: Option<f64>,
    pub temps: Vec<TempReading>,
    pub sys: Option<String>,
    pub os_id: Option<String>,
    pub uptime: Option<String>,
    pub ips: Vec<String>,
    pub gpus: Vec<Gpu>,
    pub ifaces: Vec<Value>,
    pub diskio_rate: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Network {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct TempReading {
    pub device: String,
    pub value: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Gpu {
    pub id: String,
    pub vendor: String,
    pub name: String,
    pub usage_percent: Option<f64>,
    pub temperature: Option<i64>,
    pub memory_used: Option<i64>,
    pub memory_total: Option<i64>,
    pub memory_unit: Option<String>,
}

/// TLS for [addr]: pinned when a pin is given, else the web PKI.
fn tls_config(pin: Option<&str>) -> Result<rustls::ClientConfig> {
    if let Some(p) = pin.filter(|p| !p.trim().is_empty()) {
        return sbm_redfish::cert::pinned_tls_config(sbm_redfish::cert::PinnedCert::new(Some(p))).map_err(|e| Error::Http(format!("TLS setup: {e}")));
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| Error::Http(format!("TLS setup: {e}")))?
        .with_root_certificates(roots)
        .with_no_client_auth())
}

/// Fetches the certificate at an `https` [addr] for a person to review.
pub async fn review_certificate(addr: &str) -> Result<CertInfo> {
    let url = url::Url::parse(addr).map_err(|e| Error::Address(e.to_string()))?;
    let host = url.host_str().ok_or_else(|| Error::Address(addr.into()))?;
    let port = url.port_or_known_default().unwrap_or(443);
    sbm_redfish::cert::fetch_server_cert(host, port, Duration::from_secs(10)).await.map_err(|e| Error::Http(e.to_string()))
}

/// One signed-in client.
pub struct Client {
    http: reqwest::Client,
    base: url::Url,
    cfg: Config,
    token: Mutex<Option<String>>,
    tls: Arc<rustls::ClientConfig>,
}

impl Client {
    pub fn new(cfg: Config) -> Result<Self> {
        let base = url::Url::parse(cfg.addr.trim().trim_end_matches('/')).map_err(|e| Error::Address(e.to_string()))?;
        if !matches!(base.scheme(), "http" | "https") {
            return Err(Error::Address(format!("{} is not http or https", cfg.addr)));
        }
        let tls = Arc::new(tls_config(cfg.cert_sha256.as_deref())?);
        let timeout = if cfg.timeout.is_zero() { Duration::from_secs(15) } else { cfg.timeout };
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured((*tls).clone())
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(timeout)
            .build()
            .map_err(|e| Error::Http(e.to_string()))?;
        Ok(Client { http, base, cfg, token: Mutex::new(None), tls })
    }

    pub fn base(&self) -> &url::Url {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base.as_str().trim_end_matches('/'), path)
    }

    fn map_send(&self, e: reqwest::Error) -> Error {
        let text = format!("{e:?}");
        if text.contains("ApplicationVerificationFailure") {
            return if self.cfg.cert_sha256.is_some() { Error::CertChanged } else { Error::CertUnconfirmed };
        }
        if text.contains("UnknownIssuer") || text.contains("InvalidCertificate") || text.contains("NotValidForName") || text.contains("Expired") {
            return Error::CertUnconfirmed;
        }
        Error::Http(e.to_string())
    }

    async fn send(&self, method: Method, path: &str, body: Option<&Value>, query: &[(&str, String)], token: Option<&str>) -> Result<(StatusCode, Vec<u8>)> {
        let mut req = self.http.request(method, self.url(path)).header(sbm_parser::protocol::HEADER, sbm_parser::protocol::PROTOCOL.to_string());
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| self.map_send(e))?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(|e| Error::Http(e.to_string()))?;
        Ok((status, bytes.to_vec()))
    }

    /// Checks the agent's protocol, then signs in.
    pub async fn login(&self) -> Result<String> {
        let (status, body) = self.send(Method::GET, "/api/v1/health", None, &[], None).await?;
        let health: Value = if status == StatusCode::NOT_FOUND { Value::Null } else { serde_json::from_slice(&body).unwrap_or(Value::Null) };
        let agent = health.get("protocol").and_then(Value::as_u64).map(|n| n as u32);
        let min_client = health.get("min_client_protocol").and_then(Value::as_u64).map(|n| n as u32);
        match sbm_parser::protocol::compat(agent, min_client) {
            sbm_parser::protocol::ProtocolCompat::Ok => {}
            sbm_parser::protocol::ProtocolCompat::AgentTooOld => return Err(Error::AgentTooOld),
            sbm_parser::protocol::ProtocolCompat::ClientTooOld => return Err(Error::ClientTooOld),
        }
        let mut body = serde_json::json!({ "password": self.cfg.password, "name": self.cfg.device_name });
        if let Some(d) = &self.cfg.device_token {
            body["device"] = Value::String(d.clone());
        }
        let (status, resp) = self.send(Method::POST, "/api/v1/login", Some(&body), &[], None).await?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(Error::LoginFailed);
        }
        let v = check(status, &resp)?;
        let token = v.get("token").and_then(Value::as_str).filter(|t| !t.is_empty()).ok_or_else(|| Error::Invalid("no token in the sign-in answer".into()))?.to_owned();
        *self.token.lock().await = Some(token.clone());
        Ok(v.get("device").and_then(Value::as_str).unwrap_or_default().to_owned())
    }

    async fn token(&self) -> Result<String> {
        if let Some(t) = self.token.lock().await.clone() {
            return Ok(t);
        }
        self.login().await?;
        self.token.lock().await.clone().ok_or(Error::LoginFailed)
    }

    /// An authenticated request; a 401 signs in again once.
    pub async fn request(&self, method: Method, path: &str, body: Option<&Value>, query: &[(&str, String)]) -> Result<Vec<u8>> {
        let token = self.token().await?;
        let (status, resp) = self.send(method.clone(), path, body, query, Some(&token)).await?;
        if status == StatusCode::UNAUTHORIZED {
            *self.token.lock().await = None;
            let token = self.token().await?;
            let (status, resp) = self.send(method, path, body, query, Some(&token)).await?;
            return check_bytes(status, resp);
        }
        check_bytes(status, resp)
    }

    async fn json(&self, method: Method, path: &str, body: Option<&Value>, query: &[(&str, String)]) -> Result<Value> {
        let bytes = self.request(method, path, body, query).await?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(|e| Error::Invalid(format!("{path}: {e}")))
    }

    pub async fn metrics(&self) -> Result<Metrics> {
        let v = self.json(Method::GET, "/api/v1/metrics", None, &[]).await?;
        serde_json::from_value(v).map_err(|e| Error::Invalid(e.to_string()))
    }

    /// `/capabilities`: what this agent serves, for this link.
    pub async fn capabilities(&self) -> Result<Value> {
        self.json(Method::GET, "/api/v1/capabilities", None, &[]).await
    }

    /// Runs a shell command as the agent's user; [stdin] is never logged.
    pub async fn exec(&self, cmd: &str, stdin: Option<&str>) -> Result<ExecOutput> {
        let mut body = serde_json::json!({ "cmd": cmd });
        if let Some(s) = stdin {
            body["stdin"] = Value::String(s.into());
        }
        let v = self.json(Method::POST, "/api/v1/exec", Some(&body), &[]).await?;
        serde_json::from_value(v).map_err(|e| Error::Invalid(e.to_string()))
    }

    pub async fn fs_roots(&self) -> Result<Vec<String>> {
        let v = self.json(Method::GET, "/api/v1/fs/roots", None, &[]).await?;
        let list = v.get("roots").cloned().unwrap_or(v);
        serde_json::from_value(list).map_err(|e| Error::Invalid(e.to_string()))
    }

    pub async fn fs_list(&self, path: &str) -> Result<Vec<FsEntry>> {
        let v = self.json(Method::GET, "/api/v1/fs/list", None, &[("path", path.into())]).await?;
        let list = v.get("entries").cloned().unwrap_or(v);
        serde_json::from_value(list).map_err(|e| Error::Invalid(e.to_string()))
    }

    pub async fn fs_read(&self, path: &str) -> Result<Vec<u8>> {
        self.request(Method::GET, "/api/v1/fs/read", None, &[("path", path.into())]).await
    }

    /// Writes [bytes] to [path]; answers the file's new version.
    pub async fn fs_write(&self, path: &str, bytes: Vec<u8>) -> Result<Option<String>> {
        let token = self.token().await?;
        let resp = self
            .http
            .put(self.url("/api/v1/fs/write"))
            .header(sbm_parser::protocol::HEADER, sbm_parser::protocol::PROTOCOL.to_string())
            .bearer_auth(token)
            .query(&[("path", path)])
            .body(bytes)
            .send()
            .await
            .map_err(|e| self.map_send(e))?;
        let status = resp.status();
        let body = resp.bytes().await.map_err(|e| Error::Http(e.to_string()))?;
        let v = check(status, &body)?;
        Ok(v.get("version").and_then(Value::as_str).map(str::to_owned))
    }

    pub async fn fs_mkdir(&self, path: &str) -> Result<()> {
        self.json(Method::POST, "/api/v1/fs/mkdir", Some(&serde_json::json!({ "path": path })), &[]).await.map(drop)
    }

    pub async fn fs_remove(&self, path: &str, recursive: bool) -> Result<()> {
        self.json(Method::POST, "/api/v1/fs/remove", Some(&serde_json::json!({ "path": path, "recursive": recursive })), &[]).await.map(drop)
    }

    pub async fn fs_rename(&self, from: &str, to: &str) -> Result<()> {
        self.json(Method::POST, "/api/v1/fs/rename", Some(&serde_json::json!({ "from": from, "to": to })), &[]).await.map(drop)
    }

    /// A read-only token for [client_id] (the home-screen widget's): the
    /// token and when it expires, in Unix milliseconds (0 when unsaid).
    pub async fn issue_watch_token(&self, client_id: &str) -> Result<(String, i64)> {
        let v = self.json(Method::POST, "/api/v1/watch-token", Some(&serde_json::json!({ "client_id": client_id })), &[]).await?;
        let token = v.get("token").and_then(Value::as_str).filter(|t| !t.is_empty()).ok_or_else(|| Error::Invalid("no watch token".into()))?;
        Ok((token.to_owned(), v.get("expires_at").and_then(Value::as_i64).unwrap_or(0)))
    }

    /// A one-use ticket for a WebSocket (`terminal`, `stream`, `listen`).
    pub async fn ws_ticket(&self, purpose: &str) -> Result<String> {
        let v = self.json(Method::POST, "/api/v1/ws-ticket", Some(&serde_json::json!({ "purpose": purpose })), &[]).await?;
        v.get("ticket").and_then(Value::as_str).map(str::to_owned).ok_or_else(|| Error::Invalid("no ticket".into()))
    }

    /// A shell on the agent's machine, as the agent's user.
    pub async fn terminal(&self, cols: u16, rows: u16) -> Result<MonitorTerminal> {
        let ticket = self.ws_ticket("terminal").await?;
        terminal::open(&self.base, ticket, self.tls.clone(), cols, rows).await
    }
}

fn check(status: StatusCode, body: &[u8]) -> Result<Value> {
    let bytes = check_bytes(status, body.to_vec())?;
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|e| Error::Invalid(e.to_string()))
}

fn check_bytes(status: StatusCode, body: Vec<u8>) -> Result<Vec<u8>> {
    match status.as_u16() {
        200..=299 => Ok(body),
        403 => Err(Error::NotGranted),
        404 => Err(Error::NotFound),
        s => Err(Error::Status { status: s, body: String::from_utf8_lossy(&body).chars().take(300).collect() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_from_an_older_agent_still_decode() {
        let m: Metrics = serde_json::from_str(r#"{"cpu_usage": 12.5, "memory": {"total": 100, "used": 40}, "new_field": [1,2]}"#).unwrap();
        assert_eq!(m.cpu_usage, 12.5);
        assert_eq!(m.memory.used, 40);
        assert!(m.gpus.is_empty());
    }

    #[test]
    fn only_http_and_https_addresses() {
        assert!(Client::new(Config { addr: "ftp://x".into(), ..Default::default() }).is_err());
        assert!(Client::new(Config { addr: "https://x:3770/".into(), ..Default::default() }).is_ok());
    }
}
