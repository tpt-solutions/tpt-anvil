// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub stream: bool,
}

fn default_max_tokens() -> u32 {
    2048
}
fn default_temperature() -> f32 {
    0.2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_length: u32,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    LlamaCpp,
    Candle,
    Ollama,
    OpenAi,
    AzureOpenAi,
    Anthropic,
    OpenRouter,
    OpenAiCompatible,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for ProviderError {
    fn from(e: anyhow::Error) -> Self {
        ProviderError::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProviderError>;

/// Minimal configuration for provider registry, independent of the full Anvil config.
pub struct ProviderConfig {
    pub active: Option<String>,
    pub openai_model: String,
    pub openai_api_key_entry: Option<String>,
    pub anthropic_model: String,
    pub anthropic_api_key_entry: Option<String>,
    pub openrouter_model: String,
    pub openrouter_api_key_entry: Option<String>,
    pub azure_endpoint: Option<String>,
    pub azure_api_version: String,
    pub azure_api_key_entry: Option<String>,
    pub custom_base_url: Option<String>,
    pub custom_model: Option<String>,
    pub custom_api_key_entry: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            active: None,
            // Left empty rather than hardcoding a specific model id: model
            // names change too often for a code-level default to stay
            // current. `ProviderRegistry::from_config` requires the user to
            // set one explicitly.
            openai_model: String::new(),
            openai_api_key_entry: None,
            anthropic_model: String::new(),
            anthropic_api_key_entry: None,
            openrouter_model: String::new(),
            openrouter_api_key_entry: None,
            azure_endpoint: None,
            azure_api_version: "2024-02-01".into(),
            azure_api_key_entry: None,
            custom_base_url: None,
            custom_model: None,
            custom_api_key_entry: None,
        }
    }
}
