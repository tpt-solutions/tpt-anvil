# tpt-anvil-capabilities

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-capabilities.svg)](https://crates.io/crates/tpt-anvil-capabilities)
[![Docs.rs](https://docs.rs/tpt-anvil-capabilities/badge.svg)](https://docs.rs/tpt-anvil-capabilities)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

The AI capability layer for [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil): slash
commands, prompt and context assembly, conversation state, unified diffs, secret redaction,
compilation-based verification, and a model benchmark suite.

This crate is the glue between the local inference backends and the cloud providers. It owns the
conversion between `tpt_anvil_core` types (used by the local backends) and
`tpt_anvil_providers` types — those two crates intentionally do not share types, so the
conversions here are explicit on purpose.

## Features

- **Slash commands** — `Command`, `CommandHandler`: `/generate`, `/test`, `/explain`, `/fix`,
  `/docs`, `/chat`.
- **Context assembly** — `build_system_prompt`, `assemble_context_message`, `build_messages`,
  with budget-aware selection and AST-outline compression.
- **Conversation state** — `Conversation`, `ConversationStore`.
- **Diff engine** — `DiffEngine::extract_diff`, `compute_diff`, `apply_diff`, and
  `extract_code_block`. Patching is hunk-aware.
- **Vault** — `redact_text`, `redact_request`, `log_redactions`. Built-in rules cover AWS,
  GitHub, OpenAI, Anthropic, and Slack keys, PEM private keys, JWTs, and generic patterns,
  plus user-supplied `CustomPattern`s.
- **Verification** — `verify_patch` runs the project's compiler, linter, and tests against a
  candidate patch, with a fail-open retry policy.
- **Benchmark** — `load_builtin_tasks`, `grade_task`, `core_score`, `ModelScorecard`,
  `BenchmarkStore`, `comparison::compare`, and adaptive task generation.

## Usage

```rust
use tpt_anvil_capabilities::commands::{Command, CommandHandler};
use tpt_anvil_capabilities::diff::DiffEngine;

let (command, rest) = Command::parse("/generate a retry helper for the http client");
assert_eq!(command, Command::Generate);

// Pull a unified diff out of raw model output.
if let Some(patch) = DiffEngine::extract_diff(&model_output, "src/http.rs") {
    let updated = DiffEngine::apply_diff(&original_contents, &patch)?;
}
```

Redacting secrets before anything leaves the machine:

```rust
use tpt_anvil_capabilities::vault::{log_redactions, redact_text, VaultConfig};

let config = VaultConfig::default();
let (safe_text, hits) = redact_text(&prompt_containing_a_key, &config);
for hit in &hits {
    println!("redacted {} occurrence(s) of {}", hit.count, hit.label);
}
log_redactions(&hits, Some("/generate"));
```

## Dependencies

`tpt-anvil-core`, `tpt-anvil-config`, `tpt-anvil-inference`, `tpt-anvil-providers`,
`tpt-anvil-indexer`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tokio`, `async-trait`,
`tracing`, `regex`, `once_cell`, `dirs`, `toml`.

## Design notes

Do not re-add a `tpt-anvil-providers` → `tpt-anvil-core` dependency. The providers crate is
decoupled from core on purpose so it can publish standalone to crates.io; bridging the two is
this crate's responsibility.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.