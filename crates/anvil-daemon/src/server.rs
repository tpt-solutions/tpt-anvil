// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::Path;
use std::sync::Arc;

use anvil_capabilities::commands::CommandHandler;
use anvil_config::loader::ConfigLoader;
use anvil_core::{
    ipc::{JsonRpcRequest, JsonRpcResponse, SlashCommandParams, StatusResponse},
    types::StreamChunk,
};
use anvil_inference::registry::BackendRegistry;
use anvil_providers::registry::ProviderRegistry;
use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::pid;

pub async fn run(project_root: Option<&str>) -> Result<()> {
    let project_path = project_root.map(Path::new);
    let cfg = ConfigLoader::load(project_path)?;

    let backend_registry =
        BackendRegistry::from_config(&cfg).context("failed to initialize inference backend")?;
    let provider_registry = ProviderRegistry::from_config(&cfg)?;

    let backend = Arc::clone(&backend_registry.active);
    let cloud = provider_registry.active.map(|p| Arc::clone(&p));

    let handler = Arc::new(CommandHandler::new(backend, cloud));

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

    #[cfg(unix)]
    run_unix(handler, shutdown).await?;

    #[cfg(windows)]
    run_windows(handler, shutdown).await?;

    pid::remove_pid();
    Ok(())
}

// ── Unix socket transport ──────────────────────────────────────────────────

#[cfg(unix)]
async fn run_unix(handler: Arc<CommandHandler>, shutdown: Arc<tokio::sync::Notify>) -> Result<()> {
    use tokio::net::UnixListener;

    let socket_path = unix_socket_path();
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    std::fs::create_dir_all(socket_path.parent().unwrap())?;

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind IPC socket at {}", socket_path.display()))?;

    info!("Anvil daemon listening on {}", socket_path.display());

    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, _)) => {
                        let handler = Arc::clone(&handler);
                        tokio::spawn(async move {
                            let (reader, writer) = stream.into_split();
                            handle_rpc(reader, writer, handler).await;
                        });
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
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| std::env::temp_dir())
        .join("anvil")
        .join("anvil.sock")
}

// ── Windows named pipe transport ──────────────────────────────────────────

#[cfg(windows)]
fn pipe_name() -> &'static str {
    r"\\.\pipe\anvil"
}

#[cfg(windows)]
async fn run_windows(
    handler: Arc<CommandHandler>,
    shutdown: Arc<tokio::sync::Notify>,
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

        tokio::select! {
            connect = pipe.connect() => {
                match connect {
                    Ok(()) => {
                        tokio::spawn(async move {
                            let (reader, writer) = tokio::io::split(pipe);
                            handle_rpc(reader, writer, handler).await;
                        });
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

async fn handle_rpc<R, W>(reader: R, mut writer: W, handler: Arc<CommandHandler>)
where
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

                tokio::spawn(async move {
                    let _ = handler_clone
                        .run(&command, &ctx, conv_id.as_deref(), token_tx)
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

                JsonRpcResponse::ok(id, serde_json::json!({ "content": full }))
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
