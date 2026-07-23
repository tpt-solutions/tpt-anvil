// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Scorecard types — the result of running a benchmark suite against a model.

use serde::{Deserialize, Serialize};

use super::suite::TaskKind;

/// The outcome of a single task run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunResult {
    /// The task id this result corresponds to.
    pub task_id: String,
    pub task_kind: TaskKind,
    pub passed: bool,
    /// Wall-clock duration in milliseconds.
    pub latency_ms: u64,
    /// Prompt tokens consumed (if available).
    #[serde(default)]
    pub prompt_tokens: Option<u32>,
    /// Completion tokens generated (if available).
    #[serde(default)]
    pub completion_tokens: Option<u32>,
    /// Estimated cost in USD (only for cloud backends; `None` for local).
    #[serde(default)]
    pub cost_usd: Option<f64>,
    /// Compiler/lint/test output (truncated).
    #[serde(default)]
    pub output: Option<String>,
    /// Error strings if the task did not pass.
    #[serde(default)]
    pub errors: Vec<String>,
}

/// A scorecard for a specific (provider, model) pair after one benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScorecard {
    /// Provider name (e.g. `"ollama"`, `"openai"`, `"anthropic"`).
    pub provider: String,
    /// Model id (e.g. `"deepseek-coder:6.7b"`, `"gpt-4o"`).
    pub model_id: String,
    /// ISO-8601 timestamp of when this scorecard was generated.
    pub last_run_at: String,
    /// Core task ids that were included in this run.
    pub core_task_ids_run: Vec<String>,
    /// Results for core tasks.
    pub core_results: Vec<TaskRunResult>,
    /// Results for adaptive (model-specific) tasks, if any.
    #[serde(default)]
    pub adaptive_results: Vec<TaskRunResult>,
    /// Score on core tasks: fraction of `passed` (0.0–1.0).
    pub core_score: f64,
    /// Score on adaptive tasks (only present when adaptive tasks were run).
    #[serde(default)]
    pub adaptive_score: Option<f64>,
    /// Total estimated cost in USD across all tasks.
    #[serde(default)]
    pub total_cost_usd: f64,
}

/// Compute the pass-rate score from a list of task run results.
/// Returns a value between 0.0 and 1.0.  Returns 0.0 for an empty list.
pub fn compute_score(results: &[TaskRunResult]) -> f64 {
    if results.is_empty() {
        return 0.0;
    }
    let passed = results.iter().filter(|r| r.passed).count() as f64;
    passed / results.len() as f64
}

/// Filter results to only include the given task ids.
pub fn filter_results(results: &[TaskRunResult], task_ids: &[String]) -> Vec<TaskRunResult> {
    results
        .iter()
        .filter(|r| task_ids.contains(&r.task_id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(task_id: &str, passed: bool) -> TaskRunResult {
        TaskRunResult {
            task_id: task_id.into(),
            task_kind: TaskKind::Compiler,
            passed,
            latency_ms: 100,
            prompt_tokens: None,
            completion_tokens: None,
            cost_usd: None,
            output: None,
            errors: vec![],
        }
    }

    #[test]
    fn score_all_pass() {
        let results = vec![make_result("a", true), make_result("b", true)];
        assert!((compute_score(&results) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_none_pass() {
        let results = vec![make_result("a", false), make_result("b", false)];
        assert!((compute_score(&results) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_half_pass() {
        let results = vec![make_result("a", true), make_result("b", false)];
        assert!((compute_score(&results) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn score_empty() {
        let results: Vec<TaskRunResult> = vec![];
        assert!((compute_score(&results) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn filter_results_selective() {
        let results = vec![
            make_result("a", true),
            make_result("b", false),
            make_result("c", true),
        ];
        let filtered = filter_results(&results, &["a".into(), "c".into()]);
        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .all(|r| r.task_id == "a" || r.task_id == "c"));
    }
}
