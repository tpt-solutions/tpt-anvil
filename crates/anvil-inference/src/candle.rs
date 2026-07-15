// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Pure-Rust candle inference backend.
//! Enable with feature flag: --features candle

use async_trait::async_trait;
use anvil_core::{
    AnvilError, Result,
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
};
use tokio::sync::mpsc;

use crate::backend::InferenceBackend;

pub struct CandleBackend {
    model_path: String,
}

impl CandleBackend {
    pub fn new(model_path: &str) -> Result<Self> {
        if !std::path::Path::new(model_path).exists() {
            return Err(AnvilError::ModelNotFound(model_path.to_string()));
        }
        Ok(Self { model_path: model_path.to_string() })
    }
}

#[async_trait]
impl InferenceBackend for CandleBackend {
    fn name(&self) -> &str {
        "candle"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: self.model_path.clone(),
            name: std::path::Path::new(&self.model_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            context_length: 8192,
            backend: BackendKind::Candle,
        }])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        // TODO: candle GGUF loading + inference loop
        Err(AnvilError::Inference("candle backend not yet fully integrated".into()))
    }

    async fn stream(&self, request: &CompletionRequest, tx: mpsc::Sender<StreamChunk>) -> Result<()> {
        // TODO: token-by-token streaming from candle sampling loop
        Err(AnvilError::Inference("candle streaming not yet implemented".into()))
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        Ok((text.len() as u32).saturating_div(4))
    }
}
