# Changelog — tpt-anvil-providers

All notable changes to `tpt-anvil-providers` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `CloudProvider` trait covering `name`, `default_model`, `list_models`, `complete`, `stream`,
  and `count_tokens`.
- Providers: `OpenAiProvider`, `AzureOpenAiProvider`, `AnthropicProvider`,
  `OpenRouterProvider`, and `CustomProvider` for generic OpenAI-compatible endpoints.
- `keystore` module backed by the OS keychain via `keyring`.
- `retry` module with `RetryConfig`, `with_retry` exponential backoff, and
  `scrub_error_message`.
- `cost` module with `pricing_for` and `estimate_cost` for token and cost estimation.
- `router` module performing cost-based provider selection, cheapest-first.
- `recent_models` module for persisted recently-used model history.
- `registry` module with `ProviderRegistry::from_config`.
- Integration tests against a mock HTTP server (`wiremock`), so the suite needs no API keys
  and no network access.

### Changed
- HTTP timeouts: 10s connect and 120s request on provider clients.
- Error scrubbing: HTTP error bodies are truncated and redacted before logging.

### Notes
- This crate is deliberately decoupled from `tpt-anvil-core` and `tpt-anvil-config` so it can
  be published standalone to crates.io. It defines its own `types.rs`; bridging to core types
  happens in `tpt-anvil-capabilities` via explicit conversions. Please do not re-add a core
  dependency.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-providers