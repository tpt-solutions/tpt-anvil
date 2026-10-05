# Changelog — tpt-anvil-daemon

All notable changes to `tpt-anvil-daemon` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Changed
- `anvil benchmark run` now reports its denominator as `[passed/scorable of total]`
  so a score is never mistaken for a full-suite result when tasks were skipped.
- `anvil benchmark report` annotates stored scores computed over a reduced task
  set, e.g. `50% (4)`, meaning 4 of the recorded tasks were scorable.

### Added
- `anvil` binary with `start`, `stop`, `status`, `auth`, `models`, `init`, `doctor`, and
  `benchmark` subcommands.
- JSON-RPC 2.0 IPC server over a Unix domain socket on Linux/macOS and a named pipe on
  Windows, with newline-delimited framing.
- RPC methods: `health`, `status`, `slash_command`, `benchmark.run`, and `benchmark.report`,
  plus `stream_token` and `benchmark_progress` notifications.
- PID file handling with stale-process cleanup via `is_pid_alive()`.

### Security
- IPC authentication: a per-run secret token written to a `0600` file, required on every
  request except `health`.
- Unix socket security: `0700` runtime directory, `0600` socket, and a fix for the TOCTOU
  race during bind.

### Fixed
- `benchmark run` now dispatches to local inference backends (`ollama`, `llama_cpp`,
  `candle`) through a new `BenchmarkExecutor`/`resolve_executor` pair. It previously
  built only a *cloud* `ProviderRegistry`, so the documented command
  `anvil benchmark run ollama/<model>` always failed with
  "no provider named 'ollama' configured".
- Local-backend targets now report a clear error when the requested backend is not the
  configured `inference.backend`.
- The CLI now prints a `SKIP` status and a summary line for tasks excluded because the
  verification toolchain was unavailable, so a partly-provisioned machine no longer
  looks like a bad model.

### Changed
- **Breaking:** crate renamed from `anvil-daemon` to `tpt-anvil-daemon` for workspace-wide
  naming consistency. The binary target remains `anvil`, so the installed command is
  unaffected. Update `use` paths and `Cargo.toml` dependency keys accordingly.
- HTTP timeouts on provider clients: 10s connect and 120s request.
- Error scrubbing: HTTP error bodies are truncated and redacted before logging.
- CLI types (`Cli`, `Commands`, `BenchmarkArgs`, `BenchmarkCommands`, `AuthArgs`,
  `AuthCommands`) now derive `Debug` so argument parsing can be unit tested.
- The RPC `benchmark_run` handler honors `benchmark.core_suite_path` via
  `HandlerConfig::benchmark_suite_path`, matching the CLI.

### Tests
- `cli.rs` argument-parsing tests for `BenchmarkCommands`: `run` with/without
  `--no-adaptive`/`--project`/`-p`, `report` with zero/two targets, rejection of a missing
  `run` target, of a third compare target, and of an unknown subcommand, plus
  `parse_target` splitting and the top-level `anvil benchmark …` and pre-existing
  subcommand paths.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-daemon