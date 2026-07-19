# Configuration Reference

Config files are TOML. Two locations are checked (project overrides user):

1. `~/.config/anvil/config.toml` — user-level
2. `<project>/.anvil/config.toml` — project-level

## `[inference]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `backend` | string | `"ollama"` | Inference backend: `ollama`, `llama_cpp`, `candle` |
| `model` | string | `"deepseek-coder:6.7b"` | Model name / path |
| `ollama_url` | string | `"http://localhost:11434"` | Ollama server URL |
| `model_path` | string | — | Path to GGUF file (llama_cpp / candle backends) |
| `context_length` | int | `8192` | Context window size in tokens |
| `max_tokens` | int | `2048` | Max tokens to generate |
| `temperature` | float | `0.2` | Sampling temperature (0.0–1.0) |
| `gpu_layers` | int | `-1` | GPU layers to offload (`0` = force CPU, `-1`/other = auto GPU) |

### Hardware acceleration

Local backends (`llama_cpp`, `candle`) can offload work to a GPU. Acceleration
is opt-in at build time via Cargo feature flags on the `anvil-inference` crate:

| Feature | Backend(s) | Hardware |
|---------|------------|----------|
| `cuda` | llama.cpp, candle | NVIDIA GPUs (CUDA) |
| `rocm` | llama.cpp | AMD GPUs (ROCm/HIP) |
| `webgpu` | candle | Cross-vendor GPU (WebGPU/Metal) |

Example:

```sh
cargo build --release -p anvil-daemon --features "anvil-inference/candle,anvil-inference/cuda"
```

When no acceleration feature is compiled in, or `gpu_layers = 0`, backends run
on CPU. The selected device is reported in the daemon logs at startup.

## `[providers]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `active` | string | `""` | Active cloud provider (empty = local only) |

### `[providers.openai]`
| `model` | string | `""` | Model ID (e.g. `gpt-4o`) |
| `api_key_entry` | string | `"openai_api_key"` | OS keychain entry name |

### `[providers.anthropic]`
| `model` | string | `"claude-sonnet-5"` | Model ID |
| `api_key_entry` | string | `"anthropic_api_key"` | OS keychain entry name |

### `[providers.openrouter]`
| `model` | string | `"deepseek/deepseek-coder"` | Model ID |
| `api_key_entry` | string | `"openrouter_api_key"` | OS keychain entry name |

### `[providers.azure]`
| `endpoint` | string | — | Full Azure deployment URL |
| `api_version` | string | `"2024-02-01"` | API version |
| `api_key_entry` | string | `"azure_api_key"` | OS keychain entry name |

### `[providers.custom]`
| `base_url` | string | — | Base URL for OpenAI-compatible endpoint |
| `model` | string | — | Model name |
| `api_key_entry` | string | `"custom_api_key"` | OS keychain entry name |

## `[indexing]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_file_size` | int | `1048576` | Max file size to index (bytes) |
| `exclude_patterns` | array | `["*.lock", "node_modules/**", ...]` | Glob patterns to skip |
| `top_k` | int | `10` | Number of context chunks to retrieve |
| `embedding_model` | string | `"nomic-embed-code"` | Embedding model for vector search |

### Retrieval

Anvil uses **hybrid retrieval**: BM25 lexical ranking (SQLite FTS5) and vector
cosine similarity are combined with Reciprocal Rank Fusion (RRF), then blended
with exact symbol matches. Embeddings are generated locally. By default a
dependency-free feature-hashing embedder is used so vector search works fully
offline; set `embedding_model` to a neural model served by Ollama (e.g.
`nomic-embed-text`) for higher-quality semantic search.

## `[ui]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"system"` | Color theme: `system`, `light`, `dark` |
| `font_size` | int | `14` | Chat panel font size |
