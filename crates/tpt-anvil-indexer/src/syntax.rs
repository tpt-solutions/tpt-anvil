// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Toolchain-free syntax validation via tree-sitter.
//!
//! The verification gate in `tpt-anvil-capabilities` shells out to real
//! compilers (`cargo check`, `tsc`, `mypy`). Those tools are frequently absent
//! — a benchmark sandbox is a throwaway temp directory with no
//! `node_modules`, so `npx tsc` resolves to the placeholder stub and
//! `python -m mypy` fails with `No module named mypy`. Treating that as a
//! model defect would understate the score, and excluding the task would
//! silently shrink the denominator.
//!
//! This module provides a weaker but always-available check: parse the source
//! with tree-sitter and report syntax errors. It cannot catch type errors or
//! borrow-checker failures, but it does catch malformed output — truncated
//! code, unbalanced braces — which is what models actually get wrong when
//! answering with a partial file.
//!
//! This is a fallback only. When a real toolchain is present, callers should
//! prefer it and fall back to this.

use serde::{Deserialize, Serialize};

/// A single syntax error reported by the tree-sitter parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxError {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (in bytes).
    pub column: usize,
    /// Human-readable description, e.g. `"missing `}`"`.
    pub message: String,
}

/// Resolve a tree-sitter grammar for a language name.
///
/// Returns `None` for languages with no bundled grammar, which callers should
/// treat as "cannot check" rather than "no errors".
pub fn grammar_for(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        // `ts` must use the TypeScript grammar rather than the JavaScript one:
        // type annotations are syntax errors under the JavaScript grammar.
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "javascript" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c" => Some(tree_sitter_c::LANGUAGE.into()),
        "ruby" => Some(tree_sitter_ruby::LANGUAGE.into()),
        "php" => Some(tree_sitter_php::LANGUAGE_PHP.into()),
        "c_sharp" | "csharp" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        _ => None,
    }
}

/// Parse `source` and collect every syntax error tree-sitter reports.
///
/// Returns `None` when no grammar is bundled for `language`, meaning the
/// language cannot be checked at all (as opposed to being checked and found
/// clean). An empty `Some(vec![])` means the source parsed cleanly.
///
/// Only the first [`MAX_REPORTED_ERRORS`] errors are returned so a wholly
/// malformed response cannot produce an unbounded error list.
pub fn syntax_errors(source: &str, language: &str) -> Option<Vec<SyntaxError>> {
    /// Cap on reported errors; enough to diagnose, bounded in size.
    const MAX_REPORTED_ERRORS: usize = 10;

    let grammar = grammar_for(language)?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar).ok()?;
    let tree = parser.parse(source, None)?;

    let mut errors = Vec::new();
    collect_errors(tree.root_node(), &mut errors, MAX_REPORTED_ERRORS);
    Some(errors)
}

/// Convenience wrapper: `true` when the source parses cleanly.
///
/// Returns `None` when the language has no bundled grammar.
pub fn is_syntactically_valid(source: &str, language: &str) -> Option<bool> {
    syntax_errors(source, language).map(|errors| errors.is_empty())
}

/// Format syntax errors as a human-readable, compiler-like report.
pub fn format_errors(errors: &[SyntaxError], file_path: &str) -> String {
    let mut out = String::new();
    for err in errors {
        out.push_str(&format!(
            "{file_path}:{}:{}: error: {}\n",
            err.line, err.column, err.message
        ));
    }
    out.push_str(&format!("{} error(s) found\n", errors.len()));
    out
}

/// Recursively collect `ERROR` and `MISSING` nodes beneath `node`.
///
/// Never descends *into* an error node: its children are recovery fragments
/// that would otherwise each report as a separate error.
fn collect_errors(node: tree_sitter::Node, out: &mut Vec<SyntaxError>, limit: usize) {
    if out.len() >= limit {
        return;
    }

    if node.is_error() || node.is_missing() {
        let position = node.start_position();
        let message = if node.is_missing() {
            // A missing node has no text of its own; its kind names the
            // token the parser expected but did not find.
            format!("missing `{}`", node.kind())
        } else {
            "unexpected syntax".to_string()
        };

        out.push(SyntaxError {
            line: position.row + 1,
            column: position.column + 1,
            message,
        });
        return;
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_errors(cursor.node(), out, limit);
            if out.len() >= limit || !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_python_has_no_errors() {
        let src = "def add(a, b):\n    return a + b\n";
        assert_eq!(syntax_errors(src, "python"), Some(vec![]));
    }

    #[test]
    fn valid_typescript_has_no_errors() {
        let src = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";
        assert_eq!(syntax_errors(src, "typescript"), Some(vec![]));
    }

    #[test]
    fn typescript_annotations_are_valid() {
        // A JavaScript grammar would reject these; the TypeScript grammar
        // must accept them. Guards against regressing to the JS grammar.
        let src = "const x: number = 1;\ninterface Y { a: string }\n";
        assert_eq!(syntax_errors(src, "typescript"), Some(vec![]));
    }

    #[test]
    fn truncated_typescript_reports_an_error() {
        // The classic model failure: a response cut off mid-function.
        let src = "export function add(a: number, b: number): number {\n  return a + b;\n";
        let errors = syntax_errors(src, "typescript").expect("ts grammar available");
        assert!(!errors.is_empty(), "truncated source must not parse clean");
    }

    #[test]
    fn unbalanced_braces_report_an_error() {
        let src = "function f() {\n  if (true) {\n    return 1;\n}\n";
        let errors = syntax_errors(src, "typescript").expect("ts grammar available");
        assert!(!errors.is_empty());
    }

    #[test]
    fn valid_rust_has_no_errors() {
        let src = "fn main() {\n    let v = vec![1, 2, 3];\n    println!(\"{v:?}\");\n}\n";
        assert_eq!(syntax_errors(src, "rust"), Some(vec![]));
    }

    #[test]
    fn rust_missing_main_is_still_syntactically_valid() {
        // Syntax checking cannot see a missing entry point; only a real
        // compiler can. Documents the fallback's known blind spot so a
        // future change does not mistake it for a bug.
        let src = "fn second_half(values: &[i32]) -> &[i32] {\n    &values[1..]\n}\n";
        assert_eq!(syntax_errors(src, "rust"), Some(vec![]));
    }

    #[test]
    fn valid_go_and_java_have_no_errors() {
        assert_eq!(
            syntax_errors("package main\n\nfunc main() {}\n", "go"),
            Some(vec![])
        );
        assert_eq!(
            syntax_errors(
                "public class A { public static void main(String[] a) {} }",
                "java"
            ),
            Some(vec![])
        );
    }

    #[test]
    fn unknown_language_returns_none() {
        assert_eq!(syntax_errors("whatever", "cobol"), None);
        assert_eq!(is_syntactically_valid("whatever", "cobol"), None);
    }

    #[test]
    fn error_count_is_bounded() {
        // Deeply malformed input must not produce an unbounded report.
        let src = "function f( { [ { ( ) ] } ) ".repeat(50);
        let errors = syntax_errors(&src, "typescript").expect("ts grammar available");
        assert!(errors.len() <= 10, "got {} errors", errors.len());
    }

    #[test]
    fn errors_carry_one_based_positions() {
        let src = "export function add(a: number, b: number): number {\n  return a + b;\n";
        let errors = syntax_errors(src, "typescript").expect("ts grammar available");
        assert!(!errors.is_empty());
        assert!(errors[0].line >= 1);
        assert!(errors[0].column >= 1);
    }

    #[test]
    fn single_unclosed_brace_reports_once_not_per_recovery_fragment() {
        // Without the "don't descend into error nodes" rule this produces a
        // long list of overlapping recovery fragments for a single mistake.
        let src = "function f() {\n  return 1;\n";
        let errors = syntax_errors(src, "typescript").expect("ts grammar available");
        assert_eq!(errors.len(), 1, "expected one error, got {errors:?}");
    }

    #[test]
    fn format_errors_mentions_path_and_count() {
        let errors = vec![SyntaxError {
            line: 3,
            column: 5,
            message: "missing `}`".into(),
        }];
        let out = format_errors(&errors, "main.rs");
        assert!(out.contains("main.rs:3:5"));
        assert!(out.contains("1 error(s) found"));
    }
}
