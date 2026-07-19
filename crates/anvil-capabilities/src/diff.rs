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
            return Some(DiffPatch {
                file_path: file_path.to_string(),
                unified_diff: diff,
            });
        }
        // If the output itself looks like a diff
        if model_output.trim_start().starts_with("---") || model_output.contains("\n@@") {
            return Some(DiffPatch {
                file_path: file_path.to_string(),
                unified_diff: model_output.trim().to_string(),
            });
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
            diff.push_str(&format!(
                "@@ -{hunk_start_orig},{orig_count} +{hunk_start_mod},{mod_count} @@\n"
            ));
            diff.push_str(&hunk_lines.join("\n"));
        }

        DiffPatch {
            file_path: file_path.to_string(),
            unified_diff: diff,
        }
    }

    /// Apply a unified diff to the original content, returning the result.
    ///
    /// Parses `@@ -a,b +c,d @@` hunk headers and applies context (` `),
    /// removal (`-`), and addition (`+`) lines. Falls back to returning the
    /// original content unchanged if no hunks are present.
    pub fn apply_diff(original: &str, patch: &DiffPatch) -> Result<String, String> {
        let original_lines: Vec<&str> = original.lines().collect();
        let mut result: Vec<String> = Vec::new();
        // 0-based cursor into the original file.
        let mut orig_cursor: usize = 0;
        let mut in_hunk = false;

        for line in patch.unified_diff.lines() {
            if line.starts_with("---") || line.starts_with("+++") {
                continue;
            }
            if let Some(header) = line.strip_prefix("@@") {
                // Parse the original start line: "@@ -a,b +c,d @@".
                let start = parse_hunk_orig_start(header)
                    .ok_or_else(|| format!("malformed hunk header: {line}"))?;
                // Copy untouched lines up to the hunk start (1-based -> 0-based).
                let target = start.saturating_sub(1);
                while orig_cursor < target && orig_cursor < original_lines.len() {
                    result.push(original_lines[orig_cursor].to_string());
                    orig_cursor += 1;
                }
                in_hunk = true;
                continue;
            }

            if !in_hunk {
                continue;
            }

            match line.chars().next() {
                Some(' ') => {
                    // Context line: keep and advance original cursor.
                    result.push(line[1..].to_string());
                    orig_cursor += 1;
                }
                Some('-') => {
                    // Removal: skip in output, advance original cursor.
                    orig_cursor += 1;
                }
                Some('+') => {
                    // Addition: emit, do not advance original cursor.
                    result.push(line[1..].to_string());
                }
                _ => {}
            }
        }

        // Append any remaining original lines after the last hunk.
        while orig_cursor < original_lines.len() {
            result.push(original_lines[orig_cursor].to_string());
            orig_cursor += 1;
        }

        if !in_hunk {
            // No hunks found; return original untouched.
            return Ok(original.to_string());
        }

        Ok(result.join("\n"))
    }
}

/// Parse the original-file start line from a hunk header body like
/// ` -12,5 +12,6 @@ ...`. Returns the `12`.
fn parse_hunk_orig_start(header: &str) -> Option<usize> {
    let minus = header.split('-').nth(1)?;
    let nums = minus
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?;
    nums.trim().parse::<usize>().ok()
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
    for lang in &[
        "rust",
        "python",
        "typescript",
        "javascript",
        "go",
        "java",
        "cpp",
        "c",
        "",
    ] {
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

    #[test]
    fn apply_diff_round_trip() {
        let orig = "line1\nline2\nline3\n";
        let modified = "line1\nCHANGED\nline3\n";
        let patch = DiffEngine::compute_diff(orig, modified, "f.rs");
        let applied = DiffEngine::apply_diff(orig, &patch).unwrap();
        assert_eq!(applied.trim_end(), "line1\nCHANGED\nline3");
    }

    #[test]
    fn apply_diff_no_hunks_returns_original() {
        let orig = "unchanged\n";
        let patch = DiffPatch {
            file_path: "f.rs".into(),
            unified_diff: "--- a/f.rs\n+++ b/f.rs\n".into(),
        };
        let applied = DiffEngine::apply_diff(orig, &patch).unwrap();
        assert_eq!(applied, orig);
    }

    #[test]
    fn apply_diff_addition() {
        let orig = "a\nb\n";
        let modified = "a\nb\nc\n";
        let patch = DiffEngine::compute_diff(orig, modified, "f.rs");
        let applied = DiffEngine::apply_diff(orig, &patch).unwrap();
        assert_eq!(applied.trim_end(), "a\nb\nc");
    }
}
