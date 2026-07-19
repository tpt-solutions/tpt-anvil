// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::types::{BackendKind, TokenUsage};

/// Cost in USD per million tokens.
#[derive(Debug, Clone, Copy)]
pub struct PricingTier {
    pub input_per_million: f64,
    pub output_per_million: f64,
}

impl PricingTier {
    const fn new(input: f64, output: f64) -> Self {
        Self {
            input_per_million: input,
            output_per_million: output,
        }
    }
}

/// Returns pricing for a given provider and model ID, or None for local backends.
pub fn pricing_for(backend: &BackendKind, model_id: &str) -> Option<PricingTier> {
    match backend {
        BackendKind::OpenAi => Some(openai_pricing(model_id)),
        BackendKind::AzureOpenAi => Some(openai_pricing(model_id)),
        BackendKind::Anthropic => Some(anthropic_pricing(model_id)),
        BackendKind::OpenRouter => Some(openrouter_pricing(model_id)),
        BackendKind::OpenAiCompatible => None,
        // Local backends have no API cost.
        BackendKind::LlamaCpp | BackendKind::Candle | BackendKind::Ollama => None,
    }
}

/// Estimate cost in USD for a completed request.
pub fn estimate_cost(backend: &BackendKind, model_id: &str, usage: &TokenUsage) -> Option<f64> {
    let pricing = pricing_for(backend, model_id)?;
    let input_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * pricing.input_per_million;
    let output_cost = (usage.completion_tokens as f64 / 1_000_000.0) * pricing.output_per_million;
    Some(input_cost + output_cost)
}

fn openai_pricing(model: &str) -> PricingTier {
    if model.contains("gpt-4o-mini") {
        PricingTier::new(0.15, 0.60)
    } else if model.contains("gpt-4o") {
        PricingTier::new(2.50, 10.00)
    } else if model.contains("gpt-4-turbo") {
        PricingTier::new(10.00, 30.00)
    } else if model.contains("gpt-3.5") {
        PricingTier::new(0.50, 1.50)
    } else {
        PricingTier::new(2.50, 10.00)
    }
}

fn anthropic_pricing(model: &str) -> PricingTier {
    if model.contains("haiku") {
        PricingTier::new(0.80, 4.00)
    } else if model.contains("sonnet") {
        PricingTier::new(3.00, 15.00)
    } else if model.contains("opus") {
        PricingTier::new(15.00, 75.00)
    } else {
        PricingTier::new(3.00, 15.00)
    }
}

fn openrouter_pricing(model: &str) -> PricingTier {
    // Representative pricing for common OpenRouter models.
    if model.contains("deepseek") {
        PricingTier::new(0.14, 0.28)
    } else if model.contains("llama-3") && model.contains("70b") {
        PricingTier::new(0.59, 0.79)
    } else if model.contains("mistral") {
        PricingTier::new(0.20, 0.60)
    } else {
        PricingTier::new(1.00, 2.00)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpt4o_mini_cost() {
        let usage = TokenUsage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };
        let cost = estimate_cost(&BackendKind::OpenAi, "gpt-4o-mini", &usage).unwrap();
        // 1000/1M * 0.15 + 500/1M * 0.60 = 0.00015 + 0.0003 = 0.00045
        assert!((cost - 0.00045).abs() < 1e-9);
    }

    #[test]
    fn local_backend_has_no_cost() {
        let usage = TokenUsage {
            prompt_tokens: 10000,
            completion_tokens: 2000,
            total_tokens: 12000,
        };
        assert!(estimate_cost(&BackendKind::Ollama, "deepseek-coder:6.7b", &usage).is_none());
    }
}
