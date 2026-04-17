use reqwest::Method;
use crate::request::RequestSpec;
use super::encode_id;

fn path_for(project: &str, file: &str, suffix: &str) -> String {
    let encoded_file = urlencoding::encode(file);
    if suffix.is_empty() {
        format!("projects/{}/repository/files/{}", encode_id(project), encoded_file)
    } else {
        format!("projects/{}/repository/files/{}/{suffix}", encode_id(project), encoded_file)
    }
}

#[must_use]
pub fn get(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "")).with_query([("ref", rref)])
}

#[must_use]
pub fn raw(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "raw")).with_query([("ref", rref)])
}

#[must_use]
pub fn blame(project: &str, file: &str, rref: &str) -> RequestSpec {
    RequestSpec::new(Method::GET, path_for(project, file, "blame")).with_query([("ref", rref)])
}

#[must_use]
pub fn create(project: &str, file: &str, branch: &str, content: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::POST, path_for(project, file, ""))
        .with_json(&serde_json::json!({"branch":branch,"content":content,"commit_message":message}))
}

#[must_use]
pub fn update(project: &str, file: &str, branch: &str, content: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::PUT, path_for(project, file, ""))
        .with_json(&serde_json::json!({"branch":branch,"content":content,"commit_message":message}))
}

#[must_use]
pub fn delete(project: &str, file: &str, branch: &str, message: &str) -> RequestSpec {
    RequestSpec::new(Method::DELETE, path_for(project, file, ""))
        .with_query([("branch", branch), ("commit_message", message)])
}
