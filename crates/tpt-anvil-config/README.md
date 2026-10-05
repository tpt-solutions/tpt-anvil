# tpt-anvil-config

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-config.svg)](https://crates.io/crates/tpt-anvil-config)
[![Docs.rs](https://docs.rs/tpt-anvil-config/badge.svg)](https://docs.rs/tpt-anvil-config)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

TOML configuration schema, layered loading, and hot-reload for
[TPT Anvil](https://github.com/tpt-solutions/tpt-anvil).

## Features

- `AnvilConfig` — the root schema, with every field defaulted so a partial TOML table still
  deserializes.
- `ConfigLoader` — three-layer fallback chain: built-in defaults, user config, project config.
- `ConfigWatcher` — `notify`-backed hot reload that emits the new config on change.
- Sections: `inference`, `providers` (OpenAI, Azure OpenAI, Anthropic, OpenRouter, custom),
  `indexing`, `ui`, `vault`, `smart_context`, `router`, `verify`, `benchmark`, `benchmark.adaptive`.

## Configuration layers

Layers are merged in ascending priority order:

1. Built-in defaults.
2. User config — `~/.config/anvil/config.toml`.
3. Project config — `<project_root>/.anvil/config.toml`.

## Usage

```rust
use tpt_anvil_config::loader::ConfigLoader;

let config = ConfigLoader::load(Some(std::path::Path::new("/path/to/project")))?;
println!("active backend: {}", config.inference.backend);

// Persist a config back to the user-level file.
ConfigLoader::save_user(&config)?;
# Ok::<(), anyhow::Error>(())
```

Hot reload. The watcher owns the current config behind a shared lock and refreshes it in the
background whenever either config file changes:

```rust
use tpt_anvil_config::watcher::ConfigWatcher;

let watcher = ConfigWatcher::new(Some(std::path::PathBuf::from("/path/to/project")))?;

// Anywhere else: read whatever is current. No restart needed.
let backend = watcher.config.read().await.inference.backend.clone();
# Ok::<(), anyhow::Error>(())
```

## Merge semantics

Layers are merged as raw `toml::Value` tables *before* deserialization into `AnvilConfig`, not as
already-typed structs.

This distinction matters. Merging typed structs cannot tell "the user explicitly set this to the
default value" apart from "the user did not set it", so a real override that happens to equal the
default would be silently dropped. Merging at the `toml::Value` layer preserves that intent.

For the same reason, every new config field must carry a `#[serde(default = ...)]` — a partial
table containing only some of its keys must still deserialize instead of failing on a missing
field. When adding a field, add it with a default and add a merge test alongside it.

## Dependencies

`tpt-anvil-core`, `serde`, `toml`, `anyhow`, `thiserror`, `dirs`, `tokio`, `notify`, `tracing`.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.