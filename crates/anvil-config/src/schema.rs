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

impl AnvilConfig {
    /// Merge another config on top of this one. Non-default values in `overlay`
    /// win over `self` for scalar fields; nested structs are merged recursively.
    pub fn merge_with(self, overlay: AnvilConfig) -> Self {
        Self {
            inference: merge(self.inference, overlay.inference),
            providers: merge(self.providers, overlay.providers),
            indexing: merge(self.indexing, overlay.indexing),
            ui: merge(self.ui, overlay.ui),
            vault: merge(self.vault, overlay.vault),
            smart_context: merge(self.smart_context, overlay.smart_context),
            router: merge(self.router, overlay.router),
            verify: merge(self.verify, overlay.verify),
        }
    }
}

impl InferenceConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            backend: merge_scalar(base.backend, overlay.backend, "ollama".into()),
            model: merge_scalar(base.model, overlay.model, "deepseek-coder:6.7b".into()),
            ollama_url: merge_scalar(base.ollama_url, overlay.ollama_url, "http://localhost:11434".into()),
            model_path: overlay.model_path.or(base.model_path),
            context_length: merge_scalar(base.context_length, overlay.context_length, 8192),
            max_tokens: merge_scalar(base.max_tokens, overlay.max_tokens, 2048),
            temperature: merge_scalar(base.temperature, overlay.temperature, 0.2),
            gpu_layers: merge_scalar(base.gpu_layers, overlay.gpu_layers, -1),
        }
    }
}

impl ProvidersConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            active: overlay.active.or(base.active),
            openai: merge(base.openai, overlay.openai),
            azure: merge(base.azure, overlay.azure),
            anthropic: merge(base.anthropic, overlay.anthropic),
            openrouter: merge(base.openrouter, overlay.openrouter),
            custom: merge(base.custom, overlay.custom),
        }
    }
}

impl OpenAiConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            model: merge_scalar(base.model, overlay.model, "".into()),
            api_key_entry: overlay.api_key_entry.or(base.api_key_entry),
        }
    }
}

impl AzureConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            endpoint: overlay.endpoint.or(base.endpoint),
            deployment: overlay.deployment.or(base.deployment),
            api_version: merge_scalar(base.api_version, overlay.api_version, "2024-02-01".into()),
            api_key_entry: overlay.api_key_entry.or(base.api_key_entry),
        }
    }
}

impl AnthropicConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            model: merge_scalar(base.model, overlay.model, "claude-sonnet-5".into()),
            api_key_entry: overlay.api_key_entry.or(base.api_key_entry),
        }
    }
}

impl OpenRouterConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            model: merge_scalar(base.model, overlay.model, "deepseek/deepseek-coder".into()),
            api_key_entry: overlay.api_key_entry.or(base.api_key_entry),
        }
    }
}

impl CustomProviderConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            base_url: overlay.base_url.or(base.base_url),
            model: overlay.model.or(base.model),
            api_key_entry: overlay.api_key_entry.or(base.api_key_entry),
        }
    }
}

impl IndexingConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            max_file_size: merge_scalar(base.max_file_size, overlay.max_file_size, 1_048_576),
            exclude_patterns: if overlay.exclude_patterns.is_empty() {
                base.exclude_patterns
            } else {
                overlay.exclude_patterns
            },
            top_k: merge_scalar(base.top_k, overlay.top_k, 10),
            embedding_model: merge_scalar(base.embedding_model, overlay.embedding_model, "nomic-embed-code".into()),
        }
    }
}

impl UiConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            theme: merge_scalar(base.theme, overlay.theme, "system".into()),
            font_size: merge_scalar(base.font_size, overlay.font_size, 14),
        }
    }
}

/// For scalar types: if overlay equals the default, keep base; otherwise overlay wins.
fn merge_scalar<T: PartialEq>(base: T, overlay: T, default: T) -> T {
    if overlay == default { base } else { overlay }
}

/// Generic merge that delegates to each type's `Self::merge()` method.
trait HasMerge: Sized {
    fn merge_fields(base: Self, overlay: Self) -> Self;
}

impl HasMerge for InferenceConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for ProvidersConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for IndexingConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for UiConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for VaultConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for SmartContextConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for RouterConfigSchema {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for VerifyConfigSchema {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for OpenAiConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for AzureConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for AnthropicConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for OpenRouterConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}
impl HasMerge for CustomProviderConfig {
    fn merge_fields(base: Self, overlay: Self) -> Self { Self::merge(base, overlay) }
}

fn merge<T: HasMerge>(base: T, overlay: T) -> T {
    T::merge_fields(base, overlay)
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
        Self { enabled: true, redact_local: false, custom_patterns: vec![] }
    }
}

impl VaultConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            enabled: merge_scalar(base.enabled, overlay.enabled, true),
            redact_local: merge_scalar(base.redact_local, overlay.redact_local, false),
            custom_patterns: if overlay.custom_patterns.is_empty() { base.custom_patterns } else { overlay.custom_patterns },
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
        Self { enabled: true, file_size_threshold_bytes: 2048, chunk_size_threshold_bytes: 1024 }
    }
}

impl SmartContextConfig {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            enabled: merge_scalar(base.enabled, overlay.enabled, true),
            file_size_threshold_bytes: merge_scalar(base.file_size_threshold_bytes, overlay.file_size_threshold_bytes, 2048),
            chunk_size_threshold_bytes: merge_scalar(base.chunk_size_threshold_bytes, overlay.chunk_size_threshold_bytes, 1024),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfigSchema {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub prefer_cheapest: bool,
    pub max_cost_per_request_usd: Option<f64>,
    /// If set, pin to this provider and disable auto-routing.
    pub pinned: Option<String>,
}

impl Default for RouterConfigSchema {
    fn default() -> Self {
        Self { enabled: false, prefer_cheapest: true, max_cost_per_request_usd: None, pinned: None }
    }
}

impl RouterConfigSchema {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            enabled: merge_scalar(base.enabled, overlay.enabled, false),
            prefer_cheapest: merge_scalar(base.prefer_cheapest, overlay.prefer_cheapest, true),
            max_cost_per_request_usd: overlay.max_cost_per_request_usd.or(base.max_cost_per_request_usd),
            pinned: overlay.pinned.or(base.pinned),
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
        Self { enabled: true, run_tests: false, run_linter: true, timeout_seconds: 60, max_retries: 1 }
    }
}

impl VerifyConfigSchema {
    pub fn merge(base: Self, overlay: Self) -> Self {
        Self {
            enabled: merge_scalar(base.enabled, overlay.enabled, true),
            run_tests: merge_scalar(base.run_tests, overlay.run_tests, false),
            run_linter: merge_scalar(base.run_linter, overlay.run_linter, true),
            timeout_seconds: merge_scalar(base.timeout_seconds, overlay.timeout_seconds, 60),
            max_retries: merge_scalar(base.max_retries, overlay.max_retries, 1),
        }
    }
}

fn default_true() -> bool { true }
fn default_file_size_threshold() -> usize { 2048 }
fn default_chunk_size_threshold() -> usize { 1024 }
fn default_timeout() -> u64 { 60 }
fn default_max_retries() -> u32 { 1 }
