// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use anvil_config::AnvilConfig;
use anvil_core::{AnvilError, Result};

use crate::{
    anthropic::AnthropicProvider,
    azure::AzureOpenAiProvider,
    custom::CustomProvider,
    keystore,
    openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
    provider::CloudProvider,
};

pub struct ProviderRegistry {
    pub active: Option<Arc<dyn CloudProvider>>,
}

impl ProviderRegistry {
    pub fn from_config(cfg: &AnvilConfig) -> Result<Self> {
        let active_name = match &cfg.providers.active {
            Some(name) => name.clone(),
            None => return Ok(Self { active: None }),
        };

        let provider: Arc<dyn CloudProvider> = match active_name.as_str() {
            "openai" => {
                let entry = cfg.providers.openai.api_key_entry.as_deref().unwrap_or("openai_api_key");
                let key = keystore::get_api_key(entry)?;
                let model = if cfg.providers.openai.model.is_empty() { "gpt-4o".to_string() } else { cfg.providers.openai.model.clone() };
                Arc::new(OpenAiProvider::new(key, model))
            }
            "anthropic" => {
                let entry = cfg.providers.anthropic.api_key_entry.as_deref().unwrap_or("anthropic_api_key");
                let key = keystore::get_api_key(entry)?;
                Arc::new(AnthropicProvider::new(key, &cfg.providers.anthropic.model))
            }
            "openrouter" => {
                let entry = cfg.providers.openrouter.api_key_entry.as_deref().unwrap_or("openrouter_api_key");
                let key = keystore::get_api_key(entry)?;
                Arc::new(OpenRouterProvider::new(key, &cfg.providers.openrouter.model))
            }
            "azure" => {
                let entry = cfg.providers.azure.api_key_entry.as_deref().unwrap_or("azure_api_key");
                let key = keystore::get_api_key(entry)?;
                let endpoint = cfg.providers.azure.endpoint.as_deref().ok_or_else(|| {
                    AnvilError::Config("azure provider requires providers.azure.endpoint".into())
                })?;
                Arc::new(AzureOpenAiProvider::new(key, endpoint, &cfg.providers.azure.api_version))
            }
            "custom" => {
                let entry = cfg.providers.custom.api_key_entry.as_deref().unwrap_or("custom_api_key");
                let key = keystore::get_api_key(entry).unwrap_or_default();
                let base_url = cfg.providers.custom.base_url.as_deref().ok_or_else(|| {
                    AnvilError::Config("custom provider requires providers.custom.base_url".into())
                })?;
                let model = cfg.providers.custom.model.as_deref().unwrap_or("").to_string();
                Arc::new(CustomProvider::new(key, model, base_url))
            }
            other => {
                return Err(AnvilError::Config(format!("unknown provider '{other}'")));
            }
        };

        Ok(Self { active: Some(provider) })
    }
}
