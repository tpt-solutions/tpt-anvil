// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::Path;
use std::sync::Arc;

use anvil_capabilities::commands::{CommandHandler, HandlerConfig};
use anvil_config::loader::ConfigLoader;
use anvil_core::{
    ipc::{JsonRpcRequest, JsonRpcResponse, SlashCommandParams, StatusResponse},
    types::StreamChunk,
};
use anvil_inference::registry::BackendRegistry;
use anyhow::{Context, Result};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tpt_anvil_providers::{
    registry::ProviderRegistry, router::RouterConfig, types::ProviderConfig,
};
use tracing::{error, info};

use crate::pid;

/// `tpt-anvil-providers` is decoupled from `anvil-config` for standalone
/// crates.io publishing, so it defines its own minimal `ProviderConfig`
/// instead of depending on `anvil_config::AnvilConfig` directly. This adapts
/// the full daemon config into that minimal shape.
pub(crate) fn to_provider_config(cfg: &anvil_config::AnvilConfig) -> ProviderConfig {
    let p = &cfg.providers;
    ProviderConfig {
        active: p.active.clone(),
        openai_model: p.openai.model.clone(),
        openai_api_key_entry: p.openai.api_key_entry.clone(),
        anthropic_model: p.anthropic.model.clone(),
        anthropic_api_key_entry: p.anthropic.api_key_entry.clone(),
        openrouter_model: p.openrouter.model.clone(),
        openrouter_api_key_entry: p.openrouter.api_key_entry.clone(),
        azure_endpoint: p.azure.endpoint.clone(),
        azure_api_version: p.azure.api_version.clone(),
        azure_api_key_entry: p.azure.api_key_entry.clone(),
        custom_base_url: p.custom.base_url.clone(),
        custom_model: p.custom.model.clone(),
        custom_api_key_entry: p.custom.api_key_entry.clone(),
    }
}

/// Adapts `anvil-capabilities`/`anvil-config`'s Vault/Verify/Router config
/// shapes into a `HandlerConfig` for `CommandHandler` — the wiring point for
/// todo.md Phase 16 (Vault, Smart Context, Router, Verifier).
fn to_handler_config(
    cfg: &anvil_config::AnvilConfig,
    project_root: std::path::PathBuf,
) -> HandlerConfig {
    HandlerConfig {
        vault: anvil_capabilities::vault::VaultConfig {
            enabled: cfg.vault.enabled,
            redact_local: cfg.vault.redact_local,
            custom_patterns: cfg
                .vault
                .custom_patterns
                .iter()
                .map(|p| anvil_capabilities::vault::CustomPattern {
                    name: p.name.clone(),
                    pattern: p.pattern.clone(),
                    replacement: p.replacement.clone(),
                })
                .collect(),
        },
        verify: anvil_capabilities::verify::VerifyConfig {
            enabled: cfg.verify.enabled,
            run_tests: cfg.verify.run_tests,
            run_linter: cfg.verify.run_linter,
            timeout_seconds: cfg.verify.timeout_seconds,
            max_retries: cfg.verify.max_retries,
        },
        smart_context: cfg.smart_context.clone(),
        router: RouterConfig {
            enabled: cfg.router.enabled,
            prefer_cheapest: cfg.router.prefer_cheapest,
            max_cost_per_request_usd: cfg.router.max_cost_per_request_usd,
            pinned: cfg.router.pinned.clone(),
        },
        project_root,
    }
}

fn token_path() -> std::path::PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("anvil")
        .join("anvil.token")
}

pub async fn run(project_root: Option<&str>) -> Result<()> {
    let project_path = project_root.map(Path::new);
    let cfg = ConfigLoader::load(project_path)?;

    let backend_registry =
        BackendRegistry::from_config(&cfg).context("failed to initialize inference backend")?;
    let provider_registry = ProviderRegistry::from_config(&to_provider_config(&cfg))?;

    let backend = Arc::clone(&backend_registry.active);
    let cloud = provider_registry.active.map(|p| Arc::clone(&p));
    let available_providers = provider_registry.available;

    let resolved_project_root = project_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let handler_config = to_handler_config(&cfg, resolved_project_root);

    let handler = Arc::new(CommandHandler::new(
        backend,
        cloud,
        available_providers,
        handler_config,
    ));

    // Generate per-run authentication token
    let mut token_buf = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut token_buf);
    let token: Arc<String> = Arc::new(token_buf.iter().map(|b| format!("{b:02x}")).collect());

    let tp = token_path();
    if let Some(parent) = tp.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&tp, token.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tp, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    {
        set_windows_owner_only_acl(&tp);
    }

    pid::write_pid()?;
    info!(
        "backend: {} | model: {}",
        cfg.inference.backend, cfg.inference.model
    );

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = Arc::clone(&shutdown);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        shutdown_clone.notify_one();
    });

    // Cap concurrent RPC connections to bound local resource / cost exhaustion.
    let concurrency_limit = Arc::new(tokio::sync::Semaphore::new(32));

    #[cfg(unix)]
    run_unix(handler, shutdown, token, concurrency_limit).await?;

    #[cfg(windows)]
    run_windows(handler, shutdown, token, concurrency_limit).await?;

    pid::remove_pid();
    Ok(())
}

// ── Unix socket transport ──────────────────────────────────────────────────

#[cfg(unix)]
async fn run_unix(
    handler: Arc<CommandHandler>,
    shutdown: Arc<tokio::sync::Notify>,
    token: Arc<String>,
    concurrency: Arc<tokio::sync::Semaphore>,
) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use tokio::net::UnixListener;

    let socket_path = unix_socket_path();
    let dir = socket_path.parent().unwrap();
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;

    // TOCTOU-safe bind: try bind, on AlreadyExists remove once and retry (max 3 attempts)
    let mut bound = None;
    for attempt in 0..3u32 {
        match UnixListener::bind(&socket_path) {
            Ok(l) => {
                bound = Some(l);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempt < 2 => {
                let _ = std::fs::remove_file(&socket_path);
                continue;
            }
            Err(e) => {
                return Err(e).context(format!(
                    "failed to bind IPC socket at {}",
                    socket_path.display()
                ));
            }
        }
    }
    let listener = bound.ok_or_else(|| {
        anyhow::anyhow!(
            "failed to bind IPC socket at {} after retries",
            socket_path.display()
        )
    })?;

    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;

    info!("Anvil daemon listening on {}", socket_path.display());

    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, _)) => {
                        let handler = Arc::clone(&handler);
                        let token = Arc::clone(&token);
                        let permit = Arc::clone(&concurrency).acquire_owned().await;
                        match permit {
                            Ok(_permit) => {
                                tokio::spawn(async move {
                                    let (reader, writer) = stream.into_split();
                                    handle_rpc(reader, writer, handler, token).await;
                                    drop(_permit);
                                });
                            }
                            Err(_) => error!("semaphore closed, dropping connection"),
                        }
                    }
                    Err(e) => error!("accept error: {e}"),
                }
            }
            _ = shutdown.notified() => {
                info!("shutting down");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}

#[cfg(unix)]
pub fn unix_socket_path() -> std::path::PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("anvil")
        .join("anvil.sock")
}

// ── Windows named pipe transport ──────────────────────────────────────────

#[cfg(windows)]
fn pipe_name() -> &'static str {
    r"\\.\pipe\anvil"
}

/// On Windows, set restrictive file permissions on `path` so only the
/// current user can read/write it.  This mirrors the Unix `chmod 0600`
/// treatment.  Uses the Windows `icacls` command as a best-effort approach;
/// full DACL FFI is possible but fragile across `windows-sys` versions.
#[cfg(windows)]
fn set_windows_owner_only_acl(path: &std::path::Path) {
    let path_str = path.display().to_string();

    let _ = std::process::Command::new("icacls")
        .args([&path_str, "/inheritance:r", "/grant:r", "*S-1-3-4:(F)"])
        .output();
}

#[cfg(windows)]
async fn run_windows(
    handler: Arc<CommandHandler>,
    shutdown: Arc<tokio::sync::Notify>,
    token: Arc<String>,
    concurrency: Arc<tokio::sync::Semaphore>,
) -> Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let name = pipe_name();
    info!("Anvil daemon listening on {}", name);

    loop {
        let pipe = ServerOptions::new()
            .first_pipe_instance(false)
            .create(name)
            .with_context(|| format!("failed to create named pipe {name}"))?;

        let handler = Arc::clone(&handler);
        let shutdown_clone = Arc::clone(&shutdown);
        let token = Arc::clone(&token);
        let permit = Arc::clone(&concurrency).acquire_owned().await;

        tokio::select! {
            connect = pipe.connect() => {
                match connect {
                    Ok(()) => {
                        match permit {
                            Ok(_permit) => {
                                tokio::spawn(async move {
                                    let (reader, writer) = tokio::io::split(pipe);
                                    handle_rpc(reader, writer, handler, token).await;
                                    drop(_permit);
                                });
                            }
                            Err(_) => error!("semaphore closed, dropping connection"),
                        }
                    }
                    Err(e) => error!("pipe connect error: {e}"),
                }
            }
            _ = shutdown_clone.notified() => {
                info!("shutting down");
                break;
            }
        }
    }
    Ok(())
}

// ── Shared JSON-RPC handler ────────────────────────────────────────────────

async fn handle_rpc<R, W>(
    reader: R,
    mut writer: W,
    handler: Arc<CommandHandler>,
    token: Arc<String>,
) where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err = JsonRpcResponse::err(0, -32700, format!("parse error: {e}"));
                send_line(&mut writer, &err).await;
                continue;
            }
        };

        let id = req.id;

        if req.method.as_str() != "health" {
            let provided = req.params.get("auth").and_then(|v| v.as_str());
            let authorized = match provided {
                Some(p) => {
                    let p_bytes = p.as_bytes();
                    let t_bytes = token.as_bytes();
                    if p_bytes.len() == t_bytes.len() {
                        p_bytes.ct_eq(t_bytes).into()
                    } else {
                        false
                    }
                }
                None => false,
            };
            if !authorized {
                let err =
                    JsonRpcResponse::err(id, -32001, "unauthorized: invalid or missing auth token");
                send_line(&mut writer, &err).await;
                continue;
            }
        }

        let response = match req.method.as_str() {
            "health" => JsonRpcResponse::ok(
                id,
                serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
            ),

            "status" => {
                let status = StatusResponse {
                    version: env!("CARGO_PKG_VERSION").into(),
                    active_backend: "ollama".into(),
                    active_model: None,
                    index_status: "ready".into(),
                };
                JsonRpcResponse::ok(id, status)
            }

            "slash_command" => {
                let params: SlashCommandParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        send_line(
                            &mut writer,
                            &JsonRpcResponse::err(id, -32602, format!("invalid params: {e}")),
                        )
                        .await;
                        continue;
                    }
                };

                let (token_tx, mut token_rx) = mpsc::channel::<StreamChunk>(256);
                let handler_clone = Arc::clone(&handler);
                let command = params.command.clone();
                let ctx = params.context.clone();
                let conv_id = params.conversation_id.clone();

                let (result_tx, mut result_rx) = mpsc::channel::<
                    anyhow::Result<(
                        String,
                        Option<anvil_core::types::DiffPatch>,
                        Option<anvil_capabilities::verify::VerificationResult>,
                    )>,
                >(1);

                tokio::spawn(async move {
                    let result = handler_clone
                        .run(&command, &ctx, conv_id.as_deref(), token_tx)
                        .await;
                    let _ = result_tx
                        .send(result.map_err(|e| anyhow::anyhow!("{e}")))
                        .await;
                });

                let mut full = String::new();
                while let Some(chunk) = token_rx.recv().await {
                    full.push_str(&chunk.delta);
                    let notif = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "stream_token",
                        "params": { "id": id, "delta": chunk.delta, "done": chunk.done }
                    });
                    let line = serde_json::to_string(&notif).unwrap_or_default();
                    let _ = writer.write_all(format!("{line}\n").as_bytes()).await;
                    if chunk.done {
                        break;
                    }
                }

                let mut response_value = serde_json::json!({ "content": full });

                if let Some(Ok((_text, _diff, Some(v)))) = result_rx.recv().await {
                    let retried = v.retries_used > 0;
                    response_value["verification"] = serde_json::json!({
                        "passed": v.passed,
                        "errors": v.errors,
                        "compiler_output": v.compiler_output,
                        "lint_output": v.lint_output,
                        "test_output": v.test_output,
                        "retries_used": v.retries_used,
                        "max_retries": v.max_retries,
                        "retried": retried,
                    });
                }

                JsonRpcResponse::ok(id, response_value)
            }

            "benchmark.run" => {
                let target = req
                    .params
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (provider_name, model_id) = match target.split_once('/') {
                    Some((p, m)) => (p, m),
                    None => {
                        send_line(
                            &mut writer,
                            &JsonRpcResponse::err(id, -32602, "target must be `provider/model`"),
                        )
                        .await;
                        continue;
                    }
                };

                let handler_clone = Arc::clone(&handler);
                let provider_name = provider_name.to_string();
                let model_id = model_id.to_string();

                let (result_tx, mut result_rx) =
                    mpsc::channel::<anyhow::Result<serde_json::Value>>(1);

                tokio::spawn(async move {
                    let result = run_benchmark_rpc(&handler_clone, &provider_name, &model_id).await;
                    let _ = result_tx.send(result).await;
                });

                match result_rx.recv().await {
                    Some(Ok(val)) => JsonRpcResponse::ok(id, val),
                    Some(Err(e)) => {
                        JsonRpcResponse::err(id, -32000, format!("benchmark failed: {e}"))
                    }
                    None => JsonRpcResponse::err(id, -32000, "benchmark channel closed"),
                }
            }

            "benchmark.report" => {
                let targets: Vec<String> = req
                    .params
                    .get("targets")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let result = show_benchmark_report_rpc(&targets);
                match result {
                    Ok(val) => JsonRpcResponse::ok(id, val),
                    Err(e) => JsonRpcResponse::err(id, -32000, format!("report failed: {e}")),
                }
            }

            other => JsonRpcResponse::err(id, -32601, format!("method not found: {other}")),
        };

        send_line(&mut writer, &response).await;
    }
}

async fn send_line<W: AsyncWriteExt + Unpin>(writer: &mut W, value: &impl serde::Serialize) {
    if let Ok(line) = serde_json::to_string(value) {
        let _ = writer.write_all(format!("{line}\n").as_bytes()).await;
    }
}

async fn run_benchmark_rpc(
    handler: &CommandHandler,
    provider_name: &str,
    model_id: &str,
) -> anyhow::Result<serde_json::Value> {
    use anvil_capabilities::benchmark::load_builtin_tasks;
    use anvil_capabilities::benchmark::runner::{core_score, grade_task};
    use anvil_capabilities::benchmark::scorecard::ModelScorecard;
    use anvil_capabilities::benchmark::store::BenchmarkStore;
    use tpt_anvil_providers::types::{ChatMessage, CompletionRequest, Role};

    let tasks = load_builtin_tasks();
    if tasks.is_empty() {
        return Err(anyhow::anyhow!("no benchmark tasks found"));
    }

    let provider: std::sync::Arc<dyn tpt_anvil_providers::provider::CloudProvider> =
        if let Some(entry) = handler
            .cloud_providers()
            .iter()
            .find(|e| e.name == provider_name)
        {
            entry.provider.clone()
        } else if let Some(active) = handler.active_cloud_provider() {
            active.clone()
        } else {
            return Err(anyhow::anyhow!(
                "no provider named '{provider_name}' available"
            ));
        };

    let hcfg = handler.handler_config();
    let verify_config = hcfg.verify.clone();
    let proj = hcfg.project_root.clone();

    let mut results = Vec::new();
    let mut total_cost: f64 = 0.0;

    for task in &tasks {
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
        match provider.complete(&request).await {
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
                if let Some(c) = cost {
                    total_cost += c;
                }
                results.push(task_result);
            }
            Err(e) => {
                results.push(anvil_capabilities::benchmark::scorecard::TaskRunResult {
                    task_id: task.id.clone(),
                    task_kind: task.kind,
                    passed: false,
                    latency_ms: start.elapsed().as_millis() as u64,
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
        last_run_at: now,
        core_task_ids_run: task_ids,
        core_results: results,
        adaptive_results: vec![],
        core_score: score,
        adaptive_score: None,
        total_cost_usd: total_cost,
    };

    let store_path = BenchmarkStore::default_path().unwrap_or_default();
    let mut store = BenchmarkStore::load(&store_path);
    store.record(scorecard.clone());
    let _ = store.save(&store_path);

    Ok(serde_json::json!({
        "provider": scorecard.provider,
        "model_id": scorecard.model_id,
        "core_score": scorecard.core_score,
        "total_cost_usd": scorecard.total_cost_usd,
        "tasks_run": scorecard.core_task_ids_run.len(),
        "tasks_passed": scorecard.core_results.iter().filter(|r| r.passed).count(),
        "last_run_at": scorecard.last_run_at,
    }))
}

fn show_benchmark_report_rpc(targets: &[String]) -> anyhow::Result<serde_json::Value> {
    use anvil_capabilities::benchmark::comparison::compare;
    use anvil_capabilities::benchmark::store::BenchmarkStore;

    let store_path = BenchmarkStore::default_path().unwrap_or_default();
    let store = BenchmarkStore::load(&store_path);

    if store.entries().is_empty() {
        return Ok(serde_json::json!({
            "entries": [],
            "message": "no scorecards stored yet"
        }));
    }

    if targets.len() == 2 {
        let (lp, lm) = targets[0]
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("target must be `provider/model`"))?;
        let (rp, rm) = targets[1]
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("target must be `provider/model`"))?;

        let left = store
            .find(lp, lm)
            .ok_or_else(|| anyhow::anyhow!("no scorecard for {}", targets[0]))?;
        let right = store
            .find(rp, rm)
            .ok_or_else(|| anyhow::anyhow!("no scorecard for {}", targets[1]))?;

        let cmp = compare(left, right);

        Ok(serde_json::json!({
            "left": cmp.left_label,
            "right": cmp.right_label,
            "shared_tasks": cmp.shared_task_ids.len(),
            "left_score": cmp.left_shared_score,
            "right_score": cmp.right_shared_score,
            "left_only_tasks": cmp.left_only_task_ids,
            "right_only_tasks": cmp.right_only_task_ids,
        }))
    } else {
        let entries: Vec<serde_json::Value> = store
            .entries()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "provider": e.provider,
                    "model_id": e.model_id,
                    "core_score": e.core_score,
                    "adaptive_score": e.adaptive_score,
                    "total_cost_usd": e.total_cost_usd,
                    "last_run_at": e.last_run_at,
                    "tasks_run": e.core_task_ids_run.len(),
                })
            })
            .collect();

        Ok(serde_json::json!({ "entries": entries }))
    }
}

pub(crate) fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    let secs = now % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let y = 1970 + (days / 1461) * 4 + ((days % 1461) * 4 / 1461);
    let rem = days - ((y - 1970) * 365 + (y - 1970) / 4);
    let doy = rem as u32;
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m_idx = 0;
    let mut d = doy;
    for (i, &md) in month_days.iter().enumerate() {
        if d < md {
            m_idx = i;
            break;
        }
        d -= md;
        if i == 11 {
            m_idx = 11;
        }
    }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", m_idx + 1, d + 1)
}
