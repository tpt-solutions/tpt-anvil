// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Persistent store for benchmark scorecards — cap-30 LRU by `last_run_at`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::scorecard::ModelScorecard;

/// Maximum number of scorecards retained in the store.
const MAX_STORED: usize = 30;

/// Persistent scorecard store.  Backed by a JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkStore {
    /// Stored scorecards, ordered by insertion (most recent first after
    /// `save`/`load`).
    #[serde(default)]
    entries: Vec<ModelScorecard>,
}

impl BenchmarkStore {
    /// Load the store from disk.  Returns an empty store if the file does not
    /// exist or fails to parse.
    pub fn load(path: &Path) -> Self {
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&data).unwrap_or_default()
    }

    /// Persist the store to disk, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    /// Return a reference to all stored scorecards (most-recent-first).
    pub fn entries(&self) -> &[ModelScorecard] {
        &self.entries
    }

    /// Record (or re-record) a scorecard.  If a scorecard with the same
    /// `(provider, model_id)` already exists, it is replaced.  After
    /// insertion, entries are sorted by `last_run_at` descending and the
    /// oldest entry beyond `MAX_STORED` is evicted.
    pub fn record(&mut self, scorecard: ModelScorecard) {
        // Remove any existing entry for the same provider+model
        self.entries
            .retain(|e| !(e.provider == scorecard.provider && e.model_id == scorecard.model_id));
        self.entries.insert(0, scorecard);
        // Evict oldest by timestamp (entries are most-recent-first,
        // so just truncate)
        self.entries.truncate(MAX_STORED);
    }

    /// Look up a scorecard by provider and model id.
    pub fn find(&self, provider: &str, model_id: &str) -> Option<&ModelScorecard> {
        self.entries
            .iter()
            .find(|e| e.provider == provider && e.model_id == model_id)
    }

    /// Return the default store path.
    ///
    /// Platform-dependent: `~/.config/anvil/benchmarks.json` on Unix, but
    /// `dirs::config_dir` resolves to `%APPDATA%` on Windows, so the file
    /// lands in `C:\Users\<user>\AppData\Roaming\anvil\benchmarks.json`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("anvil").join("benchmarks.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::scorecard::ModelScorecard;

    fn make_scorecard(provider: &str, model_id: &str, ts: &str) -> ModelScorecard {
        ModelScorecard {
            provider: provider.into(),
            model_id: model_id.into(),
            last_run_at: ts.into(),
            core_task_ids_run: vec![],
            core_results: vec![],
            adaptive_results: vec![],
            core_score: 0.5,
            adaptive_score: None,
            total_cost_usd: 0.0,
        }
    }

    #[test]
    fn record_and_find() {
        let mut store = BenchmarkStore::default();
        store.record(make_scorecard("ollama", "deepseek", "2026-07-01"));
        assert!(store.find("ollama", "deepseek").is_some());
        assert!(store.find("openai", "gpt-4o").is_none());
    }

    #[test]
    fn re_record_replaces() {
        let mut store = BenchmarkStore::default();
        store.record(make_scorecard("ollama", "deepseek", "2026-07-01"));
        store.record(make_scorecard("ollama", "deepseek", "2026-08-01"));
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].last_run_at, "2026-08-01");
    }

    #[test]
    fn lru_by_timestamp_not_insertion() {
        // Records are kept most-recently-recorded first (insert at 0)
        let mut store = BenchmarkStore::default();
        store.record(make_scorecard("a", "m1", "2026-01-01"));
        store.record(make_scorecard("a", "m2", "2026-06-01"));
        store.record(make_scorecard("a", "m3", "2026-03-01"));
        // Most recently recorded should be first
        assert_eq!(store.entries()[0].model_id, "m3");
        assert_eq!(store.entries()[1].model_id, "m2");
        assert_eq!(store.entries()[2].model_id, "m1");
    }

    #[test]
    fn cap_at_30() {
        let mut store = BenchmarkStore::default();
        for i in 0..35 {
            store.record(make_scorecard(
                "a",
                &format!("m{i}"),
                &format!("2026-01-{i:02}"),
            ));
        }
        assert_eq!(store.entries().len(), MAX_STORED);
        // The oldest (m0) should have been evicted
        assert!(store.find("a", "m0").is_none());
        // The newest should still be there
        assert!(store.find("a", "m34").is_some());
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let store = BenchmarkStore::load(Path::new("/nonexistent/path/benchmarks.json"));
        assert!(store.entries().is_empty());
    }

    #[test]
    fn default_path_is_platform_correct_and_under_config_dir() {
        // The path is platform-dependent, so a hardcoded `~/.config/anvil`
        // assumption would break on Windows. Assert the actual shape instead.
        let path = BenchmarkStore::default_path().expect("a config dir should resolve");
        assert!(
            path.ends_with("benchmarks.json"),
            "store file must be benchmarks.json, got {path:?}"
        );
        assert_eq!(
            path.parent().and_then(|p| p.file_name()),
            Some(std::ffi::OsStr::new("anvil")),
            "store must live in an `anvil` directory, got {path:?}"
        );

        let expected_root = dirs::config_dir().expect("config dir");
        assert_eq!(
            path.parent().and_then(|p| p.parent()),
            Some(expected_root.as_path()),
            "store must sit directly under the platform config dir"
        );

        // Pin the platform expectation explicitly so a change in `dirs`
        // behavior cannot pass unnoticed on either OS.
        if cfg!(windows) {
            assert!(
                path.to_string_lossy().contains("AppData"),
                "Windows config dir should be under AppData, got {path:?}"
            );
        } else {
            assert!(
                path.to_string_lossy().starts_with('/'),
                "Unix config dir should be absolute from root, got {path:?}"
            );
        }
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join("anvil-bench-test");
        let path = dir.join("benchmarks.json");
        let mut store = BenchmarkStore::default();
        store.record(make_scorecard("ollama", "deepseek", "2026-07-01"));
        store.save(&path).unwrap();

        let loaded = BenchmarkStore::load(&path);
        assert_eq!(loaded.entries().len(), 1);
        assert_eq!(loaded.entries()[0].provider, "ollama");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
