use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/tags", encode_id(project)))
}

#[must_use]
pub fn get(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!(
            "projects/{}/repository/tags/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}

#[must_use]
pub fn create(project: &str, name: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/repository/tags", encode_id(project)),
    )
    .with_query([("tag_name", name), ("ref", rref)])
}

#[must_use]
pub fn delete(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!(
            "projects/{}/repository/tags/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}

#[must_use]
pub fn protect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/protected_tags", encode_id(project)),
    )
    .with_query([("name", name)])
}

#[must_use]
pub fn unprotect(project: &str, name: &str) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!(
            "projects/{}/protected_tags/{}",
            encode_id(project),
            encode_id(name)
        ),
    )
}
