# Contributing to TPT Anvil

Thank you for your interest in TPT Anvil!

## Contributions are issues only

This project does **not** accept pull requests. All contributions happen through [GitHub issues](../../issues): bug reports, feature requests, and ideas. Pull requests opened from forks will be closed without review.

Please use the issue templates:
- **Bug report** — for reproducible bugs
- **Feature request** — for new functionality

The sections below are for building and testing the project locally, e.g. to reproduce a bug before reporting it.

## Prerequisites

- Rust (stable toolchain) — install via [rustup](https://rustup.rs/)
- Node.js >= 20 and npm >= 10 (for VS Code extension)
- JDK 17+ and Gradle (for JetBrains plugin)
- Optional: CUDA or ROCm toolkit for GPU-accelerated inference

## Building

```bash
# Rust workspace
cargo build

# VS Code extension
cd extensions/vscode && npm install && npm run build

# JetBrains plugin
cd plugins/jetbrains && ./gradlew buildPlugin
```

## Running Tests

```bash
cargo test
```

## Code Style

- Rust: `cargo fmt` and `cargo clippy --all-targets -- -D warnings`
- TypeScript: `eslint` + `tsc --noEmit`
- Kotlin: `ktlint`

## License

Anything you submit in an issue may be used under the same dual MIT/Apache-2.0 terms as the project.
