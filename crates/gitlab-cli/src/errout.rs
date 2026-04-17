use gitlab_core::error::GitlabError;
use std::io::{self, Write};

#[must_use]
pub fn report_error(err: &GitlabError) -> i32 {
    let payload = err.to_payload();
    let body = serde_json::json!({ "error": payload });
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(lock, "{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
    err.exit_code()
}
