use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list_project(project: &str, scope: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/jobs", encode_id(project)));
    if let Some(s) = scope {
        p.query.push(("scope".into(), s.into()));
    }
    p
}

#[must_use]
pub fn list_pipeline(project: &str, pipeline_id: u64) -> PageRequest {
    PageRequest::new(format!(
        "projects/{}/pipelines/{pipeline_id}/jobs",
        encode_id(project)
    ))
}

#[must_use]
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!("projects/{}/jobs/{id}", encode_id(project)),
    )
}

#[must_use]
pub fn play(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/jobs/{id}/play", encode_id(project)),
    )
}

#[must_use]
pub fn retry(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/jobs/{id}/retry", encode_id(project)),
    )
}

#[must_use]
pub fn cancel(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/jobs/{id}/cancel", encode_id(project)),
    )
}

#[must_use]
pub fn erase(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::POST,
        format!("projects/{}/jobs/{id}/erase", encode_id(project)),
    )
}

#[must_use]
pub fn trace(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!("projects/{}/jobs/{id}/trace", encode_id(project)),
    )
}

#[must_use]
pub fn artifacts(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::GET,
        format!("projects/{}/jobs/{id}/artifacts", encode_id(project)),
    )
}
