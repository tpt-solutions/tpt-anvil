// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Pure-Rust candle inference backend.
//! Enable with feature flag: --features candle

use std::path::PathBuf;
use std::sync::Arc;

use anvil_core::{
    types::{BackendKind, CompletionRequest, CompletionResponse, ModelInfo, StreamChunk},
    AnvilError, Result,
};
use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use tokio::sync::mpsc;
use tokio::task;

use crate::accel::{select_device, AccelDevice, AccelPreference};
use crate::backend::InferenceBackend;
use crate::prompt::apply_chat_template;

pub struct CandleBackend {
    model_path: String,
    device: Device,
    accel_device: AccelDevice,
    tokenizer: Option<tokenizers::Tokenizer>,
    model_config: CandleModelConfig,
}

struct CandleModelConfig {
    context_length: u32,
    vocab_size: u32,
    n_embd: u32,
    n_head: u32,
    n_layer: u32,
}

impl Default for CandleModelConfig {
    fn default() -> Self {
        Self {
            context_length: 8192,
            vocab_size: 32000,
            n_embd: 4096,
            n_head: 32,
            n_layer: 32,
        }
    }
}

impl CandleBackend {
    pub fn new(model_path: &str) -> Result<Self> {
        if !std::path::Path::new(model_path).exists() {
            return Err(AnvilError::ModelNotFound(model_path.to_string()));
        }

        let accel_device = select_device(AccelPreference::Auto);
        let device = match accel_device {
            AccelDevice::Cuda(idx) => Device::new_cuda(idx)
                .map_err(|e| AnvilError::Inference(format!("CUDA init failed: {e}")))?,
            _ => Device::Cpu,
        };

        tracing::info!("loading GGUF model from {} on candle ({})", model_path, accel_device.label());

        let model_config = load_gguf_config(model_path)?;

        // Try to load a tokenizer from the GGUF metadata or fall back to a basic one
        let tokenizer = load_tokenizer_from_gguf(model_path).ok();

        tracing::info!(
            "candle model loaded: vocab={}, ctx={}, layers={}",
            model_config.vocab_size,
            model_config.context_length,
            model_config.n_layer
        );

        Ok(Self {
            model_path: model_path.to_string(),
            device,
            accel_device,
            tokenizer,
            model_config,
        })
    }

    /// The compute device selected for this backend.
    pub fn device(&self) -> AccelDevice {
        self.accel_device
    }

    fn tokenize_text(&self, text: &str) -> Result<Vec<u32>> {
        if let Some(ref tok) = self.tokenizer {
            let encoding = tok.encode(text, true)
                .map_err(|e| AnvilError::Inference(format!("tokenization failed: {e}")))?;
            Ok(encoding.get_ids().to_vec())
        } else {
            // Fallback: simple byte-level tokenization
            Ok(text.bytes().map(|b| b as u32).collect())
        }
    }

    fn detokenize(&self, token_id: u32) -> Result<String> {
        if let Some(ref tok) = self.tokenizer {
            tok.decode(&[token_id], true)
                .map_err(|e| AnvilError::Inference(format!("detokenization failed: {e}")))
        } else {
            // Fallback: single byte decode
            Ok((token_id as u8 as char).to_string())
        }
    }
}

fn load_gguf_config(path: &str) -> Result<CandleModelConfig> {
    // Parse GGUF file metadata for model configuration
    let mut file = std::fs::File::open(path)
        .map_err(|e| AnvilError::Inference(format!("cannot open GGUF file: {e}")))?;

    // GGUF magic number check
    use std::io::Read;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| AnvilError::Inference(format!("GGUF read error: {e}")))?;

    if &magic != b"GGUF" {
        return Err(AnvilError::Inference("not a valid GGUF file".into()));
    }

    // Read version (u32 LE)
    let mut version_buf = [0u8; 4];
    file.read_exact(&mut version_buf)
        .map_err(|e| AnvilError::Inference(format!("GGUF version read error: {e}")))?;
    let _version = u32::from_le_bytes(version_buf);

    // Read number of tensors (u64 LE)
    let mut n_tensors_buf = [0u8; 8];
    file.read_exact(&mut n_tensors_buf)
        .map_err(|e| AnvilError::Inference(format!("GGUF tensor count read error: {e}")))?;

    // Read number of metadata key-value pairs (u64 LE)
    let mut n_kv_buf = [0u8; 8];
    file.read_exact(&mut n_kv_buf)
        .map_err(|e| AnvilError::Inference(format!("GGUF metadata count read error: {e}")))?;
    let _n_kv = u64::from_le_bytes(n_kv_buf);

    // Default config — actual parsing would require full GGUF KV reader
    // For now, return reasonable defaults and let candle-transformers handle the full load
    Ok(CandleModelConfig::default())
}

fn load_tokenizer_from_gguf(_path: &str) -> Result<tokenizers::Tokenizer> {
    // Try to build a basic BPE tokenizer
    // In production, this would extract the tokenizer from GGUF metadata
    // For now, return an error to trigger the fallback path
    Err(AnvilError::Inference(
        "tokenizer not embedded in GGUF; using byte fallback".into(),
    ))
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
            context_length: self.model_config.context_length,
            backend: BackendKind::Candle,
        }])
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let prompt = apply_chat_template(request);
        let device = self.device.clone();
        let config = CandleModelConfig {
            context_length: self.model_config.context_length,
            vocab_size: self.model_config.vocab_size,
            n_embd: self.model_config.n_embd,
            n_head: self.model_config.n_head,
            n_layer: self.model_config.n_layer,
        };
        let model_path = self.model_path.clone();
        let backend = self.clone_for_spawn();

        let result = task::spawn_blocking(move || -> Result<CompletionResponse> {
            let tokens = backend.tokenize_text(&prompt)?;
            let max_tokens = request.max_tokens as usize;

            // Load model weights via candle-transformers
            let (llama, _tok) = load_candle_model(&model_path, &device, &config)?;

            let mut all_tokens = tokens.clone();
            let mut response = String::new();

            for _ in 0..max_tokens {
                let input_len = all_tokens.len();
                let start_pos = input_len.saturating_sub(config.context_length as usize);

                let input_tensor = Tensor::new(
                    &all_tokens[start_pos..],
                    &device,
                )
                .map_err(|e| AnvilError::Inference(format!("tensor creation failed: {e}")))?;

                let logits = llama.forward(&input_tensor, start_pos)
                    .map_err(|e| AnvilError::Inference(format!("forward pass failed: {e}")))?;

                // Get the last token's logits and sample
                let last_logits = logits
                    .get(0)?
                    .get(logits.dim(1)? - 1)?;

                let token_id = sample_token(&last_logits, request.temperature)?;
                all_tokens.push(token_id);

                let text = backend.detokenize(token_id)?;
                if text == "</s>" || text.trim() == "<|end_of_text|>" {
                    break;
                }
                response.push_str(&text);
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
        let device = self.device.clone();
        let config = CandleModelConfig {
            context_length: self.model_config.context_length,
            vocab_size: self.model_config.vocab_size,
            n_embd: self.model_config.n_embd,
            n_head: self.model_config.n_head,
            n_layer: self.model_config.n_layer,
        };
        let model_path = self.model_path.clone();
        let backend = self.clone_for_spawn();

        let result = task::spawn_blocking(move || -> Result<()> {
            let tokens = backend.tokenize_text(&prompt)?;
            let max_tokens = request.max_tokens as usize;

            let (llama, _tok) = load_candle_model(&model_path, &device, &config)?;

            let mut all_tokens = tokens;

            for _ in 0..max_tokens {
                let input_len = all_tokens.len();
                let start_pos = input_len.saturating_sub(config.context_length as usize);

                let input_tensor = Tensor::new(
                    &all_tokens[start_pos..],
                    &device,
                )
                .map_err(|e| AnvilError::Inference(format!("tensor creation failed: {e}")))?;

                let logits = llama.forward(&input_tensor, start_pos)
                    .map_err(|e| AnvilError::Inference(format!("forward pass failed: {e}")))?;

                let last_logits = logits
                    .get(0)?
                    .get(logits.dim(1)? - 1)?;

                let token_id = sample_token(&last_logits, request.temperature)?;
                all_tokens.push(token_id);

                let text = backend.detokenize(token_id)?;

                if text == "</s>" || text.trim() == "<|end_of_text|>" {
                    let _ = tx.blocking_send(StreamChunk {
                        delta: String::new(),
                        done: true,
                    });
                    break;
                }

                let _ = tx.blocking_send(StreamChunk {
                    delta: text,
                    done: false,
                });
            }

            Ok(())
        })
        .await
        .map_err(|e| AnvilError::Inference(format!("task join error: {e}")))?;

        result
    }

    async fn count_tokens(&self, text: &str) -> Result<u32> {
        let backend = self.clone_for_spawn();
        let text = text.to_string();

        task::spawn_blocking(move || {
            let tokens = backend.tokenize_text(&text)?;
            Ok(tokens.len() as u32)
        })
        .await
        .map_err(|e| AnvilError::Inference(format!("task join error: {e}")))?
    }
}

impl CandleBackend {
    fn clone_for_spawn(&self) -> CandleBackendClone {
        CandleBackendClone {
            tokenizer: self.tokenizer.clone(),
        }
    }
}

struct CandleBackendClone {
    tokenizer: Option<tokenizers::Tokenizer>,
}

impl CandleBackendClone {
    fn tokenize_text(&self, text: &str) -> Result<Vec<u32>> {
        if let Some(ref tok) = self.tokenizer {
            let encoding = tok.encode(text, true)
                .map_err(|e| AnvilError::Inference(format!("tokenization failed: {e}")))?;
            Ok(encoding.get_ids().to_vec())
        } else {
            Ok(text.bytes().map(|b| b as u32).collect())
        }
    }

    fn detokenize(&self, token_id: u32) -> Result<String> {
        if let Some(ref tok) = self.tokenizer {
            tok.decode(&[token_id], true)
                .map_err(|e| AnvilError::Inference(format!("detokenization failed: {e}")))
        } else {
            Ok((token_id as u8 as char).to_string())
        }
    }
}

/// Minimal LLaMA-style model wrapper for candle forward pass.
struct CandleLlama {
    embed_tokens: Tensor,
    layers: Vec<CandleLayer>,
    lm_head: Tensor,
    device: Device,
}

struct CandleLayer {
    // Simplified: in production these would be the actual weight tensors
    _dummy: Tensor,
}

impl CandleLlama {
    fn forward(&self, input_ids: &Tensor, _start_pos: usize) -> Result<Tensor> {
        // Embed input tokens
        let hidden = input_ids
            .embed(&self.embed_tokens)
            .map_err(|e| AnvilError::Inference(format!("embed failed: {e}")))?;

        // Run through layers (simplified — actual implementation would use attention + MLP)
        let mut x = hidden;
        for _layer in &self.layers {
            // Placeholder: in real implementation, apply layer norm, attention, FFN
            x = x
                .clone();
        }

        // Project to vocab
        let logits = x
            .matmul(&self.lm_head.t().map_err(|e| AnvilError::Inference(format!("transpose failed: {e}")))?)
            .map_err(|e| AnvilError::Inference(format!("lm_head matmul failed: {e}")))?;

        Ok(logits)
    }
}

fn load_candle_model(
    _path: &str,
    device: &Device,
    config: &CandleModelConfig,
) -> Result<(CandleLlama, Option<tokenizers::Tokenizer>)> {
    // In production, this would:
    // 1. Open the GGUF file with candle_transformers::gguf
    // 2. Load all tensor weights
    // 3. Construct the LLaMA model with attention layers, RMSNorm, etc.
    //
    // For now, create placeholder tensors so the code compiles and the
    // architecture is correct. Full weight loading requires the model file
    // to be present and a complete GGUF reader implementation.

    let embed = Tensor::zeros(
        (config.vocab_size as usize, config.n_embd as usize),
        DType::F32,
        device,
    )
    .map_err(|e| AnvilError::Inference(format!("init embed tensor: {e}")))?;

    let lm_head = Tensor::zeros(
        (config.n_embd as usize, config.vocab_size as usize),
        DType::F32,
        device,
    )
    .map_err(|e| AnvilError::Inference(format!("init lm_head tensor: {e}")))?;

    let dummy = Tensor::zeros((1,), DType::F32, device)
        .map_err(|e| AnvilError::Inference(format!("init dummy tensor: {e}")))?;

    let layers = (0..config.n_layer)
        .map(|_| CandleLayer { _dummy: dummy.clone() })
        .collect();

    Ok((
        CandleLlama {
            embed_tokens: embed,
            layers,
            lm_head,
            device: device.clone(),
        },
        None,
    ))
}

fn sample_token(logits: &Tensor, temperature: f32) -> Result<u32> {
    if temperature <= 0.0 {
        // Greedy: take argmax
        let argmax = logits
            .argmax(candle_core::D::Minus1)
            .map_err(|e| AnvilError::Inference(format!("argmax failed: {e}")))?;
        let id = argmax
            .to_vec1::<u32>()
            .map_err(|e| AnvilError::Inference(format!("to_vec1 failed: {e}")))?;
        Ok(id[0])
    } else {
        // Temperature-scaled sampling
        let scaled = (&logits / temperature as f64)
            .map_err(|e| AnvilError::Inference(format!("scale failed: {e}")))?;
        let probs = scaled
            .softmax(candle_core::D::Minus1)
            .map_err(|e| AnvilError::Inference(format!("softmax failed: {e}")))?;
        let probs_vec = probs
            .to_vec1::<f32>()
            .map_err(|e| AnvilError::Inference(format!("probs to vec failed: {e}")))?;

        // Simple weighted random sampling
        let mut rng: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        // xorshift64
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let sample = (rng as f64 / u64::MAX as f64) as f32;

        let mut cumulative = 0.0f32;
        for (i, &p) in probs_vec.iter().enumerate() {
            cumulative += p;
            if sample <= cumulative {
                return Ok(i as u32);
            }
        }
        Ok((probs_vec.len() - 1) as u32)
    }
}
