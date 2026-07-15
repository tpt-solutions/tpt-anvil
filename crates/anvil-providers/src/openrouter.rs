// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use crate::{openai::OpenAiProvider, provider::CloudProvider};
use anvil_core::{
    Result,
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// OpenRouter — uses the OpenAI-compatible endpoint at api.openrouter.ai.
pub struct OpenRouterProvider(OpenAiProvider);

impl OpenRouterProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self(OpenAiProvider::with_base_url(
            api_key,
            model,
            "https://openrouter.ai/api/v1",
            BackendKind::OpenRouter,
        ))
    }
}

#[async_trait]
impl CloudProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        self.0.complete(request).await
    }

    async fn stream(&self, request: &CompletionRequest, tx: mpsc::Sender<StreamChunk>) -> Result<()> {
        self.0.stream(request, tx).await
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        self.0.count_tokens(text).await
    }
}
