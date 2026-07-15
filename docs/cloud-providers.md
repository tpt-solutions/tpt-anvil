# Cloud Providers

Cloud fallback is opt-in. Your code is only sent to cloud providers when you explicitly configure and enable one. API keys are stored in the OS keychain, never in config files.

## Setting API Keys

```bash
# Store an API key securely in the OS keychain
anvil auth set openai_api_key sk-...
anvil auth set anthropic_api_key sk-ant-...
anvil auth set openrouter_api_key sk-or-...
```

## OpenAI

```toml
# ~/.config/anvil/config.toml
[providers]
active = "openai"

[providers.openai]
model = "gpt-4o"
api_key_entry = "openai_api_key"
```

## Anthropic (Claude)

```toml
[providers]
active = "anthropic"

[providers.anthropic]
model = "claude-sonnet-5"
api_key_entry = "anthropic_api_key"
```

## OpenRouter

Gives access to 200+ models (DeepSeek, Llama, Mistral, etc.) via a single API key.

```toml
[providers]
active = "openrouter"

[providers.openrouter]
model = "deepseek/deepseek-coder"
api_key_entry = "openrouter_api_key"
```

## Azure OpenAI

```toml
[providers]
active = "azure"

[providers.azure]
endpoint = "https://my-resource.openai.azure.com/openai/deployments/my-deployment"
api_version = "2024-02-01"
api_key_entry = "azure_api_key"
```

## Custom OpenAI-Compatible Endpoint

Works with Groq, Together, Fireworks, local vLLM, LM Studio, etc.

```toml
[providers]
active = "custom"

[providers.custom]
base_url = "https://api.groq.com/openai/v1"
model = "llama-3.3-70b-versatile"
api_key_entry = "groq_api_key"
```

## Switching Between Local and Cloud

Set `providers.active` to empty string to use local inference only:

```toml
[providers]
active = ""  # local-only
```

Or set it to a provider name to use cloud for all requests.
