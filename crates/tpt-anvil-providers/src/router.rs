// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cost-based provider router — selects the cheapest provider for a request
//! when multiple providers are configured.

use std::sync::Arc;

use crate::cost::estimate_cost;
use crate::provider::CloudProvider;
use crate::types::{BackendKind, TokenUsage};
use tracing::info;

/// Configuration for the router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub enabled: bool,
    pub prefer_cheapest: bool,
    /// Maximum cost per request in USD. Requests exceeding this are rejected.
    pub max_cost_per_request_usd: Option<f64>,
    /// Optional provider name to pin all requests to, bypassing cost-based
    /// selection. When set, only the named provider is considered (if
    /// available); when absent the full pool is evaluated.
    pub pinned: Option<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prefer_cheapest: true,
            max_cost_per_request_usd: None,
            pinned: None,
        }
    }
}

/// A registered provider with its backend kind and model ID.
pub struct ProviderEntry {
    pub name: String,
    pub provider: Arc<dyn CloudProvider>,
    pub backend_kind: BackendKind,
    pub model_id: String,
}

/// Select the cheapest provider for a request based on estimated token usage.
///
/// Returns `None` if no providers are available or all exceed the cost cap.
pub fn select_provider<'a>(
    providers: &'a [ProviderEntry],
    estimated_prompt_tokens: u32,
    estimated_completion_tokens: u32,
    config: &RouterConfig,
) -> Option<&'a ProviderEntry> {
    if !config.enabled || providers.is_empty() {
        return providers.first();
    }

    // When a provider is pinned, restrict the candidate set to only that one.
    let effective: Vec<&ProviderEntry> = match &config.pinned {
        Some(name) => providers.iter().filter(|e| e.name == *name).collect(),
        None => providers.iter().collect(),
    };

    if effective.is_empty() {
        info!(
            "pinned provider '{}' not found in available pool; falling back",
            config.pinned.as_deref().unwrap_or("")
        );
        return providers.first();
    }

    let usage = TokenUsage {
        prompt_tokens: estimated_prompt_tokens,
        completion_tokens: estimated_completion_tokens,
        total_tokens: estimated_prompt_tokens + estimated_completion_tokens,
    };

    let mut candidates: Vec<(&ProviderEntry, f64)> = effective
        .into_iter()
        .filter_map(|entry| {
            let cost = estimate_cost(&entry.backend_kind, &entry.model_id, &usage)?;
            Some((entry, cost))
        })
        .collect();

    if candidates.is_empty() {
        // No cost data available (local/custom backends); return first of
        // the effective list, or first overall as a last resort.
        return config.pinned.as_ref().and_then(|_| providers.first());
    }

    // Filter by cost cap
    if let Some(max_cost) = config.max_cost_per_request_usd {
        candidates.retain(|(_, cost)| *cost <= max_cost);
        if candidates.is_empty() {
            info!("all providers exceed cost cap of ${max_cost:.4}");
            return None;
        }
    }

    if config.prefer_cheapest {
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let (entry, cost) = &candidates[0];
        info!("router selected '{}' (est. cost: ${:.6})", entry.name, cost);
        Some(entry)
    } else {
        candidates.first().map(|(e, _)| *e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ModelInfo;

    struct MockProvider {
        name: String,
    }

    #[async_trait::async_trait]
    impl CloudProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn default_model(&self) -> &str {
            "mock"
        }
        async fn list_models(&self) -> crate::types::Result<Vec<ModelInfo>> {
            Ok(vec![])
        }
        async fn complete(
            &self,
            _: &crate::types::CompletionRequest,
        ) -> crate::types::Result<crate::types::CompletionResponse> {
            Ok(crate::types::CompletionResponse {
                content: String::new(),
                model: "mock".into(),
                usage: None,
            })
        }
        async fn stream(
            &self,
            _: &crate::types::CompletionRequest,
            _: tokio::sync::mpsc::Sender<crate::types::StreamChunk>,
        ) -> crate::types::Result<()> {
            Ok(())
        }
        async fn count_tokens(&self, _: &str) -> crate::types::Result<u32> {
            Ok(0)
        }
    }

    #[test]
    fn select_cheapest_provider() {
        let providers = vec![
            ProviderEntry {
                name: "expensive".into(),
                provider: Arc::new(MockProvider {
                    name: "expensive".into(),
                }),
                backend_kind: BackendKind::OpenAi,
                model_id: "gpt-4o".into(),
            },
            ProviderEntry {
                name: "cheap".into(),
                provider: Arc::new(MockProvider {
                    name: "cheap".into(),
                }),
                backend_kind: BackendKind::OpenRouter,
                model_id: "deepseek/deepseek-coder".into(),
            },
        ];
        let config = RouterConfig {
            enabled: true,
            prefer_cheapest: true,
            max_cost_per_request_usd: None,
            pinned: None,
        };
        let selected = select_provider(&providers, 1000, 500, &config).unwrap();
        assert_eq!(selected.name, "cheap");
    }

    #[test]
    fn disabled_router_returns_first() {
        let providers = vec![ProviderEntry {
            name: "a".into(),
            provider: Arc::new(MockProvider { name: "a".into() }),
            backend_kind: BackendKind::OpenAi,
            model_id: "gpt-4o".into(),
        }];
        let config = RouterConfig {
            enabled: false,
            ..Default::default()
        };
        let selected = select_provider(&providers, 1000, 500, &config).unwrap();
        assert_eq!(selected.name, "a");
    }

    #[test]
    fn empty_providers_returns_none() {
        let config = RouterConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(select_provider(&[], 1000, 500, &config).is_none());
    }

    #[test]
    fn pinned_provider_is_selected() {
        let providers = vec![
            ProviderEntry {
                name: "cheap".into(),
                provider: Arc::new(MockProvider {
                    name: "cheap".into(),
                }),
                backend_kind: BackendKind::OpenRouter,
                model_id: "deepseek/deepseek-coder".into(),
            },
            ProviderEntry {
                name: "expensive".into(),
                provider: Arc::new(MockProvider {
                    name: "expensive".into(),
                }),
                backend_kind: BackendKind::OpenAi,
                model_id: "gpt-4o".into(),
            },
        ];
        let config = RouterConfig {
            enabled: true,
            prefer_cheapest: true,
            max_cost_per_request_usd: None,
            pinned: Some("expensive".into()),
        };
        let selected = select_provider(&providers, 1000, 500, &config).unwrap();
        assert_eq!(selected.name, "expensive");
    }

    #[test]
    fn pinned_to_unknown_falls_back() {
        let providers = vec![ProviderEntry {
            name: "only".into(),
            provider: Arc::new(MockProvider {
                name: "only".into(),
            }),
            backend_kind: BackendKind::OpenAi,
            model_id: "gpt-4o".into(),
        }];
        let config = RouterConfig {
            enabled: true,
            prefer_cheapest: true,
            max_cost_per_request_usd: None,
            pinned: Some("nonexistent".into()),
        };
        let selected = select_provider(&providers, 1000, 500, &config).unwrap();
        assert_eq!(selected.name, "only");
    }
}
