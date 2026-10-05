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

/// The core task suite, embedded at compile time via `include_str!`.
///
/// Embedding (rather than reading from `CARGO_MANIFEST_DIR` at runtime) means a
/// released binary still has the suite available; `CARGO_MANIFEST_DIR` only
/// exists on the machine that built the crate.
const EMBEDDED_CORE_TASKS: &[(&str, &str)] = &[
    (
        "py-list-comprehension-01.toml",
        include_str!("../../benchmarks/core/py-list-comprehension-01.toml"),
    ),
    (
        "py-type-annotation-01.toml",
        include_str!("../../benchmarks/core/py-type-annotation-01.toml"),
    ),
    (
        "rust-borrow-checker-01.toml",
        include_str!("../../benchmarks/core/rust-borrow-checker-01.toml"),
    ),
    (
        "rust-iterator-01.toml",
        include_str!("../../benchmarks/core/rust-iterator-01.toml"),
    ),
    (
        "rust-off-by-one-01.toml",
        include_str!("../../benchmarks/core/rust-off-by-one-01.toml"),
    ),
    (
        "rust-option-handling-01.toml",
        include_str!("../../benchmarks/core/rust-option-handling-01.toml"),
    ),
    (
        "ts-async-error-01.toml",
        include_str!("../../benchmarks/core/ts-async-error-01.toml"),
    ),
    (
        "ts-null-handling-01.toml",
        include_str!("../../benchmarks/core/ts-null-handling-01.toml"),
    ),
];

/// Scaffold files, embedded at compile time.
///
/// Each entry is keyed by its path relative to the benchmarks root (e.g.
/// `"scaffold/rust/main.rs"`). Companion build files (`Cargo.toml`,
/// `tsconfig.json`) are listed alongside the target source so the verification
/// toolchain can actually run inside the sandbox.
const EMBEDDED_SCAFFOLDS: &[(&str, &str)] = &[
    (
        "scaffold/python/utils.py",
        include_str!("../../benchmarks/scaffold/python/utils.py"),
    ),
    (
        "scaffold/rust/main.rs",
        include_str!("../../benchmarks/scaffold/rust/main.rs"),
    ),
    (
        "scaffold/rust/Cargo.toml",
        include_str!("../../benchmarks/scaffold/rust/Cargo.toml"),
    ),
    (
        "scaffold/typescript/utils.ts",
        include_str!("../../benchmarks/scaffold/typescript/utils.ts"),
    ),
    (
        "scaffold/typescript/tsconfig.json",
        include_str!("../../benchmarks/scaffold/typescript/tsconfig.json"),
    ),
];

/// Load the built-in core task suite.
///
/// If `override_dir` is `Some`, tasks are read from that directory instead
/// (the `benchmark.core_suite_path` local-dev override).
pub fn load_builtin_tasks(override_dir: Option<&Path>) -> Vec<CoreTask> {
    if let Some(dir) = override_dir {
        return load_tasks_from_dir(dir).unwrap_or_default();
    }
    EMBEDDED_CORE_TASKS
        .iter()
        .filter_map(
            |(name, content)| match toml::from_str::<CoreTask>(content) {
                Ok(task) => Some(task),
                Err(e) => {
                    tracing::warn!("skipping embedded benchmark task {name}: {e}");
                    None
                }
            },
        )
        .collect()
}

/// Return the embedded content of a scaffold file, addressed by its path
/// relative to the benchmarks root.
pub fn embedded_scaffold(rel_path: &str) -> Option<&'static str> {
    EMBEDDED_SCAFFOLDS
        .iter()
        .find(|(p, _)| *p == rel_path)
        .map(|(_, content)| *content)
}

/// Materialize a task's scaffold into `dest_root`, returning the target file
/// path.
///
/// `scaffold_path` (e.g. `"scaffold/rust"`) selects a whole embedded scaffold
/// directory; every embedded file under it — the target source *and* companion
/// build files like `Cargo.toml` — is written into `dest`. Writing only the
/// target file would leave the verification toolchain without a project root.
pub fn materialize_scaffold(
    scaffold_path: &str,
    target_file: &str,
    dest_root: &Path,
) -> std::io::Result<std::path::PathBuf> {
    let prefix = format!("{}/", scaffold_path.trim_end_matches('/'));
    let matched: Vec<&(&str, &str)> = EMBEDDED_SCAFFOLDS
        .iter()
        .filter(|(p, _)| p.starts_with(&prefix))
        .collect();

    let target_key = format!("{prefix}{target_file}");
    if !matched.iter().any(|(p, _)| *p == target_key) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no embedded scaffold for '{target_key}'"),
        ));
    }

    std::fs::create_dir_all(dest_root)?;

    for (rel, content) in &matched {
        // `rel` is "<scaffold_path>/<inner>", so strip the prefix to get the
        // path relative to the sandbox root.
        let inner = rel
            .strip_prefix(prefix.as_str())
            .expect("matched entries share the prefix");
        let out = dest_root.join(inner);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, content)?;
    }

    Ok(dest_root.join(target_file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::suite::{TaskKind, TaskLanguage};

    #[test]
    fn embedded_suite_loads_all_core_tasks() {
        let tasks = load_builtin_tasks(None);
        assert_eq!(tasks.len(), EMBEDDED_CORE_TASKS.len());
        assert!(tasks.iter().all(|t| !t.id.is_empty()));
    }

    #[test]
    fn embedded_suite_ids_are_unique() {
        let tasks = load_builtin_tasks(None);
        let mut ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate task ids in embedded suite");
    }

    #[test]
    fn embedded_suite_covers_multiple_languages() {
        let tasks = load_builtin_tasks(None);
        assert!(tasks.iter().any(|t| t.language == TaskLanguage::Rust));
        assert!(tasks.iter().any(|t| t.language == TaskLanguage::Python));
        assert!(tasks.iter().any(|t| t.language == TaskLanguage::TypeScript));
    }

    #[test]
    fn every_task_resolves_to_an_embedded_scaffold() {
        for task in load_builtin_tasks(None) {
            let key = format!("{}/{}", task.scaffold_path, task.target_file);
            assert!(
                embedded_scaffold(&key).is_some(),
                "task '{}' references missing scaffold '{key}'",
                task.id
            );
        }
    }

    #[test]
    fn unknown_scaffold_returns_none() {
        assert!(embedded_scaffold("scaffold/go/nope.go").is_none());
    }

    #[test]
    fn materialize_scaffold_writes_file_and_manifests() {
        let dir = std::env::temp_dir().join("anvil-scaffold-materialize-test");
        let _ = std::fs::remove_dir_all(&dir);
        let target = dir.join("main.rs");
        let written = materialize_scaffold("scaffold/rust", "main.rs", &dir).expect("write");
        assert_eq!(written, target);
        let content = std::fs::read_to_string(&target).expect("read back");
        assert!(content.contains("fn main"));
        // The companion manifest must land too, otherwise `cargo check`
        // cannot run in the sandbox.
        assert!(
            dir.join("Cargo.toml").exists(),
            "Cargo.toml must be materialized alongside the source"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialize_scaffold_writes_typescript_manifest() {
        let dir = std::env::temp_dir().join("anvil-scaffold-ts-test");
        let _ = std::fs::remove_dir_all(&dir);
        materialize_scaffold("scaffold/typescript", "utils.ts", &dir).expect("write");
        assert!(dir.join("utils.ts").exists());
        assert!(dir.join("tsconfig.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialize_scaffold_tolerates_trailing_slash() {
        let dir = std::env::temp_dir().join("anvil-scaffold-trailing-slash-test");
        let target = materialize_scaffold("scaffold/rust/", "main.rs", &dir).expect("write");
        assert!(target.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materialize_scaffold_unknown_key_errors() {
        let dir = std::env::temp_dir().join("anvil-scaffold-missing-test");
        let err = materialize_scaffold("scaffold/go", "main.go", &dir)
            .expect_err("unknown scaffold must error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn materialize_scaffold_rejects_missing_target_in_known_dir() {
        let dir = std::env::temp_dir().join("anvil-scaffold-wrong-target-test");
        // The directory exists but the requested target file does not.
        let err = materialize_scaffold("scaffold/rust", "lib.rs", &dir)
            .expect_err("unknown target must error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_tasks_from_dir_rejects_missing_dir() {
        let err = load_tasks_from_dir(Path::new("/nonexistent/benchmarks/core"))
            .expect_err("missing dir must error");
        assert!(err.contains("failed to read benchmark dir"));
    }

    #[test]
    fn override_dir_missing_yields_empty_suite() {
        let tasks = load_builtin_tasks(Some(Path::new("/nonexistent/benchmarks/core")));
        assert!(tasks.is_empty());
    }

    #[test]
    fn task_kind_is_parsed_from_embedded_toml() {
        let tasks = load_builtin_tasks(None);
        let rust_task = tasks
            .iter()
            .find(|t| t.language == TaskLanguage::Rust)
            .expect("a rust task");
        assert!(matches!(
            rust_task.kind,
            TaskKind::Compiler | TaskKind::Tests | TaskKind::Lint
        ));
    }
}
