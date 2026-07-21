# tpt-anvil-providers

Multi-cloud LLM provider client for TPT Anvil.

- Unified `Provider` trait over OpenAI, Azure OpenAI, Anthropic, OpenRouter, and generic OpenAI-compatible endpoints.
- OS-keychain-backed API key storage via `keyring`.
- Retry with exponential backoff, and per-provider token/cost estimation.

Part of the [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil) project. Dual-licensed under MIT OR Apache-2.0.
