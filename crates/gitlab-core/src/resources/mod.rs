pub mod branches;
pub mod commits;
pub mod files;
pub mod groups;
pub mod issues;
pub mod jobs;
pub mod merge_requests;
pub mod pipelines;
pub mod projects;
pub mod repos;
pub mod tags;
pub mod users;

/// Percent-encode a project identifier (numeric id or path-with-namespace).
#[must_use]
pub fn encode_id(id: &str) -> String {
    if id.chars().all(|c| c.is_ascii_digit()) {
        return id.to_owned();
    }
    urlencoding::encode(id).into_owned()
}
