# tpt-anvil-daemon

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-daemon.svg)](https://crates.io/crates/tpt-anvil-daemon)
[![Docs.rs](https://docs.rs/tpt-anvil-daemon/badge.svg)](https://docs.rs/tpt-anvil-daemon)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

The TPT Anvil background daemon and its `anvil` command-line binary. It owns process lifecycle,
the authenticated IPC endpoint, and the JSON-RPC dispatch that the VS Code extension and the
JetBrains plugin talk to. The extensions are thin clients — all inference, indexing, and AI
capability work happens here.

## Features

- **IPC server** over a Unix domain socket (`$XDG_RUNTIME_DIR/anvil/anvil.sock`) on Linux and
  macOS, and a named pipe on Windows.
- **JSON-RPC 2.0** dispatch with newline-delimited framing and server-to-client notifications.
- **Per-run authentication**: a secret token is written to a `0600` file at startup and must
  accompany every request except `health`.
- **CLI** with `start`, `stop`, `status`, `auth`, `models`, `init`, `doctor`, and `benchmark`.
- **Lifecycle hardening**: stale PID cleanup with an `is_pid_alive()` check, and a `0700` runtime
  directory.

## CLI

```
anvil start [--project <dir>]
anvil stop
anvil status [--cost]
anvil auth ...
anvil models
anvil init [--project]
anvil doctor [--fix]
anvil benchmark run <provider/model> [--no-adaptive] [--project <dir>]
anvil benchmark report [<provider/model> <provider/model>]
```

## IPC methods

| Method | Params | Result |
| --- | --- | --- |
| `health` | none | `{ status, version }` |
| `status` | none | `StatusResponse` |
| `slash_command` | `SlashCommandParams` | streamed tokens, then the final result |
| `benchmark.run` | `BenchmarkRunParams` | scorecard, with `benchmark_progress` notifications |
| `benchmark.report` | `BenchmarkReportParams` | comparison result |

Unknown methods return JSON-RPC error `-32601`. Streaming responses are delivered as
`stream_token` notifications.

## Usage

```bash
# Start the daemon for a project
anvil start --project .

# Check that it is up
anvil status

# Stop it
anvil stop
```

Every request other than `health` must carry the token from the per-run auth file:

```json
{"jsonrpc":"2.0","id":1,"method":"health"}
{"jsonrpc":"2.0","id":2,"method":"slash_command","params":{"auth":"<token>","command":"/generate"}}
```

## Security model

The daemon is intended to be exclusively local, and the transport is treated as untrusted
anyway:

- The auth token is written with mode `0600` and regenerated on every run.
- The socket itself is created with mode `0600` inside a `0700` runtime directory.
- Binding is guarded against a TOCTOU race.
- HTTP error bodies returned by upstream providers are truncated and scrubbed before logging.

## Dependencies

`tpt-anvil-core`, `tpt-anvil-config`, `tpt-anvil-inference`, `tpt-anvil-providers`,
`tpt-anvil-indexer`, `tpt-anvil-capabilities`, `serde`, `serde_json`, `anyhow`, `thiserror`,
`tokio`, `tokio-util`, `async-trait`, `tracing`, `tracing-subscriber`, `clap`, `dirs`, `rand`,
`subtle`, `reqwest`, and `libc` on Unix.

The binary target is named `anvil`, not `tpt-anvil-daemon`, so `cargo install` gives you a normal
command:

```bash
cargo install --path crates/tpt-anvil-daemon
anvil --help
```

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.