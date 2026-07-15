// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Variable,
    Constant,
    Import,
    Unknown,
}

/// Extract symbols from source code using tree-sitter.
pub fn extract_symbols(source: &str, language: &str, file_path: &str) -> Vec<Symbol> {
    match language {
        "rust" => extract_with_parser(source, file_path, tree_sitter_rust::LANGUAGE.into(), parse_rust_symbols),
        "python" => extract_with_parser(source, file_path, tree_sitter_python::LANGUAGE.into(), parse_generic_symbols),
        "javascript" | "typescript" => extract_with_parser(source, file_path, tree_sitter_javascript::LANGUAGE.into(), parse_generic_symbols),
        "go" => extract_with_parser(source, file_path, tree_sitter_go::LANGUAGE.into(), parse_generic_symbols),
        "java" => extract_with_parser(source, file_path, tree_sitter_java::LANGUAGE.into(), parse_generic_symbols),
        "c" => extract_with_parser(source, file_path, tree_sitter_c::LANGUAGE.into(), parse_generic_symbols),
        _ => vec![],
    }
}

fn extract_with_parser(
    source: &str,
    file_path: &str,
    language: tree_sitter::Language,
    extractor: fn(&tree_sitter::Tree, &[u8], &str) -> Vec<Symbol>,
) -> Vec<Symbol> {
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return vec![];
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };
    extractor(&tree, source.as_bytes(), file_path)
}

fn parse_rust_symbols(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    collect_rust_nodes(&mut cursor, source, file_path, &mut symbols);
    symbols
}

fn collect_rust_nodes(
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Symbol>,
) {
    let node = cursor.node();
    let kind = match node.kind() {
        "function_item" => Some(SymbolKind::Function),
        "struct_item" => Some(SymbolKind::Struct),
        "enum_item" => Some(SymbolKind::Enum),
        "trait_item" => Some(SymbolKind::Trait),
        "impl_item" => Some(SymbolKind::Class),
        "mod_item" => Some(SymbolKind::Module),
        _ => None,
    };

    if let Some(sym_kind) = kind {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("?")
            .to_string();

        out.push(Symbol {
            name,
            kind: sym_kind,
            file_path: file_path.to_string(),
            start_line: node.start_position().row as u32,
            end_line: node.end_position().row as u32,
            signature: None,
            doc_comment: None,
        });
    }

    if cursor.goto_first_child() {
        loop {
            collect_rust_nodes(cursor, source, file_path, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn parse_generic_symbols(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    collect_generic_nodes(&mut cursor, source, file_path, &mut symbols);
    symbols
}

fn collect_generic_nodes(
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Symbol>,
) {
    let node = cursor.node();
    let kind = match node.kind() {
        "function_definition" | "function_declaration" | "method_definition" => Some(SymbolKind::Function),
        "class_definition" | "class_declaration" => Some(SymbolKind::Class),
        _ => None,
    };

    if let Some(sym_kind) = kind {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("?")
            .to_string();
        out.push(Symbol {
            name,
            kind: sym_kind,
            file_path: file_path.to_string(),
            start_line: node.start_position().row as u32,
            end_line: node.end_position().row as u32,
            signature: None,
            doc_comment: None,
        });
    }

    if cursor.goto_first_child() {
        loop {
            collect_generic_nodes(cursor, source, file_path, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
