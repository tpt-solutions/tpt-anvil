# Contributing to TPT Anvil

Thank you for your interest in contributing to TPT Anvil!

## Developer Certificate of Origin

By making a contribution to this project, you certify that you have the right to submit it under the open source license indicated in the [LICENSE](LICENSE) file, and you agree to the [Developer Certificate of Origin v1.1](https://developercertificate.org/).

Add a `Signed-off-by` line to your commits:

```
git commit -s -m "your commit message"
```

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

## Submitting Changes

1. Fork the repo and create a feature branch
2. Make your changes with appropriate tests
3. Ensure `cargo test` and `cargo clippy` pass
4. Open a pull request with a clear description

## Reporting Issues

Please use the GitHub issue templates:
- **Bug report** — for reproducible bugs
- **Feature request** — for new functionality

## License

By contributing, you agree your contributions are licensed under the same dual MIT/Apache-2.0 terms as the project.
