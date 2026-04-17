use reqwest::Method;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use super::encode_id;

#[must_use]
pub fn tree(project: &str, path: Option<&str>, rref: Option<&str>, recursive: bool) -> PageRequest {
    let mut p = PageRequest::new(format!("projects/{}/repository/tree", encode_id(project)));
    if let Some(pp) = path { p.query.push(("path".into(), pp.into())); }
    if let Some(r) = rref { p.query.push(("ref".into(), r.into())); }
    if recursive { p.query.push(("recursive".into(), "true".into())); }
    p
}

#[must_use]
pub fn archive(project: &str, sha: Option<&str>, format: Option<&str>) -> RequestSpec {
    let base = format!("projects/{}/repository/archive", encode_id(project));
    let path = match format { Some(f) => format!("{base}.{f}"), None => base };
    let mut s = RequestSpec::new(Method::GET, path);
    if let Some(sh) = sha { s = s.with_query([("sha", sh)]); }
    s
}

#[must_use]
pub fn compare(project: &str, from: &str, to: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("projects/{}/repository/compare", encode_id(project)))
        .with_query([("from", from), ("to", to)])
}

#[must_use]
pub fn contributors(project: &str) -> PageRequest {
    PageRequest::new(format!("projects/{}/repository/contributors", encode_id(project)))
}

#[must_use]
pub fn merge_base(project: &str, refs: &[String]) -> RequestSpec {
    let mut s = RequestSpec::new(Method::GET, format!("projects/{}/repository/merge_base", encode_id(project)));
    for r in refs { s.query.push(("refs[]".into(), r.clone())); }
    s
}
