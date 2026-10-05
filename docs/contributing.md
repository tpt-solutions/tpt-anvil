<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Contributing to TPT Anvil

Thank you for your interest in TPT Anvil.

**Contributions are issues only.** This project does not accept pull requests; please open a [GitHub issue](https://github.com/tpt-solutions/tpt-anvil/issues) for bug reports, feature requests, and ideas. This document covers building and testing locally, e.g. to reproduce a bug before reporting it.

## Prerequisites

- **Rust (stable)** — install via [rustup.rs](https://rustup.rs/). The workspace uses the stable toolchain.
- **Node.js 18+** — required for the VS Code extension. Download from [nodejs.org](https://nodejs.org/) or via a version manager such as `nvm`.
- **Ollama** — used for local model inference during testing. Follow the setup guide at [ollama.com](https://ollama.com/) and pull a supported model before running integration tests.

## Building the Rust Workspace

```sh
cargo build --all
```

This compiles every crate in the workspace, including `tpt-anvil-core`, `tpt-anvil-config`, `tpt-anvil-indexer`, `tpt-anvil-daemon`, and all others.

## Running the Daemon

```sh
cargo run -p tpt-anvil-daemon -- start
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

All clippy warnings are treated as errors in CI. Keep the tree clean when reproducing issues.

### TypeScript (VS Code extension)

- Type check: `npx tsc --noEmit` (from `extensions/vscode/`)
- Lint: `npx eslint src --ext .ts` (from `extensions/vscode/`)

## Reporting Issues

Open an issue using the bug report or feature request template. Include steps to reproduce, your OS, and relevant versions.
