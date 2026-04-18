use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list(project: &str, rref: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!(
        "projects/{}/repository/commits",
        encode_id(project)
    ));
    if let Some(r) = rref {
        p.query.push(("ref_name".into(), r.into()));
    }
    p
}

#[must_use]
pub fn get(project: &str, sha: &str) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!("projects/{}/repository/commits/{sha}", encode_id(project)),
    )
}

#[must_use]
pub fn create(project: &str, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/repository/commits", encode_id(project)),
    )
    .with_json(body)
}

#[must_use]
pub fn diff(project: &str, sha: &str) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!(
            "projects/{}/repository/commits/{sha}/diff",
            encode_id(project)
        ),
    )
}

#[must_use]
pub fn comments(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!(
        "projects/{}/repository/commits/{sha}/comments",
        encode_id(project)
    ))
}

#[must_use]
pub fn statuses(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!(
        "projects/{}/repository/commits/{sha}/statuses",
        encode_id(project)
    ))
}

#[must_use]
pub fn cherry_pick(project: &str, sha: &str, branch: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!(
            "projects/{}/repository/commits/{sha}/cherry_pick",
            encode_id(project)
        ),
    )
    .with_json(&serde_json::json!({"branch": branch}))
}

#[must_use]
pub fn revert(project: &str, sha: &str, branch: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!(
            "projects/{}/repository/commits/{sha}/revert",
            encode_id(project)
        ),
    )
    .with_json(&serde_json::json!({"branch": branch}))
}

#[must_use]
pub fn refs(project: &str, sha: &str) -> PageRequest {
    PageRequest::new(format!(
        "projects/{}/repository/commits/{sha}/refs",
        encode_id(project)
    ))
}
