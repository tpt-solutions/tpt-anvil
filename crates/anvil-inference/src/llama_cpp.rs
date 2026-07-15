// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! llama.cpp backend via the llama-cpp-2 crate.
//! Enable with feature flag: --features llama-cpp

use async_trait::async_trait;
use anvil_core::{
    AnvilError, Result,
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
};
use tokio::sync::mpsc;

use crate::backend::InferenceBackend;

pub struct LlamaCppBackend {
    model_path: String,
    gpu_layers: i32,
}

impl LlamaCppBackend {
    pub fn new(model_path: &str, gpu_layers: i32) -> Result<Self> {
        if !std::path::Path::new(model_path).exists() {
            return Err(AnvilError::ModelNotFound(model_path.to_string()));
        }
        Ok(Self { model_path: model_path.to_string(), gpu_layers })
    }
}

#[async_trait]
impl InferenceBackend for LlamaCppBackend {
    fn name(&self) -> &str {
        "llama_cpp"
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
            backend: BackendKind::LlamaCpp,
        }])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        // TODO: integrate llama-cpp-2 synchronous inference
        // This is a stub — the llama-cpp-2 crate requires model loading at construction time.
        Err(AnvilError::Inference("llama_cpp backend not yet fully integrated".into()))
    }

    async fn stream(&self, request: &CompletionRequest, tx: mpsc::Sender<StreamChunk>) -> Result<()> {
        // TODO: streaming via llama-cpp-2
        Err(AnvilError::Inference("llama_cpp streaming not yet implemented".into()))
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        Ok((text.len() as u32).saturating_div(4))
    }
}
