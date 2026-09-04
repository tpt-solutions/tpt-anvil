// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Core and adaptive benchmark task types, plus staggered-rotation logic.

use serde::{Deserialize, Serialize};

/// A language tag for benchmark tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskLanguage {
    Rust,
    TypeScript,
    Python,
    Go,
}

/// The kind of verification a task uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Compiled language — `cargo check` / `tsc --noEmit` / `go build`.
    Compiler,
    /// Test suite — `cargo test` / `jest` / `pytest` / `go test`.
    Tests,
    /// Linter — `clippy` / `eslint` / `ruff` / `golangci-lint`.
    Lint,
}

/// A core benchmark task — part of the comparable, rotating suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreTask {
    /// Stable unique identifier (e.g. `"rust-borrow-checker-01"`).
    pub id: String,
    /// Short human-readable description shown in reports.
    pub description: String,
    pub language: TaskLanguage,
    pub kind: TaskKind,
    /// ISO-8601 date when this task was introduced to the suite.
    pub introduced_at: String,
    /// ISO-8601 date when this task will be retired (exclusive upper bound).
    pub retires_at: String,
    /// The full prompt sent to the model, referencing `scaffold/` placeholders.
    pub prompt: String,
    /// Relative path to the scaffold directory for this language (from the
    /// benchmarks root).
    pub scaffold_path: String,
    /// Relative path to the initial file the model must modify (within
    /// `scaffold_path`).
    pub target_file: String,
    /// Compiler/lint/test config overrides for this task (optional).
    #[serde(default)]
    pub verify_overrides: Option<VerifyOverrides>,
}

/// Per-task verification overrides (only fields present are overridden).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOverrides {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub run_tests: Option<bool>,
    #[serde(default)]
    pub run_linter: Option<bool>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// An adaptive (model-specific) benchmark task generated from prior failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveTask {
    /// The core task id this adaptive task was derived from.
    pub derived_from: String,
    /// Unique id for this adaptive task.
    pub id: String,
    pub description: String,
    pub language: TaskLanguage,
    pub kind: TaskKind,
    /// The prompt used to generate this task from the evaluator model.
    pub generation_prompt: String,
    /// The full prompt sent to the model under test.
    pub prompt: String,
    #[serde(default)]
    pub scaffold_path: Option<String>,
    #[serde(default)]
    pub target_file: Option<String>,
    #[serde(default)]
    pub verify_overrides: Option<VerifyOverrides>,
}

/// Filter `tasks` by the staggered rotation window: a task is active when
/// `introduced_at <= at` and `at < retires_at`.  Dates are compared as
/// lexicographic strings (ISO-8601 sorts correctly).
pub fn active_tasks<'a>(tasks: &'a [CoreTask], at: &str) -> Vec<&'a CoreTask> {
    tasks
        .iter()
        .filter(|t| t.introduced_at.as_str() <= at && at < t.retires_at.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(id: &str, introduced: &str, retires: &str) -> CoreTask {
        CoreTask {
            id: id.into(),
            description: "test".into(),
            language: TaskLanguage::Rust,
            kind: TaskKind::Compiler,
            introduced_at: introduced.into(),
            retires_at: retires.into(),
            prompt: "fix this".into(),
            scaffold_path: "scaffold/rust".into(),
            target_file: "main.rs".into(),
            verify_overrides: None,
        }
    }

    #[test]
    fn active_tasks_boundary_inclusive_start() {
        let tasks = vec![sample_task("a", "2026-01-01", "2026-07-01")];
        assert_eq!(active_tasks(&tasks, "2026-01-01").len(), 1);
    }

    #[test]
    fn active_tasks_boundary_exclusive_end() {
        let tasks = vec![sample_task("a", "2026-01-01", "2026-07-01")];
        assert!(active_tasks(&tasks, "2026-07-01").is_empty());
    }

    #[test]
    fn active_tasks_midpoint() {
        let tasks = vec![
            sample_task("a", "2026-01-01", "2026-07-01"),
            sample_task("b", "2026-04-01", "2026-10-01"),
        ];
        let active = active_tasks(&tasks, "2026-05-01");
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn active_tasks_empty_pool() {
        let tasks: Vec<CoreTask> = vec![];
        assert!(active_tasks(&tasks, "2026-05-01").is_empty());
    }

    #[test]
    fn active_tasks_window_overlap() {
        // Task a: Jan–Jul, Task b: Apr–Oct, Task c: Aug–Dec
        let tasks = vec![
            sample_task("a", "2026-01-01", "2026-07-01"),
            sample_task("b", "2026-04-01", "2026-10-01"),
            sample_task("c", "2026-08-01", "2026-12-31"),
        ];
        // May: a+b active
        assert_eq!(active_tasks(&tasks, "2026-05-01").len(), 2);
        // Jul 1: a is retired (retires_at is exclusive), b active
        assert_eq!(active_tasks(&tasks, "2026-07-01").len(), 1);
        // Sep: b+c active
        assert_eq!(active_tasks(&tasks, "2026-09-01").len(), 2);
        // Dec: c active
        assert_eq!(active_tasks(&tasks, "2026-12-15").len(), 1);
    }
}
