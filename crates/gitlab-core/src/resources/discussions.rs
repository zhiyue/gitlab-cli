use super::encode_id;
use crate::page::PageRequest;
use crate::request::RequestSpec;
use reqwest::Method;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Issue,
    Mr,
    Commit,
}

impl Kind {
    #[must_use]
    pub fn plural(self) -> &'static str {
        match self {
            Kind::Issue => "issues",
            Kind::Mr => "merge_requests",
            Kind::Commit => "repository/commits",
        }
    }
}

fn base(project: &str, kind: Kind, target: &str) -> String {
    format!(
        "projects/{}/{}/{}/discussions",
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
pub fn get(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, format!("{}/{id}", base(project, kind, target)))
}

#[must_use]
pub fn resolve(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_query([("resolved", "true")])
}

#[must_use]
pub fn unresolve(project: &str, kind: Kind, target: &str, id: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, format!("{}/{id}", base(project, kind, target)))
        .with_query([("resolved", "false")])
}
