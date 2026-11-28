//! Local and dynamic (SOCKS5) port forwards over a session.
//!
//! A forward listens on this device and opens a `direct-tcpip` channel per
//! accepted connection; dropping the [`Forward`] stops the listener and every
//! relay it started.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

use crate::{Error, Result, Session};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardSpec {
    /// `bind` here → `host:port` as the server reaches it.
    Local { bind: SocketAddr, host: String, port: u16 },
    /// A SOCKS5 proxy on `bind`, each CONNECT made from the server.
    Dynamic { bind: SocketAddr },
}

pub struct Forward {
    pub local: SocketAddr,
    tasks: JoinSet<()>,
}

impl Forward {
    pub async fn start(session: Arc<Session>, spec: ForwardSpec) -> Result<Forward> {
        let bind = match &spec {
            ForwardSpec::Local { bind, .. } | ForwardSpec::Dynamic { bind } => *bind,
        };
        let listener = TcpListener::bind(bind).await?;
        let local = listener.local_addr()?;
        let mut tasks = JoinSet::new();
        tasks.spawn(async move {
            let mut relays = JoinSet::new();
            loop {
                let Ok((sock, _)) = listener.accept().await else { break };
                let session = session.clone();
                let spec = spec.clone();
                relays.spawn(async move {
                    if let Err(e) = relay(session, spec, sock).await {
                        log::debug!("forward relay ended: {e}");
                    }
                });
                while relays.try_join_next().is_some() {}
            }
        });
        Ok(Forward { local, tasks })
    }

    pub fn stop(&mut self) {
        self.tasks.abort_all();
    }
}

impl Drop for Forward {
    fn drop(&mut self) {
        self.tasks.abort_all();
    }
}

async fn relay(session: Arc<Session>, spec: ForwardSpec, mut sock: TcpStream) -> Result<()> {
    let (host, port) = match spec {
        ForwardSpec::Local { host, port, .. } => (host, port),
        ForwardSpec::Dynamic { .. } => socks5_handshake(&mut sock).await?,
    };
    let mut remote = session.dial(&host, port).await?;
    tokio::io::copy_bidirectional(&mut sock, &mut remote).await?;
    Ok(())
}

/// The no-auth SOCKS5 CONNECT handshake (RFC 1928); answers the target.
async fn socks5_handshake(sock: &mut TcpStream) -> Result<(String, u16)> {
    let bad = || Error::Other("not a SOCKS5 CONNECT".into());
    let mut head = [0u8; 2];
    sock.read_exact(&mut head).await?;
    if head[0] != 5 {
        return Err(bad());
    }
    let mut methods = vec![0u8; head[1] as usize];
    sock.read_exact(&mut methods).await?;
    if !methods.contains(&0) {
        sock.write_all(&[5, 0xff]).await?;
        return Err(bad());
    }
    sock.write_all(&[5, 0]).await?;
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await?;
    if req[0] != 5 || req[1] != 1 {
        sock.write_all(&[5, 7, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
        return Err(bad());
    }
    let host = match req[3] {
        1 => {
            let mut a = [0u8; 4];
            sock.read_exact(&mut a).await?;
            std::net::Ipv4Addr::from(a).to_string()
        }
        3 => {
            let mut n = [0u8; 1];
            sock.read_exact(&mut n).await?;
            let mut name = vec![0u8; n[0] as usize];
            sock.read_exact(&mut name).await?;
            String::from_utf8(name).map_err(|_| bad())?
        }
        4 => {
            let mut a = [0u8; 16];
            sock.read_exact(&mut a).await?;
            std::net::Ipv6Addr::from(a).to_string()
        }
        _ => return Err(bad()),
    };
    let mut p = [0u8; 2];
    sock.read_exact(&mut p).await?;
    sock.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
    Ok((host, u16::from_be_bytes(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn socks5_reads_a_domain_connect() {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let client = tokio::spawn(async move {
            let mut c = TcpStream::connect(addr).await.unwrap();
            c.write_all(&[5, 1, 0]).await.unwrap();
            let mut r = [0u8; 2];
            c.read_exact(&mut r).await.unwrap();
            assert_eq!(r, [5, 0]);
            let mut req = vec![5, 1, 0, 3, 11];
            req.extend_from_slice(b"example.com");
            req.extend_from_slice(&443u16.to_be_bytes());
            c.write_all(&req).await.unwrap();
            let mut ok = [0u8; 10];
            c.read_exact(&mut ok).await.unwrap();
            assert_eq!(ok[1], 0);
        });
        let (mut s, _) = l.accept().await.unwrap();
        assert_eq!(socks5_handshake(&mut s).await.unwrap(), ("example.com".into(), 443));
        client.await.unwrap();
    }
}
