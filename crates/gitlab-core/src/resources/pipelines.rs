use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

#[must_use]
pub fn list(project: &str, status: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/pipelines", encode_id(project)));
    if let Some(s) = status { p.query.push(("status".into(), s.into())); }
    p
}

#[must_use]
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/pipelines/{id}", encode_id(project)))
}

#[must_use]
pub fn create(project: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipeline", encode_id(project)))
        .with_query([("ref", rref)])
}

#[must_use]
pub fn retry(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipelines/{id}/retry", encode_id(project)))
}

#[must_use]
pub fn cancel(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/pipelines/{id}/cancel", encode_id(project)))
}

#[must_use]
pub fn delete(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/pipelines/{id}", encode_id(project)))
}

#[must_use]
pub fn variables(project: &str, id: u64) -> PageRequest {
    PageRequest::new(format!("projects/{}/pipelines/{id}/variables", encode_id(project)))
}
