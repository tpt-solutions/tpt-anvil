# Changelog — tpt-anvil-capabilities

All notable changes to `tpt-anvil-capabilities` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `syntax` module: toolchain-free syntax validation via tree-sitter —
  `syntax_errors`, `is_syntactically_valid`, `grammar_for`, and `format_errors`.
  Reports `ERROR`/`MISSING` nodes with 1-based positions, bounded to 10
  reported errors. `None` means "no grammar for this language", which is
  distinct from "parsed clean".
- Tree-sitter syntax fallback in the verification gate. When a real compiler or
  linter is unavailable, verification degrades to a syntax check instead of
  failing or excluding the task.
- `extract_fenced_tag` for pulling a block with one specific fence info string.
- Slash command engine: `Command` and `CommandHandler` covering `/generate`, `/test`,
  `/explain`, `/fix`, `/docs`, and `/chat`.

### Fixed
- **Benchmark scoring silently shrank the denominator.** Both TypeScript tasks
  and both Python tasks were skipped on any machine without `tsc`/`mypy`, so a
  reported `50%` could mean 50% of 4 tasks rather than of 8 — making runs
  incomparable across machines without saying so.
- **A missing toolchain was reported as a model defect.** `npx tsc` and
  `npx eslint` fail with a placeholder stub, and `npx` can instead hang until
  the subprocess timeout when it reaches for the network. Both are now
  recognized as a missing toolchain and fall back to the syntax check. A
  timeout counts as a missing toolchain, not as a lint failure.
- **The fence info string leaked into graded code.** `extract_code_block` only
  recognized full language names, so a ` ```py ` or ` ```ts ` block fell
  through to the bare-fence branch and kept the tag as line 1 of the extracted
  code — a syntax error at 1:1 for every such response. Fences are now scanned
  in document order, short aliases (`py`, `ts`, `js`, `rs`, `tsx`, `golang`,
  …) are normalized, and the info string is always stripped.
- A ` ```diff ` block preceding the real answer no longer wins over it; the
  first block naming a known language does.
- `verify_patch` no longer records an error entry on a passing result when it
  falls back to the syntax check.
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

### Fixed
- `runner::grade_task` materializes each task's scaffold into a disposable temp
  directory (`runner::Sandbox`, removed on drop) instead of resolving it relative to the
  user's project. Scaffolds previously resolved to `project/scaffold/<lang>/…`, which
  never exists, so every task failed with "scaffold read error".
- The benchmark core suite and scaffold fixtures are now embedded at compile time via
  `include_str!`. `load_builtin_tasks` previously read `env!("CARGO_MANIFEST_DIR")` at
  runtime, which only exists on the build machine — a released binary loaded zero tasks.
- `materialize_scaffold` writes the whole scaffold directory, so companion manifests
  (`Cargo.toml`, `tsconfig.json`) land in the sandbox and the verification toolchain can
  actually run.
- Model responses are passed through `diff::extract_code_block` before verification.
  Raw responses include markdown fences, so `cargo check`/`tsc` failed on backticks
  rather than on the model's actual code.
- `verify` resolves `npx`/`npm` to their `.cmd` shims on Windows. A Node install ships an
  extensionless `npx` shell script alongside `npx.cmd`; `Command::new("npx")` failed with
  "program not found".
- `verify` resolves the Python interpreter to a real executable and rejects the
  `WindowsApps` Store alias stub, which prints "Python was not found" instead of
  running. The old hardcoded `python3` name hit exactly that stub.
- Tasks whose verification toolchain is unavailable are now marked `skipped` and
  excluded from the score rather than counted as model failures. A missing `mypy` says
  nothing about the model's ability; previously it silently depressed scores and made
  runs incomparable across machines.
- The `rust-off-by-one-01` scaffold now contains the `second_half` function its prompt
  actually tests.

### Changed
- `TaskRunResult` gains `skipped: bool` (`#[serde(default)]`, so existing scorecards still
  deserialize). `compute_score` excludes skipped tasks from numerator and denominator.
- `runner::grade_task` dropped its `project_root` parameter.
- `benchmark::load_builtin_tasks` takes an `Option<&Path>` override directory.
- `HandlerConfig` gains `benchmark_suite_path: Option<PathBuf>` and a `Default` impl.

### Tests
- Suite integrity: embedded tasks parse, ids are unique, every task resolves to an
  embedded scaffold, and scaffolds materialize with their manifests.
- Sandbox lifecycle: seeding, cleanup on drop, isolation between sandboxes, unknown
  scaffold error, and that `grade_task` never writes into the current project.
- `grade_task` strips markdown fences, verified to fail without stripping since fenced
  content breaks `cargo check`.
- `verify`: `which`, Store-stub rejection, `node_launcher` resolution plus a spawn test,
  and `is_toolchain_missing` detecting missing mypy/typescript/spawn failures while
  ignoring real compile errors.
- `compute_score` excludes skipped tasks; `skipped` round-trips through JSON and defaults
  to `false` for legacy scorecards.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-capabilities