//! The app's SSH client, on russh.
//!
//! - **Host keys are always verified.** A key never seen for a server is
//!   shown to the person ([`HostTrust::ask`]) and remembered only once they
//!   accept it; a key that differs from the remembered one is shown as a
//!   mismatch and refused unless they accept that too. Nothing here accepts a
//!   key on its own.
//! - A connection is a chain: direct, through jump servers (each hop a
//!   `direct-tcpip` channel of the one before), or through a `ProxyCommand`.
//! - Every network wait has a timeout, and every task a [`Session`] spawns
//!   ends when the session is dropped.

mod forward;
mod shell;
mod status;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, AuthResult, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{ChannelMsg, Preferred, kex};
use tokio::io::{AsyncRead, AsyncWrite};

pub use forward::{Forward, ForwardSpec};
pub use russh::keys;
pub use russh_sftp::client::SftpSession;
pub use shell::{Shell, ShellEvent};
pub use status::{SCRIPT_VERSION, ScriptPaths, StatusRunner};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("connecting to {0} timed out")]
    Timeout(String),
    #[error("network: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH: {0}")]
    Ssh(#[from] russh::Error),
    #[error("the host key of {host} was not trusted")]
    HostKeyRejected { host: String },
    #[error("authentication as {user}@{host} failed")]
    AuthFailed { user: String, host: String },
    #[error("the private key could not be read: {0}")]
    Key(String),
    #[error("SFTP: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    #[error("the channel closed before the command finished")]
    ChannelClosed,
    #[error("{0}")]
    Other(String),
}

impl From<russh::keys::Error> for Error {
    fn from(e: russh::keys::Error) -> Self {
        Error::Key(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// What the person is asked about a host key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyPrompt {
    pub host_id: String,
    pub host: String,
    pub key_type: String,
    /// `SHA256:<base64 without padding>`.
    pub fingerprint: String,
    /// The fingerprint remembered before, when this one differs from it.
    pub previous: Option<String>,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Where trusted host keys live, and who decides about a new one.
pub trait HostTrust: Send + Sync + 'static {
    /// The fingerprint remembered for [host_id] and [key_type].
    fn known(&self, host_id: &str, key_type: &str) -> Option<String>;
    /// Asks the person; `true` only on an explicit accept.
    fn ask(&self, prompt: HostKeyPrompt) -> BoxFuture<'static, bool>;
    /// Records an accepted key. Never called for a refusal.
    fn remember(&self, host_id: &str, key_type: &str, fingerprint: &str);
}

/// How to authenticate one hop.
#[derive(Clone, Default)]
pub struct Auth {
    pub password: Option<String>,
    /// OpenSSH, PEM or PuTTY private key text.
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auth")
            .field("password", &self.password.as_ref().map(|_| "…"))
            .field("private_key", &self.private_key.as_ref().map(|_| "…"))
            .finish()
    }
}

/// One hop of a connection.
#[derive(Debug, Clone)]
pub struct Hop {
    /// The server's id, which host keys are remembered under.
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    /// Allows SHA-1 key exchange and `ssh-rsa` signatures, for old servers
    /// only; off by default.
    pub legacy_algorithms: bool,
    /// `ProxyCommand` for the first hop (`%h`, `%p`, `%r` expanded).
    pub proxy_command: Option<String>,
}

/// Connection options shared by every hop.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub connect_timeout: Duration,
    pub keepalive: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Options { connect_timeout: Duration::from_secs(10), keepalive: Some(Duration::from_secs(30)) }
    }
}

/// OpenSSH's fingerprint of a public key.
pub fn fingerprint(key: &russh::keys::PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// The decision for a key: trusted, to ask, or a mismatch to ask about.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Trusted,
    Unknown,
    Mismatch { previous: String },
}

/// Compares [fingerprint] with what is remembered.
pub fn verdict(known: Option<&str>, fingerprint: &str) -> Verdict {
    match known {
        Some(k) if k == fingerprint => Verdict::Trusted,
        Some(k) => Verdict::Mismatch { previous: k.to_owned() },
        None => Verdict::Unknown,
    }
}

struct Client {
    hop_id: String,
    host: String,
    trust: Arc<dyn HostTrust>,
}

impl client::Handler for Client {
    type Error = Error;

    async fn check_server_key(&mut self, key: &PublicKeyOrCertificate) -> Result<bool> {
        let key = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            // Certificates are not advertised (`host_key_certificates` is
            // empty), so a server never presents one; refuse if it does.
            PublicKeyOrCertificate::Certificate(_) => return Ok(false),
        };
        let key_type = key.algorithm().as_str().to_owned();
        let fp = fingerprint(&key);
        let known = self.trust.known(&self.hop_id, &key_type);
        let previous = match verdict(known.as_deref(), &fp) {
            Verdict::Trusted => return Ok(true),
            Verdict::Unknown => None,
            Verdict::Mismatch { previous } => Some(previous),
        };
        let prompt = HostKeyPrompt { host_id: self.hop_id.clone(), host: self.host.clone(), key_type: key_type.clone(), fingerprint: fp.clone(), previous };
        if self.trust.ask(prompt).await {
            self.trust.remember(&self.hop_id, &key_type, &fp);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn config(hop: &Hop, opts: &Options) -> Arc<client::Config> {
    let mut preferred = Preferred::default();
    if hop.legacy_algorithms {
        let mut kexes = preferred.kex.to_vec();
        kexes.extend([kex::DH_G14_SHA1, kex::DH_GEX_SHA1, kex::DH_G1_SHA1]);
        preferred.kex = kexes.into();
    }
    Arc::new(client::Config {
        preferred,
        inactivity_timeout: None,
        keepalive_interval: opts.keepalive,
        keepalive_max: 3,
        nodelay: true,
        ..Default::default()
    })
}

/// An authenticated connection, holding the hops it goes through.
pub struct Session {
    handle: Handle<Client>,
    /// Jump connections this one rides on, kept alive with it.
    _via: Vec<Handle<Client>>,
    /// The ProxyCommand child, killed on drop.
    _proxy: Option<tokio::process::Child>,
    pub host: String,
    pub user: String,
}

type BoxStream = Box<dyn AsyncReadWrite>;
trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncReadWrite for T {}

/// Expands `%h`, `%p`, `%r` and `%%` in a ProxyCommand.
pub fn expand_proxy_command(cmd: &str, hop: &Hop) -> String {
    let mut out = String::new();
    let mut it = cmd.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('h') => out.push_str(&hop.host),
            Some('p') => out.push_str(&hop.port.to_string()),
            Some('r') => out.push_str(&hop.user),
            Some('%') => out.push('%'),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}

async fn spawn_proxy(cmd: &str, hop: &Hop) -> Result<(BoxStream, tokio::process::Child)> {
    let shell = if std::path::Path::new("/system/bin/sh").exists() { "/system/bin/sh" } else { "/bin/sh" };
    let mut child = tokio::process::Command::new(shell)
        .arg("-c")
        .arg(expand_proxy_command(cmd, hop))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdin = child.stdin.take().ok_or_else(|| Error::Other("ProxyCommand has no stdin".into()))?;
    let stdout = child.stdout.take().ok_or_else(|| Error::Other("ProxyCommand has no stdout".into()))?;
    Ok((Box::new(tokio::io::join(stdout, stdin)), child))
}

async fn handshake(hop: &Hop, stream: BoxStream, trust: &Arc<dyn HostTrust>, opts: &Options) -> Result<Handle<Client>> {
    let handler = Client { hop_id: hop.id.clone(), host: format!("{}:{}", hop.host, hop.port), trust: trust.clone() };
    let fut = client::connect_stream(config(hop, opts), stream, handler);
    let mut handle = match tokio::time::timeout(opts.connect_timeout, fut).await {
        Err(_) => return Err(Error::Timeout(hop.host.clone())),
        Ok(Err(Error::Ssh(russh::Error::UnknownKey))) => return Err(Error::HostKeyRejected { host: hop.host.clone() }),
        Ok(r) => r?,
    };
    authenticate(&mut handle, hop, opts).await?;
    Ok(handle)
}

async fn authenticate(handle: &mut Handle<Client>, hop: &Hop, opts: &Options) -> Result<()> {
    let failed = || Error::AuthFailed { user: hop.user.clone(), host: hop.host.clone() };
    let t = opts.connect_timeout;
    let wait = |e: tokio::time::error::Elapsed| -> Error {
        let _ = e;
        Error::Timeout(hop.host.clone())
    };
    if let Some(text) = &hop.auth.private_key {
        let key = russh::keys::decode_secret_key(text, hop.auth.passphrase.as_deref())?;
        let hash = tokio::time::timeout(t, handle.best_supported_rsa_hash()).await.map_err(wait)??.flatten();
        let res = tokio::time::timeout(t, handle.authenticate_publickey(&hop.user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))).await.map_err(wait)??;
        if matches!(res, AuthResult::Success) {
            return Ok(());
        }
    }
    if let Some(pw) = &hop.auth.password {
        let res = tokio::time::timeout(t, handle.authenticate_password(&hop.user, pw)).await.map_err(wait)??;
        if matches!(res, AuthResult::Success) {
            return Ok(());
        }
        // Servers that only offer keyboard-interactive: answer every
        // password-like prompt with the password, as the Flutter app did.
        let mut res = tokio::time::timeout(t, handle.authenticate_keyboard_interactive_start(&hop.user, None)).await.map_err(wait)??;
        for _ in 0..4 {
            match res {
                KeyboardInteractiveAuthResponse::Success => return Ok(()),
                KeyboardInteractiveAuthResponse::Failure { .. } => break,
                KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                    let answers = prompts.iter().map(|_| pw.clone()).collect();
                    res = tokio::time::timeout(t, handle.authenticate_keyboard_interactive_respond(answers)).await.map_err(wait)??;
                }
            }
        }
    }
    if hop.auth.private_key.is_none() && hop.auth.password.is_none() {
        let res = tokio::time::timeout(t, handle.authenticate_none(&hop.user)).await.map_err(wait)??;
        if matches!(res, AuthResult::Success) {
            return Ok(());
        }
    }
    Err(failed())
}

impl Session {
    /// Connects through [chain]: the last hop is the server, the ones before
    /// it jump servers, nearest first.
    pub async fn connect(chain: &[Hop], trust: Arc<dyn HostTrust>, opts: Options) -> Result<Session> {
        let (target, jumps) = chain.split_last().ok_or_else(|| Error::Other("nothing to connect to".into()))?;
        let first = jumps.first().unwrap_or(target);
        let (stream, proxy): (BoxStream, _) = match &first.proxy_command {
            Some(cmd) if !cmd.trim().is_empty() => {
                let (s, c) = spawn_proxy(cmd, first).await?;
                (s, Some(c))
            }
            _ => {
                let addr = (first.host.as_str(), first.port);
                let tcp = tokio::time::timeout(opts.connect_timeout, tokio::net::TcpStream::connect(addr))
                    .await
                    .map_err(|_| Error::Timeout(first.host.clone()))??;
                tcp.set_nodelay(true)?;
                (Box::new(tcp), None)
            }
        };
        let mut handle = handshake(first, stream, &trust, &opts).await?;
        let mut via = Vec::new();
        for hop in jumps.iter().skip(1).chain(if jumps.is_empty() { None } else { Some(target) }) {
            let ch = tokio::time::timeout(opts.connect_timeout, handle.channel_open_direct_tcpip(hop.host.clone(), u32::from(hop.port), "127.0.0.1", 0))
                .await
                .map_err(|_| Error::Timeout(hop.host.clone()))??;
            let next = handshake(hop, Box::new(ch.into_stream()), &trust, &opts).await?;
            via.push(std::mem::replace(&mut handle, next));
        }
        Ok(Session { handle, _via: via, _proxy: proxy, host: target.host.clone(), user: target.user.clone() })
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    /// Runs [command] and collects its output, with [stdin] written first.
    pub async fn exec(&self, command: &str, stdin: Option<&[u8]>, timeout: Duration) -> Result<ExecOutput> {
        let run = async {
            let mut ch = self.handle.channel_open_session().await?;
            ch.exec(true, command).await?;
            if let Some(input) = stdin {
                ch.data_bytes(input.to_vec()).await?;
            }
            ch.eof().await?;
            let mut out = ExecOutput::default();
            while let Some(msg) = ch.wait().await {
                match msg {
                    ChannelMsg::Data { data } => out.stdout.extend_from_slice(&data),
                    ChannelMsg::ExtendedData { data, ext: 1 } => out.stderr.extend_from_slice(&data),
                    ChannelMsg::ExitStatus { exit_status } => out.exit_code = Some(exit_status as i32),
                    ChannelMsg::ExitSignal { signal_name, .. } => out.signal = Some(format!("{signal_name:?}")),
                    ChannelMsg::Close => break,
                    _ => {}
                }
            }
            Ok::<_, Error>(out)
        };
        tokio::time::timeout(timeout, run).await.map_err(|_| Error::Timeout(format!("{}: {}", self.host, first_line(command))))?
    }

    /// An interactive shell on a PTY of [cols]×[rows].
    pub async fn shell(&self, term: &str, cols: u32, rows: u32, command: Option<&str>) -> Result<Shell> {
        let ch = self.handle.channel_open_session().await?;
        ch.request_pty(true, term, cols, rows, 0, 0, &[]).await?;
        match command {
            Some(c) => ch.exec(true, c).await?,
            None => ch.request_shell(true).await?,
        }
        Ok(Shell::spawn(ch))
    }

    /// The SFTP subsystem.
    pub async fn sftp(&self) -> Result<SftpSession> {
        let ch = self.handle.channel_open_session().await?;
        ch.request_subsystem(true, "sftp").await?;
        Ok(SftpSession::new(ch.into_stream()).await?)
    }

    /// A raw `direct-tcpip` stream to [host]:[port] as seen from the server.
    pub async fn dial(&self, host: &str, port: u16) -> Result<impl AsyncRead + AsyncWrite + Unpin + Send + 'static> {
        let ch = self.handle.channel_open_direct_tcpip(host.to_owned(), u32::from(port), "127.0.0.1", 0).await?;
        Ok(ch.into_stream())
    }

    pub async fn disconnect(&self) {
        let _ = self.handle.disconnect(russh::Disconnect::ByApplication, "", "en").await;
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

/// What a command printed and how it ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

impl ExecOutput {
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
    pub fn ok(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_key_is_a_mismatch_never_trusted() {
        assert_eq!(verdict(Some("SHA256:a"), "SHA256:a"), Verdict::Trusted);
        assert_eq!(verdict(None, "SHA256:a"), Verdict::Unknown);
        assert_eq!(verdict(Some("SHA256:a"), "SHA256:b"), Verdict::Mismatch { previous: "SHA256:a".into() });
    }

    #[test]
    fn proxy_commands_expand_like_openssh() {
        let hop = Hop { id: "x".into(), host: "db.internal".into(), port: 2222, user: "ops".into(), auth: Auth::default(), legacy_algorithms: false, proxy_command: None };
        assert_eq!(expand_proxy_command("nc -X 5 -x proxy:1080 %h %p # %r %% %z", &hop), "nc -X 5 -x proxy:1080 db.internal 2222 # ops % %z");
    }

    #[test]
    fn fingerprints_are_openssh_sha256() {
        let key = russh::keys::PublicKey::from_openssh("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOmS1C2EYmKXfumVNoeqyRSIwZEiTUlQtzRi9TkdZ+UY test").unwrap();
        let fp = fingerprint(&key);
        assert!(fp.starts_with("SHA256:"), "{fp}");
        assert!(!fp.ends_with('='), "no padding: {fp}");
        assert_eq!(key.algorithm().as_str(), "ssh-ed25519");
    }
}
