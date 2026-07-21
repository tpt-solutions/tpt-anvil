// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Call graph construction.
//!
//! Extracts caller/callee relationships from source code using tree-sitter.
//! For every function/method definition we record the names of the functions
//! it invokes. The resulting edges form a directed call graph that can be
//! queried for callers of a symbol or callees of a symbol.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// A single directed edge in the call graph: `caller` invokes `callee`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdge {
    /// Name of the enclosing function/method that performs the call.
    pub caller: String,
    /// Name of the called function/method.
    pub callee: String,
    /// File the call site lives in.
    pub file_path: String,
    /// Line (0-based) of the call site.
    pub line: u32,
}

/// A directed call graph built from one or more source files.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    edges: Vec<CallEdge>,
    /// caller name -> set of callee names
    callees: BTreeMap<String, BTreeSet<String>>,
    /// callee name -> set of caller names
    callers: BTreeMap<String, BTreeSet<String>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a batch of edges into the graph.
    pub fn add_edges(&mut self, edges: impl IntoIterator<Item = CallEdge>) {
        for edge in edges {
            self.callees
                .entry(edge.caller.clone())
                .or_default()
                .insert(edge.callee.clone());
            self.callers
                .entry(edge.callee.clone())
                .or_default()
                .insert(edge.caller.clone());
            self.edges.push(edge);
        }
    }

    /// All edges in insertion order.
    pub fn edges(&self) -> &[CallEdge] {
        &self.edges
    }

    /// Names of functions directly called by `caller`.
    pub fn callees_of(&self, caller: &str) -> Vec<String> {
        self.callees
            .get(caller)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Names of functions that directly call `callee`.
    pub fn callers_of(&self, callee: &str) -> Vec<String> {
        self.callers
            .get(callee)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Build call edges for a single source file.
///
/// Supported languages mirror [`crate::symbols::extract_symbols`].
pub fn extract_call_edges(source: &str, language: &str, file_path: &str) -> Vec<CallEdge> {
    let ts_language: tree_sitter::Language = match language {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" | "typescript" => tree_sitter_javascript::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "ruby" => tree_sitter_ruby::LANGUAGE.into(),
        "php" => tree_sitter_php::LANGUAGE_PHP.into(),
        "c_sharp" | "csharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        _ => return vec![],
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&ts_language).is_err() {
        return vec![];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec![];
    };

    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    walk(&mut cursor, bytes, file_path, "<module>", &mut out);
    out
}

/// Node kinds that introduce a new enclosing function scope, keyed by the
/// tree-sitter node kind. The value indicates the field holding the name.
fn definition_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let is_def = matches!(
        node.kind(),
        "function_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "constructor_declaration"
    );
    if !is_def {
        return None;
    }
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Node kinds that represent a call expression, keyed by tree-sitter node kind.
fn call_callee_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let is_call = matches!(
        node.kind(),
        "call_expression" | "call" | "method_invocation" | "invocation_expression"
    );
    if !is_call {
        return None;
    }
    // The function being called is usually the `function` field, or the first
    // named child for languages that don't expose it as a field.
    let func_node = node
        .child_by_field_name("function")
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.named_child(0))?;

    let text = func_node.utf8_text(source).ok()?;
    // Reduce qualified paths / member access to the final identifier.
    let name = text
        .rsplit(['.', ':', '>'])
        .next()
        .unwrap_or(text)
        .trim()
        .trim_start_matches('(')
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn walk(
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file_path: &str,
    enclosing: &str,
    out: &mut Vec<CallEdge>,
) {
    let node = cursor.node();

    // Determine the enclosing function for children of this node.
    let current = definition_name(node, source);
    let scope: String = current.clone().unwrap_or_else(|| enclosing.to_string());

    if let Some(callee) = call_callee_name(node, source) {
        out.push(CallEdge {
            caller: enclosing.to_string(),
            callee,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32,
        });
    }

    if cursor.goto_first_child() {
        loop {
            walk(cursor, source, file_path, &scope, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_call_edges() {
        let src = "fn helper() {}\nfn main() { helper(); helper(); }";
        let edges = extract_call_edges(src, "rust", "src/main.rs");
        let mut g = CallGraph::new();
        g.add_edges(edges);
        assert!(g.callees_of("main").contains(&"helper".to_string()));
        assert!(g.callers_of("helper").contains(&"main".to_string()));
    }

    #[test]
    fn python_call_edges() {
        let src = "def a():\n    pass\n\ndef b():\n    a()\n";
        let edges = extract_call_edges(src, "python", "m.py");
        let mut g = CallGraph::new();
        g.add_edges(edges);
        assert!(g.callees_of("b").contains(&"a".to_string()));
    }

    #[test]
    fn method_calls_reduced_to_identifier() {
        let src = "fn run() { obj.method(); }";
        let edges = extract_call_edges(src, "rust", "src/lib.rs");
        // Reduced to final identifier `method`.
        assert!(edges.iter().any(|e| e.callee == "method"));
    }

    #[test]
    fn unknown_language_is_empty() {
        assert!(extract_call_edges("x", "brainfuck", "a.bf").is_empty());
    }

    #[test]
    fn graph_queries_empty_for_unknown_symbol() {
        let g = CallGraph::new();
        assert!(g.callers_of("nope").is_empty());
        assert!(g.callees_of("nope").is_empty());
    }
}
