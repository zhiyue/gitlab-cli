use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list_spec(search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("groups");
    if let Some(s) = search {
        p.query.push(("search".into(), s.into()));
    }
    p
}

#[must_use]
pub fn get_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("groups/{}", encode_id(id)))
}

#[must_use]
pub fn members_spec(id: &str) -> PageRequest {
    PageRequest::new(format!("groups/{}/members", encode_id(id)))
}

#[must_use]
pub fn projects_spec(id: &str) -> PageRequest {
    PageRequest::new(format!("groups/{}/projects", encode_id(id)))
}

#[must_use]
pub fn subgroups_spec(id: &str) -> PageRequest {
    PageRequest::new(format!("groups/{}/subgroups", encode_id(id)))
}

#[must_use]
pub fn create_spec(name: &str, path: &str, parent_id: Option<u64>) -> RequestSpec {
    let mut body = serde_json::json!({"name": name, "path": path});
    if let Some(pid) = parent_id {
        body["parent_id"] = serde_json::json!(pid);
    }
    RequestSpec::new(Method::POST, "groups").with_json(&body)
}

#[must_use]
pub fn update_spec(id: &str, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("groups/{}", encode_id(id))).with_json(body)
}

#[must_use]
pub fn delete_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("groups/{}", encode_id(id)))
}
