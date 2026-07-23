// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use crate::types::{
    BackendKind, CompletionRequest, CompletionResponse, ModelInfo, ProviderError, Result, Role,
    StreamChunk, TokenUsage,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    provider::CloudProvider,
    retry::{with_retry, RetryConfig},
};
use std::time::Duration;

pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    default_model: String,
    client: Client,
    backend_kind: BackendKind,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::with_base_url(
            api_key,
            model,
            "https://api.openai.com/v1",
            BackendKind::OpenAi,
        )
    }

    pub fn with_base_url(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
        backend_kind: BackendKind,
    ) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Provider(format!("failed to create HTTP client: {e}")))?;

        Ok(Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            default_model: model.into(),
            client,
            backend_kind,
        })
    }
}

#[derive(Serialize)]
struct OaiRequest<'a> {
    model: &'a str,
    messages: Vec<OaiMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize)]
struct OaiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    model: String,
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiResponseMessage,
}

#[derive(Deserialize)]
struct OaiResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OaiStreamChunk {
    choices: Vec<OaiStreamChoice>,
}

#[derive(Deserialize)]
struct OaiStreamChoice {
    delta: OaiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiDelta {
    #[serde(default)]
    content: String,
}

#[async_trait]
impl CloudProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                context_length: 128_000,
                backend: self.backend_kind.clone(),
            },
            ModelInfo {
                id: "gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                context_length: 128_000,
                backend: self.backend_kind.clone(),
            },
        ])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let retry_cfg = RetryConfig::default();
        with_retry(&retry_cfg, || async {
            let model = request.model.as_deref().unwrap_or(&self.default_model);
            let messages: Vec<OaiMessage> = request
                .messages
                .iter()
                .map(|m| OaiMessage {
                    role: match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    },
                    content: &m.content,
                })
                .collect();

            let body = OaiRequest {
                model,
                messages,
                max_tokens: request.max_tokens,
                temperature: request.temperature,
                stream: false,
            };
            let url = format!("{}/chat/completions", self.base_url);

            let resp = self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Provider(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(ProviderError::Provider(format!(
                    "OpenAI API error {status}: {text}"
                )));
            }

            let parsed: OaiResponse = resp
                .json()
                .await
                .map_err(|e| ProviderError::Provider(e.to_string()))?;

            Ok(CompletionResponse {
                content: parsed
                    .choices
                    .into_iter()
                    .next()
                    .map(|c| c.message.content)
                    .unwrap_or_default(),
                model: parsed.model,
                usage: parsed.usage.map(|u| TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
            })
        })
        .await
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let body = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": match m.role { Role::System => "system", Role::User => "user", Role::Assistant => "assistant" },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": true,
        });

        let url = format!("{}/chat/completions", self.base_url);
        let retry_cfg = RetryConfig::default();
        let mut stream = with_retry(&retry_cfg, || async {
            let resp = self
                .client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Provider(e.to_string()))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(ProviderError::Provider(format!(
                    "OpenAI API error {status}: {text}"
                )));
            }
            Ok(resp.bytes_stream())
        })
        .await?;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| ProviderError::Provider(e.to_string()))?;
            let text = std::str::from_utf8(&bytes).unwrap_or("").trim().to_string();

            for line in text.lines() {
                let line = line.trim();
                if line == "data: [DONE]" {
                    let _ = tx
                        .send(StreamChunk {
                            delta: String::new(),
                            done: true,
                        })
                        .await;
                    return Ok(());
                }
                let Some(json) = line.strip_prefix("data: ") else {
                    continue;
                };
                if let Ok(chunk) = serde_json::from_str::<OaiStreamChunk>(json) {
                    if let Some(choice) = chunk.choices.into_iter().next() {
                        let done = choice.finish_reason.as_deref() == Some("stop");
                        let _ = tx
                            .send(StreamChunk {
                                delta: choice.delta.content,
                                done,
                            })
                            .await;
                    }
                }
            }
        }
        Ok(())
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        Ok((text.len() as u32).saturating_div(4))
    }
}
