# tpt-anvil-inference

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-inference.svg)](https://crates.io/crates/tpt-anvil-inference)
[![Docs.rs](https://docs.rs/tpt-anvil-inference/badge.svg)](https://docs.rs/tpt-anvil-inference)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Local LLM inference backends for [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil), behind a
single `InferenceBackend` trait. Nothing here talks to a cloud API — that is
[`tpt-anvil-providers`](../tpt-anvil-providers)' job.

## Features

- `InferenceBackend` trait: `name`, `list_models`, `complete`, `stream`, `count_tokens`.
- `OllamaBackend` — streams against a running Ollama server over HTTP. Default feature.
- `LlamaCppBackend` — in-process GGUF inference via `llama-cpp-2`, opt-in.
- `CandleBackend` — in-process GGUF inference via `candle`, opt-in.
- `BackendRegistry` — builds the configured backends from `AnvilConfig`.
- `prompt::format_prompt`, `PromptTemplate`, and `apply_chat_template` for model-specific chat
  formatting.
- `accel` module: `AccelDevice`, `AccelPreference`, `select_device` for GPU/CPU selection.

## Feature flags

| Feature | Default | Effect |
| --- | --- | --- |
| `ollama` | yes | Enables the Ollama HTTP backend. |
| `llama-cpp` | no | Enables in-process `llama.cpp` inference. |
| `candle` | no | Enables in-process `candle` inference. |
| `cuda` | no | CUDA support in `candle` and `llama-cpp-2`. |
| `rocm` | no | ROCm support in `llama-cpp-2`. |
| `webgpu` | no | Metal support in `candle`. |

The acceleration flags only forward features to the underlying crates; they are surfaced to
callers through `accel::select_device`.

## Usage

```rust
use tpt_anvil_config::loader::ConfigLoader;
use tpt_anvil_core::types::{ChatMessage, CompletionRequest, Role};
use tpt_anvil_inference::registry::BackendRegistry;

let config = ConfigLoader::load(None)?;

// The registry resolves the configured backend and hands back the active one.
let registry = BackendRegistry::from_config(&config)?;
let backend = registry.active.clone();

for model in backend.list_models().await? {
    println!("{} ({:?}, {} ctx)", model.id, model.backend, model.context_length);
}

let response = backend
    .complete(&CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "Explain this function".to_string(),
        }],
        model: None,
        max_tokens: 512,
        temperature: 0.2,
        stream: false,
    })
    .await?;

println!("{}", response.content);
# Ok::<(), anyhow::Error>(())
```

Picking a device:

```rust
use tpt_anvil_inference::accel::{select_device, AccelPreference};

let device = select_device(AccelPreference::from_gpu_layers(32));
println!("{} (gpu: {})", device.label(), device.is_gpu());
```

## Dependencies

`tpt-anvil-core`, `tpt-anvil-config`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tokio`,
`async-trait`, `tracing`, `futures-util`, and the optional backend crates above.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.