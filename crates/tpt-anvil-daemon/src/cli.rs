// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anyhow::Result;
use clap::{Parser, Subcommand};

use tpt_anvil_config::loader::ConfigLoader;
use tpt_anvil_inference::registry::BackendRegistry;
use tpt_anvil_providers::keystore;

use crate::server::to_provider_config;

#[derive(Debug, Parser)]
#[command(
    name = "anvil",
    about = "TPT Anvil — local AI development environment",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the Anvil daemon
    Start {
        /// Project root directory to index
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status {
        /// Show cost/usage estimates from the router
        #[arg(long)]
        cost: bool,
    },
    /// Manage API keys
    Auth(AuthArgs),
    /// List available models
    Models,
    /// Interactive setup wizard
    Init {
        /// Write to project-level config instead of user-level
        #[arg(long)]
        project: bool,
    },
    /// Run diagnostics and report issues
    Doctor {
        /// Attempt to auto-fix issues (pull missing models, scaffold config)
        #[arg(long)]
        fix: bool,
    },
    /// Benchmark a model against the coding task suite
    Benchmark(BenchmarkArgs),
}

#[derive(Debug, Parser)]
pub struct BenchmarkArgs {
    #[command(subcommand)]
    pub command: BenchmarkCommands,
}

#[derive(Debug, Subcommand)]
pub enum BenchmarkCommands {
    /// Run the benchmark suite against a model
    Run {
        /// Target in the form `provider/model` (e.g. `ollama/deepseek-coder:6.7b`)
        target: String,
        /// Skip adaptive tasks
        #[arg(long)]
        no_adaptive: bool,
        /// Project root for scaffold files
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Show stored benchmark scorecards
    Report {
        /// Compare two scorecards: `provider1/model1 provider2/model2`
        #[arg(num_args = 0..=2)]
        compare: Vec<String>,
    },
}

#[derive(Debug, Parser)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Store an API key in the OS keychain
    Set {
        /// Key name (e.g. openai_api_key)
        name: String,
        /// API key value
        key: String,
    },
    /// Remove an API key from the OS keychain
    Remove { name: String },
}

pub fn handle_auth(args: AuthArgs) -> Result<()> {
    match args.command {
        AuthCommands::Set { name, key } => {
            keystore::set_api_key(&name, &key)?;
            println!("API key '{name}' stored in OS keychain.");
        }
        AuthCommands::Remove { name } => {
            keystore::delete_api_key(&name)?;
            println!("API key '{name}' removed.");
        }
    }
    Ok(())
}

pub async fn list_models() -> Result<()> {
    let cfg = ConfigLoader::load(None)?;
    let registry = BackendRegistry::from_config(&cfg)?;
    let models = registry.active.list_models().await?;
    if models.is_empty() {
        println!("No models found. Make sure Ollama is running or a model path is configured.");
    } else {
        for model in models {
            println!(
                "  {} — {} (context: {} tokens)",
                model.id, model.name, model.context_length
            );
        }
    }
    Ok(())
}

/// Interactive setup wizard — writes `~/.config/anvil/config.toml` (user) or
/// `.anvil/config.toml` (project) without overwriting an existing config
/// without confirmation.
pub fn run_init(project: bool) -> Result<()> {
    use std::io::{self, Write};

    let config_dir = if project {
        std::env::current_dir()?.join(".anvil")
    } else {
        dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?
            .join("anvil")
    };
    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        print!(
            "Config already exists at {}. Overwrite? [y/N] ",
            config_path.display()
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("TPT Anvil Setup Wizard");
    println!("======================\n");

    // Backend selection
    println!("Select inference backend:");
    println!("  1) Ollama (recommended — easiest setup)");
    println!("  2) llama.cpp (GGUF models)");
    println!("  3) candle (pure Rust, GGUF models)");
    print!("Choice [1]: ");
    io::stdout().flush()?;
    let mut backend_input = String::new();
    io::stdin().read_line(&mut backend_input)?;
    let backend = match backend_input.trim() {
        "2" => "llama_cpp",
        "3" => "candle",
        _ => "ollama",
    };

    // Model
    let default_model = if backend == "ollama" {
        "deepseek-coder:6.7b"
    } else {
        ""
    };
    print!("Model [{default_model}]: ");
    io::stdout().flush()?;
    let mut model_input = String::new();
    io::stdin().read_line(&mut model_input)?;
    let model = if model_input.trim().is_empty() {
        default_model.to_string()
    } else {
        model_input.trim().to_string()
    };

    // Ollama URL
    let ollama_url = if backend == "ollama" {
        print!("Ollama URL [http://localhost:11434]: ");
        io::stdout().flush()?;
        let mut url_input = String::new();
        io::stdin().read_line(&mut url_input)?;
        let url = url_input.trim();
        if url.is_empty() {
            "http://localhost:11434".to_string()
        } else {
            url.to_string()
        }
    } else {
        "http://localhost:11434".to_string()
    };

    // Cloud provider
    println!("\nConfigure cloud fallback (optional):");
    println!("  1) None (local only)");
    println!("  2) OpenAI");
    println!("  3) Anthropic");
    println!("  4) OpenRouter");
    print!("Choice [1]: ");
    io::stdout().flush()?;
    let mut cloud_input = String::new();
    io::stdin().read_line(&mut cloud_input)?;
    let (active_provider, _model_key, keychain_entry) = match cloud_input.trim() {
        "2" => ("openai", "openai_api_key", Some("openai_api_key")),
        "3" => ("anthropic", "anthropic_api_key", Some("anthropic_api_key")),
        "4" => (
            "openrouter",
            "openrouter_api_key",
            Some("openrouter_api_key"),
        ),
        _ => ("", "", None),
    };

    // Ask for API key if cloud provider selected
    if let Some(entry) = keychain_entry {
        print!("API key for {active_provider} (leave blank to skip): ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();
        if !key.is_empty() {
            keystore::set_api_key(entry, key)?;
            println!("  API key stored in OS keychain.");
        }
    }

    // Build config TOML
    let cloud_model = if active_provider == "openai" {
        "\nmodel = \"gpt-4o\""
    } else if active_provider == "anthropic" {
        "\nmodel = \"claude-sonnet-5\""
    } else if active_provider == "openrouter" {
        "\nmodel = \"deepseek/deepseek-coder\""
    } else {
        ""
    };

    let config = format!(
        r#"[inference]
backend = "{backend}"
model = "{model}"
ollama_url = "{ollama_url}"

[providers]
active = "{active_provider}"
{cloud_section}

[vault]
enabled = true

[verify]
enabled = true
run_linter = true
"#,
        cloud_section = if !active_provider.is_empty() {
            format!("[providers.{active_provider}]{cloud_model}")
        } else {
            String::new()
        },
    );

    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(&config_path, &config)?;
    println!("\nConfig written to {}", config_path.display());
    println!("Start the daemon with: anvil start");

    Ok(())
}

/// Non-interactive diagnostic: config found/parses, Ollama reachable,
/// configured model present, GPU/acceleration features, keychain entries.
pub async fn run_doctor(fix: bool) -> Result<()> {
    let mut all_ok = true;

    println!("Anvil Doctor");
    println!("============\n");

    // 1. Config
    print!("[ ] Config file found... ");
    let cfg = match ConfigLoader::load(None) {
        Ok(c) => {
            println!(
                "OK ({})",
                tpt_anvil_config::loader::ConfigLoader::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
            c
        }
        Err(e) => {
            println!("FAIL ({e})");
            if fix {
                println!("  -> Scaffolding default config...");
                let dir = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?
                    .join("anvil");
                let path = dir.join("config.toml");
                if !path.exists() {
                    std::fs::create_dir_all(&dir)?;
                    std::fs::write(
                        &path,
                        "[inference]\nbackend = \"ollama\"\nmodel = \"deepseek-coder:6.7b\"\n",
                    )?;
                    println!("  -> Written default config to {}", path.display());
                }
            }
            return Ok(());
        }
    };

    // 2. Ollama reachability
    if cfg.inference.backend == "ollama" {
        print!("[ ] Ollama server reachable... ");
        let url = &cfg.inference.ollama_url;
        match reqwest::get(format!("{url}/api/tags")).await {
            Ok(resp) if resp.status().is_success() => println!("OK"),
            Ok(resp) => {
                println!("FAIL (HTTP {})", resp.status());
                all_ok = false;
            }
            Err(e) => {
                println!("FAIL ({e})");
                all_ok = false;
            }
        }

        // 3. Configured model present
        print!("[ ] Configured model present... ");
        let model = &cfg.inference.model;
        match reqwest::get(format!("{url}/api/tags")).await {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let models = body["models"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if models.iter().any(|n| n.starts_with(model)) {
                    println!("OK ({model})");
                } else {
                    println!(
                        "MISSING ({model} not found; available: {})",
                        models.join(", ")
                    );
                    all_ok = false;
                    if fix {
                        println!("  -> Pulling {model} via Ollama...");
                        let status = std::process::Command::new("ollama")
                            .args(["pull", model])
                            .status();
                        match status {
                            Ok(s) if s.success() => println!("  -> Pull succeeded"),
                            _ => println!("  -> Pull failed (is Ollama installed?)"),
                        }
                    }
                }
            }
            Err(_) => {
                println!("SKIP (Ollama not reachable)");
            }
        }
    }

    // 4. GPU / acceleration
    print!("[ ] GPU acceleration... ");
    #[cfg(feature = "cuda")]
    println!("CUDA compiled in");
    #[cfg(feature = "rocm")]
    println!("ROCm compiled in");
    #[cfg(feature = "webgpu")]
    println!("WebGPU compiled in");
    #[cfg(not(any(feature = "cuda", feature = "rocm", feature = "webgpu")))]
    println!("none (CPU only — build with --features cuda/rocm/webgpu for GPU)");

    // 5. Cloud provider keys
    print!("[ ] Cloud provider key... ");
    if let Some(ref name) = cfg.providers.active {
        let entry = match name.as_str() {
            "openai" => cfg
                .providers
                .openai
                .api_key_entry
                .as_deref()
                .unwrap_or("openai_api_key"),
            "anthropic" => cfg
                .providers
                .anthropic
                .api_key_entry
                .as_deref()
                .unwrap_or("anthropic_api_key"),
            "openrouter" => cfg
                .providers
                .openrouter
                .api_key_entry
                .as_deref()
                .unwrap_or("openrouter_api_key"),
            _ => "unknown",
        };
        match keystore::get_api_key(entry) {
            Ok(_) => println!("OK ({name}: {entry})"),
            Err(e) => {
                println!("FAIL ({e})");
                all_ok = false;
            }
        }
    } else {
        println!("SKIP (no cloud provider configured)");
    }

    // 6. Verify config
    print!("[ ] Verification enabled... ");
    if cfg.verify.enabled {
        println!("OK");
    } else {
        println!("disabled");
    }

    // 7. Vault config
    print!("[ ] Vault enabled... ");
    if cfg.vault.enabled {
        println!("OK");
    } else {
        println!("DISABLED (secrets will not be redacted)");
    }

    println!();
    if all_ok {
        println!("All checks passed.");
    } else {
        println!("Some checks failed. Run `anvil doctor --fix` for auto-remediation.");
    }

    Ok(())
}

/// Handle benchmark subcommands.
pub async fn handle_benchmark(cmd: BenchmarkCommands, project_root: Option<&str>) -> Result<()> {
    match cmd {
        BenchmarkCommands::Run {
            target,
            no_adaptive,
            project,
        } => run_benchmark(&target, no_adaptive, project.as_deref().or(project_root)).await,
        BenchmarkCommands::Report { compare } => show_benchmark_report(&compare).await,
    }
}

/// A normalized completion result, so local and cloud executors can share the
/// scoring/recording path.
struct ExecutorResponse {
    content: String,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    cost_usd: Option<f64>,
}

/// A benchmark target — either a local inference backend or a cloud provider.
enum BenchmarkExecutor {
    Local(std::sync::Arc<dyn tpt_anvil_inference::backend::InferenceBackend>),
    Cloud(std::sync::Arc<dyn tpt_anvil_providers::provider::CloudProvider>),
}

impl BenchmarkExecutor {
    async fn complete(&self, prompt: &str, model: &str) -> Result<ExecutorResponse> {
        match self {
            Self::Local(backend) => {
                use tpt_anvil_core::types::{ChatMessage, CompletionRequest, Role};

                let request = CompletionRequest {
                    messages: vec![ChatMessage {
                        role: Role::User,
                        content: prompt.to_string(),
                    }],
                    model: Some(model.to_string()),
                    max_tokens: 2048,
                    temperature: 0.2,
                    stream: false,
                };
                let response = backend
                    .complete(&request)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                Ok(ExecutorResponse {
                    content: response.content,
                    prompt_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
                    completion_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
                    // Local inference has no per-token billing.
                    cost_usd: None,
                })
            }
            Self::Cloud(provider) => {
                use tpt_anvil_providers::types::{
                    BackendKind, ChatMessage, CompletionRequest, Role,
                };

                let request = CompletionRequest {
                    messages: vec![ChatMessage {
                        role: Role::User,
                        content: prompt.to_string(),
                    }],
                    model: Some(model.to_string()),
                    max_tokens: 2048,
                    temperature: 0.2,
                    stream: false,
                };
                let response = provider
                    .complete(&request)
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let cost_usd = response.usage.as_ref().and_then(|u| {
                    let backend = match provider.name() {
                        "openai" => BackendKind::OpenAi,
                        "anthropic" => BackendKind::Anthropic,
                        "openrouter" => BackendKind::OpenRouter,
                        "azure" => BackendKind::AzureOpenAi,
                        _ => BackendKind::OpenAiCompatible,
                    };
                    tpt_anvil_providers::cost::estimate_cost(&backend, model, u)
                });
                Ok(ExecutorResponse {
                    content: response.content,
                    prompt_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
                    completion_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
                    cost_usd,
                })
            }
        }
    }
}

/// Resolve a `provider/model` target prefix to an executor.
///
/// Local backend names (`ollama`, `llama_cpp`, `candle`) are served by
/// `BackendRegistry`; everything else is looked up in the cloud provider
/// registry by exact name, falling back to the configured active provider.
async fn resolve_executor(
    cfg: &tpt_anvil_config::AnvilConfig,
    provider_name: &str,
) -> Result<BenchmarkExecutor> {
    use tpt_anvil_inference::registry::BackendRegistry;
    use tpt_anvil_providers::registry::ProviderRegistry;

    if matches!(provider_name, "ollama" | "llama_cpp" | "candle") {
        // A local backend must match the configured inference backend, otherwise
        // the requested target cannot be served.
        if cfg.inference.backend != provider_name {
            return Err(anyhow::anyhow!(
                "backend '{provider_name}' is not the configured inference backend (configured: '{}'); \
                 set `inference.backend` in your config first",
                cfg.inference.backend
            ));
        }
        let registry = BackendRegistry::from_config(cfg)
            .map_err(|e| anyhow::anyhow!("failed to build inference backend: {e}"))?;
        return Ok(BenchmarkExecutor::Local(registry.active));
    }

    let provider_cfg = to_provider_config(cfg);
    let registry = ProviderRegistry::from_config(&provider_cfg)
        .map_err(|e| anyhow::anyhow!("failed to build provider registry: {e}"))?;

    if let Some(entry) = registry.available.iter().find(|e| e.name == provider_name) {
        return Ok(BenchmarkExecutor::Cloud(entry.provider.clone()));
    }
    if let Some(active) = registry.active {
        return Ok(BenchmarkExecutor::Cloud(active));
    }
    Err(anyhow::anyhow!(
        "no provider named '{provider_name}' configured; run `anvil auth` first"
    ))
}

async fn run_benchmark(target: &str, _no_adaptive: bool, project_root: Option<&str>) -> Result<()> {
    use tpt_anvil_capabilities::benchmark::load_builtin_tasks;
    use tpt_anvil_capabilities::benchmark::runner::{core_score, grade_task};
    use tpt_anvil_capabilities::benchmark::scorecard::ModelScorecard;
    use tpt_anvil_capabilities::benchmark::store::BenchmarkStore;
    use tpt_anvil_capabilities::verify::VerifyConfig;

    let (provider_name, model_id) = target.split_once('/').ok_or_else(|| {
        anyhow::anyhow!(
            "target must be in the form `provider/model` (e.g. `ollama/deepseek-coder:6.7b`)"
        )
    })?;

    // Load config and build the provider for the given name
    let cfg = tpt_anvil_config::loader::ConfigLoader::load(project_root.map(std::path::Path::new))
        .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

    // Resolve the target to a concrete executor.  Local inference backends
    // (`ollama`, `llama_cpp`, `candle`) are `InferenceBackend`s, not
    // `CloudProvider`s, so they must be dispatched through `BackendRegistry`.
    // Anything else resolves against the configured cloud provider registry.
    let executor = resolve_executor(&cfg, provider_name).await?;

    let tasks = load_builtin_tasks(
        cfg.benchmark
            .core_suite_path
            .as_deref()
            .map(std::path::Path::new),
    );
    if tasks.is_empty() {
        return Err(anyhow::anyhow!(
            "no benchmark tasks found in benchmarks/core/ — check that the benchmark suite is present"
        ));
    }

    // Tasks are graded inside a per-task temp sandbox seeded from the
    // embedded scaffold, so nothing is written into the user's project.
    let verify_config = VerifyConfig {
        enabled: cfg.verify.enabled,
        run_tests: cfg.verify.run_tests,
        run_linter: cfg.verify.run_linter,
        timeout_seconds: cfg.verify.timeout_seconds,
        max_retries: cfg.verify.max_retries,
    };

    println!(
        "Running {} benchmark tasks against {target}...\n",
        tasks.len()
    );

    let mut results = Vec::new();
    let mut total_cost: f64 = 0.0;

    for task in &tasks {
        print!("[ ] {} ... ", task.description);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let start = std::time::Instant::now();
        let output = executor.complete(task.prompt.as_str(), model_id).await;
        let latency = start.elapsed().as_millis() as u64;

        match output {
            Ok(response) => {
                let mut task_result = grade_task(task, &response.content, &verify_config).await;
                task_result.latency_ms += latency;
                task_result.prompt_tokens = response.prompt_tokens;
                task_result.completion_tokens = response.completion_tokens;
                task_result.cost_usd = response.cost_usd;
                if let Some(c) = response.cost_usd {
                    total_cost += c;
                }
                let status = if task_result.passed {
                    "PASS"
                } else if task_result.skipped {
                    "SKIP"
                } else {
                    "FAIL"
                };
                println!("{status} ({latency}ms)");
                if task_result.skipped {
                    println!("    verification toolchain unavailable — excluded from score");
                }
                if !task_result.errors.is_empty() {
                    for err in &task_result.errors {
                        println!("    {err}");
                    }
                }
                results.push(task_result);
            }
            Err(e) => {
                println!("ERROR ({e})");
                results.push(
                    tpt_anvil_capabilities::benchmark::scorecard::TaskRunResult {
                        task_id: task.id.clone(),
                        task_kind: task.kind,
                        passed: false,
                        latency_ms: latency,
                        prompt_tokens: None,
                        completion_tokens: None,
                        cost_usd: None,
                        output: None,
                        errors: vec![e.to_string()],
                        // The provider call itself failed (unreachable
                        // backend, model not pulled). The model never
                        // produced an answer, so this is a real failure
                        // rather than an unscoreable environment gap.
                        skipped: false,
                    },
                );
            }
        }
    }

    let score = core_score(&results);
    let skipped = results.iter().filter(|r| r.skipped).count();
    let total_tasks = results.len();
    // Compute the denominator counts before `results` is moved into the
    // scorecard below. Showing them explicitly matters: a bare percentage hides
    // how many tasks actually contributed, so "50%" over 2 scorable tasks reads
    // like "50%" over 8 and invites false comparisons between machines.
    let scorable = results.iter().filter(|r| !r.skipped).count();
    let passed_count = results.iter().filter(|r| !r.skipped && r.passed).count();
    let task_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
    let now = chrono_now();

    let scorecard = ModelScorecard {
        provider: provider_name.to_string(),
        model_id: model_id.to_string(),
        last_run_at: now.clone(),
        core_task_ids_run: task_ids,
        core_results: results,
        adaptive_results: vec![],
        core_score: score,
        adaptive_score: None,
        total_cost_usd: total_cost,
    };

    let store_path = BenchmarkStore::default_path().unwrap_or_default();
    let mut store = BenchmarkStore::load(&store_path);
    store.record(scorecard);
    store
        .save(&store_path)
        .map_err(|e| anyhow::anyhow!("failed to save benchmark store: {e}"))?;

    // Show the denominator explicitly: a bare percentage hides how many tasks
    // actually contributed, so "50%" over 2 scorable tasks reads like "50%"
    // over 8 and invites false comparisons between machines.
    println!(
        "\nBenchmark complete: {:.0}% ({target}) at {now}  [{passed_count}/{scorable} scored of {total_tasks}]",
        score * 100.0,
    );
    if skipped > 0 {
        println!(
            "{skipped} of {total_tasks} task(s) skipped (verification toolchain unavailable)."
        );
        println!("Install the missing toolchains to score those tasks.");
    }
    if total_cost > 0.0 {
        println!("Estimated cost: ${total_cost:.4}");
    }
    println!("Scorecard saved to {}", store_path.display());

    Ok(())
}

async fn show_benchmark_report(targets: &[String]) -> Result<()> {
    use tpt_anvil_capabilities::benchmark::comparison::compare;
    use tpt_anvil_capabilities::benchmark::store::BenchmarkStore;

    let store_path = BenchmarkStore::default_path().unwrap_or_default();
    let store = BenchmarkStore::load(&store_path);

    if store.entries().is_empty() {
        println!("No benchmark scorecards stored yet.");
        println!("Run `anvil benchmark run <provider/model>` to generate one.");
        return Ok(());
    }

    match targets.len() {
        0 => {
            // Show all stored scorecards
            println!("Stored Benchmark Scorecards\n");
            println!(
                "{:<15} {:<25} {:>12} {:>8} {:>10}",
                "Provider", "Model", "Core", "Adaptive", "Cost"
            );
            println!("{}", "-".repeat(68));
            for entry in store.entries() {
                let adaptive = entry
                    .adaptive_score
                    .map(|s| format!("{:.0}%", s * 100.0))
                    .unwrap_or_else(|| "-".into());
                let cost = if entry.total_cost_usd > 0.0 {
                    format!("${:.4}", entry.total_cost_usd)
                } else {
                    "-".into()
                };
                let core = format!("{:.0}%", entry.core_score * 100.0);
                // Carry the denominator in the table so a score computed over a
                // reduced task set (skipped toolchains) is not mistaken for a
                // full-suite run.
                let scored = entry.core_results.iter().filter(|r| !r.skipped).count();
                let skipped_n = entry.core_results.len().saturating_sub(scored);
                let core = if skipped_n > 0 {
                    format!("{core} ({scored})")
                } else {
                    core
                };
                println!(
                    "{:<15} {:<25} {:>12} {:>8} {:>10}",
                    entry.provider, entry.model_id, core, adaptive, cost
                );
            }
            println!(
                "\nRun `anvil benchmark report <provider1/model1> <provider2/model2>` to compare."
            );
        }
        2 => {
            let (left_provider, left_model) = parse_target(&targets[0])?;
            let (right_provider, right_model) = parse_target(&targets[1])?;

            let left = store
                .find(&left_provider, &left_model)
                .ok_or_else(|| anyhow::anyhow!("no scorecard found for {}", targets[0]))?;
            let right = store
                .find(&right_provider, &right_model)
                .ok_or_else(|| anyhow::anyhow!("no scorecard found for {}", targets[1]))?;

            let cmp = compare(left, right);

            println!("Benchmark Comparison\n");
            println!("  {:<25} vs {:<25}", cmp.left_label, cmp.right_label);
            println!(
                "  {:<25} {:.0}%",
                cmp.left_label,
                cmp.left_shared_score * 100.0
            );
            println!(
                "  {:<25} {:.0}%",
                cmp.right_label,
                cmp.right_shared_score * 100.0
            );

            if !cmp.left_only_task_ids.is_empty() {
                println!(
                    "\n  Tasks only in {}: {}",
                    cmp.left_label,
                    cmp.left_only_task_ids.join(", ")
                );
            }
            if !cmp.right_only_task_ids.is_empty() {
                println!(
                    "  Tasks only in {}: {}",
                    cmp.right_label,
                    cmp.right_only_task_ids.join(", ")
                );
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "usage: `anvil benchmark report` (show all) or `anvil benchmark report <target1> <target2>` (compare two)"
            ));
        }
    }

    Ok(())
}

fn parse_target(target: &str) -> Result<(String, String)> {
    target
        .split_once('/')
        .map(|(p, m)| (p.to_string(), m.to_string()))
        .ok_or_else(|| anyhow::anyhow!("target must be in the form `provider/model`"))
}

fn chrono_now() -> String {
    crate::server::chrono_now()
}

/// Show cost/usage summary from the recent models tracker and router estimates.
pub async fn show_cost_summary() -> Result<()> {
    use tpt_anvil_providers::recent_models::RecentModels;

    let path = dirs::config_dir()
        .map(|d| d.join("anvil").join("recent_models.json"))
        .unwrap_or_default();

    let recent = RecentModels::load(&path);
    let list = recent.list();

    if list.is_empty() {
        println!("No recent model usage recorded yet.");
        println!("Use Anvil through your IDE to start tracking usage.");
        return Ok(());
    }

    println!("Recent Model Usage");
    println!("==================\n");
    println!("{:<20} Model", "Provider");
    println!("{}", "-".repeat(50));
    for entry in list {
        println!("{:<20} {}", entry.provider, entry.model_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn benchmark_args_definition_is_valid() {
        BenchmarkArgs::command().debug_assert();
    }

    #[test]
    fn parse_benchmark_run_minimal() {
        let args =
            BenchmarkArgs::try_parse_from(["benchmark", "run", "ollama/deepseek-coder:6.7b"])
                .expect("minimal `benchmark run` should parse");

        match args.command {
            BenchmarkCommands::Run {
                target,
                no_adaptive,
                project,
            } => {
                assert_eq!(target, "ollama/deepseek-coder:6.7b");
                assert!(!no_adaptive, "adaptive should default to enabled");
                assert_eq!(project, None);
            }
            other => panic!("expected BenchmarkCommands::Run, got {other:?}"),
        }
    }

    #[test]
    fn parse_benchmark_run_with_flags() {
        let args = BenchmarkArgs::try_parse_from([
            "benchmark",
            "run",
            "openai/gpt-4o-mini",
            "--no-adaptive",
            "--project",
            "/tmp/scratch",
        ])
        .expect("`benchmark run` with all flags should parse");

        match args.command {
            BenchmarkCommands::Run {
                target,
                no_adaptive,
                project,
            } => {
                assert_eq!(target, "openai/gpt-4o-mini");
                assert!(no_adaptive);
                assert_eq!(project.as_deref(), Some("/tmp/scratch"));
            }
            other => panic!("expected BenchmarkCommands::Run, got {other:?}"),
        }
    }

    #[test]
    fn parse_benchmark_run_short_project_flag() {
        let args = BenchmarkArgs::try_parse_from([
            "benchmark",
            "run",
            "ollama/qwen2.5-coder",
            "-p",
            "/tmp/x",
        ])
        .expect("short `-p` flag should parse");

        match args.command {
            BenchmarkCommands::Run { project, .. } => {
                assert_eq!(project.as_deref(), Some("/tmp/x"))
            }
            other => panic!("expected BenchmarkCommands::Run, got {other:?}"),
        }
    }

    #[test]
    fn parse_benchmark_report_no_args() {
        let args = BenchmarkArgs::try_parse_from(["benchmark", "report"])
            .expect("`benchmark report` with no targets should parse");

        match args.command {
            BenchmarkCommands::Report { compare } => assert!(compare.is_empty()),
            other => panic!("expected BenchmarkCommands::Report, got {other:?}"),
        }
    }

    #[test]
    fn parse_benchmark_report_two_targets() {
        let args = BenchmarkArgs::try_parse_from([
            "benchmark",
            "report",
            "ollama/model-a",
            "ollama/model-b",
        ])
        .expect("two compare targets should parse");

        match args.command {
            BenchmarkCommands::Report { compare } => {
                assert_eq!(compare, vec!["ollama/model-a", "ollama/model-b"]);
            }
            other => panic!("expected BenchmarkCommands::Report, got {other:?}"),
        }
    }

    #[test]
    fn parse_benchmark_report_rejects_three_targets() {
        let err = BenchmarkArgs::try_parse_from([
            "benchmark",
            "report",
            "ollama/model-a",
            "ollama/model-b",
            "ollama/model-c",
        ])
        .expect_err("three compare targets must be rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::TooManyValues);
    }

    #[test]
    fn parse_benchmark_run_requires_target() {
        let err = BenchmarkArgs::try_parse_from(["benchmark", "run"])
            .expect_err("`benchmark run` without a target must be rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn parse_benchmark_rejects_unknown_subcommand() {
        let err = BenchmarkArgs::try_parse_from(["benchmark", "explode"])
            .expect_err("unknown benchmark subcommand must be rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn parse_top_level_benchmark_subcommand() {
        let cli = Cli::try_parse_from(["anvil", "benchmark", "run", "ollama/qwen2.5-coder"])
            .expect("top-level `anvil benchmark run` should parse");

        match cli.command {
            Commands::Benchmark(BenchmarkArgs {
                command: BenchmarkCommands::Run { target, .. },
            }) => assert_eq!(target, "ollama/qwen2.5-coder"),
            other => panic!("expected Commands::Benchmark, got {other:?}"),
        }
    }

    #[test]
    fn existing_top_level_subcommands_still_parse() {
        assert!(matches!(
            Cli::try_parse_from(["anvil", "status"])
                .expect("status")
                .command,
            Commands::Status { cost: false }
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "status", "--cost"])
                .expect("status --cost")
                .command,
            Commands::Status { cost: true }
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "stop"])
                .expect("stop")
                .command,
            Commands::Stop
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "start", "-p", "/tmp/proj"])
                .expect("start -p")
                .command,
            Commands::Start { project: Some(_) }
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "doctor", "--fix"])
                .expect("doctor --fix")
                .command,
            Commands::Doctor { fix: true }
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "init", "--project"])
                .expect("init --project")
                .command,
            Commands::Init { project: true }
        ));
        assert!(matches!(
            Cli::try_parse_from(["anvil", "models"])
                .expect("models")
                .command,
            Commands::Models
        ));
    }

    #[test]
    fn parse_target_splits_provider_and_model() {
        let (provider, model) = parse_target("ollama/deepseek-coder:6.7b").expect("valid target");
        assert_eq!(provider, "ollama");
        assert_eq!(model, "deepseek-coder:6.7b");
    }

    #[test]
    fn parse_target_rejects_missing_slash() {
        let err = parse_target("deepseek-coder:6.7b").expect_err("missing provider must error");
        assert!(err.to_string().contains("provider/model"));
    }
}
