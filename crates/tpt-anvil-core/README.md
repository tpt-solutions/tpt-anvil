# tpt-anvil-core

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-core.svg)](https://crates.io/crates/tpt-anvil-core)
[![Docs.rs](https://docs.rs/tpt-anvil-core/badge.svg)](https://docs.rs/tpt-anvil-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Shared foundation types for the [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil) workspace:
the error enum, the chat/completion data model, and the JSON-RPC 2.0 IPC envelope used by the
daemon and its editor clients.

This crate is deliberately tiny and dependency-light. Every other workspace crate that talks to
the daemon or to a model depends on it, so it must not grow a dependency on any sibling crate.

## Features

- `AnvilError` / `Result<T>` — one error type for inference, provider, indexer, config, IPC, and
  IO failures, with `thiserror`-derived `Display` and `From` conversions for `std::io::Error`
  and `serde_json::Error`.
- Chat and completion types: `ChatMessage`, `Role`, `CompletionRequest`, `CompletionResponse`,
  `TokenUsage`, `StreamChunk`, `ModelInfo`, `BackendKind`.
- Context types: `CodeContext`, `TextRange`, `ContextChunk`, `ChunkType`.
- `DiffPatch` — a single-file patch exchanged between the model, the daemon, and the editor.
- JSON-RPC 2.0 envelope: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`,
  `JsonRpcNotification`, plus the `StatusResponse` and `StreamTokenNotification` payloads.

## Usage

```rust
use tpt_anvil_core::ipc::{JsonRpcRequest, JsonRpcResponse};
use tpt_anvil_core::types::{ChatMessage, CompletionRequest, Role};

let request = JsonRpcRequest::new(1, "slash_command", serde_json::json!({
    "command": "/generate",
    "context": null,
}));

let completion = CompletionRequest {
    messages: vec![ChatMessage {
        role: Role::User,
        content: "Write a Rust unit test for this function".to_string(),
    }],
    model: None,
    max_tokens: 1024,
    temperature: 0.2,
    stream: true,
};
```

Building a response never requires constructing the error variant by hand:

```rust
use tpt_anvil_core::ipc::JsonRpcResponse;

let ok = JsonRpcResponse::ok(1, serde_json::json!({"status": "ok"}));
let err = JsonRpcResponse::err(1, -32601, "method not found: nope");
```

## Dependencies

`serde`, `serde_json`, `thiserror`, `tokio`, `async-trait`, `anyhow`. It depends on no other
workspace crate.

## Design notes

`Role` is serialized in lowercase (`"system"`, `"user"`, `"assistant"`) because it crosses the
IPC boundary to clients written in TypeScript and Kotlin.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.