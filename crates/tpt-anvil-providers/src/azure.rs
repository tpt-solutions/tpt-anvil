// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use crate::{openai::OpenAiProvider, provider::CloudProvider};
use anvil_core::{
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Azure OpenAI provider — OpenAI-compatible endpoint at your Azure resource.
pub struct AzureOpenAiProvider(OpenAiProvider);

impl AzureOpenAiProvider {
    /// endpoint: e.g. "https://my-resource.openai.azure.com/openai/deployments/my-deployment"
    pub fn new(api_key: impl Into<String>, endpoint: impl Into<String>, api_version: &str) -> Self {
        let base = format!(
            "{}?api-version={}",
            endpoint.into().trim_end_matches('/'),
            api_version
        );
        Self(OpenAiProvider::with_base_url(
            api_key,
            "",
            base,
            BackendKind::AzureOpenAi,
        ))
    }
}

#[async_trait]
impl CloudProvider for AzureOpenAiProvider {
    fn name(&self) -> &str {
        "azure"
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
