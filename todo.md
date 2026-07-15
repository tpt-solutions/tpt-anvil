# TPT Anvil — Project Task Tracker

> Open-source, locally-runnable AI development environment (the "Open Copilot/Cursor")
> Copyright TPT Solutions — Dual licensed: MIT OR Apache-2.0

---

## Phase 0 — Foundation & Repo Setup

- [ ] Initialize git repository and push to GitHub (`tpt-solutions/tpt-anvil`)
- [ ] Add dual license files: `LICENSE-MIT` and `LICENSE-APACHE`
- [ ] Add `LICENSE` root file explaining dual-license choice
- [ ] Write initial `README.md` (project overview, status: pre-alpha)
- [ ] Create `CONTRIBUTING.md` (contribution guidelines, DCO sign-off)
- [ ] Add `.gitignore` (Rust, Node.js, JVM, IDE files, model weights)
- [ ] Create monorepo directory scaffold:
  - `crates/` — Rust workspace members
  - `extensions/vscode/` — VS Code extension (TypeScript)
  - `plugins/jetbrains/` — JetBrains plugin (Kotlin/Gradle)
  - `docs/` — Architecture and user documentation
  - `scripts/` — Build and release helper scripts
- [ ] Initialize `Cargo.toml` workspace (root)
- [ ] Initialize `package.json` npm workspace (root)
- [ ] Configure `rustfmt.toml` and `clippy` lint rules
- [ ] Add `CODE_OF_CONDUCT.md`
- [ ] Add GitHub issue templates (bug report, feature request)
- [ ] Add GitHub PR template

---

## Phase 1 — Daemon & IPC Infrastructure (Rust)

- [ ] Create `crates/anvil-daemon` — main long-running background process
- [ ] Create `crates/anvil-core` — shared types, errors, config structs
- [ ] Define JSON-RPC 2.0 IPC protocol (request/response + streaming)
- [ ] Implement Unix socket transport (Linux/macOS)
- [ ] Implement named pipe transport (Windows)
- [ ] Implement daemon lifecycle: start, stop, restart, PID file
- [ ] Implement health-check / status endpoint (`/health`)
- [ ] Set up structured logging (tracing crate, local log files)
- [ ] Write integration tests for IPC round-trip
- [ ] CLI entry point: `anvil` binary (start daemon, query status, stop)

---

## Phase 2 — Configuration System

- [ ] Define TOML config schema (`~/.config/anvil/config.toml` + per-project `.anvil/config.toml`)
- [ ] Implement config file discovery (project → user → system fallback chain)
- [ ] Implement config validation with helpful error messages
- [ ] Create `crates/anvil-config` crate
- [ ] Config hot-reload (watch file for changes, notify daemon)
- [ ] Config sections: `[inference]`, `[providers]`, `[indexing]`, `[ui]`
- [ ] Document all config keys with defaults in `docs/config-reference.md`

---

## Phase 3 — Inference Engine (Rust)

- [ ] Create `crates/anvil-inference` crate
- [ ] Define `InferenceBackend` trait (generate, stream, tokenize, model_info)
- [ ] Implement **llama.cpp backend** via `llama-cpp-rs` or `llama_cpp` crate
  - [ ] GGUF model loading
  - [ ] CUDA acceleration support
  - [ ] ROCm acceleration support
  - [ ] CPU fallback
- [ ] Implement **candle backend** (pure Rust)
  - [ ] GGUF/GGML model loading via candle-transformers
  - [ ] WebGPU / wgpu acceleration
  - [ ] CPU fallback
- [ ] Implement **Ollama HTTP API backend**
  - [ ] `/api/generate` streaming endpoint
  - [ ] `/api/tags` for model listing
  - [ ] Configurable Ollama server URL
- [ ] Implement backend selection logic (config-driven)
- [ ] Streaming token output (async iterator / channel-based)
- [ ] Context window management (token counting, truncation strategies)
- [ ] Prompt template system (model-specific chat templates: ChatML, Llama, Alpaca, etc.)
- [ ] Model management commands: list, pull (for Ollama), info
- [ ] Featured model configs: DeepSeek Coder, Qwen2.5-Coder presets
- [ ] Unit tests for each backend (mock + integration)

---

## Phase 4 — Cloud Provider Layer (Rust)

- [ ] Create `crates/anvil-providers` crate
- [ ] Define `CloudProvider` trait (mirrors `InferenceBackend` for remote models)
- [ ] Implement **OpenAI provider** (`/v1/chat/completions` with SSE streaming)
- [ ] Implement **Azure OpenAI provider** (endpoint + deployment name config)
- [ ] Implement **Anthropic provider** (`/v1/messages` with streaming)
- [ ] Implement **OpenRouter provider** (OpenAI-compatible endpoint)
- [ ] Implement **generic OpenAI-compatible endpoint** (user-supplied base URL)
- [ ] Secure API key storage (OS keychain via `keyring` crate)
- [ ] Provider switching (local ↔ cloud per request or global config)
- [ ] Token counting / cost estimation per provider
- [ ] Rate limiting and retry with exponential backoff
- [ ] Unit/integration tests with mock HTTP server

---

## Phase 5 — Code Indexing Engine (Rust)

- [ ] Create `crates/anvil-indexer` crate
- [ ] Integrate `tree-sitter` for language-agnostic AST parsing
- [ ] Add Tree-sitter grammars for: Rust, Python, TypeScript/JavaScript, Go, Java, C/C++, Ruby, PHP, C#
- [ ] Symbol extraction: functions, classes, structs, imports, exports
- [ ] Call graph construction (caller/callee relationships)
- [ ] Integrate `sqlite-vec` for local vector storage
- [ ] Embed local embedding model (e.g. `nomic-embed-code` via candle) for vector generation
- [ ] Implement BM25 lexical search index (tantivy or custom)
- [ ] Implement hybrid retrieval: BM25 + vector cosine similarity fusion (RRF)
- [ ] File watcher for incremental index updates (`notify` crate)
- [ ] `.gitignore`-aware file filtering
- [ ] Project indexing on daemon start + incremental updates
- [ ] Search API: query by symbol, keyword, or natural language
- [ ] Context assembly: given cursor position → retrieve relevant code chunks
- [ ] Unit tests and retrieval quality benchmarks

---

## Phase 6 — AI Capability Layer (Rust)

- [ ] Create `crates/anvil-capabilities` crate
- [ ] Slash command parser (input → command + arguments)
- [ ] Implement `/generate` — generate code from description + context
- [ ] Implement `/test` — generate unit tests for selected function/class
- [ ] Implement `/explain` — explain selected code in plain language
- [ ] Implement `/fix` — diagnose and fix selected code (error-first)
- [ ] Implement `/docs` — generate docstrings/documentation for selected code
- [ ] Diff engine: model output → unified diff format
- [ ] Diff application: apply patch to file (preview + confirm flow)
- [ ] Context assembly pipeline (file context + indexer results + cursor position + selection)
- [ ] Conversation history management (multi-turn chat)
- [ ] System prompt templates per command
- [ ] Unit tests per command with fixture inputs

---

## Phase 7 — VS Code Extension (TypeScript)

- [ ] Scaffold VS Code extension (`extensions/vscode/`)
  - [ ] `package.json` with contributes, activationEvents
  - [ ] TypeScript + esbuild build setup
  - [ ] `.vscodeignore`
- [ ] Implement IPC client (JSON-RPC over Unix socket / named pipe)
- [ ] Daemon lifecycle management (auto-start on extension activate)
- [ ] **Chat sidebar panel** (VS Code Webview)
  - [ ] Chat input with slash command autocomplete
  - [ ] Message thread rendering (markdown + code blocks)
  - [ ] Streaming response display
- [ ] **Slash command handlers** in extension:
  - [ ] `/generate`, `/test`, `/explain`, `/fix`, `/docs`
  - [ ] Pass editor selection + file context to daemon
- [ ] **Diff viewer + apply** (show diff, one-click apply via `workspace.applyEdit`)
- [ ] VS Code settings UI (contributes.configuration schema)
- [ ] Status bar indicator (active model + backend + connection status)
- [ ] Extension commands registered in Command Palette
- [ ] Context menu items (right-click → Explain / Fix / Generate Test)
- [ ] VSIX packaging (`vsce package`)
- [ ] VS Code Marketplace publish workflow (manual + automated)
- [ ] E2E tests with `@vscode/test-electron`

---

## Phase 8 — JetBrains Plugin (Kotlin)

- [ ] Scaffold JetBrains plugin (`plugins/jetbrains/`)
  - [ ] Gradle build (`build.gradle.kts`, `plugin.xml`)
  - [ ] IntelliJ Platform Plugin Gradle Plugin setup
  - [ ] `plugin.xml` metadata (name, version, description, vendor: TPT Solutions)
- [ ] Implement IPC client (JSON-RPC, Kotlin coroutines)
- [ ] Daemon lifecycle management (start on IDE startup)
- [ ] **Tool window** (chat panel sidebar)
  - [ ] Swing/JCEF-based UI
  - [ ] Slash command input and autocomplete
  - [ ] Streaming response rendering
- [ ] **Slash command actions** registered in IntelliJ action system
- [ ] **Diff viewer + apply** (IntelliJ Diff API)
- [ ] Settings page (IntelliJ `Configurable` + settings persistence)
- [ ] Status bar widget (model + backend indicator)
- [ ] Editor context menu items
- [ ] Plugin packaging (Gradle `buildPlugin`)
- [ ] JetBrains Marketplace publish workflow (future milestone)
- [ ] Plugin tests (IntelliJ Platform test framework)

---

## Phase 9 — Distribution & Release

- [ ] Cross-platform binary builds:
  - [ ] Linux x86_64 (`x86_64-unknown-linux-gnu`, static musl)
  - [ ] macOS arm64 (`aarch64-apple-darwin`)
  - [ ] macOS x86_64 (`x86_64-apple-darwin`)
  - [ ] Windows x86_64 (`x86_64-pc-windows-msvc`)
- [ ] GitHub Actions release workflow (triggered on version tag)
- [ ] Binary asset upload to GitHub Releases
- [ ] Checksum generation (SHA-256) for release assets
- [ ] VSIX bundled in GitHub Release assets
- [ ] Versioning scheme (semver, `v0.1.0` initial)
- [ ] VS Code Marketplace listing (description, screenshots, categories)
- [ ] Changelog (`CHANGELOG.md`)

---

## Phase 10 — CI / Testing (Add Later)

- [ ] GitHub Actions CI workflow
- [ ] Matrix: Linux x86_64, macOS arm64, Windows x86_64
- [ ] `cargo test` (unit + integration)
- [ ] `cargo clippy` lint
- [ ] `cargo fmt --check`
- [ ] TypeScript lint (`eslint`) + type-check (`tsc --noEmit`)
- [ ] Jest/Vitest for extension unit tests
- [ ] Kotlin plugin tests in CI
- [ ] Code coverage reporting
- [ ] Dependabot / Renovate for dependency updates

---

## Phase 11 — Documentation & Community

- [ ] `README.md` — full project overview, quickstart, screenshots
- [ ] `docs/architecture.md` — system diagram, component interactions
- [ ] `docs/getting-started.md` — install daemon + VS Code extension
- [ ] `docs/model-setup.md` — how to download and configure GGUF models
- [ ] `docs/cloud-providers.md` — API key setup for each cloud provider
- [ ] `docs/config-reference.md` — all config options with types and defaults
- [ ] `docs/slash-commands.md` — command reference with examples
- [ ] `docs/contributing.md` — dev environment setup, build instructions
- [ ] Add license headers to all source files (SPDX identifiers)
- [ ] Set up GitHub Discussions (community Q&A)
- [ ] Set up GitHub Projects board linked to this checklist
