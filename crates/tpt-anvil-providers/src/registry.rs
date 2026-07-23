// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use crate::types::{ProviderConfig, ProviderError, Result};

use crate::{
    anthropic::AnthropicProvider, azure::AzureOpenAiProvider, custom::CustomProvider, keystore,
    openai::OpenAiProvider, openrouter::OpenRouterProvider, provider::CloudProvider,
};

pub struct ProviderRegistry {
    pub active: Option<Arc<dyn CloudProvider>>,
}

impl ProviderRegistry {
    pub fn from_config(cfg: &ProviderConfig) -> Result<Self> {
        let active_name = match &cfg.active {
            Some(name) => name.clone(),
            None => return Ok(Self { active: None }),
        };

        let provider: Arc<dyn CloudProvider> = match active_name.as_str() {
            "openai" => {
                let entry = cfg
                    .openai_api_key_entry
                    .as_deref()
                    .unwrap_or("openai_api_key");
                let key = keystore::get_api_key(entry)?;
                let model = if cfg.openai_model.is_empty() {
                    "gpt-4o".to_string()
                } else {
                    cfg.openai_model.clone()
                };
                Arc::new(OpenAiProvider::new(key, model)?)
            }
            "anthropic" => {
                let entry = cfg
                    .anthropic_api_key_entry
                    .as_deref()
                    .unwrap_or("anthropic_api_key");
                let key = keystore::get_api_key(entry)?;
                Arc::new(AnthropicProvider::new(key, &cfg.anthropic_model)?)
            }
            "openrouter" => {
                let entry = cfg
                    .openrouter_api_key_entry
                    .as_deref()
                    .unwrap_or("openrouter_api_key");
                let key = keystore::get_api_key(entry)?;
                Arc::new(OpenRouterProvider::new(
                    key,
                    &cfg.openrouter_model,
                )?)
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
                    ProviderError::Config("custom provider requires providers.custom.base_url".into())
                })?;
                let model = cfg
                    .custom_model
                    .as_deref()
                    .unwrap_or("")
                    .to_string();
                Arc::new(CustomProvider::new(key, model, base_url)?)
            }
            other => {
                return Err(ProviderError::Config(format!("unknown provider '{other}'")));
            }
        };

        Ok(Self {
            active: Some(provider),
        })
    }
}
