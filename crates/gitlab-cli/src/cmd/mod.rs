pub mod api;
pub mod branch;
pub mod commit;
pub mod config;
pub mod file;
pub mod group;
pub mod issue;
pub mod job;
pub mod label;
pub mod me;
pub mod mr;
pub mod pipeline;
pub mod project;
pub mod repo;
pub mod tag;
pub mod user;
pub mod version;

pub fn load_json(raw: &str) -> anyhow::Result<serde_json::Value> {
    if let Some(p) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(p)?;
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(serde_json::from_str(raw)?)
    }
}
