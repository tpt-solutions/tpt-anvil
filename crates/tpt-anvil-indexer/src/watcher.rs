// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{store::IndexStore, walker};

pub struct IndexWatcher {
    _watcher: RecommendedWatcher,
}

impl IndexWatcher {
    pub fn new(root: &Path, store: Arc<Mutex<IndexStore>>) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<PathBuf>(64);

        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(ev) = event {
                    if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in ev.paths {
                            let _ = tx.try_send(path);
                        }
                    }
                }
            })?;

        watcher.watch(root, RecursiveMode::Recursive)?;

        tokio::spawn(async move {
            while let Some(path) = rx.recv().await {
                let Some(lang) = walker::detect_language(&path) else {
                    continue;
                };
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };

                let hash = walker::content_hash(content.as_bytes());
                let path_str = path.to_string_lossy().to_string();
                let lang = lang.to_string();
                let store_clone = Arc::clone(&store);

                tokio::task::spawn_blocking(move || {
                    let mtime = std::fs::metadata(&path_str)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    let store = store_clone.lock().unwrap();
                    if let Ok(file_id) = store.upsert_file(&path_str, mtime, &hash) {
                        let symbols = crate::symbols::extract_symbols(&content, &lang, &path_str);
                        if let Err(e) = store.insert_symbols(file_id, &symbols) {
                            warn!("failed to update symbols for {path_str}: {e}");
                        }
                        if let Err(e) = store.upsert_fts(&path_str, &content) {
                            warn!("failed to update FTS for {path_str}: {e}");
                        } else {
                            info!("re-indexed {path_str}");
                        }
                    }
                })
                .await
                .ok();
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
