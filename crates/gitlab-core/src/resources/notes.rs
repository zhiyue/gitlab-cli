use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Issue,
    Mr,
    Commit,
    Snippet,
}

impl Kind {
    #[must_use]
    pub fn plural(self) -> &'static str {
        match self {
            Kind::Issue => "issues",
            Kind::Mr => "merge_requests",
            Kind::Commit => "repository/commits",
            Kind::Snippet => "snippets",
        }
    }
}

fn base(project: &str, kind: Kind, target: &str) -> String {
    format!(
        "projects/{}/{}/{}/notes",
        encode_id(project),
        kind.plural(),
        encode_id(target)
    )
}

#[must_use]
pub fn list(project: &str, kind: Kind, target: &str) -> PageRequest {
    PageRequest::new(base(project, kind, target))
}

#[must_use]
pub fn get(project: &str, kind: Kind, target: &str, id: u64) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("{}/{id}", base(project, kind, target)))
}

#[must_use]
pub fn create(project: &str, kind: Kind, target: &str, body: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, base(project, kind, target))
        .with_json(&serde_json::json!({"body": body}))
}

#[must_use]
pub fn update(project: &str, kind: Kind, target: &str, id: u64, body: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_json(&serde_json::json!({"body": body}))
}

#[must_use]
pub fn delete(project: &str, kind: Kind, target: &str, id: u64) -> RequestSpec {
    RequestSpec::new(
        Method::DELETE,
        format!("{}/{id}", base(project, kind, target)),
    )
}
