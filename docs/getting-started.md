# Getting Started

## Prerequisites

- **Rust** (stable) — [rustup.rs](https://rustup.rs/)
- **Ollama** — [ollama.ai](https://ollama.ai/) (recommended for quickest setup)
- **VS Code** or a **JetBrains IDE**

## Install the Daemon

```bash
# Build from source
git clone https://github.com/tpt-solutions/tpt-anvil
cd tpt-anvil
cargo install --path crates/anvil-daemon

# Or download a pre-built binary from GitHub Releases
```

## Pull a Model (Ollama)

```bash
ollama pull deepseek-coder:6.7b
# or
ollama pull qwen2.5-coder:7b
```

## Start the Daemon

```bash
# In your project directory
anvil start --project .
```

The daemon indexes your project and starts listening for IDE connections.

## Install the VS Code Extension

Install from the VS Code Marketplace (search "TPT Anvil") or install the `.vsix` from GitHub Releases:

```bash
code --install-extension tpt-anvil-*.vsix
```

## Using Slash Commands

Open the Anvil chat panel from the activity bar or with `Ctrl+Shift+P → Anvil: Open Chat Panel`.

| Command | Usage |
|---------|-------|
| `/generate` | `/generate a function that parses JSON and returns a typed struct` |
| `/test` | Select a function, then `/test` |
| `/explain` | Select any code, then `/explain` |
| `/fix` | Select buggy code + paste the error, then `/fix` |
| `/docs` | Select a function, then `/docs` |

## Configuration

See [config-reference.md](config-reference.md) for all options.

Quick example at `~/.config/anvil/config.toml`:

```toml
[inference]
backend = "ollama"
model = "deepseek-coder:6.7b"
ollama_url = "http://localhost:11434"

[providers]
active = "anthropic"  # optional cloud fallback

[indexing]
top_k = 10
```
