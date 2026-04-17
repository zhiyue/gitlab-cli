use clap::Args;

fn parse_bool_loose(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid boolean value: {s}")),
    }
}

#[allow(clippy::struct_excessive_bools)]
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

    #[arg(long = "yes", short = 'y', global = true, env = "GITLAB_ASSUME_YES", value_parser = parse_bool_loose)]
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
