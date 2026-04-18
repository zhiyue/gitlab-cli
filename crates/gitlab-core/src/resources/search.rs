use super::encode_id;
use crate::page::PageRequest;

#[must_use]
pub fn global(scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new("search");
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}

#[must_use]
pub fn group(group: &str, scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new(format!("groups/{}/search", encode_id(group)));
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}

#[must_use]
pub fn project(project: &str, scope: &str, q: &str) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/search", encode_id(project)));
    p.query.push(("scope".into(), scope.into()));
    p.query.push(("search".into(), q.into()));
    p
}
