// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anyhow::Result;
use clap::{Parser, Subcommand};

use anvil_config::loader::ConfigLoader;
use anvil_inference::registry::BackendRegistry;
use tpt_anvil_providers::keystore;

use crate::server::to_provider_config;

#[derive(Parser)]
#[command(
    name = "anvil",
    about = "TPT Anvil — local AI development environment",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
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

#[derive(Parser)]
pub struct BenchmarkArgs {
    #[command(subcommand)]
    pub command: BenchmarkCommands,
}

#[derive(Subcommand)]
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

#[derive(Parser)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Subcommand)]
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
                anvil_config::loader::ConfigLoader::config_path()
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
        } => run_benchmark(&target, no_adaptive, project.as_deref().or(project_root)).await,
        BenchmarkCommands::Report { compare } => show_benchmark_report(&compare).await,
    }
}

async fn run_benchmark(target: &str, _no_adaptive: bool, project_root: Option<&str>) -> Result<()> {
async fn run_benchmark(target: &str, _no_adaptive: bool, project_root: Option<&str>) -> Result<()> {
    use anvil_capabilities::benchmark::load_builtin_tasks;
    use anvil_capabilities::benchmark::runner::{core_score, grade_task};
    use anvil_capabilities::benchmark::scorecard::ModelScorecard;
    use anvil_capabilities::benchmark::store::BenchmarkStore;
    use anvil_capabilities::verify::VerifyConfig;
    use tpt_anvil_providers::registry::ProviderRegistry;
    use tpt_anvil_providers::types::{ChatMessage, CompletionRequest, Role};

    let (provider_name, model_id) = target.split_once('/').ok_or_else(|| {
        anyhow::anyhow!(
            "target must be in the form `provider/model` (e.g. `ollama/deepseek-coder:6.7b`)"
        )
    })?;
    let (provider_name, model_id) = target.split_once('/').ok_or_else(|| {
        anyhow::anyhow!(
            "target must be in the form `provider/model` (e.g. `ollama/deepseek-coder:6.7b`)"
        )
    })?;

    // Load config and build the provider for the given name
    let cfg = anvil_config::loader::ConfigLoader::load(project_root.map(std::path::Path::new))
    let cfg = anvil_config::loader::ConfigLoader::load(project_root.map(std::path::Path::new))
        .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;
    let provider_cfg = crate::server::to_provider_config(&cfg);
    let registry = ProviderRegistry::from_config(&provider_cfg)
        .map_err(|e| anyhow::anyhow!("failed to build provider registry: {e}"))?;

    // Find a matching provider entry — try exact name match first, then fallback to active
    let provider: std::sync::Arc<dyn tpt_anvil_providers::provider::CloudProvider> =
        if let Some(entry) = registry.available.iter().find(|e| e.name == provider_name) {
            entry.provider.clone()
        } else if let Some(active) = registry.active {
            active
        } else {
            return Err(anyhow::anyhow!(
                "no provider named '{provider_name}' configured; run `anvil auth` first"
            ));
        };

    let tasks = load_builtin_tasks();
    if tasks.is_empty() {
        return Err(anyhow::anyhow!(
            "no benchmark tasks found in benchmarks/core/ — check that the benchmark suite is present"
        ));
    }

    let proj = project_root
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let proj = project_root
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let verify_config = VerifyConfig {
        enabled: cfg.verify.enabled,
        run_tests: cfg.verify.run_tests,
        run_linter: cfg.verify.run_linter,
        timeout_seconds: cfg.verify.timeout_seconds,
        max_retries: cfg.verify.max_retries,
        max_retries: cfg.verify.max_retries,
    };

    println!(
        "Running {} benchmark tasks against {target}...\n",
        tasks.len()
    );
    println!(
        "Running {} benchmark tasks against {target}...\n",
        tasks.len()
    );

    let mut results = Vec::new();
    let mut total_cost: f64 = 0.0;

    for task in &tasks {
        print!("[ ] {} ... ", task.description);

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: task.prompt.clone(),
            }],
            model: Some(model_id.to_string()),
            max_tokens: 2048,
            temperature: 0.2,
            stream: false,
        };

        let start = std::time::Instant::now();
        let output = provider.complete(&request).await;
        let latency = start.elapsed().as_millis() as u64;

        match output {
            Ok(response) => {
                let task_result = grade_task(task, &response.content, &proj, &verify_config).await;
                let cost = response.usage.as_ref().and_then(|u| {
                    let backend = match provider_name {
                        "openai" => tpt_anvil_providers::types::BackendKind::OpenAi,
                        "anthropic" => tpt_anvil_providers::types::BackendKind::Anthropic,
                        "openrouter" => tpt_anvil_providers::types::BackendKind::OpenRouter,
                        "azure" => tpt_anvil_providers::types::BackendKind::AzureOpenAi,
                        _ => tpt_anvil_providers::types::BackendKind::OpenAiCompatible,
                    };
                    tpt_anvil_providers::cost::estimate_cost(&backend, model_id, u)
                });
                let task_result = grade_task(task, &response.content, &proj, &verify_config).await;
                let cost = response.usage.as_ref().and_then(|u| {
                    let backend = match provider_name {
                        "openai" => tpt_anvil_providers::types::BackendKind::OpenAi,
                        "anthropic" => tpt_anvil_providers::types::BackendKind::Anthropic,
                        "openrouter" => tpt_anvil_providers::types::BackendKind::OpenRouter,
                        "azure" => tpt_anvil_providers::types::BackendKind::AzureOpenAi,
                        _ => tpt_anvil_providers::types::BackendKind::OpenAiCompatible,
                    };
                    tpt_anvil_providers::cost::estimate_cost(&backend, model_id, u)
                });
                if let Some(c) = cost {
                    total_cost += c;
                }
                let status = if task_result.passed { "PASS" } else { "FAIL" };
                println!("{status} ({latency}ms)");
                if !task_result.errors.is_empty() {
                    for err in &task_result.errors {
                        println!("    {err}");
                    }
                }
                results.push(task_result);
            }
            Err(e) => {
                println!("ERROR ({e})");
                results.push(anvil_capabilities::benchmark::scorecard::TaskRunResult {
                    task_id: task.id.clone(),
                    task_kind: task.kind,
                    passed: false,
                    latency_ms: latency,
                    prompt_tokens: None,
                    completion_tokens: None,
                    cost_usd: None,
                    output: None,
                    errors: vec![e.to_string()],
                });
            }
        }
    }

    let score = core_score(&results);
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

    println!(
        "\nBenchmark complete: {:.0}% ({target}) at {now}",
        score * 100.0
    );
    println!(
        "\nBenchmark complete: {:.0}% ({target}) at {now}",
        score * 100.0
    );
    if total_cost > 0.0 {
        println!("Estimated cost: ${total_cost:.4}");
    }
    println!("Scorecard saved to {}", store_path.display());

    Ok(())
}

async fn show_benchmark_report(targets: &[String]) -> Result<()> {
    use anvil_capabilities::benchmark::comparison::compare;
    use anvil_capabilities::benchmark::store::BenchmarkStore;

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
                "{:<15} {:<25} {:>8} {:>8} {:>10}",
                "Provider", "Model", "Core", "Adaptive", "Cost"
            );
            println!("{}", "-".repeat(68));
            for entry in store.entries() {
                let adaptive = entry
                    .adaptive_score
                    .map(|s| format!("{:.0}%", s * 100.0))
                    .map(|s| format!("{:.0}%", s * 100.0))
                    .unwrap_or_else(|| "-".into());
                let cost = if entry.total_cost_usd > 0.0 {
                    format!("${:.4}", entry.total_cost_usd)
                } else {
                    "-".into()
                };
                let core = format!("{:.0}%", entry.core_score * 100.0);
                println!(
                    "{:<15} {:<25} {:>8.0}% {:>8} {:>10}",
                    entry.provider,
                    entry.model_id,
                    entry.core_score * 100.0,
                    adaptive,
                    cost
                );
            }
            println!(
                "\nRun `anvil benchmark report <provider1/model1> <provider2/model2>` to compare."
            );
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

            let cmp = compare_scorecards(left, right);

            println!("Benchmark Comparison\n");
            println!("  {:<25} vs {:<25}", cmp.left_label, cmp.right_label);
            println!("  {:<25} vs {:<25}", cmp.left_label, cmp.right_label);
            println!("  Shared tasks: {}\n", cmp.shared_task_ids.len());
            println!(
                "  {:<25} {:.0}%",
                cmp.left_label,
                cmp.left_shared_score * 100.0
                "  {:<25} {:.0}%",
                cmp.left_label,
                cmp.left_shared_score * 100.0
            );
            println!(
                "  {:<25} {:.0}%",
                cmp.right_label,
                cmp.right_shared_score * 100.0
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
