pub mod api;
pub mod config;
pub mod group;
pub mod me;
pub mod mr;
pub mod project;
pub mod version;

pub fn load_json(raw: &str) -> anyhow::Result<serde_json::Value> {
    if let Some(p) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(p)?;
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(serde_json::from_str(raw)?)
    }
}
