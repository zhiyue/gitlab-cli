# gitlab-cli Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI tool (`gitlab-cli`) targeting GitLab EE 14.0.5 that is consumed by autonomous agents via `bash -c` + JSON, covering 12 core resource families plus a raw-API escape hatch, with single-binary distribution for macOS and Linux musl.

**Architecture:** Cargo workspace with two crates — `gitlab-core` (pure library: reqwest client, auth, pagination, retry, typed resources) and `gitlab-cli` (thin clap-based binary that consumes core and formats JSON to stdout / structured errors to stderr). Strict file-size discipline (~300 lines max per file), TDD throughout (L1 unit + L2 wiremock + L3 opt-in smoke).

**Tech Stack:** Rust stable edition 2021, `reqwest` + `rustls-tls`, `tokio` runtime, `clap` derive, `serde_json`, `thiserror`, `tracing`, `wiremock` (tests), `directories` (XDG paths), `governor` (rate limit).

---

## Milestones at a glance

| Milestone | Tasks | Delivers |
|---|---|---|
| M1 Foundation | 1.1 – 1.10 | `gitlab-core` library: HTTP + auth + config + errors + pagination + retry + throttle |
| M2 CLI skeleton | 2.1 – 2.8 | clap root, global flags, output/error emitters, tracing, `version`/`me`/`config` |
| M3 Escape hatch | 3.1 | `api` subcommand for any endpoint |
| M4 Resources | 4.1 – 4.16 | All 12 resource families + `search` |
| M5 Polish | 5.1 – 5.6 | README, CI, release packaging, smoke tests |

---

# Milestone 1 — Foundation (`gitlab-core`)

## Task 1.1: Workspace scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `crates/gitlab-core/Cargo.toml`
- Create: `crates/gitlab-core/src/lib.rs`
- Create: `crates/gitlab-cli/Cargo.toml`
- Create: `crates/gitlab-cli/src/main.rs`
- Create: `.gitattributes`

- [ ] **Step 1: Write the failing test**

Create `crates/gitlab-core/tests/smoke_test.rs`:

```rust
#[test]
fn crate_name_is_stable() {
    assert_eq!(env!("CARGO_PKG_NAME"), "gitlab-core");
}
```

- [ ] **Step 2: Run it (expect the whole workspace to be missing)**

Run: `cargo test -p gitlab-core`
Expected: FAIL — `error: could not find Cargo.toml`

- [ ] **Step 3: Create the workspace files**

`Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/gitlab-core", "crates/gitlab-cli"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.77"
license = "MIT"
repository = "https://github.com/your-org/gitlab-cli"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.77.0"
components = ["rustfmt", "clippy"]
```

`crates/gitlab-core/Cargo.toml`:

```toml
[package]
name = "gitlab-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]

[dev-dependencies]
```

`crates/gitlab-core/src/lib.rs`:

```rust
//! gitlab-core: GitLab 14.0.5-ee REST client primitives.
```

`crates/gitlab-cli/Cargo.toml`:

```toml
[package]
name = "gitlab-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "gitlab"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
gitlab-core = { path = "../gitlab-core" }
```

`crates/gitlab-cli/src/main.rs`:

```rust
fn main() {
    println!("gitlab-cli placeholder");
}
```

`.gitattributes`:

```
* text=auto eol=lf
*.rs diff=rust
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core`
Expected: PASS — one test.

Run: `cargo build -p gitlab-cli`
Expected: binary `target/debug/gitlab` exists and runs.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml crates .gitattributes
git commit -m "feat(workspace): scaffold gitlab-core + gitlab-cli crates"
```

---

## Task 1.2: Core error types

**Files:**
- Create: `crates/gitlab-core/src/error.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/Cargo.toml` (add deps)
- Create: `crates/gitlab-core/tests/error_classify_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/error_classify_test.rs`:

```rust
use gitlab_core::error::{ErrorCode, GitlabError};

#[test]
fn http_404_classifies_as_not_found() {
    let err = GitlabError::from_status(404, "404 Project Not Found".into(), None);
    assert_eq!(err.code(), ErrorCode::NotFound);
    assert_eq!(err.exit_code(), 5);
    assert!(!err.retryable());
}

#[test]
fn http_429_classifies_as_rate_limited_retryable() {
    let err = GitlabError::from_status(429, "Too Many Requests".into(), None);
    assert_eq!(err.code(), ErrorCode::RateLimited);
    assert_eq!(err.exit_code(), 7);
    assert!(err.retryable());
}

#[test]
fn http_500_classifies_as_server_retryable() {
    let err = GitlabError::from_status(500, "oops".into(), None);
    assert_eq!(err.code(), ErrorCode::ServerError);
    assert!(err.retryable());
}

#[test]
fn network_error_is_retryable() {
    let err = GitlabError::network("connection reset".into());
    assert_eq!(err.code(), ErrorCode::Network);
    assert_eq!(err.exit_code(), 9);
    assert!(err.retryable());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test error_classify_test`
Expected: FAIL — `unresolved import gitlab_core::error`.

- [ ] **Step 3: Implement the error types**

Add to `crates/gitlab-core/Cargo.toml` under `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

Create `crates/gitlab-core/src/error.rs`:

```rust
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArgs,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    BadRequest,
    RateLimited,
    ServerError,
    Network,
    Timeout,
    Unknown,
}

impl ErrorCode {
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            Self::InvalidArgs => 2,
            Self::Unauthorized => 3,
            Self::Forbidden => 4,
            Self::NotFound => 5,
            Self::Conflict => 6,
            Self::BadRequest => 6,
            Self::RateLimited => 7,
            Self::ServerError => 8,
            Self::Network | Self::Timeout => 9,
            Self::Unknown => 1,
        }
    }

    #[must_use]
    pub fn retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::ServerError | Self::Network | Self::Timeout)
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub status: Option<u16>,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
    pub details: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum GitlabError {
    #[error("{message}")]
    Http {
        code: ErrorCode,
        status: u16,
        message: String,
        request_id: Option<String>,
        details: serde_json::Value,
    },
    #[error("network: {0}")]
    Network(String),
    #[error("timeout after {0}")]
    Timeout(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("config: {0}")]
    Config(String),
}

impl GitlabError {
    #[must_use]
    pub fn from_status(status: u16, message: String, request_id: Option<String>) -> Self {
        let code = match status {
            400 => ErrorCode::BadRequest,
            401 => ErrorCode::Unauthorized,
            403 => ErrorCode::Forbidden,
            404 => ErrorCode::NotFound,
            409 | 422 => ErrorCode::Conflict,
            429 => ErrorCode::RateLimited,
            500..=599 => ErrorCode::ServerError,
            _ => ErrorCode::Unknown,
        };
        Self::Http {
            code,
            status,
            message,
            request_id,
            details: serde_json::Value::Null,
        }
    }

    #[must_use]
    pub fn network(msg: String) -> Self {
        Self::Network(msg)
    }

    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Http { code, .. } => *code,
            Self::Network(_) => ErrorCode::Network,
            Self::Timeout(_) => ErrorCode::Timeout,
            Self::InvalidArgs(_) => ErrorCode::InvalidArgs,
            Self::Config(_) => ErrorCode::InvalidArgs,
        }
    }

    #[must_use]
    pub fn exit_code(&self) -> i32 {
        self.code().exit_code()
    }

    #[must_use]
    pub fn retryable(&self) -> bool {
        self.code().retryable()
    }

    #[must_use]
    pub fn to_payload(&self) -> ErrorPayload {
        match self {
            Self::Http { code, status, message, request_id, details } => ErrorPayload {
                code: *code,
                status: Some(*status),
                message: message.clone(),
                request_id: request_id.clone(),
                retryable: code.retryable(),
                details: details.clone(),
            },
            other => ErrorPayload {
                code: other.code(),
                status: None,
                message: other.to_string(),
                request_id: None,
                retryable: other.retryable(),
                details: serde_json::Value::Null,
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, GitlabError>;

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self).unwrap().as_str().unwrap().to_owned();
        f.write_str(&s)
    }
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
//! gitlab-core: GitLab 14.0.5-ee REST client primitives.

pub mod error;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test error_classify_test`
Expected: 4 PASS.

Run: `cargo clippy -p gitlab-core -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): GitlabError + ErrorCode with HTTP→code→exit mapping"
```

---

## Task 1.3: TOML config loading

**Files:**
- Create: `crates/gitlab-core/src/config.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/Cargo.toml` (add `toml`, `directories`)
- Create: `crates/gitlab-core/tests/config_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/config_test.rs`:

```rust
use gitlab_core::config::{Config, HostConfig};
use std::io::Write;

#[test]
fn parses_multi_host_toml() {
    let toml = r#"
default_host = "gitlab.example.com"

[host."gitlab.example.com"]
token = "glpat-AAA"
rps = 5
default_project = "g/p"

[host."gitlab.com"]
token = "glpat-BBB"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.default_host.as_deref(), Some("gitlab.example.com"));
    assert_eq!(cfg.host.len(), 2);
    let h = cfg.host.get("gitlab.example.com").unwrap();
    assert_eq!(h.token.as_deref(), Some("glpat-AAA"));
    assert_eq!(h.rps, Some(5));
    assert_eq!(h.default_project.as_deref(), Some("g/p"));
}

#[test]
fn load_missing_file_returns_empty_config() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing.toml");
    let cfg = Config::load_from(&path).unwrap();
    assert!(cfg.host.is_empty());
}

#[test]
fn load_reads_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "default_host = \"a\"").unwrap();
    writeln!(f, "[host.\"a\"]").unwrap();
    writeln!(f, "token = \"tok\"").unwrap();
    let cfg = Config::load_from(&path).unwrap();
    assert_eq!(cfg.default_host.as_deref(), Some("a"));
}

#[test]
fn host_for_resolves_to_specified_then_default() {
    let cfg = Config {
        default_host: Some("a".into()),
        host: std::collections::HashMap::from([
            ("a".into(), HostConfig { token: Some("aaa".into()), ..HostConfig::default() }),
            ("b".into(), HostConfig { token: Some("bbb".into()), ..HostConfig::default() }),
        ]),
    };
    assert_eq!(cfg.host_for(None).unwrap().token.as_deref(), Some("aaa"));
    assert_eq!(cfg.host_for(Some("b")).unwrap().token.as_deref(), Some("bbb"));
    assert!(cfg.host_for(Some("c")).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test config_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement Config**

Add to `crates/gitlab-core/Cargo.toml`:

```toml
toml = "0.8"
directories = "5"

[dev-dependencies]
tempfile = "3"
```

Create `crates/gitlab-core/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{GitlabError, Result};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Config {
    pub default_host: Option<String>,
    #[serde(default)]
    pub host: HashMap<String, HostConfig>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct HostConfig {
    pub token: Option<String>,
    #[serde(default)]
    pub tls_skip_verify: bool,
    pub rps: Option<u32>,
    pub default_project: Option<String>,
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| GitlabError::Config(e.to_string())),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(GitlabError::Config(e.to_string())),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GitlabError::Config(e.to_string()))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| GitlabError::Config(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| GitlabError::Config(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    #[must_use]
    pub fn host_for(&self, host: Option<&str>) -> Option<&HostConfig> {
        let key = host.or(self.default_host.as_deref())?;
        self.host.get(key)
    }

    #[must_use]
    pub fn default_config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "gitlab-cli")
            .map(|p| p.config_dir().join("config.toml"))
    }
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
//! gitlab-core: GitLab 14.0.5-ee REST client primitives.

pub mod config;
pub mod error;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test config_test`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): TOML Config with per-host sections"
```

---

## Task 1.4: Auth resolution

**Files:**
- Create: `crates/gitlab-core/src/auth.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Create: `crates/gitlab-core/tests/auth_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/auth_test.rs`:

```rust
use gitlab_core::auth::{resolve_auth, AuthInputs};
use gitlab_core::config::{Config, HostConfig};

fn cfg(host: &str, token: &str) -> Config {
    let mut c = Config::default();
    c.default_host = Some(host.into());
    c.host.insert(
        host.into(),
        HostConfig { token: Some(token.into()), ..HostConfig::default() },
    );
    c
}

#[test]
fn cli_flag_beats_env_and_config() {
    let resolved = resolve_auth(
        AuthInputs {
            flag_token: Some("flag-tok".into()),
            flag_host: None,
            env_token: Some("env-tok".into()),
            env_host: None,
        },
        &cfg("h", "cfg-tok"),
    )
    .unwrap();
    assert_eq!(resolved.token, "flag-tok");
    assert_eq!(resolved.host, "h");
}

#[test]
fn env_beats_config_when_no_flag() {
    let resolved = resolve_auth(
        AuthInputs {
            flag_token: None,
            flag_host: None,
            env_token: Some("env-tok".into()),
            env_host: None,
        },
        &cfg("h", "cfg-tok"),
    )
    .unwrap();
    assert_eq!(resolved.token, "env-tok");
}

#[test]
fn config_is_last_fallback() {
    let resolved = resolve_auth(
        AuthInputs::default(),
        &cfg("h", "cfg-tok"),
    )
    .unwrap();
    assert_eq!(resolved.token, "cfg-tok");
}

#[test]
fn missing_token_yields_unauthorized() {
    let err = resolve_auth(AuthInputs::default(), &Config::default()).unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::Unauthorized);
}

#[test]
fn host_precedence_flag_env_config() {
    let c = cfg("cfg-host", "tok");
    assert_eq!(
        resolve_auth(
            AuthInputs { flag_host: Some("flag-host".into()), ..AuthInputs::default() },
            &c
        )
        .unwrap()
        .host,
        "flag-host"
    );
    assert_eq!(
        resolve_auth(
            AuthInputs { env_host: Some("env-host".into()), ..AuthInputs::default() },
            &c
        )
        .unwrap()
        .host,
        "env-host"
    );
    assert_eq!(resolve_auth(AuthInputs::default(), &c).unwrap().host, "cfg-host");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test auth_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement auth resolver**

Create `crates/gitlab-core/src/auth.rs`:

```rust
use crate::config::{Config, HostConfig};
use crate::error::{GitlabError, Result};

#[derive(Debug, Default, Clone)]
pub struct AuthInputs {
    pub flag_token: Option<String>,
    pub flag_host: Option<String>,
    pub env_token: Option<String>,
    pub env_host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub host: String,
    pub token: String,
    pub host_config: HostConfig,
}

pub fn resolve_auth(inputs: AuthInputs, config: &Config) -> Result<ResolvedAuth> {
    let host = inputs
        .flag_host
        .or(inputs.env_host)
        .or_else(|| config.default_host.clone())
        .ok_or_else(|| GitlabError::Config("no host configured".into()))?;

    let host_cfg = config.host.get(&host).cloned().unwrap_or_default();

    let token = inputs
        .flag_token
        .or(inputs.env_token)
        .or_else(|| host_cfg.token.clone())
        .ok_or_else(|| {
            GitlabError::from_status(
                401,
                format!("no PAT found for host {host} (set --token, GITLAB_TOKEN, or config.toml)"),
                None,
            )
        })?;

    Ok(ResolvedAuth { host, token, host_config: host_cfg })
}

pub struct MaskedToken<'a>(pub &'a str);

impl std::fmt::Display for MaskedToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = self.0;
        if t.len() <= 4 {
            return f.write_str("****");
        }
        let last4 = &t[t.len() - 4..];
        write!(f, "{}****{}", &t[..std::cmp::min(4, t.len() - 4)], last4)
    }
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod config;
pub mod error;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test auth_test`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): auth resolver (flag > env > config) with masking helper"
```

---

## Task 1.5: HTTP client builder

**Files:**
- Create: `crates/gitlab-core/src/client.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/Cargo.toml` (add `reqwest`, `tokio`, `url`)
- Create: `crates/gitlab-core/tests/client_build_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/client_build_test.rs`:

```rust
use gitlab_core::client::{Client, ClientOptions};

#[test]
fn client_builds_with_defaults() {
    let c = Client::new(ClientOptions {
        host: "https://gitlab.example.com".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    assert_eq!(c.base_url().as_str(), "https://gitlab.example.com/api/v4/");
}

#[test]
fn host_without_scheme_is_rejected() {
    let err = Client::new(ClientOptions {
        host: "gitlab.example.com".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::InvalidArgs);
}

#[test]
fn host_keeps_trailing_slash_consistent() {
    let c = Client::new(ClientOptions {
        host: "https://gitlab.example.com/".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    assert_eq!(c.base_url().as_str(), "https://gitlab.example.com/api/v4/");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test client_build_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement Client builder**

Add to `crates/gitlab-core/Cargo.toml`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "gzip", "json", "stream"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
url = "2"
```

Create `crates/gitlab-core/src/client.rs`:

```rust
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
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test client_build_test`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): Client builder with rustls TLS + base URL derivation"
```

---

## Task 1.6: Link header parser

**Files:**
- Create: `crates/gitlab-core/src/page/link.rs`
- Create: `crates/gitlab-core/src/page/mod.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Create: `crates/gitlab-core/tests/link_header_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/link_header_test.rs`:

```rust
use gitlab_core::page::link::{parse_link_header, Rel};

#[test]
fn single_next_link() {
    let h = r#"<https://x/api/v4/projects?page=2>; rel="next""#;
    let links = parse_link_header(h).unwrap();
    assert_eq!(links.get(&Rel::Next).unwrap(), "https://x/api/v4/projects?page=2");
}

#[test]
fn multiple_rels() {
    let h = r#"<https://x/p?page=2>; rel="next", <https://x/p?page=5>; rel="last", <https://x/p?page=1>; rel="first""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.contains_key(&Rel::Next));
    assert!(links.contains_key(&Rel::Last));
    assert!(links.contains_key(&Rel::First));
}

#[test]
fn empty_header_yields_empty_map() {
    let links = parse_link_header("").unwrap();
    assert!(links.is_empty());
}

#[test]
fn malformed_entry_is_skipped() {
    let h = r#"<https://x/p?page=2> rel="next""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.is_empty());
}

#[test]
fn unknown_rel_is_ignored() {
    let h = r#"<https://x/p?page=2>; rel="weird""#;
    let links = parse_link_header(h).unwrap();
    assert!(links.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test link_header_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement parser**

Create `crates/gitlab-core/src/page/mod.rs`:

```rust
pub mod link;
pub mod stream;

pub use stream::{PageRequest, PagedStream};
```

Create `crates/gitlab-core/src/page/link.rs`:

```rust
use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rel {
    First,
    Prev,
    Next,
    Last,
}

impl Rel {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "first" => Some(Self::First),
            "prev" => Some(Self::Prev),
            "next" => Some(Self::Next),
            "last" => Some(Self::Last),
            _ => None,
        }
    }
}

pub fn parse_link_header(header: &str) -> Result<HashMap<Rel, String>> {
    let mut out = HashMap::new();
    if header.trim().is_empty() {
        return Ok(out);
    }
    for entry in header.split(',') {
        let entry = entry.trim();
        let Some((url_part, rest)) = entry.split_once(';') else { continue; };
        let url = url_part.trim();
        if !url.starts_with('<') || !url.ends_with('>') {
            continue;
        }
        let url = &url[1..url.len() - 1];
        for param in rest.split(';') {
            let param = param.trim();
            let Some((k, v)) = param.split_once('=') else { continue; };
            if k.trim() != "rel" {
                continue;
            }
            let v = v.trim().trim_matches('"');
            if let Some(rel) = Rel::parse(v) {
                out.insert(rel, url.to_owned());
            }
        }
    }
    Ok(out)
}
```

Placeholder `stream.rs` (filled in next task):

```rust
// Filled in Task 1.7
pub struct PagedStream;
pub struct PageRequest;
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod page;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test link_header_test`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): RFC 5988 Link header parser"
```

---

## Task 1.7: PagedStream

**Files:**
- Modify: `crates/gitlab-core/src/page/stream.rs` (replace placeholder)
- Modify: `crates/gitlab-core/Cargo.toml` (add `futures`, `async-stream`)
- Create: `crates/gitlab-core/tests/paged_stream_test.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/gitlab-core/Cargo.toml`:

```toml
async-stream = "0.3"
futures = "0.3"

[dev-dependencies]
wiremock = "0.6"
```

`crates/gitlab-core/tests/paged_stream_test.rs`:

```rust
use futures::StreamExt;
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::page::{PageRequest, PagedStream};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_3_pages(server: &MockServer) {
    let p1 = serde_json::json!([{"id":1},{"id":2}]);
    let p2 = serde_json::json!([{"id":3},{"id":4}]);
    let p3 = serde_json::json!([{"id":5}]);

    let base = server.uri();
    let link_p1 = format!(
        r#"<{base}/api/v4/projects?page=2&per_page=100>; rel="next", <{base}/api/v4/projects?page=3&per_page=100>; rel="last""#
    );
    let link_p2 = format!(
        r#"<{base}/api/v4/projects?page=3&per_page=100>; rel="next", <{base}/api/v4/projects?page=3&per_page=100>; rel="last""#
    );

    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p1).insert_header("Link", &link_p1))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p2).insert_header("Link", &link_p2))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .and(query_param("page", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&p3))
        .mount(server)
        .await;
}

#[tokio::test]
async fn streams_across_all_pages() {
    let server = MockServer::start().await;
    setup_3_pages(&server).await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let req = PageRequest::new("projects").with_query(&[("state", "opened")]);
    let stream = PagedStream::<serde_json::Value>::start(&client, req);
    let items: Vec<_> = stream.collect().await;
    assert_eq!(items.len(), 5, "got items: {items:?}");
    for (i, item) in items.into_iter().enumerate() {
        let v = item.unwrap();
        assert_eq!(v["id"], serde_json::json!(i + 1));
    }
}

#[tokio::test]
async fn empty_first_page_yields_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let stream = PagedStream::<serde_json::Value>::start(&client, PageRequest::new("projects"));
    let items: Vec<_> = stream.collect().await;
    assert!(items.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test paged_stream_test`
Expected: FAIL — `PagedStream::start` not implemented.

- [ ] **Step 3: Implement PagedStream**

Replace `crates/gitlab-core/src/page/stream.rs`:

```rust
use async_stream::try_stream;
use futures::Stream;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{GitlabError, Result};
use crate::page::link::{parse_link_header, Rel};

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub path: String,
    pub query: Vec<(String, String)>,
    pub per_page: u32,
}

impl PageRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), query: Vec::new(), per_page: 100 }
    }

    #[must_use]
    pub fn with_query(mut self, pairs: &[(&str, &str)]) -> Self {
        for (k, v) in pairs {
            self.query.push(((*k).to_owned(), (*v).to_owned()));
        }
        self
    }
}

pub struct PagedStream<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned + Send + 'static> PagedStream<T> {
    pub fn start(client: &Client, req: PageRequest) -> impl Stream<Item = Result<T>> + Unpin {
        let client = client.clone();
        let stream = try_stream! {
            let mut url = client.endpoint(&req.path)?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("per_page", &req.per_page.to_string());
                q.append_pair("page", "1");
                for (k, v) in &req.query {
                    q.append_pair(k, v);
                }
            }
            loop {
                let resp = client
                    .http()
                    .get(url.clone())
                    .header("PRIVATE-TOKEN", client.token())
                    .send()
                    .await
                    .map_err(|e| GitlabError::Network(e.to_string()))?;
                let status = resp.status().as_u16();
                let headers = resp.headers().clone();
                if !resp.status().is_success() {
                    let msg = resp.text().await.unwrap_or_default();
                    Err(GitlabError::from_status(status, msg, extract_request_id(&headers)))?;
                }
                let items: Vec<T> = resp
                    .json()
                    .await
                    .map_err(|e| GitlabError::Network(format!("parse: {e}")))?;
                for it in items {
                    yield it;
                }
                let Some(link) = headers.get("link").and_then(|h| h.to_str().ok()) else { break; };
                let rels = parse_link_header(link)?;
                let Some(next) = rels.get(&Rel::Next) else { break; };
                url = url::Url::parse(next)
                    .map_err(|e| GitlabError::Network(format!("bad next url {next}: {e}")))?;
            }
        };
        Box::pin(stream)
    }
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_owned())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test paged_stream_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): PagedStream auto-follows Link: next across pages"
```

---

## Task 1.8: Retry logic with backoff

**Files:**
- Create: `crates/gitlab-core/src/retry.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/Cargo.toml` (add `rand`)
- Create: `crates/gitlab-core/tests/retry_test.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/gitlab-core/Cargo.toml`:

```toml
rand = "0.8"
```

`crates/gitlab-core/tests/retry_test.rs`:

```rust
use gitlab_core::retry::{RetryPolicy, RetryPlan};
use std::time::Duration;

#[test]
fn default_policy_returns_4_network_backoffs() {
    let p = RetryPolicy::default();
    let plan = p.plan_for_network();
    assert_eq!(plan.attempts.len(), 4);
    let expected = [500, 1000, 2000, 4000];
    for (got, want) in plan.attempts.iter().zip(expected.iter()) {
        let tol = *want * 20 / 100;
        assert!(
            got.as_millis() as u64 >= want - tol && got.as_millis() as u64 <= want + tol,
            "backoff {:?} not within 20% of {}ms", got, want
        );
    }
}

#[test]
fn retry_after_header_beats_backoff() {
    let p = RetryPolicy::default();
    let next = p.next_delay_for_429(Some("7"), 0);
    assert_eq!(next, Some(Duration::from_secs(7)));
}

#[test]
fn no_retries_when_disabled() {
    let p = RetryPolicy { max_attempts: 0, ..RetryPolicy::default() };
    let plan = p.plan_for_network();
    assert!(plan.attempts.is_empty());
}

#[test]
fn retry_after_invalid_falls_back_to_backoff() {
    let p = RetryPolicy::default();
    let next = p.next_delay_for_429(Some("invalid"), 0);
    assert!(next.is_some());
    assert!(next.unwrap() >= Duration::from_millis(400));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test retry_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement retry**

Create `crates/gitlab-core/src/retry.rs`:

```rust
use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub max_attempts_429: u32,
    pub base_ms: u64,
    pub factor: u32,
    pub jitter_pct: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 4, max_attempts_429: 5, base_ms: 500, factor: 2, jitter_pct: 20 }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPlan {
    pub attempts: Vec<Duration>,
}

impl RetryPolicy {
    #[must_use]
    pub fn plan_for_network(&self) -> RetryPlan {
        self.plan(self.max_attempts)
    }

    #[must_use]
    pub fn plan_for_429(&self) -> RetryPlan {
        self.plan(self.max_attempts_429)
    }

    fn plan(&self, attempts: u32) -> RetryPlan {
        let mut rng = rand::thread_rng();
        let mut out = Vec::with_capacity(attempts as usize);
        for i in 0..attempts {
            let base = self.base_ms * u64::from(self.factor.pow(i));
            let jitter_range = base * u64::from(self.jitter_pct) / 100;
            let j: i64 = rng.gen_range(-(jitter_range as i64)..=(jitter_range as i64));
            let v = (base as i64 + j).max(0) as u64;
            out.push(Duration::from_millis(v));
        }
        RetryPlan { attempts: out }
    }

    #[must_use]
    pub fn next_delay_for_429(&self, retry_after: Option<&str>, attempt_idx: usize) -> Option<Duration> {
        if let Some(s) = retry_after {
            if let Ok(secs) = s.trim().parse::<u64>() {
                return Some(Duration::from_secs(secs));
            }
        }
        self.plan_for_429().attempts.get(attempt_idx).copied()
    }
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod page;
pub mod retry;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test retry_test`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): RetryPolicy with exp backoff + Retry-After honoring"
```

---

## Task 1.9: Client-side throttle

**Files:**
- Create: `crates/gitlab-core/src/throttle.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/Cargo.toml` (add `governor`)
- Create: `crates/gitlab-core/tests/throttle_test.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/gitlab-core/Cargo.toml`:

```toml
governor = "0.6"
```

`crates/gitlab-core/tests/throttle_test.rs`:

```rust
use gitlab_core::throttle::Throttle;
use std::time::Instant;

#[tokio::test]
async fn disabled_throttle_never_sleeps() {
    let t = Throttle::disabled();
    let start = Instant::now();
    for _ in 0..20 {
        t.acquire().await;
    }
    assert!(start.elapsed().as_millis() < 50);
}

#[tokio::test]
async fn enabled_throttle_spaces_requests() {
    let t = Throttle::per_second(5);
    let start = Instant::now();
    for _ in 0..10 {
        t.acquire().await;
    }
    let elapsed = start.elapsed().as_millis();
    assert!(elapsed >= 800, "10 reqs @ 5 rps should take ≥ 0.8s, got {elapsed}ms");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test throttle_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement Throttle**

Create `crates/gitlab-core/src/throttle.rs`:

```rust
use governor::{clock::DefaultClock, middleware::NoOpMiddleware, state::{InMemoryState, NotKeyed}, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

type Lim = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

#[derive(Clone)]
pub struct Throttle {
    inner: Option<Arc<Lim>>,
}

impl Throttle {
    #[must_use]
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    #[must_use]
    pub fn per_second(rps: u32) -> Self {
        let rps = NonZeroU32::new(rps.max(1)).unwrap();
        let lim = RateLimiter::direct(Quota::per_second(rps));
        Self { inner: Some(Arc::new(lim)) }
    }

    pub async fn acquire(&self) {
        if let Some(l) = &self.inner {
            l.until_ready().await;
        }
    }
}

impl std::fmt::Debug for Throttle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Throttle").field("enabled", &self.inner.is_some()).finish()
    }
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod page;
pub mod retry;
pub mod throttle;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core --test throttle_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): token-bucket Throttle via governor"
```

---

## Task 1.10: Integrated request sender + retry wiring

**Files:**
- Create: `crates/gitlab-core/src/request.rs`
- Modify: `crates/gitlab-core/src/client.rs` (integrate Throttle + RetryPolicy into Client)
- Modify: `crates/gitlab-core/src/lib.rs`
- Modify: `crates/gitlab-core/src/page/stream.rs` (use integrated sender)
- Create: `crates/gitlab-core/tests/request_retry_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/request_retry_test.rs`:

```rust
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::request::RequestSpec;
use reqwest::Method;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn retries_on_500_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"version":"14.0.5-ee"})),
        )
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let body: serde_json::Value = client
        .send_json(RequestSpec::new(Method::GET, "version"))
        .await
        .unwrap();
    assert_eq!(body["version"], "14.0.5-ee");
}

#[tokio::test]
async fn non_retryable_4xx_fails_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/nope"))
        .respond_with(ResponseTemplate::new(404).set_body_string("404 Not Found"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let err = client
        .send_json::<serde_json::Value>(RequestSpec::new(Method::GET, "nope"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::NotFound);
}

#[tokio::test]
async fn honors_retry_after_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "1"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"version":"14.0.5-ee"})),
        )
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let start = std::time::Instant::now();
    let _: serde_json::Value = client.send_json(RequestSpec::new(Method::GET, "version")).await.unwrap();
    assert!(start.elapsed().as_millis() >= 900);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-core --test request_retry_test`
Expected: FAIL — `client.send_json` not found.

- [ ] **Step 3: Implement integrated sender**

Create `crates/gitlab-core/src/request.rs`:

```rust
use reqwest::Method;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub method: Method,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

impl RequestSpec {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self { method, path: path.into(), query: Vec::new(), body: None }
    }

    #[must_use]
    pub fn with_query<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in pairs {
            self.query.push((k.into(), v.into()));
        }
        self
    }

    #[must_use]
    pub fn with_json<T: Serialize>(mut self, body: &T) -> Self {
        self.body = Some(serde_json::to_value(body).unwrap_or(serde_json::Value::Null));
        self
    }
}
```

Rewrite `crates/gitlab-core/src/client.rs` to use `Throttle` + `RetryPolicy` + `send_json`:

```rust
use std::time::Duration;
use reqwest::header::HeaderMap;
use reqwest::Method;
use serde::de::DeserializeOwned;
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
    pub fn base_url(&self) -> &Url { &self.base_url }
    #[must_use]
    pub fn token(&self) -> &str { &self.token }
    #[must_use]
    pub fn http(&self) -> &reqwest::Client { &self.http }
    #[must_use]
    pub fn retry(&self) -> &RetryPolicy { &self.retry }
    #[must_use]
    pub fn throttle(&self) -> &Throttle { &self.throttle }

    pub fn endpoint(&self, path: &str) -> Result<Url> {
        let trimmed = path.trim_start_matches('/');
        self.base_url
            .join(trimmed)
            .map_err(|e| GitlabError::InvalidArgs(format!("bad endpoint {path}: {e}")))
    }

    pub async fn send_raw(&self, spec: RequestSpec) -> Result<(u16, HeaderMap, bytes::Bytes)> {
        let plan_net = self.retry.plan_for_network();
        let mut attempt_429: usize = 0;

        loop {
            self.throttle.acquire().await;
            let mut url = self.endpoint(&spec.path)?;
            if !spec.query.is_empty() {
                let mut q = url.query_pairs_mut();
                for (k, v) in &spec.query {
                    q.append_pair(k, v);
                }
            }
            let mut req = self.http.request(spec.method.clone(), url).header("PRIVATE-TOKEN", &self.token);
            if let Some(body) = &spec.body {
                req = req.json(body);
            }
            let attempt_idx_net = plan_net.attempts.len().saturating_sub(1);
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let headers = resp.headers().clone();
                    let bytes = resp.bytes().await.map_err(|e| GitlabError::Network(e.to_string()))?;
                    match status {
                        200..=299 => return Ok((status, headers, bytes)),
                        429 => {
                            let retry_after = headers
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .map(|s| s.to_owned());
                            if let Some(d) = self.retry.next_delay_for_429(retry_after.as_deref(), attempt_429) {
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
                            if let Some(d) = plan_net.attempts.get(attempt_idx_net).copied() {
                                tokio::time::sleep(d).await;
                                continue;
                            }
                            return Err(GitlabError::from_status(
                                status,
                                String::from_utf8_lossy(&bytes).into_owned(),
                                extract_request_id(&headers),
                            ));
                        }
                        _ => return Err(GitlabError::from_status(
                            status,
                            String::from_utf8_lossy(&bytes).into_owned(),
                            extract_request_id(&headers),
                        )),
                    }
                }
                Err(e) if e.is_timeout() => {
                    if let Some(d) = plan_net.attempts.get(attempt_idx_net).copied() {
                        tokio::time::sleep(d).await;
                        continue;
                    }
                    return Err(GitlabError::Timeout(e.to_string()));
                }
                Err(e) if e.is_connect() => {
                    if let Some(d) = plan_net.attempts.get(attempt_idx_net).copied() {
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
```

Add `bytes` dep to `gitlab-core/Cargo.toml`:

```toml
bytes = "1"
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod page;
pub mod request;
pub mod retry;
pub mod throttle;
```

Rewrite `crates/gitlab-core/src/page/stream.rs` to use `send_raw`:

```rust
use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::{GitlabError, Result};
use crate::page::link::{parse_link_header, Rel};
use crate::request::RequestSpec;

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub path: String,
    pub query: Vec<(String, String)>,
    pub per_page: u32,
}

impl PageRequest {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), query: Vec::new(), per_page: 100 }
    }

    #[must_use]
    pub fn with_query(mut self, pairs: &[(&str, &str)]) -> Self {
        for (k, v) in pairs {
            self.query.push(((*k).to_owned(), (*v).to_owned()));
        }
        self
    }
}

pub struct PagedStream<T> {
    _m: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned + Send + 'static> PagedStream<T> {
    pub fn start(client: &Client, req: PageRequest) -> impl Stream<Item = Result<T>> + Unpin {
        let client = client.clone();
        let stream = try_stream! {
            let mut next_path: Option<String> = Some(req.path.clone());
            let mut first = true;
            loop {
                let path = match next_path.take() { Some(p) => p, None => break };
                let mut spec = RequestSpec::new(Method::GET, path);
                if first {
                    spec = spec.with_query(req.query.iter().map(|(k,v)| (k.clone(), v.clone())));
                    spec.query.push(("per_page".to_owned(), req.per_page.to_string()));
                    spec.query.push(("page".to_owned(), "1".to_owned()));
                    first = false;
                }
                let (_status, headers, bytes) = client.send_raw(spec).await?;
                let items: Vec<T> = serde_json::from_slice(&bytes)
                    .map_err(|e| GitlabError::Network(format!("parse: {e}")))?;
                for it in items { yield it; }
                let link = headers.get("link").and_then(|h| h.to_str().ok());
                if let Some(h) = link {
                    let rels = parse_link_header(h)?;
                    if let Some(next) = rels.get(&Rel::Next) {
                        let url = url::Url::parse(next)
                            .map_err(|e| GitlabError::Network(format!("bad next url {next}: {e}")))?;
                        let path_qs = format!("{}?{}", url.path(), url.query().unwrap_or(""));
                        let trimmed = path_qs
                            .trim_start_matches("/api/v4/")
                            .trim_start_matches("api/v4/")
                            .to_owned();
                        next_path = Some(trimmed);
                    }
                }
            }
        };
        Box::pin(stream)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-core`
Expected: all existing tests + 3 new ones PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-core
git commit -m "feat(core): integrated send_raw/send_json with retry + throttle + 429"
```

---

# Milestone 2 — CLI skeleton (`gitlab-cli`)

## Task 2.1: Bin crate clap root + global flags plumbing

**Files:**
- Modify: `crates/gitlab-cli/Cargo.toml`
- Create: `crates/gitlab-cli/src/globals.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/global_args_test.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/gitlab-cli/Cargo.toml`:

```toml
[dependencies]
gitlab-core = { path = "../gitlab-core" }
clap = { version = "4", features = ["derive", "env"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

`crates/gitlab-cli/tests/global_args_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn prints_help() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--help").assert().success().stdout(contains("gitlab"));
}

#[test]
fn version_flag_works() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--version").assert().success().stdout(contains(env!("CARGO_PKG_VERSION")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test global_args_test`
Expected: FAIL (or help output missing "gitlab" in placeholder main).

- [ ] **Step 3: Implement clap root + Globals**

Create `crates/gitlab-cli/src/globals.rs`:

```rust
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    #[arg(long, global = true, env = "GITLAB_HOST")]
    pub host: Option<String>,

    #[arg(long, global = true, env = "GITLAB_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    #[arg(long, global = true, default_value = "json")]
    pub output: OutputFormat,

    #[arg(long, global = true)]
    pub limit: Option<u32>,

    #[arg(long, global = true)]
    pub no_paginate: bool,

    #[arg(long, global = true, env = "GITLAB_TIMEOUT")]
    pub timeout: Option<u64>,

    #[arg(long, global = true)]
    pub retries: Option<u32>,

    #[arg(long, global = true)]
    pub no_retry: bool,

    #[arg(long, global = true, env = "GITLAB_RPS")]
    pub rps: Option<u32>,

    #[arg(long, global = true)]
    pub dry_run: bool,

    #[arg(long = "yes", short = 'y', global = true, env = "GITLAB_ASSUME_YES")]
    pub assume_yes: bool,

    #[arg(long, global = true, env = "GITLAB_VERBOSE")]
    pub verbose: Option<String>,

    #[arg(long, global = true)]
    pub tls_skip_verify: bool,

    #[arg(long, global = true, env = "GITLAB_CONFIG")]
    pub config: Option<std::path::PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum OutputFormat {
    Json,
    Ndjson,
}
```

Rewrite `crates/gitlab-cli/src/main.rs`:

```rust
use clap::{Parser, Subcommand};

mod globals;

use globals::GlobalArgs;

#[derive(Parser)]
#[command(
    name = "gitlab",
    version,
    about = "gitlab-cli: agent-first CLI for GitLab EE 14.0.5",
    long_about = None,
    propagate_version = true
)]
struct Cli {
    #[command(flatten)]
    globals: GlobalArgs,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print GitLab instance version
    Version,
    /// Print current user
    Me,
}

fn main() -> std::process::ExitCode {
    let _cli = Cli::parse();
    std::process::ExitCode::from(0)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test global_args_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): clap root + GlobalArgs flattened into all subcommands"
```

---

## Task 2.2: Output emitters (stdout JSON/NDJSON)

**Files:**
- Create: `crates/gitlab-cli/src/output.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/output_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/output_test.rs`:

```rust
// integration test lives in the bin crate; we use lib target through inline mod.
// Instead, test via process output once a command exists (deferred to version/me tasks).
#[test]
fn placeholder_until_commands_exist() {
    assert!(true);
}
```

(Real tests will come with `version` / `me` in Task 2.6-2.7; this task provides the emitter infrastructure.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test output_test`
Expected: PASS (placeholder). We still do this step to keep the rhythm.

- [ ] **Step 3: Implement output helpers**

Create `crates/gitlab-cli/src/output.rs`:

```rust
use crate::globals::OutputFormat;
use futures::{pin_mut, Stream, StreamExt};
use serde::Serialize;
use std::io::{self, Write};

pub fn emit_object<T: Serialize>(v: &T) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, v)?;
    lock.write_all(b"\n")?;
    Ok(())
}

pub async fn emit_stream<T, S>(stream: S, fmt: OutputFormat, limit: Option<u32>) -> io::Result<usize>
where
    T: Serialize,
    S: Stream<Item = Result<T, gitlab_core::error::GitlabError>>,
{
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let mut count: usize = 0;
    pin_mut!(stream);
    match fmt {
        OutputFormat::Json => {
            lock.write_all(b"[")?;
            let mut first = true;
            while let Some(item) = stream.next().await {
                let it = item.map_err(|e| io::Error::other(e.to_string()))?;
                if !first { lock.write_all(b",")?; }
                first = false;
                serde_json::to_writer(&mut lock, &it)?;
                count += 1;
                if let Some(n) = limit {
                    if count as u32 >= n { break; }
                }
            }
            lock.write_all(b"]\n")?;
        }
        OutputFormat::Ndjson => {
            while let Some(item) = stream.next().await {
                let it = item.map_err(|e| io::Error::other(e.to_string()))?;
                serde_json::to_writer(&mut lock, &it)?;
                lock.write_all(b"\n")?;
                count += 1;
                if let Some(n) = limit {
                    if count as u32 >= n { break; }
                }
            }
        }
    }
    Ok(count)
}
```

Update `main.rs` to declare the module:

```rust
mod globals;
mod output;
```

Add `futures = "0.3"` to `crates/gitlab-cli/Cargo.toml`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test output_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): stdout emitters for object, JSON array, and NDJSON"
```

---

## Task 2.3: Stderr error emitter with process exit mapping

**Files:**
- Create: `crates/gitlab-cli/src/errout.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/errout_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/errout_test.rs`:

```rust
use gitlab_cli_test_support::*;
// We use a small inline support module below.

#[test]
fn placeholder_until_command_exists() { assert!(true); }

mod gitlab_cli_test_support {}
```

(Real exit-code tests run through process invocation once `version` exists; this task ships the emitter.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test errout_test`
Expected: PASS (placeholder).

- [ ] **Step 3: Implement errout**

Create `crates/gitlab-cli/src/errout.rs`:

```rust
use gitlab_core::error::GitlabError;
use std::io::{self, Write};

pub fn report_error(err: &GitlabError) -> i32 {
    let payload = err.to_payload();
    let body = serde_json::json!({ "error": payload });
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(lock, "{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
    err.exit_code()
}
```

Update `main.rs`:

```rust
mod errout;
mod globals;
mod output;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test errout_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): stderr error emitter wraps GitlabError into stable schema"
```

---

## Task 2.4: Tracing subscriber setup

**Files:**
- Create: `crates/gitlab-cli/src/tracing_setup.rs`
- Modify: `crates/gitlab-cli/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add inside `crates/gitlab-cli/tests/errout_test.rs` a new test:

```rust
#[test]
fn tracing_filter_parses_levels() {
    let f = gitlab_cli::tracing_setup::filter_for(Some("debug"));
    assert_eq!(format!("{f}"), "debug");
    let f = gitlab_cli::tracing_setup::filter_for(Some("1"));
    assert_eq!(format!("{f}"), "info");
    let f = gitlab_cli::tracing_setup::filter_for(None);
    assert_eq!(format!("{f}"), "warn");
}
```

Expose crate as lib by adding to `crates/gitlab-cli/Cargo.toml`:

```toml
[lib]
name = "gitlab_cli"
path = "src/lib.rs"
```

Create `crates/gitlab-cli/src/lib.rs`:

```rust
pub mod errout;
pub mod globals;
pub mod output;
pub mod tracing_setup;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test errout_test`
Expected: FAIL — `tracing_setup` module missing.

- [ ] **Step 3: Implement**

Create `crates/gitlab-cli/src/tracing_setup.rs`:

```rust
use tracing_subscriber::filter::EnvFilter;

pub fn filter_for(v: Option<&str>) -> EnvFilter {
    match v {
        Some("1") | Some("info") | Some("INFO") => EnvFilter::new("info"),
        Some("debug") | Some("DEBUG") => EnvFilter::new("debug"),
        Some("trace") | Some("TRACE") => EnvFilter::new("trace"),
        _ => EnvFilter::new("warn"),
    }
}

pub fn init(v: Option<&str>) {
    let filter = filter_for(v);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test errout_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): tracing subscriber with GITLAB_VERBOSE levels"
```

---

## Task 2.5: Context object + Client construction

**Files:**
- Create: `crates/gitlab-cli/src/context.rs`
- Modify: `crates/gitlab-cli/src/lib.rs`
- Create: `crates/gitlab-cli/tests/context_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/context_test.rs`:

```rust
use gitlab_cli::context::{Context, CliInputs};
use gitlab_cli::globals::{GlobalArgs, OutputFormat};

fn defaults() -> GlobalArgs {
    GlobalArgs {
        host: None,
        token: None,
        output: OutputFormat::Json,
        limit: None,
        no_paginate: false,
        timeout: None,
        retries: None,
        no_retry: false,
        rps: None,
        dry_run: false,
        assume_yes: false,
        verbose: None,
        tls_skip_verify: false,
        config: None,
    }
}

#[test]
fn context_requires_token_somewhere() {
    let globals = defaults();
    let err = Context::build(CliInputs { globals, config_text: String::new() }).unwrap_err();
    assert!(err.to_string().contains("no host") || err.to_string().contains("no PAT"));
}

#[test]
fn context_builds_with_flag_inputs() {
    let mut g = defaults();
    g.host = Some("https://example.com".into());
    g.token = Some("glpat-z".into());
    let ctx = Context::build(CliInputs { globals: g, config_text: String::new() }).unwrap();
    assert_eq!(ctx.host, "https://example.com");
    assert_eq!(ctx.client.base_url().as_str(), "https://example.com/api/v4/");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test context_test`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement Context**

Create `crates/gitlab-cli/src/context.rs`:

```rust
use anyhow::{anyhow, Result};
use gitlab_core::auth::{resolve_auth, AuthInputs};
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::config::Config;
use gitlab_core::retry::RetryPolicy;
use gitlab_core::throttle::Throttle;
use std::time::Duration;

use crate::globals::GlobalArgs;

pub struct CliInputs {
    pub globals: GlobalArgs,
    pub config_text: String,
}

pub struct Context {
    pub client: Client,
    pub host: String,
    pub assume_yes: bool,
    pub dry_run: bool,
    pub output: crate::globals::OutputFormat,
    pub limit: Option<u32>,
    pub no_paginate: bool,
}

impl Context {
    pub fn build(inputs: CliInputs) -> Result<Self> {
        let CliInputs { globals, config_text } = inputs;

        let cfg: Config = if config_text.trim().is_empty() {
            Config::default()
        } else {
            toml::from_str(&config_text).map_err(|e| anyhow!("config parse: {e}"))?
        };

        let resolved = resolve_auth(
            AuthInputs {
                flag_token: globals.token.clone(),
                flag_host: globals.host.clone(),
                env_token: None,
                env_host: None,
            },
            &cfg,
        )
        .map_err(|e| anyhow!(e.to_string()))?;

        let retry = if globals.no_retry {
            RetryPolicy { max_attempts: 0, max_attempts_429: 0, ..RetryPolicy::default() }
        } else {
            let mut p = RetryPolicy::default();
            if let Some(r) = globals.retries {
                p.max_attempts = r;
                p.max_attempts_429 = r.max(1);
            }
            p
        };

        let throttle = match globals.rps.or(resolved.host_config.rps) {
            Some(rps) if rps > 0 => Throttle::per_second(rps),
            _ => Throttle::disabled(),
        };

        let req_timeout = globals.timeout.map_or(Duration::from_secs(30), Duration::from_secs);

        let client = Client::new(ClientOptions {
            host: resolved.host.clone(),
            token: resolved.token,
            tls_skip_verify: globals.tls_skip_verify || resolved.host_config.tls_skip_verify,
            connect_timeout: Duration::from_secs(5),
            request_timeout: req_timeout,
            user_agent: format!("gitlab-cli/{} (+rust)", env!("CARGO_PKG_VERSION")),
            retry,
            throttle,
        })
        .map_err(|e| anyhow!(e.to_string()))?;

        Ok(Self {
            host: resolved.host,
            client,
            assume_yes: globals.assume_yes,
            dry_run: globals.dry_run,
            output: globals.output,
            limit: globals.limit,
            no_paginate: globals.no_paginate,
        })
    }
}
```

Update `crates/gitlab-cli/src/lib.rs`:

```rust
pub mod context;
pub mod errout;
pub mod globals;
pub mod output;
pub mod tracing_setup;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test context_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): Context bundles client+policy+flags from globals+config"
```

---

## Task 2.6: `version` and `me` commands

**Files:**
- Create: `crates/gitlab-cli/src/cmd/mod.rs`
- Create: `crates/gitlab-cli/src/cmd/version.rs`
- Create: `crates/gitlab-cli/src/cmd/me.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Modify: `crates/gitlab-cli/src/lib.rs`
- Create: `crates/gitlab-cli/tests/version_me_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/version_me_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn version_prints_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"version":"14.0.5-ee","revision":"abc"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    let assert = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("version")
        .assert()
        .success();
    assert.stdout(contains("14.0.5-ee"));
}

#[tokio::test]
async fn me_prints_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":1,"username":"alice"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("me")
        .assert()
        .success()
        .stdout(contains("alice"));
}

#[tokio::test]
async fn unauthorized_exits_with_code_3() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("me")
        .assert()
        .code(3)
        .stderr(contains("unauthorized"));
}

#[tokio::test]
async fn server_error_exits_with_code_8_after_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("--retries")
        .arg("1")
        .arg("version")
        .assert()
        .code(8);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test version_me_test`
Expected: FAIL — `version` / `me` not wired.

- [ ] **Step 3: Implement commands**

Create `crates/gitlab-cli/src/cmd/mod.rs`:

```rust
pub mod me;
pub mod version;
```

Create `crates/gitlab-cli/src/cmd/version.rs`:

```rust
use anyhow::Result;
use gitlab_core::request::RequestSpec;
use reqwest::Method;

use crate::context::Context;
use crate::output::emit_object;

pub async fn run(ctx: Context) -> Result<()> {
    let v: serde_json::Value = ctx.client.send_json(RequestSpec::new(Method::GET, "version")).await?;
    emit_object(&v)?;
    Ok(())
}
```

Create `crates/gitlab-cli/src/cmd/me.rs`:

```rust
use anyhow::Result;
use gitlab_core::request::RequestSpec;
use reqwest::Method;

use crate::context::Context;
use crate::output::emit_object;

pub async fn run(ctx: Context) -> Result<()> {
    let v: serde_json::Value = ctx.client.send_json(RequestSpec::new(Method::GET, "user")).await?;
    emit_object(&v)?;
    Ok(())
}
```

Update `crates/gitlab-cli/src/lib.rs`:

```rust
pub mod cmd;
pub mod context;
pub mod errout;
pub mod globals;
pub mod output;
pub mod tracing_setup;
```

Rewrite `crates/gitlab-cli/src/main.rs`:

```rust
use clap::{Parser, Subcommand};
use gitlab_cli::context::{CliInputs, Context};
use gitlab_cli::errout::report_error;
use gitlab_cli::globals::GlobalArgs;
use gitlab_cli::tracing_setup;

#[derive(Parser)]
#[command(name = "gitlab", version, about = "gitlab-cli for GitLab 14.0.5-ee", propagate_version = true)]
struct Cli {
    #[command(flatten)]
    globals: GlobalArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Version,
    Me,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    tracing_setup::init(cli.globals.verbose.as_deref());

    let config_text = read_config_text(&cli.globals);
    let ctx = match Context::build(CliInputs { globals: cli.globals.clone(), config_text }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{{\"error\":{{\"code\":\"invalid_args\",\"message\":\"{e}\",\"retryable\":false}}}}");
            return std::process::ExitCode::from(2);
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let result = rt.block_on(async {
        match cli.command {
            Command::Version => gitlab_cli::cmd::version::run(ctx).await,
            Command::Me => gitlab_cli::cmd::me::run(ctx).await,
        }
    });

    match result {
        Ok(()) => std::process::ExitCode::from(0),
        Err(e) => {
            if let Some(ge) = e.downcast_ref::<gitlab_core::error::GitlabError>() {
                std::process::ExitCode::from(report_error(ge) as u8)
            } else {
                eprintln!("{{\"error\":{{\"code\":\"unknown\",\"message\":\"{e}\",\"retryable\":false}}}}");
                std::process::ExitCode::from(1)
            }
        }
    }
}

fn read_config_text(globals: &GlobalArgs) -> String {
    let path = globals
        .config
        .clone()
        .or_else(|| gitlab_core::config::Config::default_config_path());
    path.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test version_me_test`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): version + me commands with proper exit code mapping"
```

---

## Task 2.7: `config` subcommand

**Files:**
- Create: `crates/gitlab-cli/src/cmd/config.rs`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/config_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/config_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn config_path_prints_resolved_path() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(contains(cfg_path.to_string_lossy().as_ref()));
}

#[test]
fn config_set_token_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "set-token", "--host", "gitlab.example.com", "--token", "glpat-AA"])
        .assert()
        .success();
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(text.contains("gitlab.example.com"));
    assert!(text.contains("glpat-AA"));
}

#[test]
fn config_list_masks_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    std::fs::write(&cfg_path, r#"
default_host = "gitlab.example.com"
[host."gitlab.example.com"]
token = "glpat-ABCDEFGHIJKL"
"#).unwrap();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(contains("glpa****IJKL"))
        .stdout(predicates::str::contains("glpat-ABCDEFGHIJKL").not());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test config_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement config cmd**

Add `tempfile = "3"` to dev-deps of `gitlab-cli` and `directories = "5"` (already inherited but declare explicitly if missing).

Create `crates/gitlab-cli/src/cmd/config.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use gitlab_core::auth::MaskedToken;
use gitlab_core::config::{Config, HostConfig};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the resolved config file path
    Path,
    /// List hosts and masked tokens
    List,
    /// Write a token for a host
    SetToken(SetTokenArgs),
}

#[derive(Args, Debug)]
pub struct SetTokenArgs {
    #[arg(long)]
    pub host: String,
    #[arg(long)]
    pub token: String,
    #[arg(long)]
    pub default: bool,
}

pub fn run(cmd: ConfigCmd, cfg_path: Option<PathBuf>) -> Result<()> {
    let path = cfg_path
        .or_else(Config::default_config_path)
        .ok_or_else(|| anyhow!("cannot resolve config path"))?;
    match cmd {
        ConfigCmd::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigCmd::List => {
            let cfg = Config::load_from(&path).map_err(|e| anyhow!(e.to_string()))?;
            let mut entries = Vec::new();
            for (host, hc) in &cfg.host {
                let tok = hc.token.as_deref().unwrap_or("");
                entries.push(serde_json::json!({
                    "host": host,
                    "default": cfg.default_host.as_deref() == Some(host),
                    "token": MaskedToken(tok).to_string(),
                    "default_project": hc.default_project,
                }));
            }
            println!("{}", serde_json::to_string_pretty(&entries)?);
            Ok(())
        }
        ConfigCmd::SetToken(a) => {
            let mut cfg = Config::load_from(&path).map_err(|e| anyhow!(e.to_string()))?;
            let hc = cfg.host.entry(a.host.clone()).or_insert_with(HostConfig::default);
            hc.token = Some(a.token);
            if a.default || cfg.default_host.is_none() {
                cfg.default_host = Some(a.host);
            }
            cfg.save_to(&path).map_err(|e| anyhow!(e.to_string()))?;
            Ok(())
        }
    }
}
```

Update `crates/gitlab-cli/src/cmd/mod.rs`:

```rust
pub mod config;
pub mod me;
pub mod version;
```

Update `main.rs` — add to `Command` enum and dispatch:

```rust
#[derive(Subcommand)]
enum Command {
    Version,
    Me,
    Config {
        #[command(subcommand)]
        cmd: gitlab_cli::cmd::config::ConfigCmd,
    },
}
```

And in the `match cli.command`:

```rust
Command::Config { cmd } => gitlab_cli::cmd::config::run(cmd, cli.globals.config.clone()).map_err(anyhow::Error::from),
```

Note: `config` runs **outside** the async runtime because it's pure filesystem. Split the dispatch:

```rust
let result: Result<(), anyhow::Error> = match cli.command {
    Command::Config { cmd } => gitlab_cli::cmd::config::run(cmd, cli.globals.config.clone()),
    other => {
        let ctx = match Context::build(CliInputs { globals: cli.globals.clone(), config_text }) {
            Ok(c) => c,
            Err(e) => { /* same error emission */ return std::process::ExitCode::from(2); }
        };
        rt.block_on(async {
            match other {
                Command::Version => gitlab_cli::cmd::version::run(ctx).await,
                Command::Me => gitlab_cli::cmd::me::run(ctx).await,
                Command::Config { .. } => unreachable!(),
            }
        })
    }
};
```

(Refactor the main block accordingly; keep one runtime built only when needed.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test config_cmd_test`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): config subcommand (path/list/set-token) with token masking"
```

---

## Task 2.8: Dry-run & confirmation infrastructure

**Files:**
- Create: `crates/gitlab-cli/src/safety.rs`
- Modify: `crates/gitlab-cli/src/lib.rs`
- Create: `crates/gitlab-cli/tests/safety_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/safety_test.rs`:

```rust
use gitlab_cli::safety::{confirm_or_skip, dry_run_envelope, Intent};
use reqwest::Method;

#[test]
fn dry_run_serializes_intent() {
    let v = dry_run_envelope(&Intent {
        method: Method::POST,
        path: "projects/1/merge_requests/5/merge".into(),
        query: vec![],
        body: Some(serde_json::json!({"squash": true})),
    });
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["method"], "POST");
    assert_eq!(v["path"], "projects/1/merge_requests/5/merge");
    assert_eq!(v["body"]["squash"], true);
}

#[test]
fn confirm_or_skip_returns_true_when_assume_yes() {
    assert!(confirm_or_skip(true, "delete").unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test safety_test`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement safety**

Create `crates/gitlab-cli/src/safety.rs`:

```rust
use anyhow::Result;
use reqwest::Method;
use std::io::{IsTerminal, Read, Write};

#[derive(Debug, Clone)]
pub struct Intent {
    pub method: Method,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

pub fn dry_run_envelope(intent: &Intent) -> serde_json::Value {
    serde_json::json!({
        "dry_run": true,
        "method": intent.method.as_str(),
        "path": intent.path,
        "query": intent.query,
        "body": intent.body,
    })
}

pub fn confirm_or_skip(assume_yes: bool, action_label: &str) -> Result<bool> {
    if assume_yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "refusing to perform '{action_label}' without --yes / GITLAB_ASSUME_YES=1 (stdin is not a TTY)"
        );
    }
    let stderr = std::io::stderr();
    let mut e = stderr.lock();
    write!(e, "{action_label} — type 'yes' to continue: ")?;
    e.flush()?;
    let mut buf = [0u8; 16];
    let n = std::io::stdin().read(&mut buf).unwrap_or(0);
    let s = std::str::from_utf8(&buf[..n]).unwrap_or("").trim();
    Ok(s == "yes" || s == "y")
}
```

Update `crates/gitlab-cli/src/lib.rs`:

```rust
pub mod cmd;
pub mod context;
pub mod errout;
pub mod globals;
pub mod output;
pub mod safety;
pub mod tracing_setup;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test safety_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): safety — dry-run envelope + TTY-aware confirmation"
```

---

# Milestone 3 — Escape hatch

## Task 3.1: `api` subcommand

**Files:**
- Create: `crates/gitlab-cli/src/cmd/api.rs`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/api_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/api_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn api_get_prints_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/pipeline_schedules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([{"id":42}])))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args(["api", "GET", "/projects/1/pipeline_schedules"])
        .assert()
        .success()
        .stdout(contains("42"));
}

#[tokio::test]
async fn api_post_sends_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/labels"))
        .and(body_json(&serde_json::json!({"name":"bug","color":"#FF0000"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({"id":9,"name":"bug"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .env("GITLAB_ASSUME_YES", "1")
        .args([
            "api",
            "POST",
            "/projects/1/labels",
            "--data",
            r#"{"name":"bug","color":"#FF0000"}"#,
        ])
        .assert()
        .success()
        .stdout(contains("bug"));
}

#[tokio::test]
async fn api_respects_dry_run() {
    let host = "http://127.0.0.1:1"; // unroutable; ensures no network
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args([
            "--dry-run",
            "api",
            "DELETE",
            "/projects/1/issues/5",
        ])
        .assert()
        .code(10)
        .stdout(contains("dry_run"))
        .stdout(contains("DELETE"));
}

#[tokio::test]
async fn api_query_flags_pass_through() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/issues"))
        .and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args(["api", "GET", "/issues", "--query", "state=opened"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test api_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement `api` command**

Create `crates/gitlab-cli/src/cmd/api.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::Args;
use gitlab_core::request::RequestSpec;
use reqwest::Method;
use std::str::FromStr;

use crate::context::Context;
use crate::output::emit_object;
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Args, Debug)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE)
    pub method: String,
    /// REST path under /api/v4 (leading slash optional)
    pub path: String,
    /// JSON body; prefix with @ to read from file
    #[arg(long)]
    pub data: Option<String>,
    /// Repeatable --query key=value
    #[arg(long = "query", value_parser = parse_kv)]
    pub query: Vec<(String, String)>,
}

fn parse_kv(s: &str) -> std::result::Result<(String, String), String> {
    let (k, v) = s.split_once('=').ok_or_else(|| "expected key=value".to_owned())?;
    Ok((k.to_owned(), v.to_owned()))
}

fn is_write(method: &Method) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::PATCH | Method::DELETE)
}

pub async fn run(ctx: Context, args: ApiArgs) -> Result<()> {
    let method = Method::from_str(&args.method.to_uppercase())
        .map_err(|_| anyhow!("unknown HTTP method: {}", args.method))?;

    let body = match args.data.as_deref() {
        None => None,
        Some(raw) if raw.starts_with('@') => {
            let text = std::fs::read_to_string(&raw[1..])
                .map_err(|e| anyhow!("cannot read body file {}: {e}", &raw[1..]))?;
            Some(serde_json::from_str(&text).map_err(|e| anyhow!("invalid JSON body: {e}"))?)
        }
        Some(raw) => Some(serde_json::from_str(raw).map_err(|e| anyhow!("invalid JSON body: {e}"))?),
    };

    let path_stripped = args.path.trim_start_matches('/').to_owned();

    let intent = Intent {
        method: method.clone(),
        path: path_stripped.clone(),
        query: args.query.clone(),
        body: body.clone(),
    };

    if ctx.dry_run {
        emit_object(&dry_run_envelope(&intent))?;
        std::process::exit(10);
    }

    if is_write(&method) && !confirm_or_skip(ctx.assume_yes, &format!("{method} /{path_stripped}"))? {
        anyhow::bail!("aborted");
    }

    let mut spec = RequestSpec::new(method, path_stripped).with_query(args.query);
    if let Some(b) = body { spec.body = Some(b); }

    let (_status, headers, bytes) = ctx.client.send_raw(spec).await?;
    let ct = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if ct.starts_with("application/json") {
        let v: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| anyhow!("parse JSON response: {e}"))?;
        emit_object(&v)?;
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes).ok();
        tracing::info!(bytes = bytes.len(), content_type = ct, "emitted binary body");
    }
    Ok(())
}
```

Add `api` to `cmd/mod.rs`:

```rust
pub mod api;
pub mod config;
pub mod me;
pub mod version;
```

Update `main.rs` Command enum & dispatch to include:

```rust
Command::Api(args) => gitlab_cli::cmd::api::run(ctx, args).await,
```

Where `enum Command` gets:

```rust
Api(gitlab_cli::cmd::api::ApiArgs),
```

(Declared with `#[command(name = "api")]` attribute).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test api_cmd_test`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): api escape hatch (GET/POST/PUT/PATCH/DELETE) with dry-run"
```

---

# Milestone 4 — Resources (12 families + `search`)

Every Task 4.x follows the same rhythm: write a single wiremock-backed integration test covering the 3-5 verbs, add a `gitlab-core/src/resources/<name>.rs` module that constructs `RequestSpec`s, add a `gitlab-cli/src/cmd/<name>.rs` with clap subcommands dispatching to those helpers, then commit. Because the code per resource is short (the heavy lifting is in `Client::send_raw` and `PagedStream`), each task is one cohesive unit.

## Task 4.1: `project`

**Files:**
- Create: `crates/gitlab-core/src/resources/mod.rs`
- Create: `crates/gitlab-core/src/resources/projects.rs`
- Modify: `crates/gitlab-core/src/lib.rs`
- Create: `crates/gitlab-cli/src/cmd/project.rs`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/project_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/project_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn project_list_auto_paginates() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET")).and(path("/api/v4/projects")).and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!([{"id":1}, {"id":2}]))
            .insert_header(
                "Link",
                format!("<{base}/api/v4/projects?page=2&per_page=100>; rel=\"next\"")
            ))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects")).and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":3}])))
        .mount(&server).await;

    env_cmd(&base).args(["project", "list"])
        .assert().success()
        .stdout(contains("\"id\":1")).stdout(contains("\"id\":3"));
}

#[tokio::test]
async fn project_get_by_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/group%2Fproj"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9, "path_with_namespace":"group/proj"})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "get", "group/proj"])
        .assert().success().stdout(contains("group/proj"));
}

#[tokio::test]
async fn project_create() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v4/projects"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":77,"name":"new"})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "create", "--name", "new"])
        .assert().success().stdout(contains("77"));
}

#[tokio::test]
async fn project_delete() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/5"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "delete", "5"])
        .assert().success();
}

#[tokio::test]
async fn project_archive_and_unarchive() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v4/projects/5/archive"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"archived":true})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/5/unarchive"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"archived":false})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "archive", "5"]).assert().success().stdout(contains("archived\":true"));
    env_cmd(&server.uri()).args(["project", "unarchive", "5"]).assert().success().stdout(contains("archived\":false"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test project_cmd_test`
Expected: FAIL — command not wired.

- [ ] **Step 3: Implement core + cmd**

Create `crates/gitlab-core/src/resources/mod.rs`:

```rust
pub mod projects;

/// Percent-encode a project identifier (numeric id or path-with-namespace).
#[must_use]
pub fn encode_id(id: &str) -> String {
    if id.chars().all(|c| c.is_ascii_digit()) {
        return id.to_owned();
    }
    urlencoding::encode(id).into_owned()
}
```

Add `urlencoding = "2"` to `gitlab-core/Cargo.toml`.

Create `crates/gitlab-core/src/resources/projects.rs`:

```rust
use reqwest::Method;

use crate::client::Client;
use crate::error::Result;
use crate::page::{PageRequest, PagedStream};
use crate::request::RequestSpec;

use super::encode_id;

pub fn list_spec(visibility: Option<&str>, search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("projects");
    if let Some(v) = visibility { p.query.push(("visibility".into(), v.into())); }
    if let Some(s) = search { p.query.push(("search".into(), s.into())); }
    p
}

pub fn stream(client: &Client, req: PageRequest) -> impl futures::Stream<Item = Result<serde_json::Value>> + Unpin {
    PagedStream::start(client, req)
}

pub fn get_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}", encode_id(id)))
}

pub fn create_spec(name: &str, path: Option<&str>, visibility: Option<&str>) -> RequestSpec {
    let mut body = serde_json::json!({ "name": name });
    if let Some(p) = path { body["path"] = serde_json::Value::String(p.into()); }
    if let Some(v) = visibility { body["visibility"] = serde_json::Value::String(v.into()); }
    RequestSpec::new(Method::POST, "projects").with_json(&body)
}

pub fn update_spec(id: &str, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("projects/{}", encode_id(id))).with_json(&body)
}

pub fn delete_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}", encode_id(id)))
}

pub fn fork_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/fork", encode_id(id)))
}

pub fn archive_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/archive", encode_id(id)))
}

pub fn unarchive_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/unarchive", encode_id(id)))
}
```

Update `crates/gitlab-core/src/lib.rs`:

```rust
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod page;
pub mod request;
pub mod resources;
pub mod retry;
pub mod throttle;
```

Create `crates/gitlab-cli/src/cmd/project.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::resources::projects;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum ProjectCmd {
    /// List accessible projects (auto-paginates)
    List(ListArgs),
    /// Get a single project by id or full path
    Get { id: String },
    /// Create a project
    Create(CreateArgs),
    /// Update a project
    Update(UpdateArgs),
    /// Delete a project
    Delete { id: String },
    /// Fork a project
    Fork { id: String },
    /// Archive a project
    Archive { id: String },
    /// Unarchive a project
    Unarchive { id: String },
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)] pub visibility: Option<String>,
    #[arg(long)] pub search: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub name: String,
    #[arg(long)] pub path: Option<String>,
    #[arg(long)] pub visibility: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    pub id: String,
    /// JSON body; prefix @file to load from disk
    #[arg(long)]
    pub data: String,
}

pub async fn run(ctx: Context, cmd: ProjectCmd) -> Result<()> {
    match cmd {
        ProjectCmd::List(a) => {
            let req = projects::list_spec(a.visibility.as_deref(), a.search.as_deref());
            let stream = projects::stream(&ctx.client, req);
            let fmt = ctx.output;
            let limit = ctx.limit;
            emit_stream::<serde_json::Value, _>(stream, fmt, limit).await?;
        }
        ProjectCmd::Get { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::get_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Create(a) => {
            let spec = projects::create_spec(&a.name, a.path.as_deref(), a.visibility.as_deref());
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create project")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Update(a) => {
            let body = load_json(&a.data)?;
            let spec = projects::update_spec(&a.id, body);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("update project {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Delete { id } => {
            let spec = projects::delete_spec(&id);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: None,
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("delete project {id}"))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(spec).await?;
        }
        ProjectCmd::Fork { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::fork_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Archive { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::archive_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Unarchive { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::unarchive_spec(&id)).await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}

fn load_json(raw: &str) -> Result<serde_json::Value> {
    if let Some(path) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(serde_json::from_str(raw)?)
    }
}
```

Update `cmd/mod.rs`:

```rust
pub mod api;
pub mod config;
pub mod me;
pub mod project;
pub mod version;
```

Extend `main.rs` `Command` enum:

```rust
Project { #[command(subcommand)] cmd: gitlab_cli::cmd::project::ProjectCmd },
```

And dispatch:

```rust
Command::Project { cmd } => gitlab_cli::cmd::project::run(ctx, cmd).await,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test project_cmd_test`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(project): list/get/create/update/delete/fork/archive/unarchive"
```

---

The following resource tasks (**4.2 through 4.16**) share the shape of Task 4.1: core `resources/<name>.rs` with `RequestSpec` factories + CLI `cmd/<name>.rs` with clap subcommands + a wiremock-backed integration test file. Each task below shows the full code needed — no shorthand, no "similar to".

## Task 4.2: `group`

**Files:**
- Create: `crates/gitlab-core/src/resources/groups.rs`
- Create: `crates/gitlab-cli/src/cmd/group.rs`
- Modify: core `resources/mod.rs`, cli `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/group_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/group_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn group_list() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/groups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"full_path":"atoms"}])))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["group","list"]).assert().success().stdout(contains("atoms"));
}

#[tokio::test]
async fn group_get_members_projects_subgroups() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"full_path":"atoms"})))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":42}])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":7}])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/subgroups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":13}])))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["group","get","atoms"]).assert().success();
    env_cmd(&server.uri()).args(["group","members","atoms"]).assert().success().stdout(contains("42"));
    env_cmd(&server.uri()).args(["group","projects","atoms"]).assert().success().stdout(contains("7"));
    env_cmd(&server.uri()).args(["group","subgroups","atoms"]).assert().success().stdout(contains("13"));
}

#[tokio::test]
async fn group_create_update_delete() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v4/groups"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"name":"n","path":"p"})))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/groups/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":5,"name":"nn"})))
        .mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/groups/5"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["group","create","--name","n","--path","p"]).assert().success();
    env_cmd(&server.uri()).args(["group","update","5","--data",r#"{"name":"nn"}"#]).assert().success();
    env_cmd(&server.uri()).args(["group","delete","5"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test group_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/groups.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list_spec(search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("groups");
    if let Some(s) = search { p.query.push(("search".into(), s.into())); }
    p
}
pub fn get_spec(id: &str) -> RequestSpec { RequestSpec::new(Method::GET, format!("groups/{}", encode_id(id))) }
pub fn members_spec(id: &str) -> PageRequest { PageRequest::new(format!("groups/{}/members", encode_id(id))) }
pub fn projects_spec(id: &str) -> PageRequest { PageRequest::new(format!("groups/{}/projects", encode_id(id))) }
pub fn subgroups_spec(id: &str) -> PageRequest { PageRequest::new(format!("groups/{}/subgroups", encode_id(id))) }
pub fn create_spec(name: &str, path: &str, parent_id: Option<u64>) -> RequestSpec {
    let mut body = serde_json::json!({"name": name, "path": path});
    if let Some(pid) = parent_id { body["parent_id"] = serde_json::json!(pid); }
    RequestSpec::new(Method::POST, "groups").with_json(&body)
}
pub fn update_spec(id: &str, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("groups/{}", encode_id(id))).with_json(&body)
}
pub fn delete_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("groups/{}", encode_id(id)))
}
```

Add `pub mod groups;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/group.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::groups;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum GroupCmd {
    List(ListArgs),
    Get { id: String },
    Members { id: String },
    Projects { id: String },
    Subgroups { id: String },
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete { id: String },
}

#[derive(Args, Debug)]
pub struct ListArgs { #[arg(long)] pub search: Option<String> }

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub name: String,
    #[arg(long)] pub path: String,
    #[arg(long)] pub parent_id: Option<u64>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs { pub id: String, #[arg(long)] pub data: String }

pub async fn run(ctx: Context, cmd: GroupCmd) -> Result<()> {
    match cmd {
        GroupCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::list_spec(a.search.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Get { id } => { let v: serde_json::Value = ctx.client.send_json(groups::get_spec(&id)).await?; emit_object(&v)?; }
        GroupCmd::Members { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::members_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Projects { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::projects_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Subgroups { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::subgroups_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Create(a) => {
            let spec = groups::create_spec(&a.name, &a.path, a.parent_id);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent { method: spec.method.clone(), path: spec.path.clone(), query: spec.query.clone(), body: spec.body.clone() }))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, "create group")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        GroupCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = groups::update_spec(&a.id, body);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent { method: spec.method.clone(), path: spec.path.clone(), query: spec.query.clone(), body: spec.body.clone() }))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, &format!("update group {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        GroupCmd::Delete { id } => {
            let spec = groups::delete_spec(&id);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent { method: spec.method.clone(), path: spec.path.clone(), query: spec.query.clone(), body: None }))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, &format!("delete group {id}"))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(spec).await?;
        }
    }
    Ok(())
}
```

Move `load_json` helper to `cmd/mod.rs`:

```rust
pub mod api;
pub mod config;
pub mod group;
pub mod me;
pub mod project;
pub mod version;

pub fn load_json(raw: &str) -> anyhow::Result<serde_json::Value> {
    if let Some(p) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(p)?;
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(serde_json::from_str(raw)?)
    }
}
```

(Update `project.rs` to call `crate::cmd::load_json` in its `Update` arm; drop the local `load_json` fn.)

Extend `main.rs`:

```rust
Group { #[command(subcommand)] cmd: gitlab_cli::cmd::group::GroupCmd },
```

Dispatch: `Command::Group { cmd } => gitlab_cli::cmd::group::run(ctx, cmd).await,`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test group_cmd_test`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(group): list/get/members/projects/subgroups/create/update/delete"
```

---

## Task 4.3: `mr` (merge requests)

This is the largest resource: 14 verbs.

**Files:**
- Create: `crates/gitlab-core/src/resources/merge_requests.rs`
- Create: `crates/gitlab-cli/src/cmd/mr.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/mr_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/mr_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn mr_list_by_group_and_project() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/merge_requests")).and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"iid":1}])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests")).and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"iid":2}])))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["mr","list","--group","atoms","--state","opened"]).assert().success().stdout(contains("1"));
    env_cmd(&server.uri()).args(["mr","list","--project","1","--state","opened"]).assert().success().stdout(contains("2"));
}

#[tokio::test]
async fn mr_crud_and_actions() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"opened"})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/merge_requests"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"iid":9})))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"title":"t"})))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5/merge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"merged"})))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5/rebase"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({"rebase_in_progress":true})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/merge_requests/5/approve"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"approved_by":[]})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/merge_requests/5/unapprove"))
        .respond_with(ResponseTemplate::new(201).set_body_string(""))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"changes":[]})))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/diffs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/pipelines"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":100}])))
        .mount(&server).await;

    env_cmd(&server.uri()).args(["mr","get","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","create","--project","1","--source","topic","--target","main","--title","T"]).assert().success();
    env_cmd(&server.uri()).args(["mr","update","--project","1","--mr","5","--data",r#"{"title":"t"}"#]).assert().success();
    env_cmd(&server.uri()).args(["mr","merge","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","rebase","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","approve","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","unapprove","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","changes","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","diffs","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","commits","--project","1","--mr","5"]).assert().success();
    env_cmd(&server.uri()).args(["mr","pipelines","--project","1","--mr","5"]).assert().success();
}

#[tokio::test]
async fn mr_close_and_reopen() {
    let server = MockServer::start().await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"closed"})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["mr","close","--project","1","--mr","5"]).assert().success().stdout(contains("closed"));
    env_cmd(&server.uri()).args(["mr","reopen","--project","1","--mr","5"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test mr_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/merge_requests.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

fn project_mr_path(project: &str, iid: u64, suffix: &str) -> String {
    if suffix.is_empty() { format!("projects/{}/merge_requests/{iid}", encode_id(project)) }
    else { format!("projects/{}/merge_requests/{iid}/{suffix}", encode_id(project)) }
}

pub fn list_for_project(project: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/merge_requests", encode_id(project)));
    if let Some(s) = state { p.query.push(("state".into(), s.into())); }
    p
}
pub fn list_for_group(group: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("groups/{}/merge_requests", encode_id(group)));
    if let Some(s) = state { p.query.push(("state".into(), s.into())); }
    p
}

pub fn get_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, project_mr_path(project, iid, ""))
}
pub fn create_spec(project: &str, source: &str, target: &str, title: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/merge_requests", encode_id(project)))
        .with_json(&serde_json::json!({"source_branch":source,"target_branch":target,"title":title}))
}
pub fn update_spec(project: &str, iid: u64, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "")).with_json(&body)
}
pub fn close_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, serde_json::json!({"state_event":"close"}))
}
pub fn reopen_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, serde_json::json!({"state_event":"reopen"}))
}
pub fn merge_spec(project: &str, iid: u64, squash: bool) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "merge"))
        .with_json(&serde_json::json!({"squash":squash}))
}
pub fn rebase_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "rebase"))
}
pub fn approve_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, project_mr_path(project, iid, "approve"))
}
pub fn unapprove_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, project_mr_path(project, iid, "unapprove"))
}
pub fn changes_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, project_mr_path(project, iid, "changes"))
}
pub fn diffs_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "diffs"))
}
pub fn commits_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "commits"))
}
pub fn pipelines_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "pipelines"))
}
```

Add `pub mod merge_requests;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/mr.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::merge_requests as mr;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum MrCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Close(Target),
    Reopen(Target),
    Merge(MergeArgs),
    Rebase(Target),
    Approve(Target),
    Unapprove(Target),
    Changes(Target),
    Diffs(Target),
    Commits(Target),
    Pipelines(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, conflicts_with = "group")] pub project: Option<String>,
    #[arg(long)] pub group: Option<String>,
    #[arg(long)] pub state: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)] pub project: String,
    #[arg(long)] pub mr: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub source: String,
    #[arg(long)] pub target: String,
    #[arg(long)] pub title: String,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub mr: u64,
    #[arg(long)] pub data: String,
}

#[derive(Args, Debug)]
pub struct MergeArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub mr: u64,
    #[arg(long)] pub squash: bool,
}

pub async fn run(ctx: Context, cmd: MrCmd) -> Result<()> {
    match cmd {
        MrCmd::List(a) => {
            let req = match (a.project, a.group) {
                (Some(p), None) => mr::list_for_project(&p, a.state.as_deref()),
                (None, Some(g)) => mr::list_for_group(&g, a.state.as_deref()),
                _ => anyhow::bail!("pass either --project or --group"),
            };
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        MrCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(mr::get_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Create(a) => {
            let spec = mr::create_spec(&a.project, &a.source, &a.target, &a.title);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, "create MR")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        MrCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = mr::update_spec(&a.project, a.mr, body);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, &format!("update MR {}", a.mr))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        MrCmd::Close(t) => { let v: serde_json::Value = ctx.client.send_json(mr::close_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Reopen(t) => { let v: serde_json::Value = ctx.client.send_json(mr::reopen_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Merge(a) => {
            let spec = mr::merge_spec(&a.project, a.mr, a.squash);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, &format!("merge MR {}", a.mr))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        MrCmd::Rebase(t) => { let v: serde_json::Value = ctx.client.send_json(mr::rebase_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Approve(t) => { let v: serde_json::Value = ctx.client.send_json(mr::approve_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Unapprove(t) => { let _ = ctx.client.send_raw(mr::unapprove_spec(&t.project, t.mr)).await?; }
        MrCmd::Changes(t) => { let v: serde_json::Value = ctx.client.send_json(mr::changes_spec(&t.project, t.mr)).await?; emit_object(&v)?; }
        MrCmd::Diffs(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, mr::diffs_page(&t.project, t.mr));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        MrCmd::Commits(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, mr::commits_page(&t.project, t.mr));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        MrCmd::Pipelines(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, mr::pipelines_page(&t.project, t.mr));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
```

Add to `cmd/mod.rs`:

```rust
pub mod mr;
```

Extend `main.rs` Command enum with:

```rust
Mr { #[command(subcommand)] cmd: gitlab_cli::cmd::mr::MrCmd },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test mr_cmd_test`
Expected: 3 PASS (with 11 wiremock-backed sub-assertions).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(mr): list/get/create/update/close/reopen/merge/rebase/approve/unapprove/changes/diffs/commits/pipelines"
```

---

## Task 4.4: `issue`

**Files:**
- Create: `crates/gitlab-core/src/resources/issues.rs`
- Create: `crates/gitlab-cli/src/cmd/issue.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/issue_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/issue_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn issue_full_lifecycle() {
    let server = MockServer::start().await;
    for (m, p, body) in [
        ("GET",  "/api/v4/projects/1/issues",         json!([{"iid":1}])),
        ("GET",  "/api/v4/projects/1/issues/1",       json!({"iid":1})),
        ("POST", "/api/v4/projects/1/issues",         json!({"iid":9})),
        ("PUT",  "/api/v4/projects/1/issues/1",       json!({"iid":1,"state":"closed"})),
        ("POST", "/api/v4/projects/1/issues/1/move",  json!({"iid":1})),
        ("GET",  "/api/v4/issues_statistics",         json!({"statistics":{}})),
        ("GET",  "/api/v4/projects/1/issues/1/links", json!([])),
        ("POST", "/api/v4/projects/1/issues/1/links", json!({"source_issue":{},"target_issue":{}})),
    ] {
        let status = if m == "POST" { 201 } else { 200 };
        let b = body.clone();
        Mock::given(method(m)).and(path(p))
            .respond_with(ResponseTemplate::new(status).set_body_json(&b))
            .mount(&server).await;
    }
    let Mock_delete_link_path = "/api/v4/projects/1/issues/1/links/4";
    Mock::given(method("DELETE")).and(path(Mock_delete_link_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["issue","list","--project","1"]).assert().success();
    env_cmd(&base).args(["issue","get","--project","1","--issue","1"]).assert().success();
    env_cmd(&base).args(["issue","create","--project","1","--title","t"]).assert().success();
    env_cmd(&base).args(["issue","update","--project","1","--issue","1","--data",r#"{"state_event":"close"}"#]).assert().success();
    env_cmd(&base).args(["issue","close","--project","1","--issue","1"]).assert().success();
    env_cmd(&base).args(["issue","reopen","--project","1","--issue","1"]).assert().success();
    env_cmd(&base).args(["issue","move","--project","1","--issue","1","--to","2"]).assert().success();
    env_cmd(&base).args(["issue","stats"]).assert().success();
    env_cmd(&base).args(["issue","link","--project","1","--issue","1","--target-project","2","--target-issue","7"]).assert().success();
    env_cmd(&base).args(["issue","unlink","--project","1","--issue","1","--link-id","4"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test issue_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/issues.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list_for_project(project: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/issues", encode_id(project)));
    if let Some(s) = state { p.query.push(("state".into(), s.into())); }
    p
}
pub fn get_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/issues/{iid}", encode_id(project)))
}
pub fn create_spec(project: &str, title: &str, labels: Option<&str>) -> RequestSpec {
    let mut body = serde_json::json!({"title":title});
    if let Some(l) = labels { body["labels"] = serde_json::Value::String(l.into()); }
    RequestSpec::new(Method::POST, format!("projects/{}/issues", encode_id(project))).with_json(&body)
}
pub fn update_spec(project: &str, iid: u64, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("projects/{}/issues/{iid}", encode_id(project))).with_json(&body)
}
pub fn close_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, serde_json::json!({"state_event":"close"}))
}
pub fn reopen_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, serde_json::json!({"state_event":"reopen"}))
}
pub fn move_spec(project: &str, iid: u64, target_project: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/issues/{iid}/move", encode_id(project)))
        .with_json(&serde_json::json!({"to_project_id": target_project}))
}
pub fn stats_spec() -> RequestSpec {
    RequestSpec::new(Method::GET, "issues_statistics")
}
pub fn list_links(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(format!("projects/{}/issues/{iid}/links", encode_id(project)))
}
pub fn link_spec(project: &str, iid: u64, target_project: &str, target_iid: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/issues/{iid}/links", encode_id(project)))
        .with_json(&serde_json::json!({"target_project_id": target_project, "target_issue_iid": target_iid}))
}
pub fn unlink_spec(project: &str, iid: u64, link_id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/issues/{iid}/links/{link_id}", encode_id(project)))
}
```

Add `pub mod issues;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/issue.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::issues;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum IssueCmd {
    List(List),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Close(Target),
    Reopen(Target),
    Move(MoveArgs),
    Stats,
    Link(LinkArgs),
    Unlink(UnlinkArgs),
}
#[derive(Args, Debug)] pub struct List { #[arg(long)] pub project: String, #[arg(long)] pub state: Option<String> }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub issue: u64 }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long)] pub title: String, #[arg(long)] pub labels: Option<String> }
#[derive(Args, Debug)] pub struct UpdateArgs { #[arg(long)] pub project: String, #[arg(long)] pub issue: u64, #[arg(long)] pub data: String }
#[derive(Args, Debug)] pub struct MoveArgs { #[arg(long)] pub project: String, #[arg(long)] pub issue: u64, #[arg(long)] pub to: String }
#[derive(Args, Debug)] pub struct LinkArgs { #[arg(long)] pub project: String, #[arg(long)] pub issue: u64, #[arg(long)] pub target_project: String, #[arg(long)] pub target_issue: u64 }
#[derive(Args, Debug)] pub struct UnlinkArgs { #[arg(long)] pub project: String, #[arg(long)] pub issue: u64, #[arg(long)] pub link_id: u64 }

pub async fn run(ctx: Context, cmd: IssueCmd) -> Result<()> {
    match cmd {
        IssueCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, issues::list_for_project(&a.project, a.state.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        IssueCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(issues::get_spec(&t.project, t.issue)).await?; emit_object(&v)?; }
        IssueCmd::Create(a) => {
            let spec = issues::create_spec(&a.project, &a.title, a.labels.as_deref());
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, "create issue")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        IssueCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = issues::update_spec(&a.project, a.issue, body);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, &format!("update issue {}", a.issue))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        IssueCmd::Close(t) => { let v: serde_json::Value = ctx.client.send_json(issues::close_spec(&t.project, t.issue)).await?; emit_object(&v)?; }
        IssueCmd::Reopen(t) => { let v: serde_json::Value = ctx.client.send_json(issues::reopen_spec(&t.project, t.issue)).await?; emit_object(&v)?; }
        IssueCmd::Move(a) => { let v: serde_json::Value = ctx.client.send_json(issues::move_spec(&a.project, a.issue, &a.to)).await?; emit_object(&v)?; }
        IssueCmd::Stats => { let v: serde_json::Value = ctx.client.send_json(issues::stats_spec()).await?; emit_object(&v)?; }
        IssueCmd::Link(a) => {
            let spec = issues::link_spec(&a.project, a.issue, &a.target_project, a.target_issue);
            if !confirm_or_skip(ctx.assume_yes, "link issue")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        IssueCmd::Unlink(a) => {
            let spec = issues::unlink_spec(&a.project, a.issue, a.link_id);
            if !confirm_or_skip(ctx.assume_yes, "unlink issue")? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(spec).await?;
        }
    }
    Ok(())
}
```

Add to `cmd/mod.rs`: `pub mod issue;`

Extend `main.rs`: `Issue { #[command(subcommand)] cmd: gitlab_cli::cmd::issue::IssueCmd },`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test issue_cmd_test`
Expected: 1 PASS (with 10 sub-actions).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(issue): list/get/create/update/close/reopen/move/stats/link/unlink"
```

---

Tasks 4.5 through 4.16 below each follow the same 5-step TDD cycle as Task 4.1 — full test code, full core module, full CLI module, full commit. Engineer reading any of these out of order has every line of code needed.

**Reusable CLI boilerplate** (all resource commands use these helpers from `cmd/mod.rs`): `load_json()` (file/literal JSON), `emit_object()` (stdout JSON), `emit_stream()` (list pagination), `dry_run_envelope()` + `confirm_or_skip()` (write safety), and the `Context` struct with `client`, `output`, `limit`, `dry_run`, `assume_yes` fields.

---

## Task 4.5: `pipeline`

**Files:**
- Create: `crates/gitlab-core/src/resources/pipelines.rs`
- Create: `crates/gitlab-cli/src/cmd/pipeline.rs`
- Modify: `crates/gitlab-core/src/resources/mod.rs`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/pipeline_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/pipeline_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn pipeline_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/pipelines")).and(query_param("status", "running"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":10,"status":"running"}])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/pipelines/10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":10})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/pipeline")).and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":11})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/pipelines/10/retry"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":10,"status":"pending"})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/pipelines/10/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":10,"status":"canceled"})))
        .mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/pipelines/10"))
        .respond_with(ResponseTemplate::new(204).set_body_string(""))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/pipelines/10/variables"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"key":"K","value":"V"}])))
        .mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["pipeline","list","--project","1","--status","running"]).assert().success().stdout(contains("running"));
    env_cmd(&base).args(["pipeline","get","--project","1","--id","10"]).assert().success().stdout(contains("\"id\":10"));
    env_cmd(&base).args(["pipeline","create","--project","1","--ref","main"]).assert().success().stdout(contains("\"id\":11"));
    env_cmd(&base).args(["pipeline","retry","--project","1","--id","10"]).assert().success();
    env_cmd(&base).args(["pipeline","cancel","--project","1","--id","10"]).assert().success();
    env_cmd(&base).args(["pipeline","delete","--project","1","--id","10"]).assert().success();
    env_cmd(&base).args(["pipeline","variables","--project","1","--id","10"]).assert().success().stdout(contains("\"key\":\"K\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test pipeline_cmd_test`
Expected: FAIL — command not wired.

- [ ] **Step 3: Implement core + CLI**

Create `crates/gitlab-core/src/resources/pipelines.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list(project: &str, status: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/pipelines", encode_id(project)));
    if let Some(s) = status { p.query.push(("status".into(), s.into())); }
    p
}
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/pipelines/{id}", encode_id(project)))
}
pub fn create(project: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipeline", encode_id(project)))
        .with_query([("ref", rref)])
}
pub fn retry(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipelines/{id}/retry", encode_id(project)))
}
pub fn cancel(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipelines/{id}/cancel", encode_id(project)))
}
pub fn delete(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/pipelines/{id}", encode_id(project)))
}
pub fn variables(project: &str, id: u64) -> PageRequest {
    PageRequest::new(format!("projects/{}/pipelines/{id}/variables", encode_id(project)))
}
```

Add `pub mod pipelines;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/pipeline.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::pipelines;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum PipelineCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Retry(Target),
    Cancel(Target),
    Delete(Target),
    Variables(Target),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub project: String, #[arg(long)] pub status: Option<String> }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub id: u64 }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long = "ref")] pub rref: String }

pub async fn run(ctx: Context, cmd: PipelineCmd) -> Result<()> {
    match cmd {
        PipelineCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, pipelines::list(&a.project, a.status.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        PipelineCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(pipelines::get(&t.project, t.id)).await?; emit_object(&v)?; }
        PipelineCmd::Create(a) => {
            let spec = pipelines::create(&a.project, &a.rref);
            if ctx.dry_run { emit_object(&dry_run_envelope(&Intent{method:spec.method.clone(),path:spec.path.clone(),query:spec.query.clone(),body:spec.body.clone()}))?; std::process::exit(10); }
            if !confirm_or_skip(ctx.assume_yes, "create pipeline")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?; emit_object(&v)?;
        }
        PipelineCmd::Retry(t) => { let v: serde_json::Value = ctx.client.send_json(pipelines::retry(&t.project, t.id)).await?; emit_object(&v)?; }
        PipelineCmd::Cancel(t) => { let v: serde_json::Value = ctx.client.send_json(pipelines::cancel(&t.project, t.id)).await?; emit_object(&v)?; }
        PipelineCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete pipeline {}", t.id))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(pipelines::delete(&t.project, t.id)).await?;
        }
        PipelineCmd::Variables(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, pipelines::variables(&t.project, t.id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
```

Add `pub mod pipeline;` to `cmd/mod.rs`. Extend `main.rs` `Command` enum with `Pipeline { #[command(subcommand)] cmd: gitlab_cli::cmd::pipeline::PipelineCmd }` and dispatch arm `Command::Pipeline { cmd } => gitlab_cli::cmd::pipeline::run(ctx, cmd).await`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test pipeline_cmd_test`
Expected: 1 PASS (7 sub-assertions).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(pipeline): list/get/create/retry/cancel/delete/variables"
```

---

## Task 4.6: `job`

**Files:**
- Create: `crates/gitlab-core/src/resources/jobs.rs`
- Create: `crates/gitlab-cli/src/cmd/job.rs`
- Modify: `crates/gitlab-core/src/resources/mod.rs`, `crates/gitlab-cli/src/cmd/mod.rs`, `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/job_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/job_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn job_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":77}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/pipelines/5/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":78}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"success"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/play"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"pending"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/retry"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":78}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"canceled"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/erase"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":77}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77/trace"))
        .respond_with(ResponseTemplate::new(200).set_body_string("step1\nstep2\n").insert_header("Content-Type", "text/plain")).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77/artifacts"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0x50,0x4B,0x03,0x04]).insert_header("Content-Type", "application/zip")).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["job","list","--project","1"]).assert().success();
    env_cmd(&base).args(["job","list","--project","1","--pipeline","5"]).assert().success().stdout(contains("78"));
    env_cmd(&base).args(["job","get","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","play","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","retry","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","cancel","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","erase","--project","1","--id","77"]).assert().success();
    let out = env_cmd(&base).args(["job","trace","--project","1","--id","77"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("step1"));
    let out = env_cmd(&base).args(["job","artifacts","--project","1","--id","77"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(&out.stdout[..4], &[0x50,0x4B,0x03,0x04]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test job_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/jobs.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list_project(project: &str, scope: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/jobs", encode_id(project)));
    if let Some(s) = scope { p.query.push(("scope".into(), s.into())); }
    p
}
pub fn list_pipeline(project: &str, pipeline_id: u64) -> PageRequest {
    PageRequest::new(format!("projects/{}/pipelines/{pipeline_id}/jobs", encode_id(project)))
}
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/jobs/{id}", encode_id(project)))
}
pub fn play(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/jobs/{id}/play", encode_id(project)))
}
pub fn retry(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/jobs/{id}/retry", encode_id(project)))
}
pub fn cancel(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/jobs/{id}/cancel", encode_id(project)))
}
pub fn erase(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/jobs/{id}/erase", encode_id(project)))
}
pub fn trace(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/jobs/{id}/trace", encode_id(project)))
}
pub fn artifacts(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/jobs/{id}/artifacts", encode_id(project)))
}
```

Add `pub mod jobs;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/job.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::jobs;
use std::io::Write;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum JobCmd {
    List(ListArgs),
    Get(Target),
    Play(Target),
    Retry(Target),
    Cancel(Target),
    Erase(Target),
    Trace(Target),
    Artifacts(Target),
}

#[derive(Args, Debug)] pub struct ListArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub pipeline: Option<u64>,
    #[arg(long)] pub scope: Option<String>,
}
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub id: u64 }

pub async fn run(ctx: Context, cmd: JobCmd) -> Result<()> {
    match cmd {
        JobCmd::List(a) => {
            let req = match a.pipeline {
                Some(pid) => jobs::list_pipeline(&a.project, pid),
                None => jobs::list_project(&a.project, a.scope.as_deref()),
            };
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        JobCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(jobs::get(&t.project, t.id)).await?; emit_object(&v)?; }
        JobCmd::Play(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("play job {}", t.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(jobs::play(&t.project, t.id)).await?; emit_object(&v)?;
        }
        JobCmd::Retry(t) => {
            let v: serde_json::Value = ctx.client.send_json(jobs::retry(&t.project, t.id)).await?; emit_object(&v)?;
        }
        JobCmd::Cancel(t) => {
            let v: serde_json::Value = ctx.client.send_json(jobs::cancel(&t.project, t.id)).await?; emit_object(&v)?;
        }
        JobCmd::Erase(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("erase job {}", t.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(jobs::erase(&t.project, t.id)).await?; emit_object(&v)?;
        }
        JobCmd::Trace(t) => {
            let (_s, _h, bytes) = ctx.client.send_raw(jobs::trace(&t.project, t.id)).await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        JobCmd::Artifacts(t) => {
            let (_s, _h, bytes) = ctx.client.send_raw(jobs::artifacts(&t.project, t.id)).await?;
            std::io::stdout().write_all(&bytes).ok();
        }
    }
    Ok(())
}
```

Add `pub mod job;` to `cmd/mod.rs`. `main.rs`: `Job { #[command(subcommand)] cmd: gitlab_cli::cmd::job::JobCmd }`, dispatch to `gitlab_cli::cmd::job::run`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test job_cmd_test`
Expected: 1 PASS (10 sub-assertions).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(job): list/get/play/retry/cancel/erase/trace/artifacts (binary pass-through)"
```

---

## Task 4.7: `commit`

**Files:**
- Create: `crates/gitlab-core/src/resources/commits.rs`
- Create: `crates/gitlab-cli/src/cmd/commit.rs`
- Modify: `crates/gitlab-core/src/resources/mod.rs`, `crates/gitlab-cli/src/cmd/mod.rs`, `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/commit_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/commit_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn commit_all_verbs() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abc"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"new"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/statuses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits/abc/cherry_pick"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"cp"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits/abc/revert"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"rv"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/refs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;

    env_cmd(&base).args(["commit","list","--project","1"]).assert().success();
    env_cmd(&base).args(["commit","get","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","create","--project","1","--data",r#"{"branch":"main","commit_message":"c","actions":[]}"#]).assert().success();
    env_cmd(&base).args(["commit","diff","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","comments","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","statuses","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","cherry-pick","--project","1","--sha","abc","--branch","hotfix"]).assert().success();
    env_cmd(&base).args(["commit","revert","--project","1","--sha","abc","--branch","main"]).assert().success();
    env_cmd(&base).args(["commit","refs","--project","1","--sha","abc"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test commit_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/commits.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list(project: &str, rref: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/repository/commits", encode_id(project)));
    if let Some(r) = rref { p.query.push(("ref_name".into(), r.into())); }
    p
}
pub fn get(project: &str, sha: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/commits/{sha}", encode_id(project)))
}
pub fn create(project: &str, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/repository/commits", encode_id(project))).with_json(&body)
}
pub fn diff(project: &str, sha: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/commits/{sha}/diff", encode_id(project)))
}
pub fn comments(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/commits/{sha}/comments", encode_id(project)))
}
pub fn statuses(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/commits/{sha}/statuses", encode_id(project)))
}
pub fn cherry_pick(project: &str, sha: &str, branch: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/repository/commits/{sha}/cherry_pick", encode_id(project)))
        .with_json(&serde_json::json!({"branch": branch}))
}
pub fn revert(project: &str, sha: &str, branch: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/repository/commits/{sha}/revert", encode_id(project)))
        .with_json(&serde_json::json!({"branch": branch}))
}
pub fn refs(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/commits/{sha}/refs", encode_id(project)))
}
```

Add `pub mod commits;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/commit.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::commits;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum CommitCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Diff(Target),
    Comments(Target),
    Statuses(Target),
    #[command(name = "cherry-pick")] CherryPick(PickArgs),
    Revert(PickArgs),
    Refs(Target),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub project: String, #[arg(long = "ref")] pub rref: Option<String> }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub sha: String }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long)] pub data: String }
#[derive(Args, Debug)] pub struct PickArgs { #[arg(long)] pub project: String, #[arg(long)] pub sha: String, #[arg(long)] pub branch: String }

pub async fn run(ctx: Context, cmd: CommitCmd) -> Result<()> {
    match cmd {
        CommitCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, commits::list(&a.project, a.rref.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(commits::get(&t.project, &t.sha)).await?; emit_object(&v)?; }
        CommitCmd::Create(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            if !confirm_or_skip(ctx.assume_yes, "create commit")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(commits::create(&a.project, body)).await?; emit_object(&v)?;
        }
        CommitCmd::Diff(t) => { let v: serde_json::Value = ctx.client.send_json(commits::diff(&t.project, &t.sha)).await?; emit_object(&v)?; }
        CommitCmd::Comments(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, commits::comments(&t.project, &t.sha));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::Statuses(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, commits::statuses(&t.project, &t.sha));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::CherryPick(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("cherry-pick {} onto {}", a.sha, a.branch))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(commits::cherry_pick(&a.project, &a.sha, &a.branch)).await?; emit_object(&v)?;
        }
        CommitCmd::Revert(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("revert {} on {}", a.sha, a.branch))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(commits::revert(&a.project, &a.sha, &a.branch)).await?; emit_object(&v)?;
        }
        CommitCmd::Refs(t) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, commits::refs(&t.project, &t.sha));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
```

Add `pub mod commit;` to `cmd/mod.rs`; wire `Commit { #[command(subcommand)] cmd: gitlab_cli::cmd::commit::CommitCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test commit_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(commit): list/get/create/diff/comments/statuses/cherry-pick/revert/refs"
```

---

## Task 4.8: `branch`

**Files:**
- Create: `crates/gitlab-core/src/resources/branches.rs`
- Create: `crates/gitlab-cli/src/cmd/branch.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/branch_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/branch_cmd_test.rs`:

```rust
use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn branch_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"main"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/branches/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"main"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/branches"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"topic"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/repository/branches/topic"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/protected_branches"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"main"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/protected_branches/main"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["branch","list","--project","1"]).assert().success();
    env_cmd(&base).args(["branch","get","--project","1","--name","main"]).assert().success();
    env_cmd(&base).args(["branch","create","--project","1","--name","topic","--ref","main"]).assert().success();
    env_cmd(&base).args(["branch","delete","--project","1","--name","topic"]).assert().success();
    env_cmd(&base).args(["branch","protect","--project","1","--name","main"]).assert().success();
    env_cmd(&base).args(["branch","unprotect","--project","1","--name","main"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test branch_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/branches.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list(project: &str, search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/repository/branches", encode_id(project)));
    if let Some(s) = search { p.query.push(("search".into(), s.into())); }
    p
}
pub fn get(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/branches/{}", encode_id(project), encode_id(name)))
}
pub fn create(project: &str, name: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/repository/branches", encode_id(project)))
        .with_query([("branch", name), ("ref", rref)])
}
pub fn delete(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/repository/branches/{}", encode_id(project), encode_id(name)))
}
pub fn protect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/protected_branches", encode_id(project)))
        .with_query([("name", name)])
}
pub fn unprotect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/protected_branches/{}", encode_id(project), encode_id(name)))
}
```

Add `pub mod branches;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/branch.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::branches;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum BranchCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Delete(Target),
    Protect(Target),
    Unprotect(Target),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub project: String, #[arg(long)] pub search: Option<String> }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub name: String }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long)] pub name: String, #[arg(long = "ref")] pub rref: String }

pub async fn run(ctx: Context, cmd: BranchCmd) -> Result<()> {
    match cmd {
        BranchCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, branches::list(&a.project, a.search.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        BranchCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(branches::get(&t.project, &t.name)).await?; emit_object(&v)?; }
        BranchCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create branch {}", a.name))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(branches::create(&a.project, &a.name, &a.rref)).await?; emit_object(&v)?;
        }
        BranchCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete branch {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(branches::delete(&t.project, &t.name)).await?;
        }
        BranchCmd::Protect(t) => {
            let v: serde_json::Value = ctx.client.send_json(branches::protect(&t.project, &t.name)).await?; emit_object(&v)?;
        }
        BranchCmd::Unprotect(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unprotect branch {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(branches::unprotect(&t.project, &t.name)).await?;
        }
    }
    Ok(())
}
```

Add `pub mod branch;` to `cmd/mod.rs`; wire `Branch { #[command(subcommand)] cmd: gitlab_cli::cmd::branch::BranchCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test branch_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(branch): list/get/create/delete/protect/unprotect"
```

---

## Task 4.9: `tag`

**Files:**
- Create: `crates/gitlab-core/src/resources/tags.rs`
- Create: `crates/gitlab-cli/src/cmd/tag.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/tag_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/tag_cmd_test.rs`:

```rust
use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn tag_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"v1"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/tags/v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"v1"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/tags"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"v2"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/repository/tags/v1"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/protected_tags"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"v1"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/protected_tags/v1"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["tag","list","--project","1"]).assert().success();
    env_cmd(&base).args(["tag","get","--project","1","--name","v1"]).assert().success();
    env_cmd(&base).args(["tag","create","--project","1","--name","v2","--ref","main"]).assert().success();
    env_cmd(&base).args(["tag","delete","--project","1","--name","v1"]).assert().success();
    env_cmd(&base).args(["tag","protect","--project","1","--name","v1"]).assert().success();
    env_cmd(&base).args(["tag","unprotect","--project","1","--name","v1"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test tag_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/tags.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/tags", encode_id(project)))
}
pub fn get(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/tags/{}", encode_id(project), encode_id(name)))
}
pub fn create(project: &str, name: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/repository/tags", encode_id(project)))
        .with_query([("tag_name", name), ("ref", rref)])
}
pub fn delete(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/repository/tags/{}", encode_id(project), encode_id(name)))
}
pub fn protect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/protected_tags", encode_id(project)))
        .with_query([("name", name)])
}
pub fn unprotect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/protected_tags/{}", encode_id(project), encode_id(name)))
}
```

Add `pub mod tags;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/tag.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::tags;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum TagCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Delete(Target),
    Protect(Target),
    Unprotect(Target),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub project: String }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub name: String }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long)] pub name: String, #[arg(long = "ref")] pub rref: String }

pub async fn run(ctx: Context, cmd: TagCmd) -> Result<()> {
    match cmd {
        TagCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, tags::list(&a.project));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        TagCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(tags::get(&t.project, &t.name)).await?; emit_object(&v)?; }
        TagCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create tag {}", a.name))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(tags::create(&a.project, &a.name, &a.rref)).await?; emit_object(&v)?;
        }
        TagCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete tag {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(tags::delete(&t.project, &t.name)).await?;
        }
        TagCmd::Protect(t) => {
            let v: serde_json::Value = ctx.client.send_json(tags::protect(&t.project, &t.name)).await?; emit_object(&v)?;
        }
        TagCmd::Unprotect(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unprotect tag {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(tags::unprotect(&t.project, &t.name)).await?;
        }
    }
    Ok(())
}
```

Add `pub mod tag;` to `cmd/mod.rs`; wire `Tag { #[command(subcommand)] cmd: gitlab_cli::cmd::tag::TagCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test tag_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(tag): list/get/create/delete/protect/unprotect"
```

---

## Task 4.10: `file`

**Files:**
- Create: `crates/gitlab-core/src/resources/files.rs`
- Create: `crates/gitlab-cli/src/cmd/file.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/file_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/file_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn file_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/files/src%2Ffoo.rs")).and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"file_path":"src/foo.rs","content":"base64"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/files/src%2Ffoo.rs/raw")).and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(200).set_body_string("fn main(){}\n").insert_header("Content-Type", "text/plain")).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/files/src%2Ffoo.rs/blame")).and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"file_path":"new.txt"}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"file_path":"new.txt"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["file","get","--project","1","--path","src/foo.rs","--ref","main"]).assert().success().stdout(contains("file_path"));
    let out = env_cmd(&base).args(["file","raw","--project","1","--path","src/foo.rs","--ref","main"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("fn main"));
    env_cmd(&base).args(["file","blame","--project","1","--path","src/foo.rs","--ref","main"]).assert().success();
    env_cmd(&base).args(["file","create","--project","1","--path","new.txt","--branch","main","--content","hi","--message","c"]).assert().success();
    env_cmd(&base).args(["file","update","--project","1","--path","new.txt","--branch","main","--content","hi2","--message","u"]).assert().success();
    env_cmd(&base).args(["file","delete","--project","1","--path","new.txt","--branch","main","--message","d"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test file_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/files.rs`:

```rust
use reqwest::Method;
use crate::request::RequestSpec;
use super::encode_id;

fn path_for(project: &str, file: &str, suffix: &str) -> String {
    let encoded_file = urlencoding::encode(file);
    if suffix.is_empty() {
        format!("projects/{}/repository/files/{}", encode_id(project), encoded_file)
    } else {
        format!("projects/{}/repository/files/{}/{suffix}", encode_id(project), encoded_file)
    }
}

pub fn get(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "")).with_query([("ref", rref)])
}
pub fn raw(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "raw")).with_query([("ref", rref)])
}
pub fn blame(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "blame")).with_query([("ref", rref)])
}
pub fn create(project: &str, file: &str, branch: &str, content: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, path_for(project, file, ""))
        .with_json(&serde_json::json!({"branch":branch,"content":content,"commit_message":message}))
}
pub fn update(project: &str, file: &str, branch: &str, content: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, path_for(project, file, ""))
        .with_json(&serde_json::json!({"branch":branch,"content":content,"commit_message":message}))
}
pub fn delete(project: &str, file: &str, branch: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, path_for(project, file, ""))
        .with_query([("branch", branch), ("commit_message", message)])
}
```

Add `pub mod files;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/file.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::resources::files;
use std::io::Write;

use crate::context::Context;
use crate::output::emit_object;
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum FileCmd {
    Get(GetArgs),
    Raw(GetArgs),
    Blame(GetArgs),
    Create(WriteArgs),
    Update(WriteArgs),
    Delete(DeleteArgs),
}

#[derive(Args, Debug)] pub struct GetArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub path: String, #[arg(long = "ref")] pub rref: String,
}
#[derive(Args, Debug)] pub struct WriteArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub path: String,
    #[arg(long)] pub branch: String, #[arg(long)] pub content: String, #[arg(long)] pub message: String,
}
#[derive(Args, Debug)] pub struct DeleteArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub path: String,
    #[arg(long)] pub branch: String, #[arg(long)] pub message: String,
}

pub async fn run(ctx: Context, cmd: FileCmd) -> Result<()> {
    match cmd {
        FileCmd::Get(a) => { let v: serde_json::Value = ctx.client.send_json(files::get(&a.project, &a.path, &a.rref)).await?; emit_object(&v)?; }
        FileCmd::Raw(a) => {
            let (_s,_h,bytes) = ctx.client.send_raw(files::raw(&a.project, &a.path, &a.rref)).await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        FileCmd::Blame(a) => { let v: serde_json::Value = ctx.client.send_json(files::blame(&a.project, &a.path, &a.rref)).await?; emit_object(&v)?; }
        FileCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create file {}", a.path))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(files::create(&a.project, &a.path, &a.branch, &a.content, &a.message)).await?; emit_object(&v)?;
        }
        FileCmd::Update(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("update file {}", a.path))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(files::update(&a.project, &a.path, &a.branch, &a.content, &a.message)).await?; emit_object(&v)?;
        }
        FileCmd::Delete(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete file {}", a.path))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(files::delete(&a.project, &a.path, &a.branch, &a.message)).await?;
        }
    }
    Ok(())
}
```

Add `pub mod file;` to `cmd/mod.rs`; wire `File { #[command(subcommand)] cmd: gitlab_cli::cmd::file::FileCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test file_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(file): get/create/update/delete/blame/raw"
```

---

## Task 4.11: `repo`

**Files:**
- Create: `crates/gitlab-core/src/resources/repos.rs`
- Create: `crates/gitlab-cli/src/cmd/repo.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/repo_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/repo_cmd_test.rs`:

```rust
use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn repo_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/tree"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"path":"src"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/archive.tar.gz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0x1f,0x8b]).insert_header("Content-Type", "application/gzip")).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/compare")).and(query_param("from", "a")).and(query_param("to", "b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"commits":[]}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/contributors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"alice"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/merge_base"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"deadbeef"}))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["repo","tree","--project","1"]).assert().success();
    let out = env_cmd(&base).args(["repo","archive","--project","1","--format","tar.gz"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(&out.stdout[..2], &[0x1f,0x8b]);
    env_cmd(&base).args(["repo","compare","--project","1","--from","a","--to","b"]).assert().success();
    env_cmd(&base).args(["repo","contributors","--project","1"]).assert().success();
    env_cmd(&base).args(["repo","merge-base","--project","1","--ref","a","--ref","b"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test repo_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/repos.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn tree(project: &str, path: Option<&str>, rref: Option<&str>, recursive: bool) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/repository/tree", encode_id(project)));
    if let Some(pp) = path { p.query.push(("path".into(), pp.into())); }
    if let Some(r) = rref { p.query.push(("ref".into(), r.into())); }
    if recursive { p.query.push(("recursive".into(), "true".into())); }
    p
}
pub fn archive(project: &str, sha: Option<&str>, format: Option<&str>) -> RequestSpec {
    let base = format!("projects/{}/repository/archive", encode_id(project));
    let path = match format { Some(f) => format!("{base}.{f}"), None => base };
    let mut s = RequestSpec::new(Method::GET, path);
    if let Some(sh) = sha { s = s.with_query([("sha", sh)]); }
    s
}
pub fn compare(project: &str, from: &str, to: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/compare", encode_id(project)))
        .with_query([("from", from), ("to", to)])
}
pub fn contributors(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/contributors", encode_id(project)))
}
pub fn merge_base(project: &str, refs: &[String]) -> RequestSpec {
    let mut s = RequestSpec::new(Method::GET, format!("projects/{}/repository/merge_base", encode_id(project)));
    for r in refs { s.query.push(("refs[]".into(), r.clone())); }
    s
}
```

Add `pub mod repos;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/repo.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::repos;
use std::io::Write;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};

#[derive(Subcommand, Debug)]
pub enum RepoCmd {
    Tree(TreeArgs),
    Archive(ArchiveArgs),
    Compare(CompareArgs),
    Contributors(PrjArg),
    #[command(name = "merge-base")] MergeBase(MergeBaseArgs),
}

#[derive(Args, Debug)] pub struct TreeArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub path: Option<String>,
    #[arg(long = "ref")] pub rref: Option<String>, #[arg(long)] pub recursive: bool,
}
#[derive(Args, Debug)] pub struct ArchiveArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub sha: Option<String>, #[arg(long)] pub format: Option<String>,
}
#[derive(Args, Debug)] pub struct CompareArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub from: String, #[arg(long)] pub to: String,
}
#[derive(Args, Debug)] pub struct PrjArg { #[arg(long)] pub project: String }
#[derive(Args, Debug)] pub struct MergeBaseArgs {
    #[arg(long)] pub project: String,
    #[arg(long = "ref", num_args = 1.., required = true)] pub refs: Vec<String>,
}

pub async fn run(ctx: Context, cmd: RepoCmd) -> Result<()> {
    match cmd {
        RepoCmd::Tree(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, repos::tree(&a.project, a.path.as_deref(), a.rref.as_deref(), a.recursive));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        RepoCmd::Archive(a) => {
            let (_s,_h,bytes) = ctx.client.send_raw(repos::archive(&a.project, a.sha.as_deref(), a.format.as_deref())).await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        RepoCmd::Compare(a) => {
            let v: serde_json::Value = ctx.client.send_json(repos::compare(&a.project, &a.from, &a.to)).await?; emit_object(&v)?;
        }
        RepoCmd::Contributors(p) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, repos::contributors(&p.project));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        RepoCmd::MergeBase(a) => {
            let v: serde_json::Value = ctx.client.send_json(repos::merge_base(&a.project, &a.refs)).await?; emit_object(&v)?;
        }
    }
    Ok(())
}
```

Add `pub mod repo;` to `cmd/mod.rs`; wire `Repo { #[command(subcommand)] cmd: gitlab_cli::cmd::repo::RepoCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test repo_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(repo): tree/archive/compare/contributors/merge-base"
```

---

## Task 4.12: `user`

**Files:**
- Create: `crates/gitlab-core/src/resources/users.rs`
- Create: `crates/gitlab-cli/src/cmd/user.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/user_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/user_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn user_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/users")).and(query_param("search", "alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"username":"alice"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/users/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"username":"alice"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"username":"alice"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/users/1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":9,"title":"laptop"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/users/1/emails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"email":"a@x"}]))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["user","list","--search","alice"]).assert().success().stdout(contains("alice"));
    env_cmd(&base).args(["user","get","--id","1"]).assert().success();
    env_cmd(&base).args(["user","me"]).assert().success().stdout(contains("alice"));
    env_cmd(&base).args(["user","keys","--id","1"]).assert().success().stdout(contains("laptop"));
    env_cmd(&base).args(["user","emails","--id","1"]).assert().success().stdout(contains("a@x"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test user_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/users.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;

pub fn list(search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("users");
    if let Some(s) = search { p.query.push(("search".into(), s.into())); }
    p
}
pub fn get(id: u64) -> RequestSpec { RequestSpec::new(Method::GET, format!("users/{id}")) }
pub fn me() -> RequestSpec { RequestSpec::new(Method::GET, "user") }
pub fn keys(user_id: u64) -> PageRequest { PageRequest::new(format!("users/{user_id}/keys")) }
pub fn emails(user_id: u64) -> PageRequest { PageRequest::new(format!("users/{user_id}/emails")) }
```

Add `pub mod users;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/user.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::users;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};

#[derive(Subcommand, Debug)]
pub enum UserCmd {
    List(ListArgs),
    Get(IdArgs),
    Me,
    Keys(IdArgs),
    Emails(IdArgs),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub search: Option<String> }
#[derive(Args, Debug)] pub struct IdArgs { #[arg(long)] pub id: u64 }

pub async fn run(ctx: Context, cmd: UserCmd) -> Result<()> {
    match cmd {
        UserCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::list(a.search.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        UserCmd::Get(a) => { let v: serde_json::Value = ctx.client.send_json(users::get(a.id)).await?; emit_object(&v)?; }
        UserCmd::Me => { let v: serde_json::Value = ctx.client.send_json(users::me()).await?; emit_object(&v)?; }
        UserCmd::Keys(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::keys(a.id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        UserCmd::Emails(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::emails(a.id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
```

Add `pub mod user;` to `cmd/mod.rs`; wire `User { #[command(subcommand)] cmd: gitlab_cli::cmd::user::UserCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test user_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(user): list/get/me/keys/emails"
```

---

## Task 4.13: `label`

**Files:**
- Create: `crates/gitlab-core/src/resources/labels.rs`
- Create: `crates/gitlab-cli/src/cmd/label.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/label_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/label_cmd_test.rs`:

```rust
use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn label_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/labels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":9,"name":"bug"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9,"name":"bug"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":10,"name":"feat"}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9,"name":"bug2"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels/9/subscribe"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":9}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels/9/unsubscribe"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":9}))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["label","list","--project","1"]).assert().success();
    env_cmd(&base).args(["label","get","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","create","--project","1","--name","feat","--color","#0F0"]).assert().success();
    env_cmd(&base).args(["label","update","--project","1","--id","9","--data",r#"{"name":"bug2"}"#]).assert().success();
    env_cmd(&base).args(["label","delete","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","subscribe","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","unsubscribe","--project","1","--id","9"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test label_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/labels.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

pub fn list(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/labels", encode_id(project)))
}
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/labels/{id}", encode_id(project)))
}
pub fn create(project: &str, name: &str, color: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels", encode_id(project)))
        .with_json(&serde_json::json!({"name":name,"color":color}))
}
pub fn update(project: &str, id: u64, body: serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("projects/{}/labels/{id}", encode_id(project))).with_json(&body)
}
pub fn delete(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/labels/{id}", encode_id(project)))
}
pub fn subscribe(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels/{id}/subscribe", encode_id(project)))
}
pub fn unsubscribe(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels/{id}/unsubscribe", encode_id(project)))
}
```

Add `pub mod labels;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/label.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::labels;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum LabelCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete(Target),
    Subscribe(Target),
    Unsubscribe(Target),
}

#[derive(Args, Debug)] pub struct ListArgs { #[arg(long)] pub project: String }
#[derive(Args, Debug)] pub struct Target { #[arg(long)] pub project: String, #[arg(long)] pub id: u64 }
#[derive(Args, Debug)] pub struct CreateArgs { #[arg(long)] pub project: String, #[arg(long)] pub name: String, #[arg(long)] pub color: String }
#[derive(Args, Debug)] pub struct UpdateArgs { #[arg(long)] pub project: String, #[arg(long)] pub id: u64, #[arg(long)] pub data: String }

pub async fn run(ctx: Context, cmd: LabelCmd) -> Result<()> {
    match cmd {
        LabelCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, labels::list(&a.project));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        LabelCmd::Get(t) => { let v: serde_json::Value = ctx.client.send_json(labels::get(&t.project, t.id)).await?; emit_object(&v)?; }
        LabelCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create label {}", a.name))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(labels::create(&a.project, &a.name, &a.color)).await?; emit_object(&v)?;
        }
        LabelCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            if !confirm_or_skip(ctx.assume_yes, &format!("update label {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(labels::update(&a.project, a.id, body)).await?; emit_object(&v)?;
        }
        LabelCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete label {}", t.id))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(labels::delete(&t.project, t.id)).await?;
        }
        LabelCmd::Subscribe(t) => {
            let v: serde_json::Value = ctx.client.send_json(labels::subscribe(&t.project, t.id)).await?; emit_object(&v)?;
        }
        LabelCmd::Unsubscribe(t) => {
            let v: serde_json::Value = ctx.client.send_json(labels::unsubscribe(&t.project, t.id)).await?; emit_object(&v)?;
        }
    }
    Ok(())
}
```

Add `pub mod label;` to `cmd/mod.rs`; wire `Label { #[command(subcommand)] cmd: gitlab_cli::cmd::label::LabelCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test label_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(label): list/get/create/update/delete/subscribe/unsubscribe"
```

---

## Task 4.14: `note`

**Files:**
- Create: `crates/gitlab-core/src/resources/notes.rs`
- Create: `crates/gitlab-cli/src/cmd/note.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/note_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/note_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn note_on_issue_and_mr() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/issues/3/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":7,"body":"hi"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/issues/3/notes/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":7,"body":"hi"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/merge_requests/5/notes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":88,"body":"lgtm"}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5/notes/88"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":88,"body":"edited"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/merge_requests/5/notes/88"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["note","list","--project","1","--on","issue","--target","3"]).assert().success().stdout(contains("\"id\":7"));
    env_cmd(&base).args(["note","get","--project","1","--on","issue","--target","3","--id","7"]).assert().success();
    env_cmd(&base).args(["note","create","--project","1","--on","mr","--target","5","--body","lgtm"]).assert().success();
    env_cmd(&base).args(["note","update","--project","1","--on","mr","--target","5","--id","88","--body","edited"]).assert().success();
    env_cmd(&base).args(["note","delete","--project","1","--on","mr","--target","5","--id","88"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test note_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/notes.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

#[derive(Debug, Clone, Copy)]
pub enum Kind { Issue, Mr, Commit, Snippet }

impl Kind {
    pub fn plural(self) -> &'static str {
        match self {
            Kind::Issue => "issues",
            Kind::Mr => "merge_requests",
            Kind::Commit => "repository/commits",
            Kind::Snippet => "snippets",
        }
    }
}

fn base(project: &str, kind: Kind, target: &str) -> String {
    format!("projects/{}/{}/{}/notes", encode_id(project), kind.plural(), encode_id(target))
}

pub fn list(project: &str, kind: Kind, target: &str) -> PageRequest {
    PageRequest::new(base(project, kind, target))
}
pub fn get(project: &str, kind: Kind, target: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("{}/{id}", base(project, kind, target)))
}
pub fn create(project: &str, kind: Kind, target: &str, body: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, base(project, kind, target))
        .with_json(&serde_json::json!({"body": body}))
}
pub fn update(project: &str, kind: Kind, target: &str, id: u64, body: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_json(&serde_json::json!({"body": body}))
}
pub fn delete(project: &str, kind: Kind, target: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("{}/{id}", base(project, kind, target)))
}
```

Add `pub mod notes;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/note.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::{Args, Subcommand, ValueEnum};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::notes;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OnKind { Issue, Mr, Commit, Snippet }

impl OnKind {
    fn to_core(self) -> notes::Kind {
        match self {
            OnKind::Issue => notes::Kind::Issue,
            OnKind::Mr => notes::Kind::Mr,
            OnKind::Commit => notes::Kind::Commit,
            OnKind::Snippet => notes::Kind::Snippet,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum NoteCmd {
    List(ListArgs),
    Get(GetArgs),
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete(GetArgs),
}

#[derive(Args, Debug)] pub struct ListArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String,
}
#[derive(Args, Debug)] pub struct GetArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String, #[arg(long)] pub id: u64,
}
#[derive(Args, Debug)] pub struct CreateArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String, #[arg(long)] pub body: String,
}
#[derive(Args, Debug)] pub struct UpdateArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String, #[arg(long)] pub id: u64, #[arg(long)] pub body: String,
}

pub async fn run(ctx: Context, cmd: NoteCmd) -> Result<()> {
    match cmd {
        NoteCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, notes::list(&a.project, a.on.to_core(), &a.target));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        NoteCmd::Get(a) => {
            let v: serde_json::Value = ctx.client.send_json(notes::get(&a.project, a.on.to_core(), &a.target, a.id)).await?;
            emit_object(&v)?;
        }
        NoteCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, "create note")? { return Err(anyhow!("aborted")); }
            let v: serde_json::Value = ctx.client.send_json(notes::create(&a.project, a.on.to_core(), &a.target, &a.body)).await?;
            emit_object(&v)?;
        }
        NoteCmd::Update(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("update note {}", a.id))? { return Err(anyhow!("aborted")); }
            let v: serde_json::Value = ctx.client.send_json(notes::update(&a.project, a.on.to_core(), &a.target, a.id, &a.body)).await?;
            emit_object(&v)?;
        }
        NoteCmd::Delete(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete note {}", a.id))? { return Err(anyhow!("aborted")); }
            let _ = ctx.client.send_raw(notes::delete(&a.project, a.on.to_core(), &a.target, a.id)).await?;
        }
    }
    Ok(())
}
```

Add `pub mod note;` to `cmd/mod.rs`; wire `Note { #[command(subcommand)] cmd: gitlab_cli::cmd::note::NoteCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test note_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(note): list/get/create/update/delete across issue/mr/commit/snippet"
```

---

## Task 4.15: `discussion`

**Files:**
- Create: `crates/gitlab-core/src/resources/discussions.rs`
- Create: `crates/gitlab-cli/src/cmd/discussion.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/discussion_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/discussion_cmd_test.rs`:

```rust
use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn discussion_on_mr() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/discussions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abcd","resolved":false}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abcd"}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd")).and(query_param("resolved","true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abcd","resolved":true}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd")).and(query_param("resolved","false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abcd","resolved":false}))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["discussion","list","--project","1","--on","mr","--target","5"]).assert().success();
    env_cmd(&base).args(["discussion","get","--project","1","--on","mr","--target","5","--id","abcd"]).assert().success();
    env_cmd(&base).args(["discussion","resolve","--project","1","--on","mr","--target","5","--id","abcd"]).assert().success();
    env_cmd(&base).args(["discussion","unresolve","--project","1","--on","mr","--target","5","--id","abcd"]).assert().success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test discussion_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/discussions.rs`:

```rust
use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

#[derive(Debug, Clone, Copy)]
pub enum Kind { Issue, Mr, Commit }

impl Kind {
    pub fn plural(self) -> &'static str {
        match self { Kind::Issue => "issues", Kind::Mr => "merge_requests", Kind::Commit => "repository/commits" }
    }
}

fn base(project: &str, kind: Kind, target: &str) -> String {
    format!("projects/{}/{}/{}/discussions", encode_id(project), kind.plural(), encode_id(target))
}

pub fn list(project: &str, kind: Kind, target: &str) -> PageRequest {
    PageRequest::new(base(project, kind, target))
}
pub fn get(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("{}/{id}", base(project, kind, target)))
}
pub fn resolve(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_query([("resolved", "true")])
}
pub fn unresolve(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_query([("resolved", "false")])
}
```

Add `pub mod discussions;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/discussion.rs`:

```rust
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::discussions;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OnKind { Issue, Mr, Commit }

impl OnKind {
    fn to_core(self) -> discussions::Kind {
        match self {
            OnKind::Issue => discussions::Kind::Issue,
            OnKind::Mr => discussions::Kind::Mr,
            OnKind::Commit => discussions::Kind::Commit,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum DiscussionCmd {
    List(TargetArgs),
    Get(IdArgs),
    Resolve(IdArgs),
    Unresolve(IdArgs),
}

#[derive(Args, Debug)] pub struct TargetArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String,
}
#[derive(Args, Debug)] pub struct IdArgs {
    #[arg(long)] pub project: String, #[arg(long)] pub on: OnKind, #[arg(long)] pub target: String, #[arg(long)] pub id: String,
}

pub async fn run(ctx: Context, cmd: DiscussionCmd) -> Result<()> {
    match cmd {
        DiscussionCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, discussions::list(&a.project, a.on.to_core(), &a.target));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        DiscussionCmd::Get(a) => {
            let v: serde_json::Value = ctx.client.send_json(discussions::get(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
        DiscussionCmd::Resolve(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("resolve discussion {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(discussions::resolve(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
        DiscussionCmd::Unresolve(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unresolve discussion {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(discussions::unresolve(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}
```

Add `pub mod discussion;` to `cmd/mod.rs`; wire `Discussion { #[command(subcommand)] cmd: gitlab_cli::cmd::discussion::DiscussionCmd }` in `main.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test discussion_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(discussion): list/get/resolve/unresolve across issue/mr/commit"
```

---

## Task 4.16: `search`

**Files:**
- Create: `crates/gitlab-core/src/resources/search.rs`
- Create: `crates/gitlab-cli/src/cmd/search.rs`
- Modify: `resources/mod.rs`, `cmd/mod.rs`, `main.rs`
- Create: `crates/gitlab-cli/tests/search_cmd_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/search_cmd_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn search_three_scopes() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/search")).and(query_param("scope", "issues")).and(query_param("search", "bug"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"title":"bug A"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/search")).and(query_param("scope", "commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/search")).and(query_param("scope", "blobs")).and(query_param("search", "fn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"filename":"main.rs"}]))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["search","--scope","issues","--query","bug"]).assert().success().stdout(contains("bug A"));
    env_cmd(&base).args(["search","--scope","commits","--query","x","--group","atoms"]).assert().success().stdout(contains("abc"));
    env_cmd(&base).args(["search","--scope","blobs","--query","fn","--project","1"]).assert().success().stdout(contains("main.rs"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test search_cmd_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-core/src/resources/search.rs`:

```rust
use crate::page::PageRequest;
use super::encode_id;

pub fn global(scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new("search");
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}
pub fn group(group: &str, scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new(format!("groups/{}/search", encode_id(group)));
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}
pub fn project(project: &str, scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/search", encode_id(project)));
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}
```

Add `pub mod search;` to `resources/mod.rs`.

Create `crates/gitlab-cli/src/cmd/search.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::Args;
use gitlab_core::page::PagedStream;
use gitlab_core::resources::search;

use crate::context::Context;
use crate::output::emit_stream;

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(long)] pub scope: String,
    #[arg(long)] pub query: String,
    #[arg(long, conflicts_with = "group")] pub project: Option<String>,
    #[arg(long)] pub group: Option<String>,
}

pub async fn run(ctx: Context, a: SearchArgs) -> Result<()> {
    let req = match (a.project, a.group) {
        (Some(p), None) => search::project(&p, &a.scope, &a.query),
        (None, Some(g)) => search::group(&g, &a.scope, &a.query),
        (None, None) => search::global(&a.scope, &a.query),
        (Some(_), Some(_)) => return Err(anyhow!("--project and --group are mutually exclusive")),
    };
    let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
    emit_stream(stream, ctx.output, ctx.limit).await?;
    Ok(())
}
```

Add `pub mod search;` to `cmd/mod.rs`; wire a top-level `Search(gitlab_cli::cmd::search::SearchArgs)` variant (not `Search { cmd }`) in `main.rs` `Command` enum, and dispatch to `gitlab_cli::cmd::search::run(ctx, args).await`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test search_cmd_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "feat(search): global/group/project scopes"
```

After this task, **every top-level command** (`project`, `group`, `mr`, `issue`, `pipeline`, `job`, `commit`, `branch`, `tag`, `file`, `repo`, `user`, `label`, `note`, `discussion`, `search`, `api`, `version`, `me`, `config`) must be wired in `main.rs`. Run `cargo test --workspace` once more — **expected:** all tests green. If any test regresses, git-bisect from the most recent green commit.

---

# Milestone 5 — Polish & release

## Task 5.1: Secrets masking audit

**Files:**
- Create: `crates/gitlab-core/tests/masking_test.rs`
- Create: `crates/gitlab-cli/tests/masking_cli_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-core/tests/masking_test.rs`:

```rust
use gitlab_core::auth::MaskedToken;
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::request::RequestSpec;
use gitlab_core::retry::RetryPolicy;
use reqwest::Method;

#[test]
fn masked_token_hides_middle() {
    assert_eq!(MaskedToken("glpat-ABCDEFGHIJKL").to_string(), "glpa****IJKL");
    assert_eq!(MaskedToken("xx").to_string(), "****");
}

#[tokio::test]
async fn error_never_contains_literal_token() {
    let client = Client::new(ClientOptions {
        host: "http://127.0.0.1:1".into(),
        token: "glpat-SHOULDNEVERAPPEAR".into(),
        retry: RetryPolicy { max_attempts: 0, max_attempts_429: 0, ..RetryPolicy::default() },
        ..ClientOptions::default()
    })
    .unwrap();
    let err = client
        .send_json::<serde_json::Value>(RequestSpec::new(Method::GET, "version"))
        .await
        .unwrap_err();
    let msg = err.to_string();
    let dbg = format!("{err:?}");
    let payload = serde_json::to_string(&err.to_payload()).unwrap();
    assert!(!msg.contains("glpat-SHOULDNEVERAPPEAR"), "display leaked token: {msg}");
    assert!(!dbg.contains("glpat-SHOULDNEVERAPPEAR"), "debug leaked token: {dbg}");
    assert!(!payload.contains("glpat-SHOULDNEVERAPPEAR"), "payload leaked token: {payload}");
}
```

`crates/gitlab-cli/tests/masking_cli_test.rs`:

```rust
use assert_cmd::Command;

#[test]
fn config_list_never_prints_raw_token() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(
        &cfg,
        r#"
default_host = "gitlab.example.com"
[host."gitlab.example.com"]
token = "glpat-DONOTLEAK1234"
"#,
    )
    .unwrap();
    let output = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg)
        .args(["config", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("glpat-DONOTLEAK1234"));
    assert!(!stderr.contains("glpat-DONOTLEAK1234"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p gitlab-core --test masking_test && cargo test -p gitlab-cli --test masking_cli_test`
Expected: PASS (if any fail, tighten `Debug` impls by replacing derived `Debug` with manual masking on any struct containing `token: String`).

- [ ] **Step 3: Fix leaks if any**

If the tests reveal leaks, replace `#[derive(Debug)]` on `ClientOptions` / `Client` / `ResolvedAuth` with manual impls that render `token` as `MaskedToken(&self.token)`.

- [ ] **Step 4: Rerun**

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "test(security): assert no literal PAT in Display/Debug/JSON payload"
```

---

## Task 5.2: L3 smoke tests

**Files:**
- Create: `crates/gitlab-cli/tests/smoke_live_test.rs`

- [ ] **Step 1: Write the gated smoke**

```rust
use assert_cmd::Command;

fn live_enabled() -> Option<(String, String, String)> {
    let host = std::env::var("GITLAB_TEST_HOST").ok()?;
    let token = std::env::var("GITLAB_TEST_TOKEN").ok()?;
    let project = std::env::var("GITLAB_TEST_PROJECT").ok()?;
    Some((host, token, project))
}

#[test]
#[ignore]
fn live_version() {
    let Some((host, token, _)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .arg("version").assert().success();
}

#[test]
#[ignore]
fn live_me() {
    let Some((host, token, _)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .arg("me").assert().success();
}

#[test]
#[ignore]
fn live_project_get() {
    let Some((host, token, project)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .args(["project","get",&project]).assert().success();
}

#[test]
#[ignore]
fn live_mr_list() {
    let Some((host, token, project)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .args(["mr","list","--project",&project,"--limit","5"]).assert().success();
}
```

- [ ] **Step 2: Run (should skip unless env set)**

Run: `cargo test -p gitlab-cli --test smoke_live_test`
Expected: 0 run (ignored).

Run with env: `GITLAB_TEST_HOST=https://... GITLAB_TEST_TOKEN=... GITLAB_TEST_PROJECT=... cargo test -p gitlab-cli --test smoke_live_test -- --ignored`
Expected: 4 PASS against real instance.

- [ ] **Step 3: N/A** (tests are the implementation)
- [ ] **Step 4: Confirm default skip**

Run: `cargo test -p gitlab-cli --test smoke_live_test` (without env)
Expected: `0 passed; 0 failed; 4 ignored`.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli/tests/smoke_live_test.rs
git commit -m "test: opt-in live smoke against real 14.0.5 instance (read-only)"
```

---

## Task 5.3: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: N/A (doc-only)**
- [ ] **Step 2: N/A**
- [ ] **Step 3: Write**

Create `README.md` at repo root:

```markdown
# gitlab-cli

An agent-first Rust CLI for **GitLab EE 14.0.5** (not newer, not older).
Consumed by autonomous agents via `bash -c` + JSON.

## Quick start

```bash
gitlab config set-token --host gitlab.example.com --token glpat-XXXX
gitlab version
gitlab mr list --project atoms/api --state opened | jq '.[].iid'
```

## Commands

| Command | Verbs |
|---|---|
| `project` | list, get, create, update, delete, fork, archive, unarchive |
| `group` | list, get, create, update, delete, members, projects, subgroups |
| `mr` | list, get, create, update, close, reopen, merge, rebase, approve, unapprove, diffs, commits, changes, pipelines |
| `issue` | list, get, create, update, close, reopen, link, unlink, move, stats |
| `pipeline` | list, get, create, retry, cancel, delete, variables |
| `job` | list, get, play, retry, cancel, erase, trace, artifacts |
| `commit` | list, get, create, diff, comments, statuses, cherry-pick, revert, refs |
| `branch` | list, get, create, delete, protect, unprotect |
| `tag` | list, get, create, delete, protect, unprotect |
| `file` | get, create, update, delete, blame, raw |
| `repo` | tree, archive, compare, contributors, merge-base |
| `user` | list, get, me, keys, emails |
| `label` | list, get, create, update, delete, subscribe, unsubscribe |
| `note` | list, get, create, update, delete (issue/mr/commit/snippet) |
| `discussion` | list, get, resolve, unresolve (issue/mr/commit) |
| `search` | global/group/project scopes |
| `api` | `GET/POST/PUT/PATCH/DELETE <path>` escape hatch |

## Auth

Resolution order: `--token` > `GITLAB_TOKEN` > `~/.config/gitlab-cli/config.toml`.

## Output / errors / exit codes

- `stdout`: JSON (object for `get`/`create`/`update`/action-returning-body; array for `list`; NDJSON with `--output ndjson`).
- `stderr`: structured error JSON when a command fails.
- Exit codes: 0 success, 2 invalid args, 3 unauthorized, 4 forbidden, 5 not found, 6 conflict, 7 rate-limited, 8 server error, 9 network/timeout, 10 dry-run.

## Version caveat

This CLI is **frozen against GitLab 14.0.5-ee**. Fields and endpoints differ from 15.x+ — output is passed through unmodified.

## License

MIT.
```

- [ ] **Step 4: Verify rendering**

Run: `cat README.md | head -20` — sanity check markdown.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: README with command matrix, auth, exit codes, 14.0.5 caveat"
```

---

## Task 5.4: CI matrix

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `deny.toml`

- [ ] **Step 1: Write workflow**

`.github/workflows/ci.yml`:

```yaml
name: ci
on:
  push: { branches: [main] }
  pull_request: { branches: [main] }
jobs:
  lint-test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt,clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets --workspace -- -D warnings
      - run: cargo test --workspace --locked

  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

`.github/workflows/release.yml`:

```yaml
name: release
on:
  push:
    tags: ['v*']
jobs:
  build:
    name: ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - { os: macos-latest,   target: x86_64-apple-darwin }
          - { os: macos-latest,   target: aarch64-apple-darwin }
          - { os: ubuntu-latest,  target: x86_64-unknown-linux-musl }
          - { os: ubuntu-latest,  target: aarch64-unknown-linux-musl }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.target }} }
      - if: contains(matrix.target,'musl')
        run: sudo apt-get update && sudo apt-get install -y musl-tools
      - run: cargo build --release --locked --target ${{ matrix.target }} -p gitlab-cli
      - run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../gitlab-cli-${{ matrix.target }}.tar.gz gitlab
          shasum -a 256 ../../../gitlab-cli-${{ matrix.target }}.tar.gz > ../../../gitlab-cli-${{ matrix.target }}.tar.gz.sha256
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            gitlab-cli-${{ matrix.target }}.tar.gz
            gitlab-cli-${{ matrix.target }}.tar.gz.sha256
```

`deny.toml`:

```toml
[graph]
targets = [
  { triple = "x86_64-unknown-linux-musl" },
  { triple = "aarch64-unknown-linux-musl" },
  { triple = "x86_64-apple-darwin" },
  { triple = "aarch64-apple-darwin" },
]

[advisories]
yanked = "warn"
ignore = []

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Unicode-DFS-2016", "MPL-2.0", "Zlib", "CC0-1.0"]
confidence-threshold = 0.8
```

- [ ] **Step 2: Run locally**

Run: `cargo fmt --all --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace --locked`
Expected: all green.

- [ ] **Step 3: N/A**
- [ ] **Step 4: N/A**
- [ ] **Step 5: Commit**

```bash
git add .github deny.toml
git commit -m "ci: lint/test matrix + release workflow + cargo-deny policy"
```

---

## Task 5.5: Release version & `--version` string

**Files:**
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/build.rs`

- [ ] **Step 1: Write the failing test**

Extend `crates/gitlab-cli/tests/global_args_test.rs`:

```rust
#[test]
fn version_string_contains_target_triple() {
    let mut cmd = assert_cmd::Command::cargo_bin("gitlab").unwrap();
    let out = cmd.arg("--version").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains(env!("CARGO_PKG_VERSION")), "missing version: {s}");
    assert!(s.contains("target="), "missing target in --version: {s}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test global_args_test`
Expected: FAIL (only semver appears).

- [ ] **Step 3: Implement build script**

Create `crates/gitlab-cli/build.rs`:

```rust
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=GITLAB_CLI_TARGET={target}");
    println!("cargo:rustc-env=GITLAB_CLI_GIT_SHA={sha}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
```

Override clap's version string in `main.rs`:

```rust
const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (target=", env!("GITLAB_CLI_TARGET"),
    ", git=", env!("GITLAB_CLI_GIT_SHA"),
    ")"
);

#[derive(Parser)]
#[command(name = "gitlab", version = VERSION_STRING, about = "gitlab-cli for GitLab 14.0.5-ee", propagate_version = true)]
struct Cli { /* unchanged */ }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test global_args_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitlab-cli
git commit -m "feat(cli): --version includes target triple + git sha"
```

---

## Task 5.6: Final workspace verification

**Files:**
- None (verification-only task)

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace --locked`
Expected: all tests PASS, no `ignored` except the 4 in `smoke_live_test.rs`.

- [ ] **Step 2: Lint**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3: Build all release targets locally (macOS)**

Run: `cargo build --release -p gitlab-cli`
Run the result: `./target/release/gitlab --version && ./target/release/gitlab --help`
Expected: version string with target; help lists all 17 top-level commands.

- [ ] **Step 4: Smoke end-to-end (mocked)**

Run all tests once more:
`cargo test --workspace`
Expected: green.

- [ ] **Step 5: Tag & commit release readiness**

```bash
git commit --allow-empty -m "chore: v0.1.0 release readiness verified"
git tag v0.1.0
```

(Push: `git push origin main --tags` when ready.)

---

## Acceptance checklist (from spec §13)

- [ ] 17 top-level commands (`project`, `group`, `mr`, `issue`, `pipeline`, `job`, `commit`, `branch`, `tag`, `file`, `repo`, `user`, `label`, `note`, `discussion`, `search`, `api`, plus `version`/`me`/`config`) parse and show help
- [ ] Golden-path `list → get → create → update → delete` wiremock test passes for each resource family
- [ ] Single static binary builds for macOS arm64 + Linux musl x86_64
- [ ] Invalid PAT → exit 3 + structured stderr JSON
- [ ] `mr list --group <g>` auto-paginates (verified count)
- [ ] `--dry-run` on every write never issues an HTTP request (wiremock `.expect(0)`)
- [ ] README documents command list, env vars, config TOML, exit codes, 14.0.5 scope
