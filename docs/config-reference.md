# Configuration Reference

Config files are TOML. Two locations are checked (project overrides user):

1. `~/.config/anvil/config.toml` — user-level
2. `<project>/.anvil/config.toml` — project-level

> **Security note:** Project-level config overrides user-level config.
> A cloned untrusted repository could include `.anvil/config.toml` with
> settings that redirect API keys or disable safety features (vault,
> verification). User-level config is the safe default for secrets and
> provider settings; project-level config should only contain
> local-inference and indexing settings you trust.

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

> Cloud provider `model` fields default to an empty string rather than a
> specific model id — model names change too often for a hardcoded default to
> stay current. Leaving one unset raises a clear config error naming which
> key to set (with a link to that provider's current model list) instead of
> silently picking a possibly-stale model.

### `[providers.openai]`
| `model` | string | `""` | Model ID (e.g. `gpt-4o`) — required if `active = "openai"` |
| `api_key_entry` | string | `"openai_api_key"` | OS keychain entry name |

### `[providers.anthropic]`
| `model` | string | `""` | Model ID (e.g. `claude-sonnet-5`) — required if `active = "anthropic"` |
| `api_key_entry` | string | `"anthropic_api_key"` | OS keychain entry name |

### `[providers.openrouter]`
| `model` | string | `""` | Model ID (e.g. `deepseek/deepseek-coder`) |
| `api_key_entry` | string | `"openrouter_api_key"` | OS keychain entry name |

### `[providers.azure]`
| `endpoint` | string | — | Full Azure deployment URL |
| `api_version` | string | `"2024-02-01"` | API version |
| `api_key_entry` | string | `"azure_api_key"` | OS keychain entry name |

### `[providers.custom]`
| `base_url` | string | — | Base URL for OpenAI-compatible endpoint |
| `model` | string | — | Model name |
| `api_key_entry` | string | `"custom_api_key"` | OS keychain entry name |

> **Trust boundary:** The `providers.custom.base_url` value is fully
> user-controlled and may point at an internal network service (e.g.
> `http://10.0.0.5/v1`). Any API key resolved from `api_key_entry` will be
> sent to that endpoint over the network. Project-level `.anvil/config.toml`
> can set this value — a cloned untrusted repository could redirect
> API-key-bearing requests to an attacker-controlled server. Prefer
> user-level config for cloud provider settings; use project-level config
> only for local-inference settings you trust.

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

## `[vault]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable the secrets vault (intercepts API keys and tokens in prompts) |
| `redact_local` | bool | `false` | Redact secrets even when running on a local backend |
| `custom_patterns` | array | `[]` | Additional patterns to detect and redact |

Each entry in `custom_patterns` has the following shape:

| Key | Type | Description |
|-----|------|-------------|
| `name` | string | Human-readable name for the pattern |
| `pattern` | string | Regular expression to match |
| `replacement` | string | Replacement string (e.g. `"[REDACTED]"`) |

## `[smart_context]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable smart context chunking for large files |
| `file_size_threshold_bytes` | usize | `2048` | Files larger than this are chunked before embedding |
| `chunk_size_threshold_bytes` | usize | `1024` | Maximum chunk size in bytes when splitting files |

## `[router]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `false` | Enable automatic routing between local and cloud providers |
| `prefer_cheapest` | bool | `true` | Route to the cheapest provider that meets quality requirements |
| `max_cost_per_request_usd` | f64 | — | Optional cap on cost per request in USD |
| `pinned` | string | — | Optional provider name to pin all requests to |

## `[verify]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `true` | Enable automatic verification after code generation |
| `run_tests` | bool | `false` | Run the project test suite after applying changes |
| `run_linter` | bool | `true` | Run the project linter after applying changes |
| `timeout_seconds` | u64 | `60` | Timeout for verification commands |
| `max_retries` | u32 | `1` | Number of times to retry verification on failure |

## `[ui]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"system"` | Color theme: `system`, `light`, `dark` |
| `font_size` | int | `14` | Chat panel font size |
