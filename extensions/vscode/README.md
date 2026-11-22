# TPT Anvil — VS Code Extension

Local-first AI coding assistant. TPT Anvil runs inference on your machine (via
Ollama, llama.cpp, or candle) with optional cloud provider fallback, so your
code stays private by default.

## Features

- **Chat sidebar** with slash-command autocomplete and streaming responses
- **Slash commands**: `/generate`, `/test`, `/explain`, `/fix`, `/docs`
- **Context menu actions**: right-click a selection to Explain, Fix, Test, or Docs
- **Diff viewer + one-click apply** for suggested changes
- **Status bar indicator** showing the active model, backend, and connection state
- **Configurable backends and cloud providers** (OpenAI, Anthropic, OpenRouter,
  Azure OpenAI, and any OpenAI-compatible endpoint)

## Requirements

The Anvil daemon must be installed and running. See the
[Getting Started guide](https://github.com/tpt-solutions/tpt-anvil/blob/master/docs/getting-started.md).

## Extension Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `anvil.backend` | Local inference backend (`ollama`, `llama_cpp`, `candle`) | `ollama` |
| `anvil.model` | Model to use for completions | `deepseek-coder:6.7b` |
| `anvil.ollamaUrl` | Ollama server URL | `http://localhost:11434` |
| `anvil.cloudProvider` | Cloud provider for fallback: `openai`, `anthropic`, `openrouter`, `azure`, or `custom` | `""` (local only) |
| `anvil.maxTokens` | Maximum tokens to generate | `2048` |
| `anvil.temperature` | Sampling temperature (0.0–1.0) | `0.2` |

## License

Dual licensed under MIT OR Apache-2.0. Copyright TPT Solutions.
