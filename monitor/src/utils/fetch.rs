use std::{future::Future, pin::Pin, time::Duration};

use futures::StreamExt;

/// How the agent reads an HTTPS address: at most [max] bytes, or why not.
pub trait Fetch: Send + Sync {
    fn get<'a>(&'a self, url: &'a str, max: usize) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>>;
}

/// HTTPS only, credentials refused, at most three redirects, each to HTTPS
/// again.
pub struct HttpFetch {
    client: reqwest::Client,
}

impl Default for HttpFetch {
    fn default() -> Self {
        // reqwest has no crypto provider of its own: see `monitoring::push`.
        let _ = rustls::crypto::ring::default_provider().install_default();
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(90))
                // GitHub's API refuses a request without one (`api::update`).
                .user_agent(concat!("server-box-monitor/", env!("CARGO_PKG_VERSION")))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("valid HTTP client configuration"),
        }
    }
}

impl Fetch for HttpFetch {
    fn get<'a>(&'a self, url: &'a str, max: usize) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut url = https_url(url)?;
            for _ in 0..4 {
                let res = self.client.get(url.clone()).send().await.map_err(|e| e.to_string())?;
                let status = res.status();
                if status.is_redirection() {
                    let location = res
                        .headers()
                        .get(reqwest::header::LOCATION)
                        .and_then(|v| v.to_str().ok())
                        .ok_or("Missing redirect URL")?;
                    let next = url.join(location).map_err(|e| e.to_string())?;
                    url = https_url(next.as_str())?;
                    continue;
                }
                if !status.is_success() {
                    return Err(format!("HTTP {}", status.as_u16()));
                }
                if res.content_length().is_some_and(|n| n as usize > max) {
                    return Err("Download exceeds size limit".into());
                }
                let mut bytes = Vec::new();
                let mut stream = res.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(|e| e.to_string())?;
                    if bytes.len() + chunk.len() > max {
                        return Err("Download exceeds size limit".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                return Ok(bytes);
            }
            Err("Too many redirects".into())
        })
    }
}


fn https_url(value: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(value).map_err(|e| e.to_string())?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Expected an HTTPS URL without credentials".into());
    }
    Ok(url)
}
