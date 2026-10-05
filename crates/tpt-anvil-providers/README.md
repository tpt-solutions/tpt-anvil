# tpt-anvil-providers

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-providers.svg)](https://crates.io/crates/tpt-anvil-providers)
[![Docs.rs](https://docs.rs/tpt-anvil-providers/badge.svg)](https://docs.rs/tpt-anvil-providers)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Multi-cloud LLM provider client for [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil).

This crate is intentionally decoupled from the rest of the workspace: it does **not** depend on
`tpt-anvil-core` or `tpt-anvil-config`, and defines its own `types.rs`. That keeps it publishable
standalone to crates.io. Anything bridging these types to `tpt_anvil_core` types does so with an
explicit conversion — see `tpt-anvil-capabilities/src/commands.rs`.

## Features

- `CloudProvider` trait: `name`, `default_model`, `list_models`, `complete`, `stream`,
  `count_tokens`.
- Providers: `OpenAiProvider`, `AzureOpenAiProvider`, `AnthropicProvider`,
  `OpenRouterProvider`, and `CustomProvider` for any OpenAI-compatible endpoint.
- `keystore` — OS-keychain-backed API key storage via `keyring`.
- `retry` — `RetryConfig`, `with_retry` with exponential backoff, and
  `scrub_error_message` for logging.
- `cost` — `pricing_for` and `estimate_cost` for per-provider token and cost estimation.
- `router` — cost-based provider selection, cheapest-first.
- `recent_models` — recently used model history persisted to disk.
- `registry` — `ProviderRegistry::from_config` builds and resolves providers.

## Usage

```rust
use tpt_anvil_providers::openai::OpenAiProvider;
use tpt_anvil_providers::types::{ChatMessage, CompletionRequest, Role};

let provider = OpenAiProvider::new("sk-...", "gpt-4o-mini")?;

let response = provider
    .complete(&CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "Summarise this diff".to_string(),
        }],
        model: None,
        max_tokens: 512,
        temperature: 0.2,
        stream: false,
    })
    .await?;

println!("{}", response.content);
# Ok::<(), tpt_anvil_providers::types::ProviderError>(())
```

API keys in the OS keychain:

```rust
use tpt_anvil_providers::keystore;

keystore::set_api_key("openai", "sk-...")?;
let key = keystore::get_api_key("openai")?;
# Ok::<(), anyhow::Error>(())
```

## Reliability

`with_retry` applies exponential backoff on transient failures. Because provider error bodies can
echo back credentials, `scrub_error_message` truncates and redacts them before they reach a log
sink — always run an error message through it before logging.

## Dependencies

`serde`, `serde_json`, `anyhow`, `thiserror`, `tokio`, `async-trait`, `reqwest`, `tracing`,
`regex`, `keyring`, `futures-util`. Dev-dependency: `wiremock`.

## Testing

Integration tests run against a local mock HTTP server (`wiremock`) rather than a real vendor, so
the suite needs no API keys and no network access:

```bash
cargo test -p tpt-anvil-providers
```

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.
