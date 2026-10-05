# Changelog — tpt-anvil-core

All notable changes to `tpt-anvil-core` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `AnvilError` covering inference, provider, indexer, config, IPC, IO, serialization,
  unsupported-backend, and model-not-found failures, with `From` conversions for
  `std::io::Error` and `serde_json::Error`.
- Chat and completion model: `ChatMessage`, `Role`, `CompletionRequest`,
  `CompletionResponse`, `TokenUsage`, `StreamChunk`, `ModelInfo`, `BackendKind`.
- Context types: `CodeContext`, `TextRange`, `ContextChunk`, `ChunkType`.
- `DiffPatch` for single-file patch exchange between model, daemon, and editor.
- JSON-RPC 2.0 envelope: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`,
  `JsonRpcNotification`, `StatusResponse`, `StreamTokenNotification`.
- IPC parameter types: `SlashCommandParams`, `RawCompletionParams`, `ApplyDiffParams`,
  `IndexProjectParams`.

### Changed
- **Breaking:** crate renamed from `anvil-core` to `tpt-anvil-core` for workspace-wide
  naming consistency. Update `use` paths and `Cargo.toml` dependency keys accordingly.
- `Role` serializes in lowercase to match the TypeScript and Kotlin client bindings.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-core