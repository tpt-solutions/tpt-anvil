// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tracks the last few (provider, model) pairs actually used, so IDE UIs can
//! offer a quick-pick list instead of the user re-typing a model id (or the
//! full live catalog from `CloudProvider::list_models`) every time.

use std::path::Path;

use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentModel {
    pub provider: String,
    pub model_id: String,
}

/// Most-recently-used (provider, model) pairs, newest first, capped at
/// `MAX_ENTRIES` with no duplicates (re-using a model moves it to the front
/// instead of adding a second entry).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentModels {
    entries: Vec<RecentModel>,
}

impl RecentModels {
    pub fn record(&mut self, provider: impl Into<String>, model_id: impl Into<String>) {
        let entry = RecentModel {
            provider: provider.into(),
            model_id: model_id.into(),
        };
        self.entries.retain(|e| *e != entry);
        self.entries.insert(0, entry);
        self.entries.truncate(MAX_ENTRIES);
    }

    pub fn list(&self) -> &[RecentModel] {
        &self.entries
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_most_recent_first() {
        let mut recent = RecentModels::default();
        recent.record("openai", "gpt-4o");
        recent.record("anthropic", "claude-sonnet-5");
        assert_eq!(recent.list()[0].model_id, "claude-sonnet-5");
        assert_eq!(recent.list()[1].model_id, "gpt-4o");
    }

    #[test]
    fn re_recording_moves_to_front_without_duplicating() {
        let mut recent = RecentModels::default();
        recent.record("openai", "gpt-4o");
        recent.record("anthropic", "claude-sonnet-5");
        recent.record("openai", "gpt-4o");
        assert_eq!(recent.list().len(), 2);
        assert_eq!(recent.list()[0].model_id, "gpt-4o");
    }

    #[test]
    fn caps_at_five_entries() {
        let mut recent = RecentModels::default();
        for i in 0..8 {
            recent.record("openai", format!("model-{i}"));
        }
        assert_eq!(recent.list().len(), 5);
        assert_eq!(recent.list()[0].model_id, "model-7");
    }

    #[test]
    fn round_trips_through_disk() {
        let mut recent = RecentModels::default();
        recent.record("openai", "gpt-4o");
        let path = std::env::temp_dir().join(format!(
            "anvil-recent-models-test-{}.json",
            std::process::id()
        ));
        recent.save(&path).unwrap();
        let loaded = RecentModels::load(&path);
        assert_eq!(loaded.list(), recent.list());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("anvil-recent-models-does-not-exist.json");
        let loaded = RecentModels::load(&path);
        assert!(loaded.list().is_empty());
    }
}
