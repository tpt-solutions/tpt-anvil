// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Adaptive benchmark slice — generates model-specific tasks from prior failures.

use super::scorecard::ModelScorecard;
use super::suite::{AdaptiveTask, TaskKind, TaskLanguage};

/// Given a model's prior scorecard failures, generate prompts for an
/// evaluator model to produce new adaptive tasks targeting the same
/// weaknesses.  Returns a list of `AdaptiveTask` ready to be sent to
/// the model under test.
pub fn generate_adaptive_prompts(
    prior: &ModelScorecard,
    max_tasks: u32,
) -> Vec<AdaptivePromptRequest> {
    let failed_task_ids: Vec<String> = prior
        .core_results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.task_id.clone())
        .take(max_tasks as usize)
        .collect();

    failed_task_ids
        .into_iter()
        .enumerate()
        .map(|(i, task_id)| AdaptivePromptRequest {
            derived_from: task_id.clone(),
            adaptive_id: format!("adaptive-{}-{}", prior.model_id, i),
            generation_prompt: format!(
                "The model '{}' previously failed the benchmark task '{}'. \
                 Generate a new, different coding task that tests the same \
                 weakness but with a fresh scenario. Return a TOML task \
                 definition with fields: id, description, language, kind, \
                 prompt, scaffold_path, target_file.",
                prior.model_id, task_id
            ),
        })
        .collect()
}

/// A request to generate an adaptive task from the evaluator model.
#[derive(Debug, Clone)]
pub struct AdaptivePromptRequest {
    /// The core task id this was derived from.
    pub derived_from: String,
    /// The id to assign to the new adaptive task.
    pub adaptive_id: String,
    /// The prompt to send to the evaluator model.
    pub generation_prompt: String,
}

/// Parse an adaptive task from the evaluator model's response.
/// Returns `None` if the response cannot be parsed.
pub fn parse_adaptive_task(
    response: &str,
    request: &AdaptivePromptRequest,
) -> Option<AdaptiveTask> {
    // Minimal TOML-like key extraction; the evaluator is expected to
    // produce structured output but we handle malformed responses
    // gracefully by falling back to defaults.
    let description = extract_field(response, "description")
        .unwrap_or_else(|| format!("Adaptive task derived from {}", request.derived_from));
    let language_str = extract_field(response, "language").unwrap_or_default();
    let language = match language_str.as_str() {
        "rust" => TaskLanguage::Rust,
        "typescript" | "ts" => TaskLanguage::TypeScript,
        "python" | "py" => TaskLanguage::Python,
        "go" => TaskLanguage::Go,
        _ => TaskLanguage::Rust,
    };
    let kind_str = extract_field(response, "kind").unwrap_or_default();
    let kind = match kind_str.as_str() {
        "tests" => TaskKind::Tests,
        "lint" => TaskKind::Lint,
        _ => TaskKind::Compiler,
    };
    let prompt = extract_field(response, "prompt").unwrap_or_else(|| response.to_string());
    let scaffold_path = extract_field(response, "scaffold_path");
    let target_file = extract_field(response, "target_file");

    Some(AdaptiveTask {
        derived_from: request.derived_from.clone(),
        id: request.adaptive_id.clone(),
        description,
        language,
        kind,
        generation_prompt: request.generation_prompt.clone(),
        prompt,
        scaffold_path,
        target_file,
        verify_overrides: None,
    })
}

/// Attempt to extract a TOML-style `key = "value"` field from text.
fn extract_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let val = rest.trim();
                // Strip surrounding quotes
                let val = val
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(val);
                return Some(val.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_prompts_from_failures() {
        let prior = ModelScorecard {
            provider: "ollama".into(),
            model_id: "deepseek".into(),
            last_run_at: "2026-07-01".into(),
            core_task_ids_run: vec!["t1".into(), "t2".into(), "t3".into()],
            core_results: vec![
                crate::benchmark::scorecard::TaskRunResult {
                    task_id: "t1".into(),
                    task_kind: TaskKind::Compiler,
                    passed: true,
                    latency_ms: 100,
                    prompt_tokens: None,
                    completion_tokens: None,
                    cost_usd: None,
                    output: None,
                    errors: vec![],
                },
                crate::benchmark::scorecard::TaskRunResult {
                    task_id: "t2".into(),
                    task_kind: TaskKind::Compiler,
                    passed: false,
                    latency_ms: 100,
                    prompt_tokens: None,
                    completion_tokens: None,
                    cost_usd: None,
                    output: None,
                    errors: vec!["compile error".into()],
                },
                crate::benchmark::scorecard::TaskRunResult {
                    task_id: "t3".into(),
                    task_kind: TaskKind::Lint,
                    passed: false,
                    latency_ms: 100,
                    prompt_tokens: None,
                    completion_tokens: None,
                    cost_usd: None,
                    output: None,
                    errors: vec!["lint warning".into()],
                },
            ],
            adaptive_results: vec![],
            core_score: 1.0 / 3.0,
            adaptive_score: None,
            total_cost_usd: 0.0,
        };

        let prompts = generate_adaptive_prompts(&prior, 5);
        assert_eq!(prompts.len(), 2); // only 2 failures
        assert_eq!(prompts[0].derived_from, "t2");
        assert_eq!(prompts[1].derived_from, "t3");
    }

    #[test]
    fn parse_adaptive_task_from_toml_like() {
        let request = AdaptivePromptRequest {
            derived_from: "t1".into(),
            adaptive_id: "adaptive-deepseek-0".into(),
            generation_prompt: "generate a task".into(),
        };
        let response = r#"
            id = "adaptive-deepseek-0"
            description = "Fix ownership error in nested loops"
            language = "rust"
            kind = "compiler"
            prompt = "Fix the code so it compiles."
            scaffold_path = "scaffold/rust"
            target_file = "main.rs"
        "#;
        let task = parse_adaptive_task(response, &request).unwrap();
        assert_eq!(task.id, "adaptive-deepseek-0");
        assert_eq!(task.language, TaskLanguage::Rust);
        assert_eq!(task.kind, TaskKind::Compiler);
        assert!(task.prompt.contains("compiles"));
    }

    #[test]
    fn parse_adaptive_task_malformed_returns_fallback() {
        let request = AdaptivePromptRequest {
            derived_from: "t1".into(),
            adaptive_id: "adaptive-0".into(),
            generation_prompt: "gen".into(),
        };
        let task = parse_adaptive_task("just some random text", &request).unwrap();
        assert_eq!(task.id, "adaptive-0");
        assert_eq!(task.language, TaskLanguage::Rust); // default
        assert_eq!(task.kind, TaskKind::Compiler); // default
    }

    #[test]
    fn extract_field_basic() {
        assert_eq!(
            extract_field("name = \"hello\"", "name"),
            Some("hello".into())
        );
        assert_eq!(extract_field("other = \"world\"", "name"), None);
    }

    #[test]
    fn extract_field_unquoted() {
        assert_eq!(extract_field("count = 42", "count"), Some("42".into()));
    }
}
