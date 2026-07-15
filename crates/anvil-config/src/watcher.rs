// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::{loader::ConfigLoader, schema::AnvilConfig};

pub struct ConfigWatcher {
    pub config: Arc<RwLock<AnvilConfig>>,
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn new(project_root: Option<PathBuf>) -> Result<Self> {
        let initial = ConfigLoader::load(project_root.as_deref())?;
        let config: Arc<RwLock<AnvilConfig>> = Arc::new(RwLock::new(initial));
        let config_clone = Arc::clone(&config);

        let (tx, mut rx) = mpsc::channel::<PathBuf>(16);

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

        if let Some(dir) = dirs::config_dir().map(|d| d.join("anvil")) {
            if dir.exists() {
                watcher.watch(&dir, RecursiveMode::NonRecursive)?;
            }
        }

        if let Some(root) = &project_root {
            let anvil_dir = root.join(".anvil");
            if anvil_dir.exists() {
                watcher.watch(&anvil_dir, RecursiveMode::NonRecursive)?;
            }
        }

        let root_clone = project_root.clone();
        tokio::spawn(async move {
            while let Some(path) = rx.recv().await {
                info!("config file changed: {}", path.display());
                match ConfigLoader::load(root_clone.as_deref()) {
                    Ok(new_cfg) => {
                        *config_clone.write().await = new_cfg;
                        info!("config reloaded");
                    }
                    Err(e) => warn!("failed to reload config: {e}"),
                }
            }
        });

        Ok(Self { config, _watcher: watcher })
    }
}
