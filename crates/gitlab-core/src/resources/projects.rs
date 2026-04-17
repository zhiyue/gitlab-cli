use reqwest::Method;

use crate::client::Client;
use crate::error::Result;
use crate::page::{PageRequest, PagedStream};
use crate::request::RequestSpec;

use super::encode_id;

#[must_use]
pub fn list_spec(visibility: Option<&str>, search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("projects");
    if let Some(v) = visibility { p.query.push(("visibility".into(), v.into())); }
    if let Some(s) = search { p.query.push(("search".into(), s.into())); }
    p
}

pub fn stream(client: &Client, req: PageRequest) -> impl futures::Stream<Item = Result<serde_json::Value>> + Unpin {
    PagedStream::start(client, req)
}

#[must_use]
pub fn get_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}", encode_id(id)))
}

#[must_use]
pub fn create_spec(name: &str, path: Option<&str>, visibility: Option<&str>) -> RequestSpec {
    let mut body = serde_json::json!({ "name": name });
    if let Some(p) = path { body["path"] = serde_json::Value::String(p.into()); }
    if let Some(v) = visibility { body["visibility"] = serde_json::Value::String(v.into()); }
    RequestSpec::new(Method::POST, "projects").with_json(&body)
}

#[must_use]
pub fn update_spec(id: &str, body: &serde_json::Value) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("projects/{}", encode_id(id))).with_json(body)
}

#[must_use]
pub fn delete_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, format!("projects/{}", encode_id(id)))
}

#[must_use]
pub fn fork_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/fork", encode_id(id)))
}

#[must_use]
pub fn archive_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/archive", encode_id(id)))
}

#[must_use]
pub fn unarchive_spec(id: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, format!("projects/{}/unarchive", encode_id(id)))
}
