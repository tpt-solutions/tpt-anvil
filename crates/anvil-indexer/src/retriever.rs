// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use anvil_config::AnvilConfig;
use tracing::info;

use crate::{store::IndexStore, walker, watcher::IndexWatcher};
use anvil_core::types::{ChunkType, ContextChunk};

pub struct Retriever {
    store: Arc<Mutex<IndexStore>>,
    _watcher: IndexWatcher,
    top_k: usize,
}

impl Retriever {
    pub fn new(root: &Path, cfg: &AnvilConfig) -> Result<Self> {
        let db_path = root.join(".anvil").join("index.db");
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let store = Arc::new(Mutex::new(IndexStore::open(&db_path)?));

        let store_clone = Arc::clone(&store);
        let root_owned = root.to_path_buf();

        tokio::task::spawn_blocking(move || {
            info!("indexing project at {}", root_owned.display());
            let store = store_clone.lock().unwrap();
            for path in walker::walk_project(&root_owned) {
                if let Some(lang) = walker::detect_language(&path) {
                    let Ok(content) = std::fs::read_to_string(&path) else { continue };
                    let hash = walker::content_hash(content.as_bytes());
                    let path_str = path.to_string_lossy().to_string();
                    let mtime = std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    if let Ok(file_id) = store.upsert_file(&path_str, mtime, &hash) {
                        let symbols = crate::symbols::extract_symbols(&content, lang, &path_str);
                        let _ = store.insert_symbols(file_id, &symbols);
                        let _ = store.upsert_fts(&path_str, &content);
                    }
                }
            }
            info!("initial indexing complete");
        });

        let watcher = IndexWatcher::new(root, Arc::clone(&store))?;

        Ok(Self { store, _watcher: watcher, top_k: cfg.indexing.top_k })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<ContextChunk>> {
        let store = Arc::clone(&self.store);
        let query = query.to_string();
        let top_k = self.top_k;

        tokio::task::spawn_blocking(move || {
            let store = store.lock().unwrap();
            let mut results = Vec::new();

            let fts = store.search_fts(&query, top_k)?;
            for (i, (path, snippet)) in fts.into_iter().enumerate() {
                results.push(ContextChunk {
                    file_path: path,
                    content: snippet,
                    score: 1.0 / (i as f32 + 1.0),
                    chunk_type: ChunkType::Snippet,
                });
            }

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

            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(top_k);
            Ok(results)
        })
        .await?
    }
}
