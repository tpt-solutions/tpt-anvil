# Changelog

All notable changes to TPT Anvil will be documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project uses [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- Monorepo scaffold: Cargo workspace + npm workspace
- `anvil-core`: shared types, IPC protocol (JSON-RPC 2.0), error types
- `anvil-config`: TOML config with hot-reload, project + user fallback chain
- `anvil-inference`: `InferenceBackend` trait; Ollama, llama.cpp, candle backends
- `anvil-providers`: OpenAI, Anthropic, Azure OpenAI, OpenRouter, custom endpoint
- `anvil-indexer`: Tree-sitter AST parsing (9 languages), SQLite FTS5 (BM25), symbol search
- `anvil-capabilities`: slash command engine (`/generate`, `/test`, `/explain`, `/fix`, `/docs`), diff engine, conversation history
- `anvil-daemon`: Unix socket IPC server, CLI (`anvil start/stop/status/auth/models`)
- VS Code extension: chat panel, daemon IPC client, slash command handlers, diff apply
- JetBrains plugin: tool window, chat panel, settings page, editor context actions
- GitHub Actions release workflow (Linux musl, macOS arm64/x86_64, Windows)
- Dual MIT/Apache-2.0 license — TPT Solutions
- `anvil-indexer`: call graph construction (caller/callee), local embeddings (offline feature-hashing + Ollama), hybrid retrieval with BM25 + vector RRF fusion
- `anvil-inference`: GPU acceleration feature flags (`cuda`, `rocm`, `webgpu`) with device selection
- `anvil-providers`: integration tests against a mock HTTP server (wiremock); cloud fallback in the capability layer
- `anvil-capabilities`: full unified-diff application (hunk-aware patcher)
- VS Code extension: Vitest unit tests, `@vscode/test-electron` E2E suite, ESLint flat config, VSIX packaging + Marketplace publish workflow
- JetBrains plugin: JUnit 5 unit tests + Kotlin CI job
- CI: Vitest + Kotlin test jobs, code coverage (cargo-llvm-cov + Vitest → Codecov), Dependabot config
