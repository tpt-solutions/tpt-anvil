// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::{
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, Role, StreamChunk},
    AnvilError, Result,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::debug;

use crate::backend::InferenceBackend;

pub struct OllamaBackend {
    base_url: String,
    client: Client,
}

impl OllamaBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    done: bool,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[async_trait]
impl InferenceBackend for OllamaBackend {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AnvilError::Inference(e.to_string()))?
            .json::<OllamaTagsResponse>()
            .await
            .map_err(|e| AnvilError::Inference(e.to_string()))?;

        Ok(resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                context_length: 8192,
                backend: BackendKind::Ollama,
            })
            .collect())
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let model = request.model.as_deref().unwrap_or("deepseek-coder:6.7b");
        let messages: Vec<OllamaMessage> = request
            .messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let body = OllamaChatRequest {
            model,
            messages,
            stream: false,
            options: OllamaOptions {
                num_predict: request.max_tokens,
                temperature: request.temperature,
            },
        };

        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AnvilError::Inference(e.to_string()))?
            .json::<OllamaChatResponse>()
            .await
            .map_err(|e| AnvilError::Inference(e.to_string()))?;

        Ok(CompletionResponse {
            content: resp.message.content,
            model: model.to_string(),
            usage: Some(anvil_core::types::TokenUsage {
                prompt_tokens: resp.prompt_eval_count,
                completion_tokens: resp.eval_count,
                total_tokens: resp.prompt_eval_count + resp.eval_count,
            }),
        })
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        use futures_util::StreamExt;

        let model = request.model.as_deref().unwrap_or("deepseek-coder:6.7b");
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role { Role::System => "system", Role::User => "user", Role::Assistant => "assistant" },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "options": { "num_predict": request.max_tokens, "temperature": request.temperature },
        });

        let url = format!("{}/api/chat", self.base_url);
        let mut stream = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AnvilError::Inference(e.to_string()))?
            .bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| AnvilError::Inference(e.to_string()))?;
            let line = std::str::from_utf8(&bytes).unwrap_or("").trim().to_string();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<OllamaChatResponse>(&line) {
                Ok(parsed) => {
                    let done = parsed.done;
                    let _ = tx
                        .send(StreamChunk {
                            delta: parsed.message.content,
                            done,
                        })
                        .await;
                    if done {
                        break;
                    }
                }
                Err(e) => {
                    debug!("failed to parse stream chunk: {e} — line: {line}");
                }
            }
        }
        Ok(())
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        // Rough approximation: ~4 chars per token
        Ok((text.len() as u32).saturating_div(4))
    }
}
