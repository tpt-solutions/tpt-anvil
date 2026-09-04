# TPT Anvil

AI-powered code assistant for IntelliJ-based IDEs with local-first privacy.

## Overview

TPT Anvil brings AI code generation, explanation, and refactoring directly into
your JetBrains IDE. All processing can run locally — no code leaves your machine
unless you choose a cloud provider.

## Features

- **Chat Panel** — Conversational code assistance in a dedicated tool window
- **Inline Actions** — Generate, explain, fix, and test code from the editor
- **Diff Preview** — Review and apply AI-generated changes with a side-by-side diff
- **Multi-Backend** — Supports Ollama, llama.cpp, OpenAI, Anthropic, and OpenRouter
- **Vault** — Automatic secret detection and redaction in prompts
- **Code Indexing** — Project-wide context retrieval for accurate suggestions

## Requirements

- IntelliJ Platform 2024.1 or later
- A running Ollama instance (or configured cloud provider)

## Privacy

All processing runs locally by default. API keys are stored in your OS keychain.
No telemetry is collected.

[Screenshot: Chat Panel in IntelliJ]

[Screenshot: Diff Preview]
