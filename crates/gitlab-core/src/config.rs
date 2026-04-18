use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{GitlabError, Result};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Config {
    pub default_host: Option<String>,
    #[serde(default)]
    pub host: HashMap<String, HostConfig>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct HostConfig {
    pub token: Option<String>,
    #[serde(default)]
    pub tls_skip_verify: bool,
    pub rps: Option<u32>,
    pub default_project: Option<String>,
    /// When true, write commands skip the interactive 'type yes' prompt for this host.
    /// Equivalent to passing `--yes` / setting `GITLAB_ASSUME_YES=1`.
    #[serde(default)]
    pub assume_yes: bool,
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| GitlabError::Config(e.to_string())),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(GitlabError::Config(e.to_string())),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GitlabError::Config(e.to_string()))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| GitlabError::Config(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| GitlabError::Config(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    #[must_use]
    pub fn host_for(&self, host: Option<&str>) -> Option<&HostConfig> {
        let key = host.or(self.default_host.as_deref())?;
        self.host.get(key)
    }

    #[must_use]
    pub fn default_config_path() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA")
                .map(|p| PathBuf::from(p).join("gitlab-cli").join("config.toml"))
        }
        #[cfg(not(windows))]
        {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
            Some(base.join("gitlab-cli").join("config.toml"))
        }
    }
}
