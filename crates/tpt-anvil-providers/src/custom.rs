// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use crate::{openai::OpenAiProvider, provider::CloudProvider};
use crate::types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, Result, StreamChunk};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Generic OpenAI-compatible provider (Groq, Together, Fireworks, local vLLM, etc.)
///
/// # Trust Boundary
///
/// The base URL is user-controlled and may point at internal network services.
/// API keys attached to requests (via the `Authorization` header) will be sent
/// to whatever host the base URL resolves to. Users should ensure their base URL
/// points to a trusted endpoint.
pub struct CustomProvider(OpenAiProvider);

impl CustomProvider {
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self(OpenAiProvider::with_base_url(
            api_key,
            model,
            base_url,
            BackendKind::OpenAiCompatible,
        )?))
    }
}

#[async_trait]
impl CloudProvider for CustomProvider {
    fn name(&self) -> &str {
        "custom"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        self.0.complete(request).await
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        self.0.stream(request, tx).await
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        self.0.count_tokens(text).await
    }
}
