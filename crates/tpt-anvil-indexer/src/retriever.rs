// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tracing::info;

use crate::embedding::{cosine_similarity, Embedder, HashingEmbedder};
use crate::fusion::{reciprocal_rank_fusion, RankedItem, DEFAULT_RRF_K};
use crate::types::{ChunkType, ContextChunk, IndexerConfig};
use crate::{store::IndexStore, walker, watcher::IndexWatcher};

pub struct Retriever {
    store: Arc<Mutex<IndexStore>>,
    _watcher: IndexWatcher,
    top_k: usize,
    embedder: Arc<dyn Embedder>,
}

impl Retriever {
    pub fn new(root: &Path, cfg: &IndexerConfig) -> Result<Self> {
        let db_path = root.join(".anvil").join("index.db");
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let store = Arc::new(Mutex::new(IndexStore::open(&db_path)?));

        // Default to the dependency-free hashing embedder so vector search works
        // fully offline without a model download.
        let embedder: Arc<dyn Embedder> = Arc::new(HashingEmbedder::default());

        let store_clone = Arc::clone(&store);
        let embedder_clone = Arc::clone(&embedder);
        let root_owned = root.to_path_buf();

        tokio::task::spawn_blocking(move || {
            info!("indexing project at {}", root_owned.display());
            for path in walker::walk_project(&root_owned) {
                if let Some(lang) = walker::detect_language(&path) {
                    let Ok(content) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let hash = walker::content_hash(content.as_bytes());
                    let path_str = path.to_string_lossy().to_string();
                    let mtime = std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    // Compute embeddings for line-window chunks off the async runtime.
                    let chunks = chunk_content(&content, 40);
                    let embeddings: Vec<(String, Vec<f32>)> = chunks
                        .iter()
                        .map(|c| {
                            let v = futures_executor::block_on(embedder_clone.embed(c))
                                .unwrap_or_default();
                            (c.clone(), v)
                        })
                        .collect();

                    let store = store_clone.lock().unwrap();
                    if let Ok(file_id) = store.upsert_file(&path_str, mtime, &hash) {
                        let symbols = crate::symbols::extract_symbols(&content, lang, &path_str);
                        let _ = store.insert_symbols(file_id, &symbols);
                        let _ = store.upsert_fts(&path_str, &content);
                        let _ = store.upsert_embeddings(&path_str, &embeddings);
                    }
                }
            }
            info!("initial indexing complete");
        });

        let watcher = IndexWatcher::new(root, Arc::clone(&store))?;

        Ok(Self {
            store,
            _watcher: watcher,
            top_k: cfg.top_k,
            embedder,
        })
    }

    /// Hybrid search: fuse BM25 lexical ranking with vector cosine similarity
    /// via Reciprocal Rank Fusion, then blend in exact symbol matches.
    pub async fn search(&self, query: &str) -> Result<Vec<ContextChunk>> {
        let top_k = self.top_k;

        // Vector side: embed the query, then rank stored chunks by cosine similarity.
        let query_vec = self.embedder.embed(query).await.unwrap_or_default();

        let store = Arc::clone(&self.store);
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let store = store.lock().unwrap();

            // BM25 list (already ordered best-first by FTS5 rank).
            let bm25_hits = store.search_fts(&query, top_k * 2)?;
            let bm25_list: Vec<RankedItem> = bm25_hits
                .into_iter()
                .enumerate()
                .map(|(i, (path, snippet))| RankedItem {
                    key: format!("{path}#bm25:{i}"),
                    content: snippet,
                    file_path: path,
                    raw_score: 0.0,
                })
                .collect();

            // Vector list: score every stored embedding, keep the best.
            let mut vector_scored: Vec<(f32, String, String)> = store
                .all_embeddings()?
                .into_iter()
                .map(|(path, content, vec)| (cosine_similarity(&query_vec, &vec), path, content))
                .collect();
            vector_scored
                .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let vector_list: Vec<RankedItem> = vector_scored
                .into_iter()
                .take(top_k * 2)
                .enumerate()
                .map(|(i, (score, path, content))| RankedItem {
                    key: format!("{path}#vec:{i}"),
                    content,
                    file_path: path,
                    raw_score: score,
                })
                .collect();

            let fused = reciprocal_rank_fusion(&[bm25_list, vector_list], DEFAULT_RRF_K, top_k);

            let mut results: Vec<ContextChunk> = fused
                .into_iter()
                .map(|f| ContextChunk {
                    file_path: f.file_path,
                    content: f.content,
                    score: f.rrf_score,
                    chunk_type: ChunkType::Snippet,
                })
                .collect();

            // Exact symbol matches are high-signal; append them.
            let syms = store.search_symbols(&query, top_k / 2)?;
            for sym in syms {
                results.push(ContextChunk {
                    file_path: sym.file_path,
                    content: format!(
                        "{} {} (line {})",
                        format!("{:?}", sym.kind).to_lowercase(),
                        sym.name,
                        sym.start_line
                    ),
                    score: 0.8,
                    chunk_type: ChunkType::Symbol,
                });
            }

            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(top_k);
            Ok(results)
        })
        .await?
    }
}

/// Split source into overlapping-free line windows of `window` lines each.
fn chunk_content(content: &str, window: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }
    lines
        .chunks(window.max(1))
        .map(|c| c.join("\n"))
        .filter(|s| !s.trim().is_empty())
        .collect()
}
