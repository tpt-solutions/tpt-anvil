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
    /// Whether the task was skipped because a required verification toolchain
    /// (compiler, linter, or test runner) was unavailable in this environment.
    ///
    /// Skipped tasks are excluded from the score entirely: a missing `mypy`
    /// says nothing about the model's ability, so counting it as a failure
    /// would understate the score and make runs non-comparable across machines.
    #[serde(default)]
    pub skipped: bool,
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
///
/// Skipped tasks (missing verification toolchain) are excluded from both the
/// numerator and the denominator.  Returns a value between 0.0 and 1.0.
/// Returns 0.0 when no scorable results remain.
pub fn compute_score(results: &[TaskRunResult]) -> f64 {
    let scorable: Vec<&TaskRunResult> = results.iter().filter(|r| !r.skipped).collect();
    if scorable.is_empty() {
        return 0.0;
    }
    let passed = scorable.iter().filter(|r| r.passed).count() as f64;
    passed / scorable.len() as f64
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
            skipped: false,
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

    #[test]
    fn skipped_tasks_are_excluded_from_score() {
        // One pass, one genuine fail, two skipped (missing toolchain).
        // Score must be 1/2, not 1/4.
        let mut s1 = make_result("s1", false);
        s1.skipped = true;
        let mut s2 = make_result("s2", false);
        s2.skipped = true;
        let results = vec![make_result("a", true), make_result("b", false), s1, s2];
        assert!((compute_score(&results) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn all_skipped_scores_zero() {
        let mut s1 = make_result("s1", false);
        s1.skipped = true;
        let mut s2 = make_result("s2", false);
        s2.skipped = true;
        assert_eq!(compute_score(&[s1, s2]), 0.0);
    }

    #[test]
    fn skipped_deserializes_as_false_when_absent() {
        // Backwards compatibility: scorecards written before `skipped` existed
        // must still load.
        let json = r#"{
            "provider":"ollama","model_id":"m","last_run_at":"2026-01-01",
            "core_task_ids_run":[],"core_results":[],"core_score":0.5
        }"#;
        let card: ModelScorecard = serde_json::from_str(json).expect("legacy scorecard loads");
        assert!(card.core_results.is_empty());
        assert_eq!(card.core_score, 0.5);
    }

    #[test]
    fn skipped_round_trips_through_json() {
        let mut s = make_result("s", false);
        s.skipped = true;
        let card = ModelScorecard {
            provider: "ollama".into(),
            model_id: "m".into(),
            last_run_at: "2026-01-01".into(),
            core_task_ids_run: vec!["s".into()],
            core_results: vec![s],
            adaptive_results: vec![],
            core_score: 0.0,
            adaptive_score: None,
            total_cost_usd: 0.0,
        };
        let json = serde_json::to_string(&card).expect("serialize");
        assert!(json.contains("\"skipped\":true"));
        let back: ModelScorecard = serde_json::from_str(&json).expect("deserialize");
        assert!(back.core_results[0].skipped);
    }
}
