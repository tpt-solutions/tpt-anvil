// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// BM25 lexical search is handled via SQLite FTS5 (which uses BM25 ranking natively).
// This module re-exports the search interface for clarity.

pub use crate::store::IndexStore;
