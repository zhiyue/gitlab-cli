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

#[derive(Debug)]
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
        let CliInputs {
            globals,
            config_text,
        } = inputs;

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
            RetryPolicy {
                max_attempts: 0,
                max_attempts_429: 0,
                ..RetryPolicy::default()
            }
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

        let req_timeout = globals
            .timeout
            .map_or(Duration::from_secs(30), Duration::from_secs);

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
            assume_yes: globals.assume_yes || resolved.host_config.assume_yes,
            dry_run: globals.dry_run,
            output: globals.output,
            limit: globals.limit,
            no_paginate: globals.no_paginate,
        })
    }
}
