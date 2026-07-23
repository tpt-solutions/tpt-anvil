// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use tracing::warn;

use crate::router::ProviderEntry;
use crate::types::{BackendKind, ProviderConfig, ProviderError, Result};

use crate::{
    anthropic::AnthropicProvider, azure::AzureOpenAiProvider, custom::CustomProvider, keystore,
    openai::OpenAiProvider, openrouter::OpenRouterProvider, provider::CloudProvider,
};

pub struct ProviderRegistry {
    /// The single cloud provider selected via `providers.active`, used as the
    /// local-backend fallback when the router is disabled. Preserved for
    /// backward compatibility with the pre-Router single-provider design.
    pub active: Option<Arc<dyn CloudProvider>>,
    /// Every provider with enough config to be usable, regardless of which
    /// one is `active` — this is what `router::select_provider` chooses from
    /// when routing is enabled. Providers that fail to construct (e.g. a
    /// missing keychain entry) are skipped rather than treated as fatal,
    /// since routing is opportunistic across whatever is available.
    pub available: Vec<ProviderEntry>,
}

impl ProviderRegistry {
    pub fn from_config(cfg: &ProviderConfig) -> Result<Self> {
        let active = match &cfg.active {
            Some(name) => Some(build_provider(name, cfg)?),
            None => None,
        };

        let mut available = Vec::new();
        for name in ["openai", "anthropic", "openrouter", "custom"] {
            // Azure is excluded here: `ProviderConfig` doesn't currently carry
            // a deployment/model id for it, so there's no `model_id` to cost
            // against — it remains reachable only via `providers.active`.
            if !provider_config_is_present(name, cfg) {
                continue;
            }
            match build_provider(name, cfg) {
                Ok(provider) => available.push(ProviderEntry {
                    name: name.to_string(),
                    backend_kind: backend_kind_for(name),
                    model_id: model_id_for(name, cfg),
                    provider,
                }),
                Err(e) => warn!("skipping provider '{name}' for routing: {e}"),
            }
        }

        Ok(Self { active, available })
    }
}

/// Whether `cfg` has enough set for `name` to be worth attempting to build
/// (a non-empty model plus, for `custom`, a base URL). Doesn't verify the
/// keychain entry actually resolves — that's discovered at construction time.
fn provider_config_is_present(name: &str, cfg: &ProviderConfig) -> bool {
    match name {
        "openai" => !cfg.openai_model.is_empty(),
        "anthropic" => !cfg.anthropic_model.is_empty(),
        "openrouter" => !cfg.openrouter_model.is_empty(),
        "custom" => cfg.custom_base_url.is_some() && cfg.custom_model.is_some(),
        _ => false,
    }
}

fn backend_kind_for(name: &str) -> BackendKind {
    match name {
        "openai" => BackendKind::OpenAi,
        "anthropic" => BackendKind::Anthropic,
        "openrouter" => BackendKind::OpenRouter,
        "azure" => BackendKind::AzureOpenAi,
        _ => BackendKind::OpenAiCompatible,
    }
}

fn model_id_for(name: &str, cfg: &ProviderConfig) -> String {
    match name {
        "openai" => cfg.openai_model.clone(),
        "anthropic" => cfg.anthropic_model.clone(),
        "openrouter" => cfg.openrouter_model.clone(),
        "custom" => cfg.custom_model.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn build_provider(name: &str, cfg: &ProviderConfig) -> Result<Arc<dyn CloudProvider>> {
    let provider: Arc<dyn CloudProvider> = match name {
        "openai" => {
                let entry = cfg
                    .openai_api_key_entry
                    .as_deref()
                    .unwrap_or("openai_api_key");
                let key = keystore::get_api_key(entry)?;
                if cfg.openai_model.is_empty() {
                    return Err(ProviderError::Config(
                        "providers.openai.model is not set — model names change too often to hardcode a default; pick one from https://platform.openai.com/docs/models".into(),
                    ));
                }
                Arc::new(OpenAiProvider::new(key, cfg.openai_model.clone())?)
            }
            "anthropic" => {
                let entry = cfg
                    .anthropic_api_key_entry
                    .as_deref()
                    .unwrap_or("anthropic_api_key");
                let key = keystore::get_api_key(entry)?;
                if cfg.anthropic_model.is_empty() {
                    return Err(ProviderError::Config(
                        "providers.anthropic.model is not set — model names change too often to hardcode a default; pick one from https://docs.anthropic.com/en/docs/about-claude/models".into(),
                    ));
                }
                Arc::new(AnthropicProvider::new(key, &cfg.anthropic_model)?)
            }
            "openrouter" => {
                let entry = cfg
                    .openrouter_api_key_entry
                    .as_deref()
                    .unwrap_or("openrouter_api_key");
                let key = keystore::get_api_key(entry)?;
                Arc::new(OpenRouterProvider::new(key, &cfg.openrouter_model)?)
            }
            "azure" => {
                let entry = cfg
                    .azure_api_key_entry
                    .as_deref()
                    .unwrap_or("azure_api_key");
                let key = keystore::get_api_key(entry)?;
                let endpoint = cfg.azure_endpoint.as_deref().ok_or_else(|| {
                    ProviderError::Config("azure provider requires providers.azure.endpoint".into())
                })?;
                Arc::new(AzureOpenAiProvider::new(
                    key,
                    endpoint,
                    &cfg.azure_api_version,
                )?)
            }
            "custom" => {
                let entry = cfg
                    .custom_api_key_entry
                    .as_deref()
                    .unwrap_or("custom_api_key");
                let key = keystore::get_api_key(entry).unwrap_or_default();
                let base_url = cfg.custom_base_url.as_deref().ok_or_else(|| {
                    ProviderError::Config(
                        "custom provider requires providers.custom.base_url".into(),
                    )
                })?;
                let model = cfg.custom_model.as_deref().unwrap_or("").to_string();
                Arc::new(CustomProvider::new(key, model, base_url)?)
            }
        other => {
            return Err(ProviderError::Config(format!("unknown provider '{other}'")));
        }
    };

    Ok(provider)
}
