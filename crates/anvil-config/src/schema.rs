// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnvilConfig {
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub indexing: IndexingConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Which backend to use: "ollama", "llama_cpp", "candle"
    pub backend: String,
    /// Model identifier (e.g. "deepseek-coder:6.7b" for Ollama, or path to GGUF)
    pub model: String,
    /// Ollama server URL (only used when backend = "ollama")
    pub ollama_url: String,
    /// Path to GGUF model file (only used when backend = "llama_cpp" or "candle")
    pub model_path: Option<String>,
    pub context_length: u32,
    pub max_tokens: u32,
    pub temperature: f32,
    /// GPU layers to offload (-1 = all)
    pub gpu_layers: i32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            backend: "ollama".into(),
            model: "deepseek-coder:6.7b".into(),
            ollama_url: "http://localhost:11434".into(),
            model_path: None,
            context_length: 8192,
            max_tokens: 2048,
            temperature: 0.2,
            gpu_layers: -1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    /// Which provider to use for cloud fallback: "openai", "anthropic", "openrouter", "azure", "custom"
    pub active: Option<String>,
    #[serde(default)]
    pub openai: OpenAiConfig,
    #[serde(default)]
    pub azure: AzureConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub custom: CustomProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAiConfig {
    pub model: String,
    /// API key stored in OS keychain; this field is just the keychain entry name
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    pub endpoint: Option<String>,
    pub deployment: Option<String>,
    pub api_version: String,
    pub api_key_entry: Option<String>,
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            deployment: None,
            api_version: "2024-02-01".into(),
            api_key_entry: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub model: String,
    pub api_key_entry: Option<String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-5".into(),
            api_key_entry: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub model: String,
    pub api_key_entry: Option<String>,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            model: "deepseek/deepseek-coder".into(),
            api_key_entry: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomProviderConfig {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Max file size to index in bytes
    pub max_file_size: u64,
    /// File glob patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Number of context chunks to retrieve per query
    pub top_k: usize,
    pub embedding_model: String,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1_048_576, // 1 MB
            exclude_patterns: vec![
                "*.lock".into(),
                "node_modules/**".into(),
                "target/**".into(),
                ".git/**".into(),
                "*.min.js".into(),
                "dist/**".into(),
            ],
            top_k: 10,
            embedding_model: "nomic-embed-code".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub font_size: u8,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            font_size: 14,
        }
    }
}
