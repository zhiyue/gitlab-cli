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

#[must_use]
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
