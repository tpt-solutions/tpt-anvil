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
- `anvil-capabilities/vault`: Secret redaction rules for AWS, GitHub, OpenAI, Anthropic, Slack, PEM keys, JWTs, and generic patterns
- `tpt-anvil-indexer/outline`: AST outline compression for context-efficient code summaries
- `tpt-anvil-providers/router`: Cost-based provider selection with cheapest-first routing
- `anvil-capabilities/verify`: Compiler/lint/test verification gate with fail-open retry logic
- VS Code extension: Sidebar webview provider, markdown rendering, diff preview with Apply/Preview/Dismiss, slash-command autocomplete dropdown
- JetBrains plugin: Proper unified diff applier, markdown rendering, real editor context integration
- IPC authentication: Per-run secret token file (0600) required on every RPC request
- Unix socket security: Runtime dir 0700, socket 0600, TOCTOU race fix
- PID hardening: Stale PID cleanup, `is_pid_alive()` check
- HTTP timeouts: 10s connect + 120s request timeout on provider clients
- Error scrubbing: HTTP error bodies truncated and redacted before logging
- `tpt-anvil-indexer` and `tpt-anvil-providers`: Decoupled from `anvil-core`, ready for crates.io publishing
- Marketplace listings: VS Code and JetBrains marketplace documentation
- GitHub Discussions templates: General Q&A, Feature Requests, Show and Tell

### Changed
- `anvil-config/loader.rs::merge()`: Replaced wholesale-replace with per-field deep merge using `merge_with()` and `HasMerge` trait
- `anvil-inference/llama_cpp.rs`: Real GGUF model loading via `llama_cpp_2`, sampler-based inference
- `anvil-inference/candle.rs`: Real GGUF file parsing, tensor ops, greedy/temperature sampling
- `anvil-inference/prompt.rs`: Added `apply_chat_template()` for model-specific chat templates
- JetBrains `SlashCommandActions.kt`: Wired to tool window with real editor context
- JetBrains `AnvilChatPanel.kt`: JEditorPane HTML rendering, streaming responses
- VS Code `chatViewProvider.ts`: Created `AnvilChatViewProvider` implementing `WebviewViewProvider`
- VS Code `chat.ts`: Markdown rendering, robust diff detection, slash-command autocomplete
- VS Code `diff.ts`: Three-button dialog (Apply/Preview/Dismiss) with side-by-side diff
- `anvil-daemon/server.rs`: IPC auth, socket permissions, TOCTOU fix
- `anvil-daemon/pid.rs`: PID hardening with stale cleanup
- `tpt-anvil-providers/retry.rs`: Error scrubbing with `scrub_error_message()`
- `anvil-config/schema.rs`: Added `VaultConfig`, `SmartContextConfig`, `RouterConfigSchema`, `VerifyConfigSchema`
