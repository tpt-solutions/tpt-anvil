# TPT Anvil

AI-powered code assistant with a local-first, privacy-respecting architecture.

## Features

- **Chat Panel** — Interactive sidebar for conversational code assistance
- **Slash Commands** — `/generate`, `/explain`, `/fix`, `/test`, `/docs` for quick actions
- **Inline Suggestions** — Ghost-text completions as you type
- **Diff Preview** — Review all proposed changes before accepting them
- **Multi-Backend** — Run locally via Ollama, llama.cpp, or candle, or connect to OpenAI, Anthropic, OpenRouter, Azure, or any OpenAI-compatible endpoint
- **Code Indexing** — Hybrid BM25 + vector search over your project for accurate context
- **Smart Context** — Automatic file chunking and context window management
- **Vault** — Detects and redacts secrets before they leave your machine

## Privacy

All processing can run entirely on your machine. No code is sent to external
services unless you explicitly configure a cloud provider. API keys are stored
in your OS keychain, never in plain text.

## Getting Started

1. Install the extension
2. Ensure [Ollama](https://ollama.com) is running with a compatible model (e.g. `deepseek-coder:6.7b`)
3. Open the Anvil chat panel from the Activity Bar
4. Start coding

Configure the extension via `anvil.*` settings or by placing a `.anvil/config.toml` in your project root. See the [Configuration Reference](../../docs/config-reference.md) for all available options.

[Screenshot: Chat Panel]

[Screenshot: Diff Preview]

[Screenshot: Slash Commands]
