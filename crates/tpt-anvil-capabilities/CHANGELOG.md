# Changelog — tpt-anvil-capabilities

All notable changes to `tpt-anvil-capabilities` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- Slash command engine: `Command` and `CommandHandler` covering `/generate`, `/test`,
  `/explain`, `/fix`, `/docs`, and `/chat`.
- Context assembly: `build_system_prompt`, `assemble_context_message`, and `build_messages`
  with budget-aware context selection.
- Conversation history: `Conversation` and `ConversationStore`.
- `DiffEngine` with `extract_diff`, `compute_diff`, `apply_diff`, and `extract_code_block`.
- `vault` module with secret-redaction rules for AWS, GitHub, OpenAI, Anthropic, and Slack
  keys, PEM private keys, JWTs, and generic patterns, plus user-defined custom patterns.
- `verify` module: compiler, linter, and test verification gate with fail-open retry logic.
- `benchmark` module: task loading, `grade_task`, `core_score`, `ModelScorecard`,
  `BenchmarkStore` (cap-30 LRU by `last_run_at`), `comparison::compare` on the shared task
  subset, and adaptive task generation.
- Seed benchmark suite and per-language scaffold fixtures under `benchmarks/`.

### Changed
- **Breaking:** crate renamed from `anvil-capabilities` to `tpt-anvil-capabilities` for
  workspace-wide naming consistency. Update `use` paths and `Cargo.toml` dependency keys
  accordingly.
- `DiffEngine::apply_diff` is now a full hunk-aware unified-diff patcher.
- Explicit conversion functions between `tpt_anvil_core` and `tpt_anvil_providers` types keep
  the providers crate decoupled from core.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-capabilities