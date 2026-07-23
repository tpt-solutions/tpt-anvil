// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use crate::types::{
    BackendKind, CompletionRequest, CompletionResponse, ModelInfo, ProviderError, Result, Role,
    StreamChunk, TokenUsage,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::provider::CloudProvider;
use std::time::Duration;

pub struct AnthropicProvider {
    api_key: String,
    default_model: String,
    base_url: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::with_base_url(api_key, model, "https://api.anthropic.com/v1")
    }

    /// Construct with a custom base URL (used for testing against a mock server).
    pub fn with_base_url(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Provider(format!("failed to create HTTP client: {e}")))?;

        Ok(Self {
            api_key: api_key.into(),
            default_model: model.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        })
    }
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[async_trait]
impl CloudProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "claude-sonnet-5".into(),
                name: "Claude Sonnet 5".into(),
                context_length: 200_000,
                backend: BackendKind::Anthropic,
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".into(),
                name: "Claude Haiku 4.5".into(),
                context_length: 200_000,
                backend: BackendKind::Anthropic,
            },
        ])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);

        let system = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.as_str());
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": if m.role == Role::User { "user" } else { "assistant" },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys.to_string());
        }

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!(
                "Anthropic API error {status}: {text}"
            )));
        }

        let parsed: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?;
        let text = parsed
            .content
            .into_iter()
            .filter(|c| c.kind == "text")
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content: text,
            model: parsed.model,
            usage: Some(TokenUsage {
                prompt_tokens: parsed.usage.input_tokens,
                completion_tokens: parsed.usage.output_tokens,
                total_tokens: parsed.usage.input_tokens + parsed.usage.output_tokens,
            }),
        })
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let model = request.model.as_deref().unwrap_or(&self.default_model);
        let system = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": if m.role == Role::User { "user" } else { "assistant" },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "stream": true,
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        let mut stream = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(e.to_string()))?
            .bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| ProviderError::Provider(e.to_string()))?;
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            for line in text.lines() {
                let line = line.trim();
                let Some(json) = line.strip_prefix("data: ") else {
                    continue;
                };
                if let Ok(ev) = serde_json::from_str::<AnthropicStreamEvent>(json) {
                    if ev.kind == "content_block_delta" {
                        if let Some(delta) = ev.delta {
                            if delta.kind == "text_delta" {
                                let _ = tx
                                    .send(StreamChunk {
                                        delta: delta.text,
                                        done: false,
                                    })
                                    .await;
                            }
                        }
                    } else if ev.kind == "message_stop" {
                        let _ = tx
                            .send(StreamChunk {
                                delta: String::new(),
                                done: true,
                            })
                            .await;
                        return Ok(());
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
