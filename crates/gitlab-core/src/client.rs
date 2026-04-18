use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

use crate::error::{GitlabError, Result};
use crate::request::RequestSpec;
use crate::retry::RetryPolicy;
use crate::throttle::Throttle;

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub host: String,
    pub token: String,
    pub tls_skip_verify: bool,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub user_agent: String,
    pub retry: RetryPolicy,
    pub throttle: Throttle,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            host: String::new(),
            token: String::new(),
            tls_skip_verify: false,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            user_agent: format!("gitlab-cli/{} (+rust)", env!("CARGO_PKG_VERSION")),
            retry: RetryPolicy::default(),
            throttle: Throttle::disabled(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    token: String,
    retry: RetryPolicy,
    throttle: Throttle,
}

impl Client {
    pub fn new(opts: ClientOptions) -> Result<Self> {
        if opts.host.is_empty() {
            return Err(GitlabError::InvalidArgs("host is empty".into()));
        }
        let mut host = opts.host.clone();
        if !host.ends_with('/') {
            host.push('/');
        }
        let host_url = Url::parse(&host)
            .map_err(|e| GitlabError::InvalidArgs(format!("invalid host {host}: {e}")))?;
        if host_url.scheme() != "http" && host_url.scheme() != "https" {
            return Err(GitlabError::InvalidArgs(
                "host must start with http:// or https://".into(),
            ));
        }
        let base_url = host_url
            .join("api/v4/")
            .map_err(|e| GitlabError::InvalidArgs(format!("bad base url: {e}")))?;

        let http = reqwest::Client::builder()
            .user_agent(&opts.user_agent)
            .connect_timeout(opts.connect_timeout)
            .timeout(opts.request_timeout)
            .gzip(true)
            .danger_accept_invalid_certs(opts.tls_skip_verify)
            .build()
            .map_err(|e| GitlabError::Network(format!("reqwest build: {e}")))?;

        Ok(Self {
            http,
            base_url,
            token: opts.token,
            retry: opts.retry,
            throttle: opts.throttle,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
    #[must_use]
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }
    #[must_use]
    pub fn retry(&self) -> &RetryPolicy {
        &self.retry
    }
    #[must_use]
    pub fn throttle(&self) -> &Throttle {
        &self.throttle
    }

    pub fn endpoint(&self, path: &str) -> Result<Url> {
        let trimmed = path.trim_start_matches('/');
        self.base_url
            .join(trimmed)
            .map_err(|e| GitlabError::InvalidArgs(format!("bad endpoint {path}: {e}")))
    }

    #[allow(clippy::redundant_closure_for_method_calls)]
    pub async fn send_raw(&self, spec: RequestSpec) -> Result<(u16, HeaderMap, bytes::Bytes)> {
        let plan_net = self.retry.plan_for_network();
        let mut attempt_429: usize = 0;
        let mut attempt_net: usize = 0;

        loop {
            self.throttle.acquire().await;
            let mut url = self.endpoint(&spec.path)?;
            if !spec.query.is_empty() {
                let mut q = url.query_pairs_mut();
                for (k, v) in &spec.query {
                    q.append_pair(k, v);
                }
            }
            let mut req = self
                .http
                .request(spec.method.clone(), url)
                .header("PRIVATE-TOKEN", &self.token);
            if let Some(body) = &spec.body {
                req = req.json(body);
            }
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let headers = resp.headers().clone();
                    let bytes = resp
                        .bytes()
                        .await
                        .map_err(|e| GitlabError::Network(e.to_string()))?;
                    match status {
                        200..=299 => return Ok((status, headers, bytes)),
                        429 => {
                            let retry_after = headers
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .map(|s| s.to_owned());
                            if let Some(d) = self
                                .retry
                                .next_delay_for_429(retry_after.as_deref(), attempt_429)
                            {
                                attempt_429 += 1;
                                if attempt_429 > self.retry.max_attempts_429 as usize {
                                    return Err(GitlabError::from_status(
                                        status,
                                        String::from_utf8_lossy(&bytes).into_owned(),
                                        extract_request_id(&headers),
                                    ));
                                }
                                tokio::time::sleep(d).await;
                                continue;
                            }
                            return Err(GitlabError::from_status(
                                status,
                                String::from_utf8_lossy(&bytes).into_owned(),
                                extract_request_id(&headers),
                            ));
                        }
                        500..=599 => {
                            if let Some(d) = plan_net.attempts.get(attempt_net).copied() {
                                attempt_net += 1;
                                tokio::time::sleep(d).await;
                                continue;
                            }
                            return Err(GitlabError::from_status(
                                status,
                                String::from_utf8_lossy(&bytes).into_owned(),
                                extract_request_id(&headers),
                            ));
                        }
                        _ => {
                            return Err(GitlabError::from_status(
                                status,
                                String::from_utf8_lossy(&bytes).into_owned(),
                                extract_request_id(&headers),
                            ))
                        }
                    }
                }
                Err(e) if e.is_timeout() => {
                    if let Some(d) = plan_net.attempts.get(attempt_net).copied() {
                        attempt_net += 1;
                        tokio::time::sleep(d).await;
                        continue;
                    }
                    return Err(GitlabError::Timeout(e.to_string()));
                }
                Err(e) if e.is_connect() => {
                    if let Some(d) = plan_net.attempts.get(attempt_net).copied() {
                        attempt_net += 1;
                        tokio::time::sleep(d).await;
                        continue;
                    }
                    return Err(GitlabError::Network(e.to_string()));
                }
                Err(e) => return Err(GitlabError::Network(e.to_string())),
            }
        }
    }

    pub async fn send_json<T: DeserializeOwned>(&self, spec: RequestSpec) -> Result<T> {
        let (_status, _headers, bytes) = self.send_raw(spec).await?;
        serde_json::from_slice(&bytes).map_err(|e| GitlabError::Network(format!("parse: {e}")))
    }
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned)
}
