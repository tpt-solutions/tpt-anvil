// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

pub mod backend;
pub mod ollama;
pub mod prompt;
pub mod registry;

#[cfg(feature = "llama-cpp")]
pub mod llama_cpp;

#[cfg(feature = "candle")]
pub mod candle;

pub use backend::InferenceBackend;
pub use registry::BackendRegistry;
