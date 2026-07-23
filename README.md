# TPT Anvil

**The open-source, privacy-first AI development environment.**

> Status: pre-alpha — active development

Anvil is a fully open-source alternative to GitHub Copilot and Cursor. Everything runs on your machine — your code never leaves.

## Features

- **Local LLM inference** — llama.cpp, candle (pure Rust), or Ollama; any GGUF model
- **GPU acceleration** — CUDA, ROCm, WebGPU
- **Codebase-aware context** — Tree-sitter AST + hybrid BM25/vector semantic search
- **Cloud fallback (BYOK)** — OpenAI, Anthropic, OpenRouter, Azure OpenAI, or any OpenAI-compatible endpoint; model lists are fetched live from each provider's API rather than hardcoded, so new releases show up without an Anvil update
- **VS Code extension** — chat panel, slash commands, diff-based edits
- **JetBrains plugin** — IntelliJ IDEA, PyCharm, GoLand, and all JetBrains IDEs
- **Privacy-first** — 100% local by default, zero telemetry

### In progress

Standalone building blocks that exist and are unit-tested, but aren't yet wired into the live request path (tracked in `todo.md` Phase 16):

- **Vault** — regex-based redaction of API keys, tokens, and other secrets from prompts before they reach a cloud provider
- **Smart Context** — AST-outline compression of large files (signatures only, no bodies) to cut token usage on oversized context
- **Router** — cost-based provider selection across multiple configured cloud providers, picking the cheapest for a given request
- **Verifier** — runs the real compiler/type-checker, linter, and (optionally) test suite on generated diffs before they're presented, with a fail-open warning rather than a silent bad patch
- **Recent models** — tracks the last 5 (provider, model) pairs actually used so IDE UIs can offer a quick-pick list

## Slash Commands

| Command | Description |
|---------|-------------|
| `/generate` | Generate code from a description |
| `/test` | Generate unit tests for selected code |
| `/explain` | Explain selected code in plain language |
| `/fix` | Diagnose and fix selected code |
| `/docs` | Generate docstrings and documentation |

## Architecture

```
┌─────────────────────────────────────┐
│  IDE Extension (VS Code / JetBrains)│
│  TypeScript / Kotlin                │
└────────────────┬────────────────────┘
                 │ JSON-RPC (Unix socket / named pipe)
┌────────────────▼────────────────────┐
│         anvil-daemon (Rust)         │
│  ┌─────────────┐ ┌───────────────┐  │
│  │  Inference  │ │   Indexer     │  │
│  │  Backends   │ │  Tree-sitter  │  │
│  │  llama.cpp  │ │  BM25+Vector  │  │
│  │  candle     │ │  sqlite-vec   │  │
│  │  Ollama     │ └───────────────┘  │
│  │  Cloud APIs │ ┌───────────────┐  │
│  └─────────────┘ │  Capabilities │  │
│                  │  /generate    │  │
│                  │  /test /fix   │  │
│                  └───────────────┘  │
└─────────────────────────────────────┘
```

## Getting Started

**Quick start** (requires [Rust](https://rustup.rs/) and [Ollama](https://ollama.ai/)):

```bash
# Install the daemon
cargo install --path crates/anvil-daemon

# Pull a model
ollama pull deepseek-coder:6.7b

# Start the daemon (in your project directory)
anvil start --project .
```

Then install the **VS Code extension** from the Marketplace (search "TPT Anvil") or from the `.vsix` in [GitHub Releases](https://github.com/tpt-solutions/tpt-anvil/releases).

For JetBrains IDEs, install the plugin from the Marketplace or build from source (see `plugins/jetbrains/`).

See [docs/getting-started.md](docs/getting-started.md) for full details, and [docs/config-reference.md](docs/config-reference.md) for all config options including `[vault]`, `[smart_context]`, `[router]`, and `[verify]`.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

Copyright (c) 2026 TPT Solutions
