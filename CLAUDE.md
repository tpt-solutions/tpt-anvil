# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

TPT Anvil is a privacy-first, locally-runnable AI development environment (an open alternative to Copilot/Cursor). It's a Rust daemon (`anvil-daemon`, binary name `anvil`) that does all inference, indexing, and AI-capability work, talked to over JSON-RPC by a VS Code extension and a JetBrains plugin. Pre-alpha, active development.

## Repo layout

This is a polyglot monorepo: a Cargo workspace (`crates/`) plus an npm workspace (`extensions/vscode`) plus a standalone Gradle project (`plugins/jetbrains`). They are built and tested independently — there is no unified top-level build command.

## Commands

### Rust workspace (`crates/`)

```sh
cargo check --workspace --all-targets   # fastest correctness check
cargo test --workspace                  # all crates
cargo test -p anvil-capabilities --lib  # single crate
cargo test -p anvil-config loader::tests::merge_optional_fields  # single test
cargo clippy --workspace --all-targets -- -D warnings  # matches CI exactly
cargo fmt --all -- --check              # matches CI exactly (use `cargo fmt` to fix)
```

**Windows-specific gotcha:** in a Git-Bash/MSYS shell, `link.exe` on PATH often resolves to Git's own coreutils `link.exe`, not the MSVC linker, and the default `stable-x86_64-pc-windows-gnu` toolchain may be missing `dlltool.exe`. If `cargo check`/`test` fails with `dlltool.exe: program not found` or `link: extra operand`, prepend an MSYS2 mingw64 bin dir that has `dlltool.exe` to PATH for the command, e.g.:
```sh
export PATH="/c/msys64/mingw64/bin:$PATH" && cargo check --workspace --all-targets
```
This is an environment issue, not a code issue — don't "fix" it by changing Cargo.toml or code.

### VS Code extension (`extensions/vscode/`)

```sh
npm run build              # esbuild bundle to dist/extension.js
npm run typecheck          # tsc --noEmit
npm run lint               # eslint src --ext .ts
npm run test                # vitest run (unit tests)
npm run pretest:e2e && npm run test:e2e   # compile then run @vscode/test-electron E2E suite
```

### JetBrains plugin (`plugins/jetbrains/`)

```sh
gradle test --no-daemon    # JUnit 5 tests
```

### CI

`.github/workflows/ci.yml` runs, per job: Rust (fmt check, clippy `-D warnings`, test) on ubuntu/macos/windows; TypeScript (typecheck, lint, vitest, E2E compile, build); JetBrains (gradle test); and a coverage job (cargo-llvm-cov + vitest coverage → Codecov). Match these exact invocations locally before pushing.

## Architecture

### Client–server split

IDE extensions never talk to models or embed logic directly — they are thin JSON-RPC 2.0 clients (newline-delimited messages) to `anvil-daemon`, over a Unix socket (`$XDG_RUNTIME_DIR/anvil/anvil.sock`) or Windows named pipe. RPC requests carry a per-run auth token written to a 0600 file at daemon startup — the daemon is meant to be exclusively local. See `docs/architecture.md` for the full IPC method table and message shapes.

### Crate roles (dependency flows roughly top-to-bottom)

| Crate | Role |
|-------|------|
| `anvil-core` | Shared types, error types, IPC protocol — used internally by everything except the two crates below |
| `anvil-config` | `AnvilConfig` schema + `ConfigLoader` (see merge behavior below) |
| `anvil-inference` | `InferenceBackend` trait; Ollama, llama.cpp, candle backends |
| `tpt-anvil-providers` | `CloudProvider` trait; OpenAI, Anthropic, OpenRouter, Azure, custom-endpoint. **Deliberately decoupled from `anvil-core`/`anvil-config`** (publishable standalone to crates.io) — defines its own `types.rs` (`CompletionRequest`, `ChatMessage`, etc.), so anything that bridges this crate to `anvil-core` types (e.g. `anvil-capabilities/src/commands.rs`, `anvil-daemon/src/server.rs`) needs an explicit conversion function, not a shared type. Don't "fix" this by re-adding an `anvil-core` dependency. |
| `tpt-anvil-indexer` | Tree-sitter parsing, symbol/AST-outline extraction, SQLite FTS5 + vector hybrid search, file watcher. Also decoupled from `anvil-core` for the same reason (own `types.rs`). |
| `anvil-capabilities` | Slash commands (`/generate /test /explain /fix /docs`), diff engine, context assembly, conversation history — the glue layer that bridges `anvil-inference` (uses `anvil_core` types) and `tpt-anvil-providers` (uses its own types) |
| `anvil-daemon` | Binary `anvil`: JSON-RPC server, CLI, daemon lifecycle (PID file, socket permissions) |

### Config loading and merging

`ConfigLoader::load` (`anvil-config/src/loader.rs`) layers three sources — built-in defaults, `~/.config/anvil/config.toml`, then `<project>/.anvil/config.toml` (highest priority) — by merging raw **TOML tables** before deserializing into `AnvilConfig` once, not by merging already-typed/defaulted Rust structs field-by-field. This matters: struct-level merging can't distinguish "explicitly set to a value that happens to equal the type's default" from "not set at all," which silently drops real overrides. If you touch config merging, keep it at the `toml::Value` layer (`merge_toml_values`) and make sure every new config field has a `#[serde(default = ...)]` so partial tables (only some keys present) still deserialize instead of erroring on "missing field."

### Cloud provider models are never hardcoded

`CloudProvider::list_models()` implementations call the provider's real `/models` endpoint rather than returning a static list — model names change too often for a hardcoded list to stay current. Provider config (`providers.openai.model`, `providers.anthropic.model`, etc.) has no hardcoded fallback either; leaving it unset is a config error at `ProviderRegistry::from_config`, not a silent default. Don't reintroduce a hardcoded default model id anywhere in `tpt-anvil-providers`.

### In-progress features (standalone, not yet wired into the request path)

`vault.rs` (secret redaction), `outline.rs` (AST-outline "Smart Context" compression), `router.rs` (cost-based provider selection), `verify.rs` (compiler/lint/test gate on generated diffs), and `recent_models.rs` (last-5-models-used tracker) all exist with their own config schema and unit tests, but none are called from the live `CommandHandler::run` request path yet (see `todo.md` Phase 16 for exact status per feature — its checkboxes are kept accurate to what's actually wired, not just what module exists). Don't assume a feature is active in the running daemon just because its module and tests exist.
