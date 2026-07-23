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
    #[serde(default)]
    pub vault: VaultConfig,
    #[serde(default)]
    pub smart_context: SmartContextConfig,
    #[serde(default)]
    pub router: RouterConfigSchema,
    #[serde(default)]
    pub verify: VerifyConfigSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Which backend to use: "ollama", "llama_cpp", "candle"
    #[serde(default = "default_inference_backend")]
    pub backend: String,
    /// Model identifier (e.g. "deepseek-coder:6.7b" for Ollama, or path to GGUF)
    #[serde(default = "default_inference_model")]
    pub model: String,
    /// Ollama server URL (only used when backend = "ollama")
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    /// Path to GGUF model file (only used when backend = "llama_cpp" or "candle")
    #[serde(default)]
    pub model_path: Option<String>,
    #[serde(default = "default_context_length")]
    pub context_length: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// GPU layers to offload (-1 = all)
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: i32,
}

fn default_inference_backend() -> String {
    "ollama".into()
}
fn default_inference_model() -> String {
    "deepseek-coder:6.7b".into()
}
fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}
fn default_context_length() -> u32 {
    8192
}
fn default_max_tokens() -> u32 {
    2048
}
fn default_temperature() -> f32 {
    0.2
}
fn default_gpu_layers() -> i32 {
    -1
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            backend: default_inference_backend(),
            model: default_inference_model(),
            ollama_url: default_ollama_url(),
            model_path: None,
            context_length: default_context_length(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            gpu_layers: default_gpu_layers(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    /// Which provider to use for cloud fallback: "openai", "anthropic", "openrouter", "azure", "custom"
    #[serde(default)]
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

// Cloud provider model fields default to an empty string rather than a
// specific model id: model names change too often for a code-level default
// to stay current, so `ProviderRegistry::from_config` requires the user to
// set one explicitly instead of silently picking a (potentially stale) one.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAiConfig {
    #[serde(default)]
    pub model: String,
    /// API key stored in OS keychain; this field is just the keychain entry name
    #[serde(default)]
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub deployment: Option<String>,
    #[serde(default = "default_azure_api_version")]
    pub api_version: String,
    #[serde(default)]
    pub api_key_entry: Option<String>,
}

fn default_azure_api_version() -> String {
    "2024-02-01".into()
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            deployment: None,
            api_version: default_azure_api_version(),
            api_key_entry: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicConfig {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenRouterConfig {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomProviderConfig {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub api_key_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Max file size to index in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    /// File glob patterns to exclude
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    /// Number of context chunks to retrieve per query
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_max_file_size() -> u64 {
    1_048_576
} // 1 MB
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "*.lock".into(),
        "node_modules/**".into(),
        "target/**".into(),
        ".git/**".into(),
        "*.min.js".into(),
        "dist/**".into(),
    ]
}
fn default_top_k() -> usize {
    10
}
fn default_embedding_model() -> String {
    "nomic-embed-code".into()
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
            exclude_patterns: default_exclude_patterns(),
            top_k: default_top_k(),
            embedding_model: default_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_size")]
    pub font_size: u8,
}

fn default_theme() -> String {
    "system".into()
}
fn default_font_size() -> u8 {
    14
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            font_size: default_font_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub redact_local: bool,
    #[serde(default)]
    pub custom_patterns: Vec<CustomPatternConfig>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            redact_local: false,
            custom_patterns: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomPatternConfig {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContextConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_file_size_threshold")]
    pub file_size_threshold_bytes: usize,
    #[serde(default = "default_chunk_size_threshold")]
    pub chunk_size_threshold_bytes: usize,
}

impl Default for SmartContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_size_threshold_bytes: 2048,
            chunk_size_threshold_bytes: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfigSchema {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub prefer_cheapest: bool,
    #[serde(default)]
    pub max_cost_per_request_usd: Option<f64>,
    /// If set, pin to this provider and disable auto-routing.
    #[serde(default)]
    pub pinned: Option<String>,
}

impl Default for RouterConfigSchema {
    fn default() -> Self {
        Self {
            enabled: false,
            prefer_cheapest: true,
            max_cost_per_request_usd: None,
            pinned: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyConfigSchema {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub run_tests: bool,
    #[serde(default = "default_true")]
    pub run_linter: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for VerifyConfigSchema {
    fn default() -> Self {
        Self {
            enabled: true,
            run_tests: false,
            run_linter: true,
            timeout_seconds: 60,
            max_retries: 1,
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_file_size_threshold() -> usize {
    2048
}
fn default_chunk_size_threshold() -> usize {
    1024
}
fn default_timeout() -> u64 {
    60
}
fn default_max_retries() -> u32 {
    1
}
