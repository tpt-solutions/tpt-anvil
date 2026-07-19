// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Pure-Rust candle inference backend.
//! Enable with feature flag: --features candle

use anvil_core::{
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    AnvilError, Result,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::accel::{select_device, AccelDevice, AccelPreference};
use crate::backend::InferenceBackend;

pub struct CandleBackend {
    model_path: String,
    device: AccelDevice,
}

impl CandleBackend {
    pub fn new(model_path: &str) -> Result<Self> {
        if !std::path::Path::new(model_path).exists() {
            return Err(AnvilError::ModelNotFound(model_path.to_string()));
        }
        // candle supports CUDA and WebGPU (via wgpu/metal); pick the best available.
        let device = select_device(AccelPreference::Auto);
        tracing::info!("candle backend initialized on {}", device.label());
        Ok(Self {
            model_path: model_path.to_string(),
            device,
        })
    }

    /// The compute device selected for this backend.
    pub fn device(&self) -> AccelDevice {
        self.device
    }
}

#[async_trait]
impl InferenceBackend for CandleBackend {
    fn name(&self) -> &str {
        "candle"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: self.model_path.clone(),
            name: std::path::Path::new(&self.model_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            context_length: 8192,
            backend: BackendKind::Candle,
        }])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let _ = request;
        // TODO: candle GGUF loading + inference loop
        Err(AnvilError::Inference(format!(
            "candle backend not yet fully integrated (device: {})",
            self.device.label()
        )))
    }

    async fn stream(
        &self,
        request: &CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let _ = (request, tx);
        // TODO: token-by-token streaming from candle sampling loop
        Err(AnvilError::Inference(
            "candle streaming not yet implemented".into(),
        ))
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        Ok((text.len() as u32).saturating_div(4))
    }
}
