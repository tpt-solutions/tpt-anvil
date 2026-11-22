# TPT Anvil

**The open-source, privacy-first AI development environment.**

> Status: pre-alpha — active development

Anvil is a fully open-source alternative to GitHub Copilot and Cursor. Everything runs on your machine — your code never leaves.

## Features

- **Local LLM inference** — llama.cpp, candle (pure Rust), or Ollama; any GGUF model
- **GPU acceleration** — CUDA, ROCm, WebGPU
- **Codebase-aware context** — Tree-sitter AST, symbol and call-graph extraction, and hybrid
  BM25 + vector search with reciprocal-rank fusion
- **Cloud fallback (BYOK)** — OpenAI, Anthropic, OpenRouter, Azure OpenAI, or any OpenAI-compatible
  endpoint; model lists are fetched live from each provider's API rather than hardcoded, so new
  releases show up without an Anvil update
- **Secret redaction** — API keys, tokens, and private keys are stripped from prompts before any
  cloud request
- **VS Code extension** — chat panel, slash commands, diff-based edits
- **JetBrains plugin** — IntelliJ IDEA, PyCharm, GoLand, and all JetBrains IDEs
- **Model benchmark suite** — `anvil benchmark run` scores a model against built-in coding tasks,
  and `anvil benchmark report` compares scorecards
- **Privacy-first** — 100% local by default, zero telemetry

### Live in the daemon

These run on every request, configured in `config.toml`:

- **Secret redaction (Vault)** — API keys, tokens, and PEM private keys are redacted before a
  prompt reaches a cloud provider. Only the rule name and a count are logged, never the value.
- **Smart Context** — large files and oversized context chunks are sent as AST outlines
  (signatures, not bodies) to save tokens.
- **Cost-based routing** — the opt-in `[router]` section picks the cheapest configured cloud
  provider for each request, with an optional pinned provider.
- **Recent models** — the last 5 (provider, model) pairs used are tracked so IDE UIs can offer a
  quick-pick list.
- **Verifier** — runs the compiler/type-checker and linter on generated patches before they are
  offered, with an optional test run and one bounded retry that feeds errors back to the model.
  Failures are reported as warnings; the IDE warning banner is still in progress.

Per-feature status is tracked in `todo.md` (Phase 16).

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
┌──────────────────────────────────────────────┐
│ IDE extension (VS Code / JetBrains)          │
│ TypeScript / Kotlin                          │
└──────────────────┬───────────────────────────┘
                   │ JSON-RPC (Unix socket / named pipe)
┌──────────────────▼───────────────────────────┐
│ tpt-anvil-daemon (Rust)                      │
│                                              │
│ Inference     llama.cpp · candle · Ollama    │
│               cloud APIs (BYOK)              │
│ Indexer       Tree-sitter · FTS5 + BM25      │
│               embeddings · call graph        │
│ Capabilities  /generate /test /fix /docs     │
│              Vault · Smart Context · Verifier │
└──────────────────────────────────────────────┘
```

## Getting Started

**Quick start** (requires [Rust](https://rustup.rs/) and [Ollama](https://ollama.ai/)):

```bash
# Install the daemon
cargo install --path crates/tpt-anvil-daemon

# Pull a model
ollama pull deepseek-coder:6.7b

# Check the setup (config, Ollama, model, keychain)
anvil doctor

# Start the daemon (in your project directory)
anvil start --project .
```

`anvil init` runs an interactive wizard that writes the config file for you.

Then install the **VS Code extension** from the Marketplace (search "TPT Anvil") or from the `.vsix` in [GitHub Releases](https://github.com/tpt-solutions/tpt-anvil/releases).

For JetBrains IDEs, install the plugin from the Marketplace or build from source (see `plugins/jetbrains/`).

See [docs/getting-started.md](docs/getting-started.md) for full details, and [docs/config-reference.md](docs/config-reference.md) for all config options including `[vault]`, `[smart_context]`, `[router]`, and `[verify]`.

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

Copyright (c) 2026 TPT Solutions
