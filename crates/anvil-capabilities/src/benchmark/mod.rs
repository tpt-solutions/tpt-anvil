// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Model benchmarking — runs coding tasks against a model, grades results
//! objectively, and stores scorecards for cross-model comparison.

pub mod adaptive;
pub mod comparison;
pub mod runner;
pub mod scorecard;
pub mod store;
pub mod suite;

use std::path::Path;

use suite::CoreTask;

/// Load core tasks from a TOML directory on disk (local-dev override path).
pub fn load_tasks_from_dir(dir: &Path) -> Result<Vec<CoreTask>, String> {
    let mut tasks = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read benchmark dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let task: CoreTask = toml::from_str(&content)
                .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
            tasks.push(task);
        }
    }
    Ok(tasks)
}

/// Load the built-in core task suite (embedded at compile time).
pub fn load_builtin_tasks() -> Vec<CoreTask> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let core_dir = format!("{manifest_dir}/benchmarks/core");
    let path = std::path::Path::new(&core_dir);
    if path.exists() {
        load_tasks_from_dir(path).unwrap_or_default()
    } else {
        vec![]
    }
}
