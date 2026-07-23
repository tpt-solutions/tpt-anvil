// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! llama.cpp backend via the llama-cpp-2 crate.
//! Enable with feature flag: --features llama-cpp

use std::sync::Arc;

use anvil_core::{
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    AnvilError, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task;

use crate::accel::{select_device, AccelDevice, AccelPreference};
use crate::backend::InferenceBackend;
use crate::prompt::apply_chat_template;

pub struct LlamaCppBackend {
    model_path: String,
    gpu_layers: i32,
    device: AccelDevice,
    model: llama_cpp_2::LLamaModel,
}

impl LlamaCppBackend {
    pub fn new(model_path: &str, gpu_layers: i32) -> Result<Self> {
        if !std::path::Path::new(model_path).exists() {
            return Err(AnvilError::ModelNotFound(model_path.to_string()));
        }
        let device = select_device(AccelPreference::from_gpu_layers(gpu_layers));

        tracing::info!(
            "loading GGUF model from {} on {} ({} gpu layers)",
            model_path,
            device.label(),
            gpu_layers
        );

        let mut params = llama_cpp_2::LLamaModelParams::default();
        if gpu_layers >= 0 {
            params = params.with_n_gpu_layers(gpu_layers as u32);
        }

        let model = llama_cpp_2::LLamaModel::load_from_file(std::path::Path::new(model_path), params)
            .map_err(|e| AnvilError::Inference(format!("failed to load GGUF model: {e}")))?;

        tracing::info!("GGUF model loaded successfully from {}", model_path);

        Ok(Self {
            model_path: model_path.to_string(),
            gpu_layers,
            device,
            model,
        })
    }

    /// The compute device selected for this backend.
    pub fn device(&self) -> AccelDevice {
        self.device
    }

    fn context_length(&self) -> u32 {
        // Query model metadata for context length, default to 8192
        self.model
            .metadata()
            .get_u32("llama.context_length")
            .unwrap_or(8192)
    }
}

#[async_trait]
impl InferenceBackend for LlamaCppBackend {
    fn name(&self) -> &str {
        "llama_cpp"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: self.model_path.clone(),
            name: std::path::Path::new(&self.model_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            context_length: self.context_length(),
            backend: BackendKind::LlamaCpp,
        }])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let prompt = apply_chat_template(request);
        let model = self.model.clone();
        let max_tokens = request.max_tokens;
        let temperature = request.temperature;

        let result = task::spawn_blocking(move || -> Result<CompletionResponse> {
            let ctx_params = llama_cpp_2::LLamaContextParams::default()
                .with_n_ctx(request.max_tokens + 4096);

            let mut ctx = model.create_context(ctx_params)
                .map_err(|e| AnvilError::Inference(format!("failed to create context: {e}")))?;

            let tokens = model.tokenize(&prompt, true, true)
                .map_err(|e| AnvilError::Inference(format!("tokenization failed: {e}")))?;

            ctx.clear_kv_cache();

            let _ = ctx.input_tokenize(&tokens, true);

            let mut sampler = llama_cpp_2::LLamaSampler::new()
                .with_temp(temperature)
                .with_top_k(40)
                .with_top_p(0.95);

            let mut response = String::new();
            let mut n_past = tokens.len() as i32;

            for _ in 0..max_tokens {
                let new_token = ctx.sample(&sampler, n_past)
                    .map_err(|e| AnvilError::Inference(format!("sampling failed: {e}")))?;

                let token_text = model.token_to_str(new_token)
                    .unwrap_or_default()
                    .to_string();

                if token_text == "</s>" || token_text.trim() == "<|end_of_text|>" {
                    break;
                }

                response.push_str(&token_text);

                let _ = ctx.input_tokenize(&[new_token], false);
                n_past += 1;
            }

            Ok(CompletionResponse {
                content: response,
                model: "local".to_string(),
                usage: None,
            })
        })
        .await
        .map_err(|e| AnvilError::Inference(format!("task join error: {e}")))?;

        result
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let prompt = apply_chat_template(request);
        let model = self.model.clone();
        let max_tokens = request.max_tokens;
        let temperature = request.temperature;

        let result = task::spawn_blocking(move || -> Result<()> {
            let ctx_params = llama_cpp_2::LLamaContextParams::default()
                .with_n_ctx(request.max_tokens + 4096);

            let mut ctx = model.create_context(ctx_params)
                .map_err(|e| AnvilError::Inference(format!("failed to create context: {e}")))?;

            let tokens = model.tokenize(&prompt, true, true)
                .map_err(|e| AnvilError::Inference(format!("tokenization failed: {e}")))?;

            ctx.clear_kv_cache();
            let _ = ctx.input_tokenize(&tokens, true);

            let mut sampler = llama_cpp_2::LLamaSampler::new()
                .with_temp(temperature)
                .with_top_k(40)
                .with_top_p(0.95);

            let mut n_past = tokens.len() as i32;

            for _ in 0..max_tokens {
                let new_token = ctx.sample(&sampler, n_past)
                    .map_err(|e| AnvilError::Inference(format!("sampling failed: {e}")))?;

                let token_text = model.token_to_str(new_token)
                    .unwrap_or_default()
                    .to_string();

                if token_text == "</s>" || token_text.trim() == "<|end_of_text|>" {
                    let _ = tx.blocking_send(StreamChunk {
                        delta: String::new(),
                        done: true,
                    });
                    break;
                }

                let done = false;
                let _ = tx.blocking_send(StreamChunk {
                    delta: token_text,
                    done,
                });

                let _ = ctx.input_tokenize(&[new_token], false);
                n_past += 1;
            }

            Ok(())
        })
        .await
        .map_err(|e| AnvilError::Inference(format!("task join error: {e}")))?;

        result
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        let model = self.model.clone();
        let text = text.to_string();

        task::spawn_blocking(move || {
            let tokens = model.tokenize(&text, false, false)
                .map_err(|e| AnvilError::Inference(format!("tokenization failed: {e}")))?;
            Ok(tokens.len() as u32)
        })
        .await
        .map_err(|e| AnvilError::Inference(format!("task join error: {e}")))?
    }
}
