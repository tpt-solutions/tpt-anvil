# TPT Anvil

**The open-source, privacy-first AI development environment.**

> Status: pre-alpha — active development

Anvil is a fully open-source alternative to GitHub Copilot and Cursor. Everything runs on your machine — your code never leaves.

## Features

- **Local LLM inference** — llama.cpp, candle (pure Rust), or Ollama; any GGUF model
- **GPU acceleration** — CUDA, ROCm, WebGPU
- **Codebase-aware context** — Tree-sitter AST + hybrid BM25/vector semantic search
- **Cloud fallback** — OpenAI, Anthropic, OpenRouter, or any OpenAI-compatible endpoint
- **VS Code extension** — chat panel, slash commands, diff-based edits
- **JetBrains plugin** — IntelliJ IDEA, PyCharm, GoLand, and all JetBrains IDEs
- **Privacy-first** — 100% local by default, zero telemetry

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

> Installation instructions coming soon. See [docs/getting-started.md](docs/getting-started.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

Copyright (c) 2026 TPT Solutions
