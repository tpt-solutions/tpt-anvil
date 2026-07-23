// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Compare two scorecards on their shared task subset.

use super::scorecard::{compute_score, filter_results, ModelScorecard};

/// The result of comparing two scorecards.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Provider + model of the "left" scorecard.
    pub left_label: String,
    /// Provider + model of the "right" scorecard.
    pub right_label: String,
    /// Core task ids present in both scorecards (the comparable subset).
    pub shared_task_ids: Vec<String>,
    /// Left score restricted to the shared subset.
    pub left_shared_score: f64,
    /// Right score restricted to the shared subset.
    pub right_shared_score: f64,
    /// Task ids present in `left` but missing from `right`.
    pub left_only_task_ids: Vec<String>,
    /// Task ids present in `right` but missing from `left`.
    pub right_only_task_ids: Vec<String>,
}

/// Compare two scorecards.  Intersects their `core_task_ids_run` and
/// recomputes scores restricted to the shared subset so the comparison is
/// fair even when the suite rotated between runs.
pub fn compare(a: &ModelScorecard, b: &ModelScorecard) -> ComparisonResult {
    let shared: Vec<String> = a
        .core_task_ids_run
        .iter()
        .filter(|id| b.core_task_ids_run.contains(id))
        .cloned()
        .collect();

    let left_only: Vec<String> = a
        .core_task_ids_run
        .iter()
        .filter(|id| !b.core_task_ids_run.contains(id))
        .cloned()
        .collect();

    let right_only: Vec<String> = b
        .core_task_ids_run
        .iter()
        .filter(|id| !a.core_task_ids_run.contains(id))
        .cloned()
        .collect();

    let left_filtered = filter_results(&a.core_results, &shared);
    let right_filtered = filter_results(&b.core_results, &shared);

    let left_shared_score = compute_score(&left_filtered);
    let right_shared_score = compute_score(&right_filtered);

    ComparisonResult {
        left_label: format!("{}/{}", a.provider, a.model_id),
        right_label: format!("{}/{}", b.provider, b.model_id),
        shared_task_ids: shared,
        left_shared_score,
        right_shared_score,
        left_only_task_ids: left_only,
        right_only_task_ids: right_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::scorecard::TaskRunResult;
    use crate::benchmark::suite::TaskKind;

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

    fn make_scorecard(
        provider: &str,
        model: &str,
        task_ids: &[&str],
        results: Vec<TaskRunResult>,
    ) -> ModelScorecard {
        let core_score = compute_score(&results);
        ModelScorecard {
            provider: provider.into(),
            model_id: model.into(),
            last_run_at: "2026-07-01".into(),
            core_task_ids_run: task_ids.iter().map(|s| s.to_string()).collect(),
            core_results: results,
            adaptive_results: vec![],
            core_score,
            adaptive_score: None,
            total_cost_usd: 0.0,
        }
    }

    #[test]
    fn full_overlap() {
        let a = make_scorecard(
            "ollama",
            "m1",
            &["t1", "t2", "t3"],
            vec![
                make_result("t1", true),
                make_result("t2", true),
                make_result("t3", false),
            ],
        );
        let b = make_scorecard(
            "openai",
            "m2",
            &["t1", "t2", "t3"],
            vec![
                make_result("t1", false),
                make_result("t2", true),
                make_result("t3", true),
            ],
        );
        let cmp = compare(&a, &b);
        assert_eq!(cmp.shared_task_ids.len(), 3);
        assert!(cmp.left_only_task_ids.is_empty());
        assert!(cmp.right_only_task_ids.is_empty());
        // a: 2/3, b: 2/3
        assert!((cmp.left_shared_score - 2.0 / 3.0).abs() < 0.01);
        assert!((cmp.right_shared_score - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn partial_overlap() {
        let a = make_scorecard(
            "ollama",
            "m1",
            &["t1", "t2", "t3"],
            vec![
                make_result("t1", true),
                make_result("t2", true),
                make_result("t3", true),
            ],
        );
        let b = make_scorecard(
            "openai",
            "m2",
            &["t2", "t3", "t4"],
            vec![
                make_result("t2", false),
                make_result("t3", true),
                make_result("t4", true),
            ],
        );
        let cmp = compare(&a, &b);
        // Shared: t2, t3
        assert_eq!(cmp.shared_task_ids.len(), 2);
        assert!(cmp.left_only_task_ids.contains(&"t1".to_string()));
        assert!(cmp.right_only_task_ids.contains(&"t4".to_string()));
        // a: t2 pass, t3 pass = 2/2 = 1.0
        assert!((cmp.left_shared_score - 1.0).abs() < 0.01);
        // b: t2 fail, t3 pass = 1/2 = 0.5
        assert!((cmp.right_shared_score - 0.5).abs() < 0.01);
    }

    #[test]
    fn no_overlap() {
        let a = make_scorecard("ollama", "m1", &["t1"], vec![make_result("t1", true)]);
        let b = make_scorecard("openai", "m2", &["t2"], vec![make_result("t2", true)]);
        let cmp = compare(&a, &b);
        assert!(cmp.shared_task_ids.is_empty());
        assert!((cmp.left_shared_score - 0.0).abs() < f64::EPSILON);
        assert!((cmp.right_shared_score - 0.0).abs() < f64::EPSILON);
        assert_eq!(cmp.left_only_task_ids.len(), 1);
        assert_eq!(cmp.right_only_task_ids.len(), 1);
    }
}
