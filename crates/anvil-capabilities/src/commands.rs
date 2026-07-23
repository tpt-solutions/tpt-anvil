// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use anvil_core::{
    types::{CodeContext, CompletionRequest, DiffPatch, Role, StreamChunk},
    Result,
};
use anvil_inference::backend::InferenceBackend;
use tokio::sync::{mpsc, Mutex};
use tpt_anvil_providers::{provider::CloudProvider, recent_models::RecentModels};
use tracing::info;

use crate::{
    context::build_messages,
    conversation::ConversationStore,
    diff::{extract_code_block, DiffEngine},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Generate,
    Test,
    Explain,
    Fix,
    Docs,
    Chat,
}

impl Command {
    pub fn parse(input: &str) -> (Self, &str) {
        let trimmed = input.trim();
        if let Some(rest) = trimmed.strip_prefix("/generate") {
            (Command::Generate, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("/test") {
            (Command::Test, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("/explain") {
            (Command::Explain, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("/fix") {
            (Command::Fix, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("/docs") {
            (Command::Docs, rest.trim())
        } else {
            (Command::Chat, trimmed)
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Command::Generate => "/generate",
            Command::Test => "/test",
            Command::Explain => "/explain",
            Command::Fix => "/fix",
            Command::Docs => "/docs",
            Command::Chat => "/chat",
        }
    }
}

/// `tpt-anvil-providers` is decoupled from `anvil-core` for standalone
/// crates.io publishing, so it defines its own `CompletionRequest`/`ChatMessage`
/// instead of depending on `anvil_core::types` directly. These convert between
/// the two shapes at the one place they meet: the cloud-provider call site.
fn to_provider_request(req: &CompletionRequest) -> tpt_anvil_providers::types::CompletionRequest {
    tpt_anvil_providers::types::CompletionRequest {
        messages: req
            .messages
            .iter()
            .map(|m| tpt_anvil_providers::types::ChatMessage {
                role: match m.role {
                    Role::System => tpt_anvil_providers::types::Role::System,
                    Role::User => tpt_anvil_providers::types::Role::User,
                    Role::Assistant => tpt_anvil_providers::types::Role::Assistant,
                },
                content: m.content.clone(),
            })
            .collect(),
        model: req.model.clone(),
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        stream: req.stream,
    }
}

fn from_provider_chunk(chunk: tpt_anvil_providers::types::StreamChunk) -> StreamChunk {
    StreamChunk {
        delta: chunk.delta,
        done: chunk.done,
    }
}

/// Where the recently-used-models list is persisted, so it survives daemon
/// restarts (`~/.config/anvil/recent_models.json` or platform equivalent).
fn recent_models_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("anvil").join("recent_models.json"))
}

pub struct CommandHandler {
    local_backend: Arc<dyn InferenceBackend>,
    cloud_provider: Option<Arc<dyn CloudProvider>>,
    conversations: Arc<Mutex<ConversationStore>>,
    recent_models: Arc<Mutex<RecentModels>>,
}

impl CommandHandler {
    pub fn new(
        local_backend: Arc<dyn InferenceBackend>,
        cloud_provider: Option<Arc<dyn CloudProvider>>,
    ) -> Self {
        let recent_models = recent_models_path()
            .map(|p| RecentModels::load(&p))
            .unwrap_or_default();
        Self {
            local_backend,
            cloud_provider,
            conversations: Arc::new(Mutex::new(ConversationStore::default())),
            recent_models: Arc::new(Mutex::new(recent_models)),
        }
    }

    /// The last few (provider, model) pairs actually used, most recent first —
    /// lets IDE UIs offer a quick-pick list instead of the full live catalog.
    pub async fn recent_models(&self) -> Vec<tpt_anvil_providers::recent_models::RecentModel> {
        self.recent_models.lock().await.list().to_vec()
    }

    /// Run a slash command, streaming output tokens via `tx`.
    /// Returns the full response text and optionally a diff patch.
    pub async fn run(
        &self,
        input: &str,
        ctx: &CodeContext,
        conversation_id: Option<&str>,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(String, Option<DiffPatch>)> {
        let (command, user_text) = Command::parse(input);
        info!("running command {:?} on {}", command, ctx.file_path);

        let mut messages = build_messages(command.as_str(), user_text, ctx);

        // Append conversation history if this is a multi-turn chat
        if let Some(conv_id) = conversation_id {
            let store = self.conversations.lock().await;
            if let Some(conv) = store.get(conv_id) {
                // Insert history before the last user message
                let user_msg = messages.pop().unwrap();
                for m in &conv.messages {
                    messages.push(m.clone());
                }
                messages.push(user_msg);
            }
        }

        let request = CompletionRequest {
            messages: messages.clone(),
            model: None,
            max_tokens: 2048,
            temperature: 0.2,
            stream: true,
        };

        // Collect streamed tokens. Prefer the local backend; if it fails to
        // produce any output and a cloud provider is configured, fall back to it.
        let (collect_tx, mut collect_rx) = mpsc::channel::<StreamChunk>(256);
        let backend = Arc::clone(&self.local_backend);
        let cloud = self.cloud_provider.clone();
        let req_clone = request.clone();
        let forward_tx = tx.clone();
        let collect_tx_clone = collect_tx.clone();
        let recent_models = Arc::clone(&self.recent_models);

        tokio::spawn(async move {
            match backend.stream(&req_clone, collect_tx_clone.clone()).await {
                Ok(()) => {}
                Err(e) => {
                    if let Some(provider) = cloud {
                        info!(
                            "local backend failed ({e}); falling back to cloud provider {}",
                            provider.name()
                        );
                        let provider_req = to_provider_request(&req_clone);
                        let (provider_tx, mut provider_rx) =
                            mpsc::channel::<tpt_anvil_providers::types::StreamChunk>(256);
                        let forward = tokio::spawn(async move {
                            while let Some(chunk) = provider_rx.recv().await {
                                let done = chunk.done;
                                let _ = collect_tx_clone.send(from_provider_chunk(chunk)).await;
                                if done {
                                    break;
                                }
                            }
                        });
                        let result = provider.stream(&provider_req, provider_tx).await;
                        let _ = forward.await;
                        if result.is_ok() {
                            let model_used = req_clone
                                .model
                                .clone()
                                .unwrap_or_else(|| provider.default_model().to_string());
                            let mut recent = recent_models.lock().await;
                            recent.record(provider.name(), model_used);
                            if let Some(path) = recent_models_path() {
                                let _ = recent.save(&path);
                            }
                        }
                    }
                }
            }
        });

        let mut full_response = String::new();
        while let Some(chunk) = collect_rx.recv().await {
            full_response.push_str(&chunk.delta);
            let done = chunk.done;
            let _ = forward_tx.send(chunk).await;
            if done {
                break;
            }
        }

        // Store in conversation history
        if let Some(conv_id) = conversation_id {
            let mut store = self.conversations.lock().await;
            let conv = store.get_or_create(conv_id);
            if let Some(last_user) = messages.last() {
                conv.push_user(last_user.content.clone());
            }
            conv.push_assistant(full_response.clone());
        }

        // Try to extract a diff for commands that modify code
        let patch = match command {
            Command::Generate | Command::Fix => extract_code_block(&full_response)
                .map(|new_code| DiffEngine::compute_diff(&ctx.content, &new_code, &ctx.file_path)),
            _ => None,
        };

        Ok((full_response, patch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_slash_commands() {
        let cases = [
            (
                "/generate a REST endpoint",
                Command::Generate,
                "a REST endpoint",
            ),
            ("/test", Command::Test, ""),
            (
                "/explain focus on concurrency",
                Command::Explain,
                "focus on concurrency",
            ),
            (
                "/fix TypeError: cannot read property",
                Command::Fix,
                "TypeError: cannot read property",
            ),
            ("/docs include examples", Command::Docs, "include examples"),
            ("what does this do?", Command::Chat, "what does this do?"),
        ];

        for (input, expected_cmd, expected_rest) in cases {
            let (cmd, rest) = Command::parse(input);
            assert_eq!(cmd, expected_cmd, "input: {input}");
            assert_eq!(rest, expected_rest, "input: {input}");
        }
    }

    #[test]
    fn parse_strips_whitespace() {
        let (cmd, rest) = Command::parse("  /generate   a function  ");
        assert_eq!(cmd, Command::Generate);
        assert_eq!(rest, "a function");
    }

    #[test]
    fn command_as_str_round_trips() {
        assert_eq!(Command::Generate.as_str(), "/generate");
        assert_eq!(Command::Test.as_str(), "/test");
        assert_eq!(Command::Explain.as_str(), "/explain");
        assert_eq!(Command::Fix.as_str(), "/fix");
        assert_eq!(Command::Docs.as_str(), "/docs");
    }
}
