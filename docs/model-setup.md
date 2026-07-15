# Model Setup

## Ollama (Recommended)

Ollama is the easiest way to run local models. Install it from [ollama.ai](https://ollama.ai/).

```bash
# Recommended code models
ollama pull deepseek-coder:6.7b      # 6.7B — excellent code quality, 8 GB RAM
ollama pull deepseek-coder:33b       # 33B — best quality, 20+ GB RAM
ollama pull qwen2.5-coder:7b         # 7B — fast, strong multilingual support
ollama pull codellama:13b            # 13B — Meta's code model

# Configure Anvil to use Ollama
# ~/.config/anvil/config.toml
[inference]
backend = "ollama"
model = "deepseek-coder:6.7b"
ollama_url = "http://localhost:11434"
```

## GGUF / llama.cpp

Download a GGUF model from [Hugging Face](https://huggingface.co/) and configure the path:

```bash
# Example: DeepSeek Coder Q4_K_M quantization
wget https://huggingface.co/TheBloke/deepseek-coder-6.7B-instruct-GGUF/resolve/main/deepseek-coder-6.7b-instruct.Q4_K_M.gguf

# ~/.config/anvil/config.toml
[inference]
backend = "llama_cpp"
model_path = "/path/to/deepseek-coder-6.7b-instruct.Q4_K_M.gguf"
gpu_layers = -1   # -1 = offload all layers to GPU
```

## Featured Models

| Model | Params | VRAM | Best For |
|-------|--------|------|---------|
| DeepSeek Coder 6.7B | 6.7B | 6 GB | General coding |
| DeepSeek Coder 33B | 33B | 20 GB | High-quality code |
| Qwen2.5-Coder 7B | 7B | 6 GB | Multilingual + code |
| CodeLlama 13B | 13B | 10 GB | Python / general |
| Mistral 7B | 7B | 6 GB | Fast, general purpose |

## GPU Acceleration

- **CUDA** (NVIDIA): Install CUDA toolkit 12.x, then build with `--features llama-cpp`
- **ROCm** (AMD): Install ROCm 6.x, then build with `--features llama-cpp`
- **WebGPU** (candle): Build with `--features candle` for cross-platform GPU via wgpu
- **CPU**: Works on all platforms, just slower

## Cloud Fallback

For large tasks where local models are too slow, configure a cloud provider. See [cloud-providers.md](cloud-providers.md).
