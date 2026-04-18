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
            let hc = cfg
                .host
                .entry(a.host.clone())
                .or_insert_with(HostConfig::default);
            hc.token = Some(a.token);
            if a.default || cfg.default_host.is_none() {
                cfg.default_host = Some(a.host);
            }
            cfg.save_to(&path).map_err(|e| anyhow!(e.to_string()))?;
            Ok(())
        }
    }
}
