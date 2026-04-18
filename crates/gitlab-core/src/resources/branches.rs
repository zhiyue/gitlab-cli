use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list(project: &str, search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!(
        "projects/{}/repository/branches",
        encode_id(project)
    ));
    if let Some(s) = search {
        p.query.push(("search".into(), s.into()));
    }
    p
}

#[must_use]
pub fn get(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!(
            "projects/{}/repository/branches/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}

#[must_use]
pub fn create(project: &str, name: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/repository/branches", encode_id(project)),
    )
    .with_query([("branch", name), ("ref", rref)])
}

#[must_use]
pub fn delete(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!(
            "projects/{}/repository/branches/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}

#[must_use]
pub fn protect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/protected_branches", encode_id(project)),
    )
    .with_query([("name", name)])
}

#[must_use]
pub fn unprotect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!(
            "projects/{}/protected_branches/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}
