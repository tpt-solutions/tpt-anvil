// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Benchmark execution engine — dispatches tasks to local or cloud backends,
//! grades results via the verify gate.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::benchmark::scorecard::{compute_score, TaskRunResult};
use crate::benchmark::suite::CoreTask;
use crate::diff::extract_code_block;
use crate::verify::{self, VerificationResult, VerifyConfig};

/// Summary of a single benchmark run across all tasks.
pub struct BenchmarkRunResult {
    pub results: Vec<TaskRunResult>,
    pub total_cost_usd: f64,
}

/// A throwaway project root seeded from a task's embedded scaffold.
///
/// Verification runs real toolchains (`cargo check`, `mypy`, …) against the
/// working tree, so each task gets its own temp directory that is removed on
/// drop. Nothing is written into the user's actual project.
#[derive(Debug)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// Create a sandbox and seed `task`'s scaffold into it.
    pub fn create(task: &CoreTask, label: &str) -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "anvil-bench-{}-{}-{}",
            std::process::id(),
            sanitize(label),
            unique_suffix(),
        ));
        std::fs::create_dir_all(&root)?;
        crate::benchmark::materialize_scaffold(&task.scaffold_path, &task.target_file, &root)?;
        Ok(Self { root })
    }

    /// The sandbox root, to be used as the `project_root` for verification.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The seeded target file's absolute path.
    pub fn target_file(&self, task: &CoreTask) -> PathBuf {
        self.root.join(&task.target_file)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Make `label` safe to use as a single path segment.
fn sanitize(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// A monotonic-ish suffix so concurrent runs never collide.
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Grade a single core task against the given code output.
///
/// `code_output` is the full model response text. The task's scaffold is
/// materialized into a fresh [`Sandbox`] so the verification toolchain runs
/// against the original file content and any edits are discarded afterwards.
/// `base_verify_config` supplies the defaults; `task.verify_overrides` wins.
pub async fn grade_task(
    task: &CoreTask,
    code_output: &str,
    base_verify_config: &VerifyConfig,
) -> TaskRunResult {
    let start = Instant::now();

    // Apply per-task verify overrides
    let verify_config = apply_overrides(base_verify_config, &task.verify_overrides);

    let sandbox = match Sandbox::create(task, &task.id) {
        Ok(s) => s,
        Err(e) => {
            return TaskRunResult {
                task_id: task.id.clone(),
                task_kind: task.kind,
                passed: false,
                latency_ms: start.elapsed().as_millis() as u64,
                prompt_tokens: None,
                completion_tokens: None,
                cost_usd: None,
                output: Some(format!("Failed to seed scaffold: {e}")),
                errors: vec![format!("scaffold seed error: {e}")],
                skipped: false,
            };
        }
    };

    let target = sandbox.target_file(task);
    let original_content = match tokio::fs::read_to_string(&target).await {
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
                skipped: false,
            };
        }
    };

    // Models answer with prose and markdown fences; compiling the raw
    // response would fail on the ``` markers rather than on the model's
    // actual code. Fall back to the raw text if there is no fence.
    let candidate = extract_code_block(code_output).unwrap_or_else(|| code_output.to_string());

    let result = verify::verify_patch(
        &original_content,
        &candidate,
        &task.target_file,
        sandbox.root(),
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
        errors: result.errors.clone(),
        // A missing verification toolchain is an environment problem, not a
        // defect in the model's answer — exclude it from the score.
        skipped: !result.passed
            && result
                .compiler_output
                .as_deref()
                .is_some_and(verify::is_toolchain_missing)
            && result.lint_output.is_none()
            && result.test_output.is_none(),
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
        skipped: false,
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

    #[test]
    fn sandbox_seeds_embedded_scaffold() {
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        let sandbox = Sandbox::create(&task, &task.id).expect("sandbox");
        let target = sandbox.target_file(&task);
        assert!(target.exists(), "scaffold target should be written");
        let content = std::fs::read_to_string(&target).expect("read");
        assert!(content.contains("fn main"));
    }

    #[test]
    fn sandbox_is_removed_on_drop() {
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        let root = {
            let sandbox = Sandbox::create(&task, &task.id).expect("sandbox");
            let root = sandbox.root().to_path_buf();
            assert!(root.exists());
            root
        };
        assert!(!root.exists(), "sandbox must be cleaned up on drop");
    }

    #[test]
    fn sandboxes_are_isolated_from_each_other() {
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        let a = Sandbox::create(&task, "same-label").expect("a");
        let b = Sandbox::create(&task, "same-label").expect("b");
        assert_ne!(a.root(), b.root(), "sandboxes must not share a directory");

        std::fs::write(a.target_file(&task), "// mutated\n").expect("write");
        let b_content = std::fs::read_to_string(b.target_file(&task)).expect("read");
        assert!(
            b_content.contains("fn main"),
            "mutating one sandbox must not affect another"
        );
    }

    #[test]
    fn sandbox_unknown_scaffold_fails_to_create() {
        let mut task = sample_task("t1");
        task.scaffold_path = "scaffold/go".into();
        task.target_file = "main.go".into();

        let err = Sandbox::create(&task, "t1").expect_err("unknown scaffold must error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(sanitize("rust/off-by-one 01"), "rust-off-by-one-01");
        assert_eq!(sanitize(".."), "--");
    }

    #[tokio::test]
    async fn grade_task_with_verification_disabled_passes() {
        // With verification disabled the verify gate is a no-op, so any output
        // passes; this exercises the sandbox + prompt plumbing without shelling
        // out to a compiler toolchain.
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        let config = VerifyConfig {
            enabled: false,
            ..VerifyConfig::default()
        };
        let result = grade_task(&task, "anything at all", &config).await;
        assert!(result.passed);
        assert_eq!(result.task_id, "rust-off-by-one-01");
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn grade_task_strips_markdown_fences_before_verifying() {
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        // Verification is enabled with only the compiler stage, so this
        // actually runs `cargo check` against the sandbox.
        let config = VerifyConfig {
            enabled: true,
            run_tests: false,
            run_linter: false,
            timeout_seconds: 180,
            max_retries: 0,
        };

        let fenced = "Here is the corrected file:\n\n```rust\nfn main() {}\n```\n\nThat fixes it.";
        let result = grade_task(&task, fenced, &config).await;

        // If fences were NOT stripped, `cargo check` would fail on the
        // backticks and the task would be marked failed.
        assert!(
            result.passed,
            "fenced code should compile; errors: {:?}\noutput: {:?}",
            result.errors, result.output
        );
    }

    #[tokio::test]
    async fn grade_task_does_not_touch_the_real_project() {
        let mut task = sample_task("rust-off-by-one-01");
        task.scaffold_path = "scaffold/rust".into();
        task.target_file = "main.rs".into();

        let config = VerifyConfig {
            enabled: false,
            ..VerifyConfig::default()
        };
        // Run from the crate root; grade_task must not write main.rs here.
        let before = std::fs::metadata("Cargo.toml").is_ok();
        let _ = grade_task(&task, "junk", &config).await;
        assert!(before);
        assert!(
            !std::path::Path::new("main.rs").exists(),
            "benchmark must not write into the current project"
        );
    }
}
