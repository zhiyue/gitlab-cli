use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

#[must_use]
pub fn list(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/labels", encode_id(project)))
}

#[must_use]
pub fn get(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/labels/{id}", encode_id(project)))
}

#[must_use]
pub fn create(project: &str, name: &str, color: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels", encode_id(project)))
        .with_json(&serde_json::json!({"name":name,"color":color}))
}

#[must_use]
pub fn update(project: &str, id: u64, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("projects/{}/labels/{id}", encode_id(project))).with_json(body)
}

#[must_use]
pub fn delete(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}/labels/{id}", encode_id(project)))
}

#[must_use]
pub fn subscribe(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels/{id}/subscribe", encode_id(project)))
}

#[must_use]
pub fn unsubscribe(project: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/labels/{id}/unsubscribe", encode_id(project)))
}
