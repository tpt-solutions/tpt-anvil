<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Contributing to TPT Anvil

Thank you for your interest in contributing. This document covers everything you need to get the project building and your changes submitted.

## Prerequisites

- **Rust (stable)** — install via [rustup.rs](https://rustup.rs/). The workspace uses the stable toolchain.
- **Node.js 18+** — required for the VS Code extension. Download from [nodejs.org](https://nodejs.org/) or via a version manager such as `nvm`.
- **Ollama** — used for local model inference during testing. Follow the setup guide at [ollama.com](https://ollama.com/) and pull a supported model before running integration tests.

## Building the Rust Workspace

```sh
cargo build --all
```

This compiles every crate in the workspace, including `anvil-core`, `anvil-config`, `anvil-indexer`, `anvil-daemon`, and all others.

## Running the Daemon

```sh
cargo run -p anvil-daemon -- start
```

The daemon listens on a local socket and orchestrates indexing, search, and model interactions. See `docs/architecture.md` for a full description of the runtime components.

## Building the VS Code Extension

```sh
cd extensions/vscode
npm install
npm run build
```

The compiled extension is written to `extensions/vscode/out/`. Load it in VS Code via **Extensions → Install from VSIX** or by opening the `extensions/vscode` folder as a workspace and pressing `F5` to launch the Extension Development Host.

## Building the JetBrains Plugin

```sh
cd plugins/jetbrains
./gradlew buildPlugin
```

The plugin archive is produced under `plugins/jetbrains/build/distributions/`.

## Running Tests

```sh
cargo test --all
```

Individual crates can be tested in isolation with `cargo test -p <crate-name>`. For integration tests that require Ollama, ensure the daemon is running and a model is available before executing the test suite.

## Code Style

### Rust

- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`

All clippy warnings are treated as errors in CI. Fix them before opening a pull request.

### TypeScript (VS Code extension)

- Type check: `npx tsc --noEmit` (from `extensions/vscode/`)
- Lint: `npx eslint src --ext .ts` (from `extensions/vscode/`)

## Submitting Changes

1. **Fork** the repository on GitHub.
2. **Create a branch** from `main` with a descriptive name, e.g. `feat/ruby-grammar` or `fix/indexer-crash`.
3. Make your changes, ensuring all tests pass and the linters are clean.
4. **Signed commits are preferred.** Configure Git commit signing with a GPG key or SSH signing key.
5. Add a [DCO](https://developercertificate.org/) sign-off to each commit:
   ```sh
   git commit -s -m "your commit message"
   ```
6. **Open a pull request** against `main`. Fill in the PR description with a summary of the change and how to test it.
7. Address any review feedback. Once approved and CI passes, a maintainer will merge.
