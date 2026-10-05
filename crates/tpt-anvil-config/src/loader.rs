// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml::Value;
use tracing::debug;

use crate::schema::AnvilConfig;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Return the path to the user-level config file, if determinable.
    pub fn config_path() -> Option<PathBuf> {
        user_config_path()
    }

    /// Load config using a fallback chain: project → user → defaults.
    ///
    /// Layers are merged as raw TOML tables *before* being deserialized into
    /// `AnvilConfig`, so a higher-priority layer overrides a lower one only
    /// for keys it actually specifies — including a key whose explicit value
    /// happens to equal that field's built-in default. Merging already-typed
    /// `AnvilConfig` values field-by-field can't make that distinction (an
    /// explicit "ollama" and an absent value are indistinguishable once
    /// defaults have been filled in), so the merge has to happen at this
    /// layer, before defaults are applied.
    pub fn load(project_root: Option<&Path>) -> Result<AnvilConfig> {
        let mut merged = Value::Table(Default::default());

        if let Some(user_path) = user_config_path() {
            if user_path.exists() {
                debug!("loading user config from {}", user_path.display());
                let user_value = Self::load_value(&user_path)?;
                merged = merge_toml_values(merged, user_value);
            }
        }

        if let Some(root) = project_root {
            let project_path = root.join(".anvil").join("config.toml");
            if project_path.exists() {
                debug!("loading project config from {}", project_path.display());
                let project_value = Self::load_value(&project_path)?;
                merged = merge_toml_values(merged, project_value);
            }
        }

        let config: AnvilConfig = merged
            .try_into()
            .context("failed to apply defaults to merged config")?;
        Ok(config)
    }

    fn load_value(path: &Path) -> Result<Value> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let value: Value = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(value)
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

/// Recursively merge two TOML values: for tables, `overlay` wins per-key
/// (recursing into nested tables); any other value type is wholesale
/// replaced by `overlay` when present.
fn merge_toml_values(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Table(mut base_table), Value::Table(overlay_table)) => {
            for (key, overlay_val) in overlay_table {
                let merged_val = match base_table.remove(&key) {
                    Some(base_val) => merge_toml_values(base_val, overlay_val),
                    None => overlay_val,
                };
                base_table.insert(key, merged_val);
            }
            Value::Table(base_table)
        }
        (_, overlay) => overlay,
    }
}

fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("anvil").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::merge_toml_values;
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

    fn merged_config(base_toml: &str, overlay_toml: &str) -> AnvilConfig {
        let base: toml::Value = toml::from_str(base_toml).unwrap();
        let overlay: toml::Value = toml::from_str(overlay_toml).unwrap();
        merge_toml_values(base, overlay).try_into().unwrap()
    }

    #[test]
    fn merge_preserves_base_when_overlay_is_empty() {
        let merged = merged_config(
            r#"
            [inference]
            backend = "llama_cpp"
            model = "codellama:13b"
            "#,
            "",
        );
        assert_eq!(merged.inference.backend, "llama_cpp");
        assert_eq!(merged.inference.model, "codellama:13b");
    }

    #[test]
    fn merge_overlay_wins_for_non_default() {
        let merged = merged_config(
            r#"
            [inference]
            backend = "ollama"
            model = "deepseek-coder:6.7b"
            "#,
            r#"
            [inference]
            backend = "candle"
            model = "codellama:34b"
            "#,
        );
        assert_eq!(merged.inference.backend, "candle");
        assert_eq!(merged.inference.model, "codellama:34b");
    }

    /// Regression test: an overlay explicitly setting a field to the same
    /// value as that field's built-in default must still win over a base
    /// that set a *different* value. Merging already-typed `AnvilConfig`
    /// structs field-by-field (the old approach) couldn't tell "explicitly
    /// set to the default" apart from "not set at all" once defaults had
    /// been filled in, so this case used to silently keep the base value.
    #[test]
    fn merge_explicit_overlay_value_equal_to_default_still_wins() {
        let merged = merged_config(
            r#"
            [inference]
            backend = "llama_cpp"
            model = "codellama:13b"
            ollama_url = "http://custom:11434"
            context_length = 16384

            [providers]
            active = "openai"
            "#,
            r#"
            [inference]
            backend = "ollama"
            "#,
        );
        // backend explicitly overridden to "ollama" — which happens to be
        // the field's default — must still win over the base's "llama_cpp".
        assert_eq!(merged.inference.backend, "ollama");
        // model wasn't in the overlay at all, so the base value survives.
        assert_eq!(merged.inference.model, "codellama:13b");
        assert_eq!(merged.inference.ollama_url, "http://custom:11434");
        assert_eq!(merged.inference.context_length, 16384);
        assert_eq!(merged.providers.active, Some("openai".into()));
    }

    #[test]
    fn merge_optional_fields() {
        let merged = merged_config(
            r#"
            [providers.openai]
            api_key_entry = "my_key"
            "#,
            r#"
            [providers.openai]
            model = "gpt-4o"
            "#,
        );
        assert_eq!(merged.providers.openai.api_key_entry, Some("my_key".into()));
        assert_eq!(merged.providers.openai.model, "gpt-4o".to_string());
    }

    /// Regression test: a partial `[benchmark]` table (only some keys set)
    /// merges correctly with defaults rather than failing on missing fields.
    #[test]
    fn merge_partial_benchmark_table() {
        let merged = merged_config(
            "",
            r#"
            [benchmark]
            enabled = true
            rotation_period_days = 90
            "#,
        );
        assert!(merged.benchmark.enabled);
        assert_eq!(merged.benchmark.rotation_period_days, 90);
        // Other fields should retain defaults
        assert_eq!(merged.benchmark.stagger_interval_days, 30);
        assert_eq!(merged.benchmark.max_stored, 30);
        assert!(!merged.benchmark.adaptive.enabled);
    }
}
