# Changelog — tpt-anvil-inference

All notable changes to `tpt-anvil-inference` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `InferenceBackend` trait covering `name`, `list_models`, `complete`, `stream`, and
  `count_tokens`.
- `OllamaBackend` — HTTP backend for a running Ollama server (default feature).
- `LlamaCppBackend` — in-process GGUF loading and sampler-based inference via `llama-cpp-2`.
- `CandleBackend` — in-process GGUF parsing, tensor ops, and greedy/temperature sampling.
- `BackendRegistry` for building configured backends from `AnvilConfig`.
- `prompt::format_prompt`, `PromptTemplate`, and `apply_chat_template` for model-specific
  chat templates.
- Hardware acceleration feature flags (`cuda`, `rocm`, `webgpu`) plus the `accel` module:
  `AccelDevice`, `AccelPreference`, and `select_device`.

### Changed
- **Breaking:** crate renamed from `anvil-inference` to `tpt-anvil-inference` for
  workspace-wide naming consistency. Update `use` paths and `Cargo.toml` dependency keys
  accordingly.

### Fixed
- `LlamaCppBackend` now performs real GGUF model loading instead of a stub.
- `CandleBackend` now performs real GGUF file parsing and tensor operations.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-inference