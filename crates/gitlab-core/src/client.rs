use std::time::Duration;
use url::Url;

use crate::error::{GitlabError, Result};

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub host: String,
    pub token: String,
    pub tls_skip_verify: bool,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub user_agent: String,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    token: String,
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

        Ok(Self { http, base_url, token: opts.token })
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

    pub fn endpoint(&self, path: &str) -> Result<Url> {
        let trimmed = path.trim_start_matches('/');
        self.base_url
            .join(trimmed)
            .map_err(|e| GitlabError::InvalidArgs(format!("bad endpoint {path}: {e}")))
    }
}
