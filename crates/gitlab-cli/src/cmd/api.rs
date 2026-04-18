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
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;
    Ok((k.to_owned(), v.to_owned()))
}

fn is_write(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
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
        Some(raw) => {
            Some(serde_json::from_str(raw).map_err(|e| anyhow!("invalid JSON body: {e}"))?)
        }
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

    if is_write(&method) && !confirm_or_skip(ctx.assume_yes, &format!("{method} /{path_stripped}"))?
    {
        anyhow::bail!("aborted");
    }

    let mut spec = RequestSpec::new(method, path_stripped).with_query(args.query);
    if let Some(b) = body {
        spec.body = Some(b);
    }

    let (_status, headers, bytes) = ctx.client.send_raw(spec).await?;
    let ct = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if ct.starts_with("application/json") {
        let v: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| anyhow!("parse JSON response: {e}"))?;
        emit_object(&v)?;
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes).ok();
        tracing::info!(
            bytes = bytes.len(),
            content_type = ct,
            "emitted binary body"
        );
    }
    Ok(())
}
