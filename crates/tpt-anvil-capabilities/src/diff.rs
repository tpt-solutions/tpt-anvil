// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use tpt_anvil_core::types::DiffPatch;

pub struct DiffEngine;

impl DiffEngine {
    /// Extract a unified diff from model output.
    /// Handles both raw diff output and fenced code blocks.
    pub fn extract_diff(model_output: &str, file_path: &str) -> Option<DiffPatch> {
        // Try to find a fenced diff block first
        if let Some(diff) = extract_fenced_tag(model_output, "diff") {
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

/// Read the body of the fenced block whose opening fence starts at `start`.
///
/// Strips the info string (everything from the opening backticks to the end of
/// that line) so a ```` ```py ```` block yields code only; leaving `py` on line
/// 1 would make the snippet a syntax error at 1:1. Returns `None` when the block
/// is never closed.
fn block_at(text: &str, start: usize) -> Option<String> {
    let after = text.get(start + 3..)?;

    // Skip the info string: everything up to the first newline. An unterminated
    // line means the fence closed immediately, so there is no body.
    let body = match after.find('\n') {
        Some(nl) => &after[nl + 1..],
        None => {
            // No newline at all before the closing fence: the rest of the line
            // is the info string and there is no code.
            return if after.trim_start().starts_with("```") {
                Some(String::new())
            } else {
                None
            };
        }
    };

    let end = body.find("```")?;
    Some(body[..end].trim().to_string())
}

/// The info string of the opening fence at `start`, if it is a simple
/// `word`/`word:extra` tag. Fences with spaces in the info string are prose
/// blocks (or plain unfenced text) and carry no language.
fn fence_info_string(text: &str, start: usize) -> Option<&str> {
    let after = text.get(start + 3..)?;
    let line = after.split('\n').next()?.trim();
    let name = line.split(':').next()?.trim();
    if name.is_empty() || name.contains(' ') {
        None
    } else {
        Some(name)
    }
}

/// Fence info strings that map onto a canonical language name.
///
/// Models emit many spellings for the same language; without this mapping a
/// ```` ```py ```` block would not be recognized as Python at all.
const LANGUAGE_ALIASES: &[(&str, &str)] = &[
    ("py", "python"),
    ("python3", "python"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("node", "javascript"),
    ("rs", "rust"),
    ("golang", "go"),
];

/// Normalize a fence info string to a canonical language name.
fn canonical_language(info: &str) -> String {
    let base = info.to_ascii_lowercase();
    match LANGUAGE_ALIASES
        .iter()
        .find(|(alias, _)| *alias == base)
        .map(|(_, canonical)| (*canonical).to_string())
    {
        Some(canonical) => canonical,
        None => base,
    }
}

/// Extract the body of the first fenced block tagged exactly `tag`.
///
/// Unlike [`extract_code_block`], this matches a specific info string and does
/// not normalize aliases, because callers here want a literal `diff` block.
pub fn extract_fenced_tag(text: &str, tag: &str) -> Option<String> {
    let mut offset = 0usize;
    while let Some(rel) = text.get(offset..)?.find("```") {
        let start = offset + rel;
        if let Some(info) = fence_info_string(text, start) {
            if info.eq_ignore_ascii_case(tag) {
                return block_at(text, start);
            }
        }
        offset = start + 3;
    }
    None
}

/// Languages this extractor understands.
const KNOWN_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "typescript",
    "javascript",
    "go",
    "java",
    "cpp",
    "c",
];

/// Extract the model's code from a response.
///
/// Scans opening fences in document order and returns the body of the first one
/// whose info string names a language we understand (after alias
/// normalization, so ```` ```py ```` and ```` ```ts ```` are honored). Falls
/// back to the first fence of any kind, info string stripped.
pub fn extract_code_block(text: &str) -> Option<String> {
    let mut first_fence: Option<usize> = None;
    let mut offset = 0usize;

    while let Some(rel) = text.get(offset..)?.find("```") {
        let start = offset + rel;
        if first_fence.is_none() {
            first_fence = Some(start);
        }

        // Only opening fences carry an info string; a closing fence is
        // followed by prose, so `fence_info_string` returning `Some` implies
        // this is an opening fence.
        if let Some(info) = fence_info_string(text, start) {
            if KNOWN_LANGUAGES.contains(&canonical_language(info).as_str()) {
                if let Some(block) = block_at(text, start) {
                    return Some(block);
                }
            }
        }

        offset = start + 3;
    }

    first_fence.and_then(|start| block_at(text, start))
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
    fn extract_code_block_short_alias_fence_is_recognized() {
        // Regression: a ```py block previously fell through to the bare-fence
        // branch and kept `py` as the first line of the extracted "code",
        // producing a syntax error at 1:1 when graded.
        let text = "Here you go:\n```py\ndef add(a, b):\n    return a + b\n```\nDone.";
        let block = extract_code_block(text).unwrap();
        assert_eq!(block, "def add(a, b):\n    return a + b");
        assert!(
            !block.starts_with("py"),
            "info string must be stripped, got: {block:?}"
        );
    }

    #[test]
    fn extract_code_block_ts_alias() {
        let text = "```ts\nexport const x: number = 1;\n```";
        assert_eq!(
            extract_code_block(text).unwrap(),
            "export const x: number = 1;"
        );
    }

    #[test]
    fn extract_code_block_info_string_is_always_stripped() {
        // Even an unrecognized tag must not leak into the code.
        let text = "```unknownlang\nsome code\n```";
        assert_eq!(extract_code_block(text).unwrap(), "some code");
    }

    #[test]
    fn extract_code_block_prefers_first_recognized_block() {
        // A prose/diff block before the real answer must not win.
        let text = "First, the diff:\n```diff\n- old\n+ new\n```\nNow the file:\n```rust\nfn main() {}\n```";
        assert_eq!(extract_code_block(text).unwrap(), "fn main() {}");
    }

    #[test]
    fn extract_code_block_skips_prose_fence_before_code() {
        let text =
            "Thinking out loud:\n```\nstep 1\nstep 2\n```\nAnswer:\n```python\nprint(1)\n```";
        assert_eq!(extract_code_block(text).unwrap(), "print(1)");
    }

    #[test]
    fn extract_code_block_survives_round_trip_through_syntax_check() {
        // The end-to-end property that matters: whatever fence the model used,
        // the extracted text must parse as the language it claims to be.
        let text = "```python\ndef add(a, b):\n    return a + b\n```";
        let block = extract_code_block(text).unwrap();
        let errors = tpt_anvil_indexer::syntax::syntax_errors(&block, "python");
        assert_eq!(
            errors,
            Some(vec![]),
            "extracted python must be syntactically valid, errors: {errors:?}"
        );
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
