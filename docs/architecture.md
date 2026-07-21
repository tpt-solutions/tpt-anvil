# Architecture

## Overview

TPT Anvil follows a client–server architecture where a long-running Rust daemon handles all AI inference and code indexing, and IDE extensions communicate with it over a local Unix socket (or named pipe on Windows) using JSON-RPC 2.0.

```
┌─────────────────────────────────────────┐
│  IDE Extension (VS Code / JetBrains)    │
│  TypeScript / Kotlin                    │
│  - Chat panel (Webview / Swing)         │
│  - Slash command handlers               │
│  - Diff viewer + apply                  │
│  - Settings UI                          │
└────────────────┬────────────────────────┘
                 │ JSON-RPC 2.0 (newline-delimited)
                 │ Unix socket ($XDG_RUNTIME_DIR/anvil/anvil.sock)
                 │ Named pipe on Windows
┌────────────────▼────────────────────────┐
│           anvil-daemon (Rust)           │
│                                         │
│  ┌──────────────────────────────────┐   │
│  │       anvil-capabilities         │   │
│  │  /generate  /test  /explain      │   │
│  │  /fix  /docs  diff engine        │   │
│  │  conversation history            │   │
│  └───────┬──────────────┬───────────┘   │
│          │              │               │
│  ┌───────▼──────┐ ┌─────▼───────────┐  │
│  │anvil-inference│ │ tpt-anvil-indexer   │  │
│  │  InferenceBE  │ │ Tree-sitter AST │  │
│  │  ┌──────────┐ │ │ BM25 (FTS5)    │  │
│  │  │  Ollama  │ │ │ sqlite-vec     │  │
│  │  │ llama.cpp│ │ │ File watcher   │  │
│  │  │  candle  │ │ └────────────────┘  │
│  │  └──────────┘ │                     │
│  └───────────────┘                     │
│  ┌──────────────────────────────────┐  │
│  │       tpt-anvil-providers            │  │
│  │  OpenAI  Anthropic  OpenRouter   │  │
│  │  Azure   Custom endpoint         │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │       anvil-config               │  │
│  │  TOML  hot-reload  keychain      │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Crates

| Crate | Role |
|-------|------|
| `anvil-core` | Shared types, error types, IPC protocol definitions |
| `anvil-config` | Config schema, file loading, hot-reload watcher |
| `anvil-inference` | `InferenceBackend` trait; Ollama, llama.cpp, candle |
| `tpt-anvil-providers` | Cloud provider trait; OpenAI, Anthropic, OpenRouter, Azure, custom |
| `tpt-anvil-indexer` | Tree-sitter parsing, SQLite FTS5, symbol extraction, file watcher |
| `anvil-capabilities` | Slash commands, diff engine, context assembly, conversation store |
| `anvil-daemon` | Main binary: IPC server, CLI, daemon lifecycle |

## IPC Protocol

All messages are newline-delimited JSON-RPC 2.0.

**Request (client → daemon):**
```json
{"jsonrpc":"2.0","id":1,"method":"slash_command","params":{...}}
```

**Streaming notification (daemon → client, during generation):**
```json
{"jsonrpc":"2.0","method":"stream_token","params":{"id":1,"delta":"def ","done":false}}
```

**Response (daemon → client, when generation completes):**
```json
{"jsonrpc":"2.0","id":1,"result":{"content":"def foo():\n    ..."}}
```

## Methods

| Method | Description |
|--------|-------------|
| `health` | Liveness check |
| `status` | Backend, model, index status |
| `slash_command` | Run `/generate`, `/test`, `/explain`, `/fix`, `/docs` |

## Config File Locations

- User config: `~/.config/anvil/config.toml`
- Project config: `<project>/.anvil/config.toml` (overrides user)
- Index DB: `<project>/.anvil/index.db`
- Socket: `$XDG_RUNTIME_DIR/anvil/anvil.sock` (Linux/macOS), `\\.\pipe\anvil` (Windows)
- PID file: `$XDG_RUNTIME_DIR/anvil/anvil.pid`
