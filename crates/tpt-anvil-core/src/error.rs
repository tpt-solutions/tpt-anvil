// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AnvilError>;

#[derive(Debug, Error)]
pub enum AnvilError {
    #[error("inference error: {0}")]
    Inference(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("indexer error: {0}")]
    Indexer(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("unsupported backend: {0}")]
    UnsupportedBackend(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AnvilError {
    fn from(e: anyhow::Error) -> Self {
        AnvilError::Other(e.to_string())
    }
}
