// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use async_trait::async_trait;
use tokio::sync::mpsc;
use tpt_anvil_core::{
    types::{CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    Result,
};

#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Backend identifier (e.g. "ollama", "llama_cpp", "candle").
    fn name(&self) -> &str;

    /// List models available on this backend.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Non-streaming completion.
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse>;

    /// Streaming completion — sends chunks over the returned channel.
    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()>;

    /// Approximate token count for a string (backend-specific).
    async fn count_tokens(&self, text: &str) -> Result<u32>;
}
