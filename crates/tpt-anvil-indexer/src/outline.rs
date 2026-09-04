// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! AST-outline compression for Smart Context.
//! Generates compact outlines of source files from extracted symbols,
//! reducing token consumption while preserving structural information.

use crate::symbols::extract_symbols;
use serde::{Deserialize, Serialize};

/// Statistics about outline compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineStats {
    pub original_chars: usize,
    pub outline_chars: usize,
    pub symbol_count: usize,
}

/// Generate a compact outline for a source file.
///
/// The outline contains symbol signatures and doc comments but omits
/// function bodies and implementation details.
pub fn outline_for_file(source: &str, language: &str, file_path: &str) -> String {
    let symbols = extract_symbols(source, language, file_path);

    if symbols.is_empty() {
        // Fallback: return first 80 lines of source
        return source.lines().take(80).collect::<Vec<_>>().join("\n");
    }

    let mut outline = String::new();
    outline.push_str(&format!(
        "// Outline of {} ({} symbols)\n\n",
        file_path,
        symbols.len()
    ));

    for sym in &symbols {
        let kind_label = match sym.kind {
            crate::symbols::SymbolKind::Function => "fn",
            crate::symbols::SymbolKind::Method => "method",
            crate::symbols::SymbolKind::Class => "class",
            crate::symbols::SymbolKind::Struct => "struct",
            crate::symbols::SymbolKind::Enum => "enum",
            crate::symbols::SymbolKind::Interface => "interface",
            crate::symbols::SymbolKind::Trait => "trait",
            crate::symbols::SymbolKind::Module => "mod",
            crate::symbols::SymbolKind::Variable => "let",
            crate::symbols::SymbolKind::Constant => "const",
            crate::symbols::SymbolKind::Import => "use",
            crate::symbols::SymbolKind::Unknown => "??",
        };

        if let Some(ref doc) = sym.doc_comment {
            outline.push_str(&format!("/// {}\n", doc.trim()));
        }

        if let Some(ref sig) = sym.signature {
            outline.push_str(&format!(
                "{} {} (lines {}-{})\n",
                kind_label,
                sig,
                sym.start_line + 1,
                sym.end_line + 1
            ));
        } else {
            outline.push_str(&format!(
                "{} {} (lines {}-{})\n",
                kind_label,
                sym.name,
                sym.start_line + 1,
                sym.end_line + 1
            ));
        }
    }

    outline
}

/// Compute compression stats.
pub fn outline_stats(original: &str, outline: &str) -> OutlineStats {
    let symbols = extract_symbols(original, "rust", ""); // language-independent count
    OutlineStats {
        original_chars: original.len(),
        outline_chars: outline.len(),
        symbol_count: symbols.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_produces_compact_output() {
        let src = r#"
/// A greeting function
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// A person struct
pub struct Person {
    pub name: String,
    pub age: u32,
}

impl Person {
    pub fn new(name: &str, age: u32) -> Self {
        Self { name: name.to_string(), age }
    }
}
"#;
        let outline = outline_for_file(src, "rust", "src/lib.rs");
        assert!(outline.contains("fn greet"));
        assert!(outline.contains("struct Person"));
        assert!(
            outline.len() < src.len(),
            "outline should be shorter than source"
        );
    }

    #[test]
    fn outline_fallback_for_unknown_language() {
        let src = "line1\nline2\nline3\nline4\nline5\n";
        let outline = outline_for_file(src, "brainfuck", "file.bf");
        // Should return first 80 lines (or all if less)
        assert!(outline.contains("line1"));
    }

    #[test]
    fn outline_empty_source() {
        let outline = outline_for_file("", "rust", "empty.rs");
        assert!(outline.is_empty() || outline.contains("Outline"));
    }
}
