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
| `gpu_layers` | int | `-1` | GPU layers to offload (-1 = all) |

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

## `[ui]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"system"` | Color theme: `system`, `light`, `dark` |
| `font_size` | int | `14` | Chat panel font size |
