//! Against a real OpenSSH server on loopback, started here with a throwaway
//! host key and user key. Skipped (passing) where `/usr/sbin/sshd` is absent.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sbm_ssh::{Auth, BoxFuture, Error, Forward, ForwardSpec, Hop, HostKeyPrompt, HostTrust, Options, Session, ShellEvent, StatusRunner};

const SSHD: &str = "/usr/sbin/sshd";

struct Sshd {
    child: Child,
    port: u16,
    key: String,
    _dir: tempfile::TempDir,
}

impl Drop for Sshd {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn keygen(path: &Path) {
    let ok = Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-f"]).arg(path).status().unwrap().success();
    assert!(ok);
}

fn start() -> Option<Sshd> {
    if !Path::new(SSHD).exists() {
        eprintln!("skipped: no sshd");
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    keygen(&d.join("host"));
    keygen(&d.join("user"));
    std::fs::copy(d.join("user.pub"), d.join("authorized_keys")).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let cfg = format!(
        "Port {port}\nListenAddress 127.0.0.1\nHostKey {h}\nPidFile {pid}\nAuthorizedKeysFile {ak}\nStrictModes no\n\
         PermitRootLogin yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\n\
         Subsystem sftp internal-sftp\nAllowTcpForwarding yes\nLogLevel ERROR\n",
        h = d.join("host").display(),
        pid = d.join("pid").display(),
        ak = d.join("authorized_keys").display(),
    );
    std::fs::write(d.join("sshd_config"), cfg).unwrap();
    // The privilege separation directory, needed only by an sshd run as root.
    std::fs::create_dir_all("/run/sshd").ok();
    let mut child = Command::new(SSHD).arg("-D").arg("-e").arg("-f").arg(d.join("sshd_config")).spawn().unwrap();
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            let key = std::fs::read_to_string(d.join("user")).unwrap();
            return Some(Sshd { child, port, key, _dir: dir });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("sshd did not start");
}

/// The account the test runs as: a non-root sshd can only let in itself.
fn me() -> String {
    Command::new("id").arg("-un").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()).unwrap_or_else(|_| "root".into())
}

#[derive(Default)]
struct Memory {
    known: Mutex<HashMap<(String, String), String>>,
    asked: Mutex<Vec<HostKeyPrompt>>,
    accept: Mutex<bool>,
}

impl HostTrust for Memory {
    fn known(&self, id: &str, t: &str) -> Option<String> {
        self.known.lock().unwrap().get(&(id.into(), t.into())).cloned()
    }
    fn ask(&self, p: HostKeyPrompt) -> BoxFuture<'static, bool> {
        self.asked.lock().unwrap().push(p);
        let yes = *self.accept.lock().unwrap();
        Box::pin(async move { yes })
    }
    fn remember(&self, id: &str, t: &str, fp: &str) {
        self.known.lock().unwrap().insert((id.into(), t.into()), fp.into());
    }
}

fn hop(s: &Sshd, id: &str) -> Hop {
    Hop {
        id: id.into(),
        host: "127.0.0.1".into(),
        port: s.port,
        user: me(),
        auth: Auth { private_key: Some(s.key.clone()), ..Default::default() },
        legacy_algorithms: false,
        proxy_command: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn against_openssh() {
    let Some(sshd) = start() else { return };
    let trust = Arc::new(Memory::default());
    let opts = Options::default();

    // An unknown key, refused: no connection and nothing remembered.
    let r = Session::connect(&[hop(&sshd, "a")], trust.clone(), opts).await;
    assert!(matches!(r, Err(Error::HostKeyRejected { .. })), "{:?}", r.err());
    assert_eq!(trust.asked.lock().unwrap().len(), 1);
    assert!(trust.asked.lock().unwrap()[0].previous.is_none());
    assert!(trust.known.lock().unwrap().is_empty());

    // Accepted once, remembered, not asked again.
    *trust.accept.lock().unwrap() = true;
    let s = Session::connect(&[hop(&sshd, "a")], trust.clone(), opts).await.unwrap();
    drop(s);
    let s = Arc::new(Session::connect(&[hop(&sshd, "a")], trust.clone(), opts).await.unwrap());
    assert_eq!(trust.asked.lock().unwrap().len(), 2);

    // exec
    let out = s.exec("echo hello; echo oops >&2; exit 3", None, Duration::from_secs(10)).await.unwrap();
    assert_eq!(out.stdout_str(), "hello\n");
    assert_eq!(out.stderr_str(), "oops\n");
    assert_eq!(out.exit_code, Some(3));
    let out = s.exec("cat", Some(b"from stdin"), Duration::from_secs(10)).await.unwrap();
    assert_eq!(out.stdout_str(), "from stdin");

    // A PTY shell.
    let mut sh = s.shell("xterm-256color", 80, 24, None).await.unwrap();
    sh.resize(100, 30);
    sh.write("stty size; echo marker-$((40+2)); exit\n");
    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let ev = tokio::time::timeout_at(deadline, sh.recv()).await.expect("shell output").expect("shell event");
        match ev {
            ShellEvent::Data(d) => seen.push_str(&String::from_utf8_lossy(&d)),
            ShellEvent::Exit(..) => {}
            ShellEvent::Closed => break,
        }
    }
    assert!(seen.contains("marker-42"), "{seen}");
    assert!(seen.contains("30 100"), "the PTY follows a resize: {seen}");

    // SFTP
    let sftp = s.sftp().await.unwrap();
    let path = format!("/tmp/sbm_ssh_e2e_{}", sshd.port);
    sftp.write(&path, b"sftp bytes").await.unwrap();
    assert_eq!(sftp.read(&path).await.unwrap(), b"sftp bytes");
    let names: Vec<String> = sftp.read_dir("/tmp").await.unwrap().map(|e| e.file_name()).collect();
    assert!(names.iter().any(|n| path.ends_with(n.as_str())));
    sftp.remove_file(&path).await.unwrap();

    // Status through the shared script.
    let system = StatusRunner::detect(&s).await.unwrap();
    assert_eq!(system, sbm_parser::SystemType::Linux);
    let mut runner = StatusRunner::new(system, Some(format!("/tmp/sbm_ssh_e2e_script_{}", sshd.port)), vec![]);
    let st = runner.status(&s, 1000.0).await.unwrap();
    assert!(!st.cpu.is_empty(), "cpu readings");
    assert!(st.mem.is_some(), "memory reading");

    // A local forward to the server's own sshd answers with its banner.
    let fwd = Forward::start(s.clone(), ForwardSpec::Local { bind: "127.0.0.1:0".parse().unwrap(), host: "127.0.0.1".into(), port: sshd.port }).await.unwrap();
    let mut c = tokio::net::TcpStream::connect(fwd.local).await.unwrap();
    let mut banner = [0u8; 4];
    tokio::io::AsyncReadExt::read_exact(&mut c, &mut banner).await.unwrap();
    assert_eq!(&banner, b"SSH-");
    drop(fwd);

    // Through a jump server (the same sshd under another id: its key is
    // asked about once for that id too).
    let via = Session::connect(&[hop(&sshd, "jump"), hop(&sshd, "b")], trust.clone(), opts).await.unwrap();
    assert_eq!(via.exec("echo via", None, Duration::from_secs(10)).await.unwrap().stdout_str(), "via\n");
    assert_eq!(trust.asked.lock().unwrap().len(), 4);

    // A changed key is asked about as a mismatch, and refused when declined.
    trust.known.lock().unwrap().insert(("a".into(), "ssh-ed25519".into()), "SHA256:not-this-one".into());
    *trust.accept.lock().unwrap() = false;
    let r = Session::connect(&[hop(&sshd, "a")], trust.clone(), opts).await;
    assert!(matches!(r, Err(Error::HostKeyRejected { .. })));
    let last = trust.asked.lock().unwrap().last().cloned().unwrap();
    assert_eq!(last.previous.as_deref(), Some("SHA256:not-this-one"));
    assert_eq!(trust.known("a", "ssh-ed25519").as_deref(), Some("SHA256:not-this-one"), "a refusal records nothing");

    // A wrong key is an authentication failure.
    let mut bad = hop(&sshd, "b");
    let other: PathBuf = sshd._dir.path().join("other");
    keygen(&other);
    bad.auth.private_key = Some(std::fs::read_to_string(other).unwrap());
    assert!(matches!(Session::connect(&[bad], trust.clone(), opts).await, Err(Error::AuthFailed { .. })));
}
