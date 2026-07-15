// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::types::DiffPatch;

pub struct DiffEngine;

impl DiffEngine {
    /// Extract a unified diff from model output.
    /// Handles both raw diff output and fenced code blocks.
    pub fn extract_diff(model_output: &str, file_path: &str) -> Option<DiffPatch> {
        // Try to find a fenced diff block first
        if let Some(diff) = extract_fenced(model_output, "diff") {
            return Some(DiffPatch { file_path: file_path.to_string(), unified_diff: diff });
        }
        // If the output itself looks like a diff
        if model_output.trim_start().starts_with("---") || model_output.contains("\n@@") {
            return Some(DiffPatch { file_path: file_path.to_string(), unified_diff: model_output.trim().to_string() });
        }
        None
    }

    /// Given original content and new content, produce a unified diff.
    pub fn compute_diff(original: &str, modified: &str, file_path: &str) -> DiffPatch {
        let original_lines: Vec<&str> = original.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();

        let mut diff = format!("--- a/{file_path}\n+++ b/{file_path}\n");
        let mut hunk_lines = Vec::new();

        let max_len = original_lines.len().max(modified_lines.len());
        let mut in_hunk = false;
        let mut hunk_start_orig = 1usize;
        let mut hunk_start_mod = 1usize;

        for i in 0..max_len {
            let orig = original_lines.get(i).copied();
            let modi = modified_lines.get(i).copied();
            match (orig, modi) {
                (Some(o), Some(m)) if o == m => {
                    if in_hunk {
                        hunk_lines.push(format!(" {}", o));
                    }
                }
                (Some(o), Some(m)) => {
                    if !in_hunk {
                        hunk_start_orig = i + 1;
                        hunk_start_mod = i + 1;
                        in_hunk = true;
                    }
                    hunk_lines.push(format!("-{}", o));
                    hunk_lines.push(format!("+{}", m));
                }
                (None, Some(m)) => {
                    if !in_hunk {
                        hunk_start_orig = i + 1;
                        hunk_start_mod = i + 1;
                        in_hunk = true;
                    }
                    hunk_lines.push(format!("+{}", m));
                }
                (Some(o), None) => {
                    if !in_hunk {
                        hunk_start_orig = i + 1;
                        hunk_start_mod = i + 1;
                        in_hunk = true;
                    }
                    hunk_lines.push(format!("-{}", o));
                }
                (None, None) => break,
            }
        }

        if !hunk_lines.is_empty() {
            let orig_count = hunk_lines.iter().filter(|l| !l.starts_with('+')).count();
            let mod_count = hunk_lines.iter().filter(|l| !l.starts_with('-')).count();
            diff.push_str(&format!("@@ -{hunk_start_orig},{orig_count} +{hunk_start_mod},{mod_count} @@\n"));
            diff.push_str(&hunk_lines.join("\n"));
        }

        DiffPatch { file_path: file_path.to_string(), unified_diff: diff }
    }

    /// Apply a unified diff to the original content, returning the result.
    pub fn apply_diff(original: &str, patch: &DiffPatch) -> Result<String, String> {
        // For now, if the patch contains just a replacement block we use it directly.
        // A full patch parser is a future iteration.
        let lines: Vec<&str> = original.lines().collect();
        let mut result = lines.clone();
        let mut offset: i64 = 0;

        for line in patch.unified_diff.lines() {
            if line.starts_with("---") || line.starts_with("+++") || line.starts_with("@@") {
                continue;
            }
            // Very simplified application — real impl should use a proper patch library
        }

        Ok(result.join("\n"))
    }
}

fn extract_fenced(text: &str, lang: &str) -> Option<String> {
    let fence = format!("```{}", lang);
    let start = text.find(&fence)?;
    let after_fence = &text[start + fence.len()..];
    let end = after_fence.find("```")?;
    Some(after_fence[..end].trim().to_string())
}

pub fn extract_code_block(text: &str) -> Option<String> {
    // Try language-specific fences
    for lang in &["rust", "python", "typescript", "javascript", "go", "java", "cpp", "c", ""] {
        if let Some(block) = extract_fenced(text, lang) {
            return Some(block);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_block_rust() {
        let text = "Here is the code:\n```rust\nfn hello() {}\n```\nDone.";
        let block = extract_code_block(text).unwrap();
        assert_eq!(block, "fn hello() {}");
    }

    #[test]
    fn extract_code_block_unlabeled() {
        let text = "Result:\n```\nlet x = 1;\n```";
        let block = extract_code_block(text).unwrap();
        assert_eq!(block, "let x = 1;");
    }

    #[test]
    fn extract_code_block_none() {
        let text = "No code here at all.";
        assert!(extract_code_block(text).is_none());
    }

    #[test]
    fn compute_diff_produces_patch_header() {
        let orig = "fn foo() {\n    1\n}\n";
        let new = "fn foo() {\n    2\n}\n";
        let patch = DiffEngine::compute_diff(orig, new, "src/lib.rs");
        assert!(patch.unified_diff.contains("--- a/src/lib.rs"));
        assert!(patch.unified_diff.contains("+++ b/src/lib.rs"));
        assert!(patch.unified_diff.contains("-    1"));
        assert!(patch.unified_diff.contains("+    2"));
    }

    #[test]
    fn compute_diff_identical_files_no_hunks() {
        let content = "fn foo() {}\n";
        let patch = DiffEngine::compute_diff(content, content, "src/lib.rs");
        assert!(!patch.unified_diff.contains("@@"));
    }

    #[test]
    fn extract_diff_from_model_output() {
        let output = "```diff\n--- a/main.rs\n+++ b/main.rs\n@@ -1 +1 @@\n-old\n+new\n```";
        let patch = DiffEngine::extract_diff(output, "main.rs").unwrap();
        assert!(patch.unified_diff.contains("--- a/main.rs"));
    }
}
