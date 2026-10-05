# TPT Anvil — Project Task Tracker

> Open-source, locally-runnable AI development environment (the "Open Copilot/Cursor")
> Copyright TPT Solutions — Dual licensed: MIT OR Apache-2.0

---

## Phase 0 — Foundation & Repo Setup

- [x] Initialize git repository and push to GitHub (`tpt-solutions/tpt-anvil`)
- [x] Add dual license files: `LICENSE-MIT` and `LICENSE-APACHE`
- [x] Add `LICENSE` root file explaining dual-license choice
- [x] Write initial `README.md` (project overview, status: pre-alpha)
- [x] Create `CONTRIBUTING.md` (contribution guidelines, DCO sign-off)
- [x] Add `.gitignore` (Rust, Node.js, JVM, IDE files, model weights)
- [x] Create monorepo directory scaffold:
  - `crates/` — Rust workspace members
  - `extensions/vscode/` — VS Code extension (TypeScript)
  - `plugins/jetbrains/` — JetBrains plugin (Kotlin/Gradle)
  - `docs/` — Architecture and user documentation
  - `scripts/` — Build and release helper scripts
- [x] Initialize `Cargo.toml` workspace (root)
- [x] Initialize `package.json` npm workspace (root)
- [x] Configure `rustfmt.toml` and `clippy` lint rules
- [x] Add `CODE_OF_CONDUCT.md`
- [x] Add GitHub issue templates (bug report, feature request)
- [x] Add GitHub PR template

---

## Phase 1 — Daemon & IPC Infrastructure (Rust)

- [x] Create `crates/anvil-daemon` — main long-running background process
- [x] Create `crates/anvil-core` — shared types, errors, config structs
- [x] Define JSON-RPC 2.0 IPC protocol (request/response + streaming)
- [x] Implement Unix socket transport (Linux/macOS)
- [x] Implement named pipe transport (Windows)
- [x] Implement daemon lifecycle: start, stop, restart, PID file
- [x] Implement health-check / status endpoint (`/health`)
- [x] Set up structured logging (tracing crate, local log files)
- [x] Write integration tests for IPC round-trip
- [x] CLI entry point: `anvil` binary (start daemon, query status, stop)

---

## Phase 2 — Configuration System

- [x] Define TOML config schema (`~/.config/anvil/config.toml` + per-project `.anvil/config.toml`)
- [x] Implement config file discovery (project → user → system fallback chain)
- [x] Implement config validation with helpful error messages
- [x] Create `crates/anvil-config` crate
- [x] Config hot-reload (watch file for changes, notify daemon)
- [x] Config sections: `[inference]`, `[providers]`, `[indexing]`, `[ui]`
- [x] Document all config keys with defaults in `docs/config-reference.md`

---

## Phase 3 — Inference Engine (Rust)

- [x] Create `crates/anvil-inference` crate
- [x] Define `InferenceBackend` trait (generate, stream, tokenize, model_info)
- [x] Implement **llama.cpp backend** via `llama-cpp-rs` or `llama_cpp` crate
  - [x] GGUF model loading (stub — full integration TODO)
  - [x] CUDA acceleration support (feature flag `cuda` + device selection)
  - [x] ROCm acceleration support (feature flag `rocm` + device selection)
  - [x] CPU fallback
- [x] Implement **candle backend** (pure Rust)
  - [x] GGUF/GGML model loading via candle-transformers (stub — full integration TODO)
  - [x] WebGPU / wgpu acceleration (feature flag `webgpu` + device selection)
  - [x] CPU fallback
- [x] Implement **Ollama HTTP API backend**
  - [x] `/api/generate` streaming endpoint
  - [x] `/api/tags` for model listing
  - [x] Configurable Ollama server URL
- [x] Implement backend selection logic (config-driven)
- [x] Streaming token output (async iterator / channel-based)
- [x] Context window management (token counting, truncation strategies)
- [x] Prompt template system (model-specific chat templates: ChatML, Llama, Alpaca, etc.)
- [x] Model management commands: list, pull (for Ollama), info
- [x] Featured model configs: DeepSeek Coder, Qwen2.5-Coder presets
- [x] Unit tests for each backend (prompt template tests; mock backend tests)

---

## Phase 4 — Cloud Provider Layer (Rust)

- [x] Create `crates/anvil-providers` crate
- [x] Define `CloudProvider` trait (mirrors `InferenceBackend` for remote models)
- [x] Implement **OpenAI provider** (`/v1/chat/completions` with SSE streaming)
- [x] Implement **Azure OpenAI provider** (endpoint + deployment name config)
- [x] Implement **Anthropic provider** (`/v1/messages` with streaming)
- [x] Implement **OpenRouter provider** (OpenAI-compatible endpoint)
- [x] Implement **generic OpenAI-compatible endpoint** (user-supplied base URL)
- [x] Secure API key storage (OS keychain via `keyring` crate)
- [x] Provider switching (local ↔ cloud per request or global config)
- [x] Token counting / cost estimation per provider (`cost.rs`)
- [x] Rate limiting and retry with exponential backoff (`retry.rs`)
- [x] Unit/integration tests with mock HTTP server

---

## Phase 5 — Code Indexing Engine (Rust)

- [x] Create `crates/anvil-indexer` crate
- [x] Integrate `tree-sitter` for language-agnostic AST parsing
- [x] Add Tree-sitter grammars for: Rust, Python, TypeScript/JavaScript, Go, Java, C/C++
- [x] Add Tree-sitter grammars for: Ruby, PHP, C#
- [x] Symbol extraction: functions, classes, structs, imports, exports
- [x] Call graph construction (caller/callee relationships)
- [x] Integrate `sqlite-vec` for local vector storage (SQLite FTS5 / BM25)
- [x] Embed local embedding model (offline feature-hashing + Ollama `nomic-embed-*`) for vector generation
- [x] Implement BM25 lexical search index (SQLite FTS5)
- [x] Implement hybrid retrieval: BM25 + vector cosine similarity fusion (RRF)
- [x] File watcher for incremental index updates (`notify` crate)
- [x] `.gitignore`-aware file filtering
- [x] Project indexing on daemon start + incremental updates
- [x] Search API: query by symbol, keyword, or natural language
- [x] Context assembly: given cursor position → retrieve relevant code chunks
- [x] Unit tests for symbol extraction

---

## Phase 6 — AI Capability Layer (Rust)

- [x] Create `crates/anvil-capabilities` crate
- [x] Slash command parser (input → command + arguments)
- [x] Implement `/generate` — generate code from description + context
- [x] Implement `/test` — generate unit tests for selected function/class
- [x] Implement `/explain` — explain selected code in plain language
- [x] Implement `/fix` — diagnose and fix selected code (error-first)
- [x] Implement `/docs` — generate docstrings/documentation for selected code
- [x] Diff engine: model output → unified diff format
- [x] Diff application: apply patch to file (preview + confirm flow)
- [x] Context assembly pipeline (file context + indexer results + cursor position + selection)
- [x] Conversation history management (multi-turn chat)
- [x] System prompt templates per command
- [x] Unit tests per command with fixture inputs

---

## Phase 7 — VS Code Extension (TypeScript)

- [x] Scaffold VS Code extension (`extensions/vscode/`)
  - [x] `package.json` with contributes, activationEvents
  - [x] TypeScript + esbuild build setup
  - [x] `.vscodeignore`
- [x] Implement IPC client (JSON-RPC over Unix socket / named pipe)
- [x] Daemon lifecycle management (auto-start on extension activate)
- [x] **Chat sidebar panel** (VS Code Webview)
  - [x] Chat input with slash command autocomplete
  - [x] Message thread rendering (markdown + code blocks)
  - [x] Streaming response display
- [x] **Slash command handlers** in extension:
  - [x] `/generate`, `/test`, `/explain`, `/fix`, `/docs`
  - [x] Pass editor selection + file context to daemon
- [x] **Diff viewer + apply** (show diff, one-click apply via `workspace.applyEdit`)
- [x] VS Code settings UI (contributes.configuration schema)
- [x] Status bar indicator (active model + backend + connection status)
- [x] Extension commands registered in Command Palette
- [x] Context menu items (right-click → Explain / Fix / Generate Test)
- [x] VSIX packaging (`vsce package`)
- [x] VS Code Marketplace publish workflow (manual + automated)
- [x] E2E tests with `@vscode/test-electron`

---

## Phase 8 — JetBrains Plugin (Kotlin)

- [x] Scaffold JetBrains plugin (`plugins/jetbrains/`)
  - [x] Gradle build (`build.gradle.kts`, `plugin.xml`)
  - [x] IntelliJ Platform Plugin Gradle Plugin setup
  - [x] `plugin.xml` metadata (name, version, description, vendor: TPT Solutions)
- [x] Implement IPC client (JSON-RPC, Kotlin coroutines)
- [x] Daemon lifecycle management (start on IDE startup)
- [x] **Tool window** (chat panel sidebar)
  - [x] Swing-based UI
  - [x] Slash command input and autocomplete
  - [x] Streaming response rendering
- [x] **Slash command actions** registered in IntelliJ action system
- [x] **Diff viewer + apply** (IntelliJ Diff API)
- [x] Settings page (IntelliJ `Configurable` + settings persistence)
- [x] Status bar widget (model + backend indicator)
- [x] Editor context menu items
- [x] Plugin packaging (Gradle `buildPlugin`) — provided by plugin automatically
- [ ] JetBrains Marketplace publish workflow (future milestone)
- [x] Plugin tests (IntelliJ Platform test framework)

---

## Phase 9 — Distribution & Release

- [x] Cross-platform binary builds:
  - [x] Linux x86_64 (`x86_64-unknown-linux-gnu`, static musl)
  - [x] macOS arm64 (`aarch64-apple-darwin`)
  - [x] macOS x86_64 (`x86_64-apple-darwin`)
  - [x] Windows x86_64 (`x86_64-pc-windows-msvc`)
- [x] GitHub Actions release workflow (triggered on version tag)
- [x] Binary asset upload to GitHub Releases
- [x] Checksum generation (SHA-256) for release assets
- [x] VSIX bundled in GitHub Release assets
- [x] Versioning scheme (semver, `v0.1.0` initial)
- [ ] VS Code Marketplace listing (description, screenshots, categories)
- [x] Changelog (`CHANGELOG.md`)

---

## Phase 10 — CI / Testing

- [x] GitHub Actions CI workflow
- [x] Matrix: Linux x86_64, macOS arm64, Windows x86_64
- [x] `cargo test` (unit + integration)
- [x] `cargo clippy` lint
- [x] `cargo fmt --check`
- [x] TypeScript lint (`eslint`) + type-check (`tsc --noEmit`)
- [x] Jest/Vitest for extension unit tests
- [x] Kotlin plugin tests in CI
- [x] Code coverage reporting
- [x] Dependabot / Renovate for dependency updates

---

## Phase 11 — Documentation & Community

- [x] `README.md` — full project overview, quickstart, screenshots
- [x] `docs/architecture.md` — system diagram, component interactions
- [x] `docs/getting-started.md` — install daemon + VS Code extension
- [x] `docs/model-setup.md` — how to download and configure GGUF models
- [x] `docs/cloud-providers.md` — API key setup for each cloud provider
- [x] `docs/config-reference.md` — all config options with types and defaults
- [x] `docs/slash-commands.md` — command reference with examples
- [x] `docs/contributing.md` — dev environment setup, build instructions
- [x] Add license headers to all source files (SPDX identifiers)
- [ ] Set up GitHub Discussions (community Q&A)
- [ ] Set up GitHub Projects board linked to this checklist
