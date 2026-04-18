use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[must_use]
pub fn list(search: Option<&str>) -> PageRequest {
    let mut p = PageRequest::new("users");
    if let Some(s) = search {
        p.query.push(("search".into(), s.into()));
    }
    p
}

#[must_use]
pub fn get(id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("users/{id}"))
}

#[must_use]
pub fn me() -> RequestSpec {
    RequestSpec::new(Method::GET, "user")
}

#[must_use]
pub fn keys(user_id: u64) -> PageRequest {
    PageRequest::new(format!("users/{user_id}/keys"))
}

#[must_use]
pub fn emails(user_id: u64) -> PageRequest {
    PageRequest::new(format!("users/{user_id}/emails"))
}
