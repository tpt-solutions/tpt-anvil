// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::debug;

use crate::schema::AnvilConfig;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load config using a fallback chain: project → user → defaults.
    pub fn load(project_root: Option<&Path>) -> Result<AnvilConfig> {
        let mut config = AnvilConfig::default();

        // User-level config
        if let Some(user_path) = user_config_path() {
            if user_path.exists() {
                debug!("loading user config from {}", user_path.display());
                let merged = Self::load_file(&user_path)?;
                config = merge(config, merged);
            }
        }

        // Project-level config (highest priority)
        if let Some(root) = project_root {
            let project_path = root.join(".anvil").join("config.toml");
            if project_path.exists() {
                debug!("loading project config from {}", project_path.display());
                let merged = Self::load_file(&project_path)?;
                config = merge(config, merged);
            }
        }

        Ok(config)
    }

    pub fn load_file(path: &Path) -> Result<AnvilConfig> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let cfg: AnvilConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save_user(config: &AnvilConfig) -> Result<()> {
        let path = user_config_path().context("cannot determine user config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("anvil").join("config.toml"))
}

fn merge(base: AnvilConfig, overlay: AnvilConfig) -> AnvilConfig {
    // Simple field-level merge: overlay non-default values win.
    // For a production impl this would be more granular (e.g. serde merge crate).
    // For now, overlay replaces base wholesale — config files are self-contained.
    let _ = base;
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = AnvilConfig::default();
        assert_eq!(cfg.inference.backend, "ollama");
        assert_eq!(cfg.inference.model, "deepseek-coder:6.7b");
    }

    #[test]
    fn round_trip_toml() {
        let cfg = AnvilConfig::default();
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let parsed: AnvilConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed.inference.backend, cfg.inference.backend);
    }
}
