use gitlab_core::error::{ErrorCode, GitlabError};
use std::io::{self, Write};

#[must_use]
pub fn report_error(err: &GitlabError) -> i32 {
    let mut payload = err.to_payload();
    payload.hint = lookup_hint(err);
    let body = serde_json::json!({ "error": payload });
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(
        lock,
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string())
    );
    err.exit_code()
}

fn lookup_hint(err: &GitlabError) -> Option<String> {
    let (code, msg) = match err {
        GitlabError::Http { code, message, .. } => (*code, message.as_str()),
        _ => return None,
    };
    let m = msg.to_lowercase();
    let hint = match code {
        ErrorCode::Unauthorized =>
            "Token is missing, expired, or revoked. Verify with: gitlab config list  (token shown masked). \
             Regenerate at: <host>/-/profile/personal_access_tokens",
        ErrorCode::Forbidden if m.contains("approve") =>
            "MR approval requires GitLab EE license and may be blocked for self-approval. \
             Try a different reviewer's PAT.",
        ErrorCode::Forbidden =>
            "Token lacks required scope, or you're not a member with sufficient role. \
             Check token scopes at: <host>/-/profile/personal_access_tokens",
        ErrorCode::NotFound if m.contains("file") =>
            "File not found at this ref. If the file was deleted, try the parent commit: \
             gitlab commit get --project <p> --sha <last-known-good-sha>  (then read .parent_ids[0])",
        ErrorCode::NotFound if m.contains("ref") || m.contains("commit") =>
            "Ref/commit not found. Check spelling, or that you have access to that branch.",
        ErrorCode::NotFound if m.contains("project") =>
            "Project not found or PAT lacks access. Verify path-with-namespace is correct (case-sensitive).",
        ErrorCode::NotFound =>
            "Resource not found. Verify ids/paths and that your PAT has access.",
        ErrorCode::Conflict if m.contains("already") =>
            "Resource already exists or in conflicting state. Re-fetch current state before retrying.",
        ErrorCode::Conflict =>
            "Validation failed. Check 'details' field for which fields are invalid.",
        ErrorCode::RateLimited =>
            "GitLab rate limit hit. CLI already retried with backoff. Reduce parallelism or set --rps 5.",
        ErrorCode::ServerError =>
            "GitLab returned 5xx. CLI retried automatically. If persistent, check instance status.",
        ErrorCode::BadRequest if m.contains("not allowed") =>
            "Operation not allowed in current state (e.g., merging closed MR, deleting protected branch).",
        ErrorCode::BadRequest =>
            "Bad request shape. Check 'details' for field-level validation errors.",
        _ => return None,
    };
    Some(hint.to_string())
}
