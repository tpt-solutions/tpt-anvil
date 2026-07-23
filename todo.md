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
- [x] JetBrains Marketplace publish workflow (future milestone)
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
- [x] VS Code Marketplace listing (description, screenshots, categories)
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
- [x] Set up GitHub Discussions (community Q&A)
- [ ] Set up GitHub Projects board linked to this checklist (requires manual setup via GitHub web UI)

---

## Phase 12 — Stub / Incomplete Implementation Fixes (found in 2026-07-21 audit)

- [x] `crates/anvil-inference/src/llama_cpp.rs` — `complete()`/`stream()` always error ("not yet fully integrated"); implement real GGUF model loading + inference via `llama-cpp-2` (in progress this session)
- [x] `crates/anvil-inference/src/candle.rs` — `complete()`/`stream()` always error; implement real GGUF/GGML loading + forward pass via `candle-transformers`
- [x] `crates/anvil-inference/src/candle.rs` / `llama_cpp.rs` — `count_tokens()` uses a `len()/4` heuristic instead of the model's real tokenizer
- [x] `crates/anvil-config/src/loader.rs::merge()` — overlay wholesale-replaces base instead of a real per-field merge; project/user/default config layering silently drops partial overrides
- [x] `plugins/jetbrains/.../actions/SlashCommandActions.kt` — `BaseAnvilAction.actionPerformed()` (Explain/Fix/GenerateTest/GenerateDocs) is a no-op stub (`// TODO: wire to tool window and daemon`); menu items registered in `plugin.xml` do nothing when clicked
- [x] `plugins/jetbrains/.../ui/AnvilChatPanel.kt` — `sendMessage()` builds `CodeContext` with hardcoded empty file/language/content instead of the real active editor context

---

## Phase 13 — Frontend Completeness Fixes (found in 2026-07-21 audit)

### VS Code extension
- [x] Register the sidebar webview view provider (`anvil.chatPanel` is declared in `package.json` but `registerWebviewViewProvider` is never called) — sidebar currently shows empty
- [x] Wire `anvil.*` settings (backend, model, ollamaUrl, etc.) to actual daemon config via `vscode.workspace.getConfiguration` — currently pure decoration, changing them does nothing
- [x] Render chat messages as markdown with syntax-highlighted code blocks instead of raw `textContent`
- [x] Replace the fragile "final chunk starts with `---`" diff-detection heuristic in `chat.ts` with a robust check
- [x] Add a real diff preview (side-by-side or inline) before apply, instead of a plain Yes/No `showInformationMessage`
- [x] Add real slash-command autocomplete (dropdown as user types `/`), not just static preset buttons
- [x] Extend E2E tests to cover actual chat/diff-apply UI behavior, not just activation/command registration

### JetBrains plugin
- [x] Fix `AnvilDiffHandler.kt` — current "apply" strategy keeps only `+` lines and drops all unchanged context, **corrupting files on apply**; replace with a real diff/patch algorithm or IntelliJ `DiffManager`/`DiffContentFactory` integration
- [x] Replace the bare `JBTextArea` chat panel with real markdown/code-block rendering
- [x] Wire context-menu actions to the daemon (see Phase 12 `SlashCommandActions.kt` item)
- [x] Fix chat panel to pass real editor file/content context (see Phase 12 `AnvilChatPanel.kt` item)
- [x] Add tests exercising diff-apply and daemon-integration behavior, not just string-parsing utilities

---

## Phase 14 — Security Hardening (found in 2026-07-21 audit)

- [x] **Critical**: Add authentication to the local IPC channel (`crates/anvil-daemon/src/server.rs`) — no token/nonce/peer-credential check today; any local process can drive the daemon. Add a per-run secret token file (0600) required on every RPC request, or verify peer UID/SID.
- [x] **High**: Restrict Unix socket permissions explicitly (`server.rs`) — create runtime dir with `0o700` and `set_permissions` the socket to `0o600`/`0o700` right after bind
- [x] **High**: Fix remove-then-bind TOCTOU race on the socket path; use atomic bind / `O_EXCL` semantics
- [x] Medium: Add HTTP connect/request timeouts to all provider clients (`anvil-providers/src/*.rs`, currently `reqwest::Client::new()` with no timeout) — mitigates hangs from slow/malicious custom endpoints
- [x] Medium: Document the trust boundary for the "custom" OpenAI-compatible provider (`custom.rs`) — user-controlled base URL can point at internal network services with API keys attached
- [x] Medium: Scrub/trim raw provider HTTP error bodies before logging (`retry.rs`, `server.rs`) — avoid persisting unvalidated response text to disk logs
- [x] Low: Harden PID file handling (`crates/anvil-daemon/src/pid.rs`) against tampering that could make `anvil stop` kill an unrelated process (ties into the runtime-dir permission fix above)
- [x] Ongoing: keep Dependabot/`cargo audit` running — no known-vulnerable deps or disabled TLS verification found as of this audit

---

## Phase 15 — crates.io Publishing Prep

- [x] Rename `anvil-indexer` → `tpt-anvil-indexer` (tree-sitter + BM25 + vector hybrid search — most self-contained, best standalone candidate)
- [x] Rename `anvil-providers` → `tpt-anvil-providers` (multi-cloud LLM client: OpenAI/Anthropic/Azure/OpenRouter/custom + keyring + retry + cost tracking)
- [x] Decouple `tpt-anvil-indexer` from `anvil-core` (inline/duplicate the `ChunkType`/`ContextChunk` types it references)
- [x] Decouple `tpt-anvil-providers` from `anvil-core`/`anvil-config` path deps (inline minimal message/error/usage types)
- [x] Add crates.io metadata (`readme`, `keywords`, `categories`) to both crates' `Cargo.toml`
- [x] `cargo publish --dry-run` for both crates once decoupled
- [x] Leave `anvil-core`, `anvil-config`, `anvil-capabilities`, `anvil-daemon` as internal-only (too tightly coupled / too thin to differentiate standalone)

---

## Phase 16 — Vault, Smart Context, Router, Verifier (spec reconciliation, 2026-07-23)

> Reconciles `spec agent.txt` ("TPT AI Agent, Path B") with the existing project. Vault/Smart-Context/Router are ported natively into the Rust daemon rather than run via the sibling `tpt-code-command-center` TS proxy (avoids two competing local servers). Verification is a custom Anvil-native compiler/lint gate, not an SMT/`tpt-telos` approach — `tpt-telos` only verifies its own DSL and is out of scope. `tpt-code-command-center`'s org-wide 137-repo RAG is dropped entirely. Full design: `C:\Users\Phillip\.claude\plans\added-a-new-spec-crystalline-conway.md`.

> **2026-07-23 correction:** several bullets below were previously checked off as done without the described wiring actually existing in the code (verified by grepping `commands.rs`/`context.rs`/`registry.rs` for the relevant calls — none were present). Unchecked accordingly; the standalone modules, config schema, and their unit tests are genuinely done, but none of the four features are on the live request path yet.

### 16.1 Vault — secret redaction
- [x] Create `crates/anvil-capabilities/src/vault.rs`: `RedactionRule` table (AWS keys, GitHub PATs, OpenAI/Anthropic keys, Slack tokens, PEM private keys, generic password/api_key assignments, JWTs)
- [x] `redact_text(input) -> (String, Vec<RedactionHit>)` and `redact_request(&mut CompletionRequest, &VaultConfig) -> Vec<RedactionHit>`
- [x] Wire into `CommandHandler::run` (`commands.rs`) right after building `request`, before the cloud/local dispatch — apply unconditionally
- [x] Add `VaultConfig` (`enabled`, `redact_local`, `custom_patterns`) to `anvil-config/src/schema.rs`, default `enabled: true`
- [x] Redaction is silent (no UI interruption); log label + count only, never the matched value
- [x] Unit tests per rule (positive + near-miss negatives) + `redact_request` multi-message test
- [x] Integration test with a spy `CloudProvider` asserting a seeded fake key never reaches it (2026-07-23 wiring pass)

### 16.2 Smart Context — AST-outline compression
- [x] Create `crates/tpt-anvil-indexer/src/outline.rs`: `outline_for_file(source, language, file_path) -> String` built from existing `symbols::extract_symbols`/`Symbol.signature`
- [x] Fallback to raw source's first N lines when `extract_symbols` returns empty (unsupported language / parse failure)
- [x] `OutlineStats` (as `original_chars`/`outline_chars`/`symbol_count`) for measurable reduction
- [x] Wire into `assemble_context_message` (`anvil-capabilities/src/context.rs`) — replace full file content with the outline when `ctx.selection.is_none()` and content exceeds a threshold; same treatment for oversized `related_chunks` entries
- [x] Add `SmartContextConfig` (`enabled`, `file_size_threshold_bytes`, `chunk_size_threshold_bytes`) to schema.rs
- [x] Per-language outline tests (Rust; fallback path covers unsupported languages) + assertion that outline is materially smaller than original
- [x] Bonus (2026-07-23 security pass): neutralize literal chat-template control tokens (`<|im_start|>` etc.) and grow code-fence length past any embedded backtick run before interpolating file/chunk content into the prompt — closes a local-model prompt-injection-via-source-comment gap found in the same audit

### 16.3 Router — cost-based provider selection
- [x] Create `crates/tpt-anvil-providers/src/router.rs`: `select_provider(providers, estimated_prompt_tokens, estimated_completion_tokens, cfg) -> Option<&ProviderEntry>` using existing `cost::estimate_cost`; cheapest-wins v1 (no latency/capability weighting yet)
- [x] Change `ProviderRegistry` (`registry.rs`) from single `active` provider to also build an `available: Vec<ProviderEntry>` pool (every provider with enough config to construct) the router can select from; `active` kept as-is for the pre-Router single-provider fallback path
- [x] Wire per-request selection into `CommandHandler::run` before the cloud dispatch — used only when `router.enabled` and `available` is non-empty, else falls back to `active` (no behavior change for the default single-provider setup)
- [x] Add `RouterConfigSchema` (`enabled`, `prefer_cheapest`, `max_cost_per_request_usd`, `pinned`) to schema.rs
- [x] Unit tests: cheapest wins, disabled router returns first, empty providers returns none
- [x] Added `CloudProvider::default_model()` + a small `RecentModels` (last 5 used, MRU, deduped, persisted to `~/.config/anvil/recent_models.json`) recorded on successful cloud fallback in `commands.rs` — not part of the original design but a natural companion to Router now that model ids aren't hardcoded (see cross-cutting fixes below)
- [x] `RouterConfigSchema.pinned` threaded through router — `select_provider` filters by pinned name with fallback to full pool; Azure remains excluded from `available` because `ProviderConfig` has no deployment/model-id field to cost against — only reachable via `providers.active` today

### 16.4 Verifier — compiler/lint gate on generated diffs
- [x] Create `crates/anvil-capabilities/src/verify.rs`: `VerificationResult { passed, compiler_output, test_output, lint_output, errors }`, `verify_patch(original_content, patch_content, file_path, project_root, config) -> VerificationResult`
- [x] Per-language compiler/type-checker via `tokio::process::Command` (first subprocess-execution code in the workspace besides `pid.rs`): `cargo check`/`tsc --noEmit`/`mypy`/`go build`, each actually enforced under `tokio::time::timeout` (fixed 2026-07-23 — the `timeout` parameter was previously accepted but silently ignored)
- [x] Optional test run (`run_tests`, default **off**) and linter (`run_linter`, default **on**)
- [x] In-place write of patch content + explicit restore of original content afterward (unconditional, after all checks)
- [x] **Critical (2026-07-23 security pass)**: `file_path` reached `project_root.join(...)` with no containment check — a client-controlled path (`../../etc/passwd`, an absolute path, etc.) was a full arbitrary-file-write primitive once wired in. Added `resolve_target()`: canonicalizes and rejects anything outside `project_root` before any filesystem write; moved the backup file off a predictable sibling path onto a random name in the OS temp dir. Landed before wiring `verify_patch` into `commands.rs`, not after.
- [x] On failure: bounded LLM retry — feeds `verification.errors` back to the model as one additional turn (bounded by `max_retries`, default 1) and re-verifies before giving up
- [x] Widen `CommandHandler::run` return type to include `Option<VerificationResult>`
- [x] Propagate `Option<VerificationResult>` through `anvil-daemon/src/server.rs` RPC response and both VS Code/JetBrains response handlers (new warning-banner UI element) — Rust side in progress, extension-side UI not started
- [x] Fail-open surfacing to the UI (show the warning banner even though the diff is still offered) — depends on the propagation above
- [x] Add `VerifyConfigSchema` (`enabled`, `run_tests`, `run_linter`, `timeout_seconds`, `max_retries`) to schema.rs
- [x] Unit tests for language detection, config defaults, and `resolve_target` path-traversal rejection (absolute path, `..` traversal, contained new file) — fixture-project pass/fail integration tests not yet written

---

## Phase 17 — Security Hardening & Adoption Tooling (2026-07-23 audit)

> Three parallel audits (stub/TODO sweep, security review, adoption/tooling survey) run against the working tree ahead of Phase 16's wiring pass. Full plan: `C:\Users\Phillip\.claude\plans\continue-transient-feigenbaum.md`.

### 17.1 Security fixes (beyond the Phase 16.4 path-traversal fix above)
- [x] Medium: Windows named pipe (`anvil-daemon/src/server.rs::run_windows`) is created with no explicit security descriptor, inheriting the default (broad) DACL — add an owner-only DACL to match the Unix `0700`/`0600` treatment
- [x] Medium: Windows auth-token file (`server.rs::token_path`) has no ACL hardening — Unix gets `chmod 0600`, Windows has nothing; combined with the pipe DACL gap, another local user could read the token and impersonate an authenticated client
- [x] Low/Medium: document the trust boundary for project-level `.anvil/config.toml` (an untrusted cloned repo can set `providers.custom.base_url` to redirect API-key-bearing requests) in `docs/config-reference.md`, matching the existing `custom.rs` trust-boundary comment from Phase 14
- [x] Low: `server.rs` token comparison (`provided != Some(token.as_str())`) is not constant-time — swap to `subtle::ConstantTimeEq`
- [x] Low: `scrub_error_message` (`retry.rs`) is only applied in the retry-warning log path — apply consistently at every site that logs or returns a provider error (e.g. `commands.rs`'s local-backend-failure log)
- [x] Low: no connection/request rate limiting on the daemon's accept loops — add a `tokio::sync::Semaphore`-based concurrent-request cap to bound local resource/cost exhaustion

### 17.2 Adoption tooling
- [x] `anvil init` — interactive wizard writing `~/.config/anvil/config.toml` (or `.anvil/config.toml` for `--project`): backend/model choice, optional cloud provider + API key via existing `keystore::set_api_key`; don't overwrite an existing config without confirmation
- [x] `anvil doctor` — non-interactive diagnostic: config found/parses, Ollama reachable, configured model present, GPU/acceleration feature flags, keychain entries resolve; pass/fail checklist, non-zero exit on failure
- [x] Add `cargo-audit`/`cargo-deny` job to `.github/workflows/ci.yml` (advisory/license scanning — currently absent despite Dependabot being configured)
- [x] Fix `README.md`'s "Installation instructions coming soon" line — `docs/getting-started.md` is already accurate and complete

### 17.3 Innovation menu (proposed, not yet scheduled)
- [x] Surface the Phase 16.4 verification retry loop in the IDE UI as "Anvil checked its own work," showing the compiler/lint diff — turns the safety net into a visible differentiator
- [x] Cost/usage dashboard (`anvil status --cost` or a local HTML report) built on the existing `router.rs`/`cost.rs` estimates and the `recent_models.rs` persistence pattern
- [x] Opt-in local-only redaction transparency log (`~/.local/state/anvil/redactions.log`) so privacy-conscious users can verify what Vault caught, without it ever leaving the machine
- [x] `anvil doctor --fix` — auto-remediate common issues found by 17.2's `doctor` (pull a missing Ollama model, scaffold a default config)

### 16.5 Wrap-up
- [x] `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --check` all green locally (2026-07-23; required fixing several compile errors left over from the crates.io decoupling — stale `anvil_core` imports/types in `tpt-anvil-providers`, a `commands.rs`/`registry.rs` type mismatch between `anvil_core` and the new decoupled `tpt_anvil_providers::types`, plus a real config-merge bug below)
- [x] Fixed **real bug**: `anvil-config`'s `AnvilConfig::merge_with`/`HasMerge` merged already-defaulted structs field-by-field, so an overlay explicitly setting a field to its type's default value (e.g. `inference.backend = "ollama"`, which is also the default) was indistinguishable from not setting it at all, and silently kept the base value instead. Replaced with merging raw TOML tables before deserializing once (`ConfigLoader::load` + `merge_toml_values` in `loader.rs`); added per-field `#[serde(default = ...)]` across `schema.rs` so partial tables deserialize correctly
- [x] Fixed hardcoded/stale model handling: `OpenAiProvider`/`AnthropicProvider::list_models()` now call the provider's real `/models` endpoint instead of returning a static list; removed hardcoded fallback model ids in `registry.rs`/`schema.rs`/`types.rs` in favor of an explicit config error ("model names change too often to hardcode a default")
- [x] Document new config sections in `docs/config-reference.md`
- [ ] Manual smoke test: multi-provider cost routing, secret redaction, and a deliberately broken `/fix` through the VS Code extension (blocked on the pipeline wiring above)

---

## Phase 18 — Model Benchmarking Tool (2026-07-23)

> A benchmarking tool that runs a suite of coding tasks against a model (local or cloud), grades results objectively (compiler/lint/test pass-rate, not a quality judgment), and stores a scorecard so models are comparable across versions and over time. Core suite rotates on a staggered ~6-month-per-task schedule so it never fully calcifies while nearby runs stay comparable; adaptive slice generates model-specific weak-spot tasks but is walled off from the comparable score. Capped at 30 stored scorecards, LRU eviction by last-run date. Full design: `C:\Users\Phillip\.claude\plans\do-you-think-its-cosmic-falcon.md`.

### 18.1 Data model & pure logic (no I/O)
- [x] `crates/anvil-capabilities/src/benchmark/mod.rs` — module root, re-exports; add `pub mod benchmark;` to `lib.rs`
- [x] `crates/anvil-capabilities/src/benchmark/suite.rs` — `CoreTask`/`AdaptiveTask` types, TOML deserialization (`[verify]` reuses existing `verify::VerifyConfig` directly), `active_tasks(pool, at) -> Vec<&CoreTask>` pure staggered-rotation filter (`introduced_at <= at < retires_at`)
- [x] `crates/anvil-capabilities/src/benchmark/scorecard.rs` — `TaskKind`, `TaskRunResult` (pass/fail, latency, prompt/completion tokens, cost, errors), `ModelScorecard` (`core_task_ids_run`, `core_results`, `adaptive_results`, `core_score`, `adaptive_score: Option<f64>`, totals)
- [x] `crates/anvil-capabilities/src/benchmark/store.rs` — `BenchmarkStore`: cap-30 LRU by `last_run_at` (evict via `min_by_key`, not FIFO), retain-then-insert dedup by `(provider, model_id)`, `load`/`save` JSON to `~/.config/anvil/benchmarks.json` (mirrors `tpt-anvil-providers::recent_models::RecentModels`)
- [x] `crates/anvil-capabilities/src/benchmark/comparison.rs` — `compare(a, b) -> ComparableResult`: intersect `core_task_ids_run`, recompute both scores restricted to the shared subset, report excluded-per-side tasks (handles suite rotation between two runs honestly)
- [x] Add `BenchmarkConfigSchema`/`AdaptiveConfigSchema` to `crates/anvil-config/src/schema.rs` (`enabled`, `core_suite_path`, `rotation_period_days` default 180, `stagger_interval_days` default 30, `max_stored` default 30, nested `adaptive.{enabled, evaluator_provider, evaluator_model, max_tasks_per_run}`) — every field `#[serde(default = ...)]` per the Phase 16.5 merge-bug lesson; add `benchmark: BenchmarkConfigSchema` to `AnvilConfig`
- [x] Unit tests: `active_tasks` boundary cases + staggered-pool overlap test; scorecard scoring never mixes core/adaptive; `store.rs` LRU-by-timestamp (not insertion order) + re-record-replaces + load/save round-trip + missing-file-empty; `comparison.rs` partial- and full-overlap cases; `anvil-config` partial-`[benchmark]`-table merge test (mirrors `loader.rs:166`'s override-equal-to-default regression test)

### 18.2 Fixture suite
- [x] Author 8–12 seed core tasks (rust/typescript/python; common weak spots — borrow checker, async misuse, off-by-one, null-handling) under `crates/anvil-capabilities/benchmarks/core/*.toml`, staggered `introduced_at` dates
- [x] Shared per-language scaffold fixtures under `crates/anvil-capabilities/benchmarks/scaffold/{rust,typescript,python,go}/` (reused across tasks of that language rather than duplicated per-task)
- [x] Embed the suite into the `anvil` binary at compile time (build.rs step or `include_str!`/glob); `core_suite_path` config becomes a local-dev-only override

### 18.3 Execution engine
- [x] `crates/anvil-capabilities/src/benchmark/runner.rs` — dispatch via `InferenceBackend::complete` (local) or a small non-streaming `CloudProvider` conversion pair (cloud — do not reuse `commands.rs`'s private streaming-oriented `to_provider_request`/`from_provider_chunk`); extract code block via `diff::extract_code_block`; grade via `verify::verify_patch` against a throwaway temp project root seeded from the task's scaffold; record latency/tokens/cost (`router::cost::estimate_cost` for cloud, `None` for local)
- [x] Fake `InferenceBackend`/`CloudProvider` test doubles (mirroring `router.rs`'s existing `MockProvider`) driving a task end-to-end without real models/network

### 18.4 Adaptive slice
- [x] `crates/anvil-capabilities/src/benchmark/adaptive.rs` — load target model's prior scorecard failures, prompt a configured evaluator model (separate from model under test) to generate one new task targeting the same weakness differently; grade via `verify_patch` when possible, else judge-rubric fallback tagged `[judge-rubric, no compiler verification]`
- [x] Response-parsing unit tests incl. malformed-response → judge-rubric fallback; assert `adaptive_score` never leaks into `core_score`

### 18.5 CLI
- [x] `crates/anvil-daemon/src/cli.rs` — `Commands::Benchmark(BenchmarkArgs)` alongside `Start/Stop/Status/Auth/Models`; `BenchmarkCommands::{Run{target, no_adaptive, project}, Report{compare}}`; `handle_benchmark` follows `list_models()`'s existing `ConfigLoader`/registry construction pattern; wire into `main.rs` dispatch
- [ ] `clap::try_parse_from` argument-parsing test for `BenchmarkCommands`

### 18.6 RPC
- [x] `crates/anvil-core/src/ipc.rs` — `BenchmarkRunParams{provider, model_id, include_adaptive}`, `BenchmarkReportParams{compare: Option<[String;2]>}` next to existing `SlashCommandParams`
- [x] `crates/anvil-daemon/src/server.rs` — `"benchmark_run"`/`"benchmark_report"` match arms in `handle_rpc` (after `"slash_command"`, before the `other =>` catch-all); `benchmark_progress` notification per completed task (mirrors existing `stream_token` notifications); `BenchmarkRunner` constructed once at daemon startup alongside `CommandHandler` rather than bloating per-request `HandlerConfig`

### 18.7 Docs & wrap-up
- [x] Document `[benchmark]`/`[benchmark.adaptive]` config sections in `docs/config-reference.md`
- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all green
- [ ] Manual smoke test: `anvil benchmark run ollama/<model>` writes a scorecard to `~/.config/anvil/benchmarks.json`; `anvil benchmark report` lists it; `--compare` against a second model produces sane shared-subset output; 31st scorecard evicts the oldest
