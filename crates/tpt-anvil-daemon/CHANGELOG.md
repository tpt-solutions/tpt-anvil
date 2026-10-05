# Changelog — tpt-anvil-daemon

All notable changes to `tpt-anvil-daemon` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

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

### Changed
- **Breaking:** crate renamed from `anvil-daemon` to `tpt-anvil-daemon` for workspace-wide
  naming consistency. The binary target remains `anvil`, so the installed command is
  unaffected. Update `use` paths and `Cargo.toml` dependency keys accordingly.
- HTTP timeouts on provider clients: 10s connect and 120s request.
- Error scrubbing: HTTP error bodies are truncated and redacted before logging.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-daemon