pub mod projects;

/// Percent-encode a project identifier (numeric id or path-with-namespace).
#[must_use]
pub fn encode_id(id: &str) -> String {
    if id.chars().all(|c| c.is_ascii_digit()) {
        return id.to_owned();
    }
    urlencoding::encode(id).into_owned()
}
