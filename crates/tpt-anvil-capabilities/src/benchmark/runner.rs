// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Benchmark execution engine — dispatches tasks to local or cloud backends,
//! grades results via the verify gate.

use std::path::Path;
use std::time::Instant;

use crate::benchmark::scorecard::{compute_score, TaskRunResult};
use crate::benchmark::suite::CoreTask;
use crate::verify::{self, VerificationResult, VerifyConfig};

/// Summary of a single benchmark run across all tasks.
pub struct BenchmarkRunResult {
    pub results: Vec<TaskRunResult>,
    pub total_cost_usd: f64,
}

/// Grade a single core task against the given code output.
///
/// `code_output` is the full model response text; `project_root` is the
/// directory containing the scaffold; `task` defines the target file and
/// verify overrides.
pub async fn grade_task(
    task: &CoreTask,
    code_output: &str,
    project_root: &Path,
    base_verify_config: &VerifyConfig,
) -> TaskRunResult {
    let start = Instant::now();

    // Apply per-task verify overrides
    let verify_config = apply_overrides(base_verify_config, &task.verify_overrides);

    // The scaffold file content is the "original"
    let scaffold_path = project_root
        .join(&task.scaffold_path)
        .join(&task.target_file);
    let original_content = match tokio::fs::read_to_string(&scaffold_path).await {
        Ok(c) => c,
        Err(e) => {
            return TaskRunResult {
                task_id: task.id.clone(),
                task_kind: task.kind,
                passed: false,
                latency_ms: start.elapsed().as_millis() as u64,
                prompt_tokens: None,
                completion_tokens: None,
                cost_usd: None,
                output: Some(format!("Failed to read scaffold: {e}")),
                errors: vec![format!("scaffold read error: {e}")],
            };
        }
    };

    let result = verify::verify_patch(
        &original_content,
        code_output,
        &task.target_file,
        project_root,
        &verify_config,
    )
    .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    TaskRunResult {
        task_id: task.id.clone(),
        task_kind: task.kind,
        passed: result.passed,
        latency_ms,
        prompt_tokens: None,
        completion_tokens: None,
        cost_usd: None,
        output: merge_output(&result),
        errors: result.errors,
    }
}

/// Grade a single core task using pre-existing code (for test doubles
/// that already have the output).
pub fn grade_task_sync(
    task: &CoreTask,
    passed: bool,
    latency_ms: u64,
    output: Option<String>,
    errors: Vec<String>,
) -> TaskRunResult {
    TaskRunResult {
        task_id: task.id.clone(),
        task_kind: task.kind,
        passed,
        latency_ms,
        prompt_tokens: None,
        completion_tokens: None,
        cost_usd: None,
        output,
        errors,
    }
}

/// Compute the core score from a list of task run results.
pub fn core_score(results: &[TaskRunResult]) -> f64 {
    compute_score(results)
}

fn apply_overrides(
    base: &VerifyConfig,
    overrides: &Option<crate::benchmark::suite::VerifyOverrides>,
) -> VerifyConfig {
    let mut config = base.clone();
    if let Some(o) = overrides {
        if let Some(v) = o.enabled {
            config.enabled = v;
        }
        if let Some(v) = o.run_tests {
            config.run_tests = v;
        }
        if let Some(v) = o.run_linter {
            config.run_linter = v;
        }
        if let Some(v) = o.timeout_seconds {
            config.timeout_seconds = v;
        }
    }
    config
}

fn merge_output(result: &VerificationResult) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(ref compiler) = result.compiler_output {
        parts.push(format!("Compiler:\n{compiler}"));
    }
    if let Some(ref lint) = result.lint_output {
        parts.push(format!("Lint:\n{lint}"));
    }
    if let Some(ref tests) = result.test_output {
        parts.push(format!("Tests:\n{tests}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::suite::{TaskKind, TaskLanguage};

    fn sample_task(id: &str) -> CoreTask {
        CoreTask {
            id: id.into(),
            description: "test".into(),
            language: TaskLanguage::Rust,
            kind: TaskKind::Compiler,
            introduced_at: "2026-01-01".into(),
            retires_at: "2026-07-01".into(),
            prompt: "fix this".into(),
            scaffold_path: "scaffold/rust".into(),
            target_file: "main.rs".into(),
            verify_overrides: None,
        }
    }

    #[test]
    fn grade_task_sync_basic() {
        let task = sample_task("t1");
        let result = grade_task_sync(&task, true, 50, None, vec![]);
        assert!(result.passed);
        assert_eq!(result.latency_ms, 50);
    }

    #[test]
    fn apply_overrides_none() {
        let base = VerifyConfig::default();
        let config = apply_overrides(&base, &None);
        assert_eq!(config.max_retries, base.max_retries);
    }

    #[test]
    fn apply_overrides_some() {
        let base = VerifyConfig::default();
        let overrides = crate::benchmark::suite::VerifyOverrides {
            enabled: Some(false),
            run_tests: None,
            run_linter: Some(false),
            timeout_seconds: Some(10),
        };
        let config = apply_overrides(&base, &Some(overrides));
        assert!(!config.enabled);
        assert!(!config.run_linter);
        assert_eq!(config.timeout_seconds, 10);
        // Unchanged
        assert_eq!(config.max_retries, base.max_retries);
    }
}
