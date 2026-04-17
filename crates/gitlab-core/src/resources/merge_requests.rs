use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

fn project_mr_path(project: &str, iid: u64, suffix: &str) -> String {
    if suffix.is_empty() {
        format!("projects/{}/merge_requests/{iid}", encode_id(project))
    } else {
        format!("projects/{}/merge_requests/{iid}/{suffix}", encode_id(project))
    }
}

#[must_use]
pub fn list_for_project(project: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/merge_requests", encode_id(project)));
    if let Some(s) = state { p.query.push(("state".into(), s.into())); }
    p
}

#[must_use]
pub fn list_for_group(group: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("groups/{}/merge_requests", encode_id(group)));
    if let Some(s) = state { p.query.push(("state".into(), s.into())); }
    p
}

#[must_use]
pub fn get_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, project_mr_path(project, iid, ""))
}

#[must_use]
pub fn create_spec(project: &str, source: &str, target: &str, title: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/merge_requests", encode_id(project)))
        .with_json(&serde_json::json!({"source_branch": source, "target_branch": target, "title": title}))
}

#[must_use]
pub fn update_spec(project: &str, iid: u64, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "")).with_json(body)
}

#[must_use]
pub fn close_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, &serde_json::json!({"state_event": "close"}))
}

#[must_use]
pub fn reopen_spec(project: &str, iid: u64) -> RequestSpec {
    update_spec(project, iid, &serde_json::json!({"state_event": "reopen"}))
}

#[must_use]
pub fn merge_spec(project: &str, iid: u64, squash: bool) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "merge"))
        .with_json(&serde_json::json!({"squash": squash}))
}

#[must_use]
pub fn rebase_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::PUT, project_mr_path(project, iid, "rebase"))
}

#[must_use]
pub fn approve_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, project_mr_path(project, iid, "approve"))
}

#[must_use]
pub fn unapprove_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, project_mr_path(project, iid, "unapprove"))
}

#[must_use]
pub fn changes_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, project_mr_path(project, iid, "changes"))
}

#[must_use]
pub fn diffs_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "diffs"))
}

#[must_use]
pub fn commits_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "commits"))
}

#[must_use]
pub fn pipelines_page(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(project_mr_path(project, iid, "pipelines"))
}
