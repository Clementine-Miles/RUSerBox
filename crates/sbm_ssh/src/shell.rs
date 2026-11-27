//! An interactive PTY channel: bytes in, bytes out, resizes, and how it ended.
//!
//! The channel lives in one task; the UI writes through an unbounded queue
//! and reads events without blocking ([`Shell::try_recv`]), with a waker it
//! sets to be told when there is something to read.

use std::sync::{Arc, Mutex};

use russh::ChannelMsg;
use russh::client::Msg;
use tokio::sync::mpsc;

/// What the remote side did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellEvent {
    Data(Vec<u8>),
    /// The program ended with this status (or a signal's name).
    Exit(Option<i32>, Option<String>),
    /// The channel is gone.
    Closed,
}

enum Cmd {
    Data(Vec<u8>),
    Resize(u32, u32),
    Close,
}

type Waker = Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>;

pub struct Shell {
    tx: mpsc::UnboundedSender<Cmd>,
    rx: mpsc::UnboundedReceiver<ShellEvent>,
    waker: Waker,
}

impl Shell {
    pub(crate) fn spawn(mut ch: russh::Channel<Msg>) -> Shell {
        let (tx, mut cmd_rx) = mpsc::unbounded_channel::<Cmd>();
        let (ev_tx, rx) = mpsc::unbounded_channel();
        let waker: Waker = Arc::default();
        let wake = waker.clone();
        let send = move |e: ShellEvent| {
            let _ = ev_tx.send(e);
            if let Some(w) = wake.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                w();
            }
        };
        tokio::spawn(async move {
            let (mut code, mut signal) = (None, None);
            loop {
                tokio::select! {
                    msg = ch.wait() => match msg {
                        Some(ChannelMsg::Data { data }) => send(ShellEvent::Data(data.to_vec())),
                        Some(ChannelMsg::ExtendedData { data, .. }) => send(ShellEvent::Data(data.to_vec())),
                        Some(ChannelMsg::ExitStatus { exit_status }) => code = Some(exit_status as i32),
                        Some(ChannelMsg::ExitSignal { signal_name, .. }) => signal = Some(format!("{signal_name:?}")),
                        Some(ChannelMsg::Close) | None => break,
                        Some(_) => {}
                    },
                    cmd = cmd_rx.recv() => match cmd {
                        Some(Cmd::Data(d)) => { if ch.data_bytes(d).await.is_err() { break; } }
                        Some(Cmd::Resize(c, r)) => { let _ = ch.window_change(c, r, 0, 0).await; }
                        Some(Cmd::Close) | None => { let _ = ch.close().await; break; }
                    },
                }
            }
            if code.is_some() || signal.is_some() {
                send(ShellEvent::Exit(code, signal));
            }
            send(ShellEvent::Closed);
        });
        Shell { tx, rx, waker }
    }

    pub fn write(&self, data: impl Into<Vec<u8>>) {
        let _ = self.tx.send(Cmd::Data(data.into()));
    }

    pub fn resize(&self, cols: u32, rows: u32) {
        let _ = self.tx.send(Cmd::Resize(cols, rows));
    }

    pub fn close(&self) {
        let _ = self.tx.send(Cmd::Close);
    }

    pub fn try_recv(&mut self) -> Option<ShellEvent> {
        self.rx.try_recv().ok()
    }

    pub async fn recv(&mut self) -> Option<ShellEvent> {
        self.rx.recv().await
    }

    /// Called from the channel's task whenever an event is queued.
    pub fn set_waker(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.waker.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Close);
    }
}
