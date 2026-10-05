# Changelog — tpt-anvil-config

All notable changes to `tpt-anvil-config` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `AnvilConfig` root schema with defaulted fields throughout, plus `InferenceConfig`,
  `ProvidersConfig`, `OpenAiConfig`, `AzureConfig`, `AnthropicConfig`, `OpenRouterConfig`,
  `CustomProviderConfig`, `IndexingConfig`, and `UiConfig`.
- `ConfigLoader` with the defaults → user → project fallback chain, plus `config_path`,
  `load_file`, and `save_user`.
- `ConfigWatcher` — `notify`-backed hot reload.
- `VaultConfig` and `CustomPatternConfig` for secret-redaction rules.
- `SmartContextConfig` for context-budget and outline controls.
- `RouterConfigSchema` for cost-based provider routing.
- `VerifyConfigSchema` for the compiler/lint/test verification gate.
- `BenchmarkConfigSchema` and `AdaptiveConfigSchema` for the model benchmark suite.

### Changed
- **Breaking:** crate renamed from `anvil-config` to `tpt-anvil-config` for workspace-wide
  naming consistency. Update `use` paths and `Cargo.toml` dependency keys accordingly.
- `ConfigLoader::load` now merges layers as raw `toml::Value` tables before deserializing,
  so an explicit override that happens to equal a field's default is no longer dropped.
- Every config field carries a `#[serde(default = ...)]`, so partial tables deserialize
  instead of erroring on a missing field.

### Fixed
- Merging a partial `[benchmark]` table no longer clobbers sibling settings from lower layers.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-config