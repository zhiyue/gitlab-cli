use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list_for_project(project: &str, state: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/issues", encode_id(project)));
    if let Some(s) = state {
        p.query.push(("state".into(), s.into()));
    }
    p
}

#[must_use]
pub fn get_spec(project: &str, iid: u64) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!("projects/{}/issues/{iid}", encode_id(project)),
    )
}

#[must_use]
pub fn create_spec(project: &str, title: &str, labels: Option<&str>) -> RequestSpec {
    let mut body = serde_json::json!({"title": title});
    if let Some(l) = labels {
        body["labels"] = serde_json::Value::String(l.into());
    }
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/issues", encode_id(project)),
    )
    .with_json(&body)
}

#[must_use]
pub fn update_spec(project: &str, iid: u64, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(
        Method::PUT,
        format!("projects/{}/issues/{iid}", encode_id(project)),
    )
    .with_json(body)
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
pub fn move_spec(project: &str, iid: u64, target_project: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/issues/{iid}/move", encode_id(project)),
    )
    .with_json(&serde_json::json!({"to_project_id": target_project}))
}

#[must_use]
pub fn stats_spec() -> RequestSpec {
    RequestSpec::new(Method::GET, "issues_statistics")
}

#[must_use]
pub fn list_links(project: &str, iid: u64) -> PageRequest {
    PageRequest::new(format!(
        "projects/{}/issues/{iid}/links",
        encode_id(project)
    ))
}

#[must_use]
pub fn link_spec(project: &str, iid: u64, target_project: &str, target_iid: u64) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/issues/{iid}/links", encode_id(project)),
    )
    .with_json(
        &serde_json::json!({"target_project_id": target_project, "target_issue_iid": target_iid}),
    )
}

#[must_use]
pub fn unlink_spec(project: &str, iid: u64, link_id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!(
            "projects/{}/issues/{iid}/links/{link_id}",
            encode_id(project)
        ),
    )
}
