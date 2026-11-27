//! Against the workspace's own Monitor binary, started on loopback in a
//! temporary directory. Skipped (passing) when the binary has not been built
//! (`cargo build -p server_box_monitor`); `SBM_MONITOR_BIN` names another.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use sbm_monitor_client::{Client, Config, Error, TerminalEvent};

struct Agent {
    child: Child,
    port: u16,
    password: String,
    _dir: tempfile::TempDir,
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SBM_MONITOR_BIN") {
        return Some(PathBuf::from(p));
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/server_box_monitor");
    p.exists().then_some(p)
}

fn start() -> Option<Agent> {
    let Some(bin) = binary() else {
        eprintln!("skipped: build server_box_monitor first");
        return None;
    };
    let dir = tempfile::tempdir().unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let child = Command::new(bin)
        .current_dir(dir.path())
        .env("SBM_HOST", "127.0.0.1")
        .env("SBM_PORT", port.to_string())
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let creds = dir.path().join("initial-admin-credentials.txt");
    for _ in 0..200 {
        if let Ok(text) = std::fs::read_to_string(&creds)
            && let Some(pw) = text.lines().find_map(|l| l.strip_prefix("password: "))
            && std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
        {
            return Some(Agent { child, port, password: pw.trim().to_owned(), _dir: dir });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the Monitor did not start");
}

fn client(a: &Agent, password: &str) -> Client {
    Client::new(Config {
        addr: format!("http://127.0.0.1:{}", a.port),
        password: password.into(),
        device_name: "sbm_monitor_client e2e".into(),
        timeout: Duration::from_secs(20),
        ..Default::default()
    })
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn against_the_monitor() {
    let Some(agent) = start() else { return };

    assert!(matches!(client(&agent, "wrong").login().await, Err(Error::LoginFailed)));

    let c = client(&agent, &agent.password);
    c.login().await.unwrap();

    // The first sample takes a collection cycle.
    let mut metrics = None;
    for _ in 0..60 {
        match c.metrics().await {
            Ok(m) => {
                metrics = Some(m);
                break;
            }
            Err(Error::Status { status: 503, .. }) => tokio::time::sleep(Duration::from_millis(500)).await,
            Err(e) => panic!("{e}"),
        }
    }
    let m = metrics.expect("metrics");
    assert!(m.memory.total > 0, "{m:?}");

    let caps = c.capabilities().await.unwrap();
    let features: Vec<String> = serde_json::from_value(caps["features"].clone()).unwrap();
    assert!(!features.iter().any(|f| f == "agent_mode" || f == "mcp"), "no AI features: {features:?}");

    let out = c.exec("echo from-monitor; exit 4", None).await.unwrap();
    assert_eq!(out.stdout, "from-monitor\n");
    assert_eq!(out.exit_code, Some(4));

    // The removed AI endpoints are gone.
    assert!(matches!(c.request(reqwest::Method::GET, "/api/v1/agent/flows", None, &[]).await, Err(Error::NotFound)));

    // A terminal on the agent's machine.
    let mut t = c.terminal(80, 24).await.unwrap();
    t.write("echo term-$((6*7)); exit\n");
    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        match tokio::time::timeout_at(deadline, t.recv()).await.expect("terminal output") {
            Some(TerminalEvent::Data(d)) => seen.push_str(&String::from_utf8_lossy(&d)),
            Some(TerminalEvent::Error(e)) => panic!("terminal: {e}"),
            Some(TerminalEvent::Exit(_)) | Some(TerminalEvent::Closed) | None => break,
            Some(TerminalEvent::Ready { .. }) => {}
        }
        if seen.contains("term-42") {
            break;
        }
    }
    assert!(seen.contains("term-42"), "{seen}");
}
