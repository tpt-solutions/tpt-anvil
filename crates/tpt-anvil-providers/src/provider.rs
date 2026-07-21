// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::{
    types::{CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait CloudProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()>;
    async fn count_tokens(&self, text: &str) -> Result<u32>;
}
