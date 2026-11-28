//! `GET /api/v1/terminal/ws`: binary frames are PTY bytes both ways, text
//! frames control JSON (`open`, `resize`, `close`; `ready`, `exit`, `error`,
//! `hb`). The ticket rides in `Sec-WebSocket-Protocol` as `sbm-ticket.<t>`.

use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Ready { session: String },
    Data(Vec<u8>),
    Exit(Option<u32>),
    Error(String),
    Closed,
}

enum Cmd {
    Data(Vec<u8>),
    Resize(u16, u16),
    Close,
}

type Waker = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

pub struct MonitorTerminal {
    tx: mpsc::UnboundedSender<Cmd>,
    rx: mpsc::UnboundedReceiver<TerminalEvent>,
    waker: Waker,
}

pub(crate) async fn open(base: &url::Url, ticket: String, tls: Arc<rustls::ClientConfig>, cols: u16, rows: u16) -> Result<MonitorTerminal> {
    let mut url = base.clone();
    url.set_scheme(if base.scheme() == "https" { "wss" } else { "ws" }).map_err(|_| Error::Address(base.to_string()))?;
    url.set_path("/api/v1/terminal/ws");
    let mut req = url.as_str().into_client_request().map_err(|e| Error::Http(e.to_string()))?;
    req.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("sbm-ticket.{ticket}").parse().map_err(|_| Error::Invalid("ticket".into()))?,
    );
    let connector = tokio_tungstenite::Connector::Rustls(tls);
    let (ws, _) = tokio::time::timeout(std::time::Duration::from_secs(15), tokio_tungstenite::connect_async_tls_with_config(req, None, false, Some(connector)))
        .await
        .map_err(|_| Error::Http("the terminal connection timed out".into()))?
        .map_err(|e| Error::Http(e.to_string()))?;
    let (mut sink, mut stream) = ws.split();
    let open = json!({ "type": "open", "auth": { "kind": "local" }, "cols": cols, "rows": rows, "term": "xterm-256color" });
    sink.send(Message::Text(open.to_string().into())).await.map_err(|e| Error::Http(e.to_string()))?;

    let (tx, mut cmd_rx) = mpsc::unbounded_channel();
    let (ev_tx, rx) = mpsc::unbounded_channel();
    let waker: Waker = Arc::default();
    let wake = waker.clone();
    let send = move |e: TerminalEvent| {
        let _ = ev_tx.send(e);
        if let Some(w) = wake.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            w();
        }
    };
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = stream.next() => match msg {
                    Some(Ok(Message::Binary(b))) => send(TerminalEvent::Data(b.to_vec())),
                    Some(Ok(Message::Text(t))) => {
                        let v: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
                        match v.get("type").and_then(Value::as_str) {
                            Some("ready") => send(TerminalEvent::Ready { session: v["session"].as_str().unwrap_or_default().into() }),
                            Some("exit") => send(TerminalEvent::Exit(v["status"].as_u64().map(|s| s as u32))),
                            Some("error") => send(TerminalEvent::Error(v["message"].as_str().unwrap_or("error").into())),
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                },
                cmd = cmd_rx.recv() => {
                    let msg = match cmd {
                        Some(Cmd::Data(d)) => Message::Binary(d.into()),
                        Some(Cmd::Resize(c, r)) => Message::Text(json!({ "type": "resize", "cols": c, "rows": r }).to_string().into()),
                        Some(Cmd::Close) | None => {
                            let _ = sink.send(Message::Text(json!({ "type": "close" }).to_string().into())).await;
                            let _ = sink.close().await;
                            break;
                        }
                    };
                    if sink.send(msg).await.is_err() { break; }
                }
            }
        }
        send(TerminalEvent::Closed);
    });
    Ok(MonitorTerminal { tx, rx, waker })
}

impl MonitorTerminal {
    pub fn write(&self, data: impl Into<Vec<u8>>) {
        let _ = self.tx.send(Cmd::Data(data.into()));
    }
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.tx.send(Cmd::Resize(cols, rows));
    }
    pub fn close(&self) {
        let _ = self.tx.send(Cmd::Close);
    }
    pub fn try_recv(&mut self) -> Option<TerminalEvent> {
        self.rx.try_recv().ok()
    }
    pub async fn recv(&mut self) -> Option<TerminalEvent> {
        self.rx.recv().await
    }
    pub fn set_waker(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.waker.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    }
}

impl Drop for MonitorTerminal {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Close);
    }
}
