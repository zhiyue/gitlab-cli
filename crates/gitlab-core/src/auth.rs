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
        .ok_or_else(|| {
            GitlabError::from_status(
                401,
                "no host configured (set --host, GITLAB_HOST, or config.toml)".into(),
                None,
            )
        })?;

    let host_cfg = config
        .host
        .get(&host)
        .cloned()
        .or_else(|| config.host_for(None).cloned())
        .unwrap_or_default();

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
