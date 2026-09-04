// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    Symbol,
    Snippet,
    Doc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub file_path: String,
    pub content: String,
    pub score: f32,
    pub chunk_type: ChunkType,
}

/// Minimal configuration for the indexer, independent of the full Anvil config.
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub top_k: usize,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self { top_k: 10 }
    }
}
