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
                let user_cfg = Self::load_file(&user_path)?;
                config = config.merge_with(user_cfg);
            }
        }

        // Project-level config (highest priority)
        if let Some(root) = project_root {
            let project_path = root.join(".anvil").join("config.toml");
            if project_path.exists() {
                debug!("loading project config from {}", project_path.display());
                let project_cfg = Self::load_file(&project_path)?;
                config = config.merge_with(project_cfg);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;

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

    #[test]
    fn merge_preserves_base_when_overlay_is_default() {
        let base = AnvilConfig {
            inference: InferenceConfig {
                backend: "llama_cpp".into(),
                model: "codellama:13b".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        // Overlay with all defaults — base values should survive
        let overlay = AnvilConfig::default();
        let merged = base.clone().merge_with(overlay);
        assert_eq!(merged.inference.backend, "llama_cpp");
        assert_eq!(merged.inference.model, "codellama:13b");
    }

    #[test]
    fn merge_overlay_wins_for_non_default() {
        let base = AnvilConfig {
            inference: InferenceConfig {
                backend: "ollama".into(),
                model: "deepseek-coder:6.7b".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let overlay = AnvilConfig {
            inference: InferenceConfig {
                backend: "candle".into(),
                model: "codellama:34b".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let merged = base.merge_with(overlay);
        assert_eq!(merged.inference.backend, "candle");
        assert_eq!(merged.inference.model, "codellama:34b");
    }

    #[test]
    fn merge_partial_overlay() {
        let base = AnvilConfig {
            inference: InferenceConfig {
                backend: "llama_cpp".into(),
                model: "codellama:13b".into(),
                ollama_url: "http://custom:11434".into(),
                context_length: 16384,
                ..Default::default()
            },
            providers: ProvidersConfig {
                active: Some("openai".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let overlay = AnvilConfig {
            inference: InferenceConfig {
                backend: "ollama".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let merged = base.merge_with(overlay);
        // backend changed to ollama (non-default in overlay)
        assert_eq!(merged.inference.backend, "ollama");
        // model preserved from base (overlay is default)
        assert_eq!(merged.inference.model, "codellama:13b");
        // ollama_url preserved from base
        assert_eq!(merged.inference.ollama_url, "http://custom:11434");
        // context_length preserved from base
        assert_eq!(merged.inference.context_length, 16384);
        // providers.active preserved from base
        assert_eq!(merged.providers.active, Some("openai".into()));
    }

    #[test]
    fn merge_optional_fields() {
        let base = AnvilConfig {
            providers: ProvidersConfig {
                openai: OpenAiConfig {
                    api_key_entry: Some("my_key".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let overlay = AnvilConfig {
            providers: ProvidersConfig {
                openai: OpenAiConfig {
                    model: "gpt-4o".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let merged = base.merge_with(overlay);
        assert_eq!(merged.providers.openai.api_key_entry, Some("my_key".into()));
        assert_eq!(merged.providers.openai.model, "gpt-4o".into());
    }
}
