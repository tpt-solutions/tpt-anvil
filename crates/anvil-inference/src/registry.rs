// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use anvil_config::AnvilConfig;
use anvil_core::{AnvilError, Result};

use crate::{backend::InferenceBackend, ollama::OllamaBackend};

pub struct BackendRegistry {
    pub active: Arc<dyn InferenceBackend>,
}

impl BackendRegistry {
    pub fn from_config(cfg: &AnvilConfig) -> Result<Self> {
        let backend: Arc<dyn InferenceBackend> = match cfg.inference.backend.as_str() {
            "ollama" => Arc::new(OllamaBackend::new(&cfg.inference.ollama_url)),

            #[cfg(feature = "llama-cpp")]
            "llama_cpp" => {
                use crate::llama_cpp::LlamaCppBackend;
                let path = cfg
                    .inference
                    .model_path
                    .as_deref()
                    .ok_or_else(|| AnvilError::Config("llama_cpp backend requires inference.model_path".into()))?;
                Arc::new(LlamaCppBackend::new(path, cfg.inference.gpu_layers)?)
            }

            #[cfg(feature = "candle")]
            "candle" => {
                use crate::candle::CandleBackend;
                let path = cfg
                    .inference
                    .model_path
                    .as_deref()
                    .ok_or_else(|| AnvilError::Config("candle backend requires inference.model_path".into()))?;
                Arc::new(CandleBackend::new(path)?)
            }

            other => {
                return Err(AnvilError::UnsupportedBackend(format!(
                    "unknown backend '{other}'. Available: ollama, llama_cpp, candle"
                )));
            }
        };

        Ok(Self { active: backend })
    }
}
