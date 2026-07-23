// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::PathBuf;
use std::sync::Arc;

use anvil_config::schema::SmartContextConfig;
use anvil_core::{
    types::{CodeContext, CompletionRequest, DiffPatch, Role, StreamChunk},
    Result,
};
use anvil_inference::backend::InferenceBackend;
use tokio::sync::{mpsc, Mutex};
use tpt_anvil_providers::{
    provider::CloudProvider,
    recent_models::RecentModels,
    router::{self, ProviderEntry, RouterConfig},
};
use tracing::info;

use crate::{
    context::build_messages,
    conversation::ConversationStore,
    diff::{extract_code_block, DiffEngine},
    vault::{self, VaultConfig},
    verify::{self, VerificationResult, VerifyConfig},
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

    /// Whether this command's output is expected to contain a code block we
    /// should try to turn into a diff (and, in turn, verify).
    fn produces_code(&self) -> bool {
        matches!(self, Command::Generate | Command::Fix)
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

/// Config bundle threaded through to `CommandHandler` at construction time,
/// covering the Vault/Verify/Smart-Context/Router features (todo.md Phase
/// 16) that previously had no caller anywhere in the request path.
#[derive(Debug, Clone)]
pub struct HandlerConfig {
    pub vault: VaultConfig,
    pub verify: VerifyConfig,
    pub smart_context: SmartContextConfig,
    pub router: RouterConfig,
    /// Root of the project being edited; used to confine `verify::verify_patch`
    /// file writes and as the working directory for compiler/lint/test runs.
    pub project_root: PathBuf,
}

pub struct CommandHandler {
    local_backend: Arc<dyn InferenceBackend>,
    /// Single-provider cloud fallback used when the router is disabled or no
    /// provider is available in `providers` — preserves the pre-Router
    /// behavior for the common single-provider setup.
    cloud_provider: Option<Arc<dyn CloudProvider>>,
    /// Every usable cloud provider, for router-based selection.
    providers: Vec<ProviderEntry>,
    conversations: Arc<Mutex<ConversationStore>>,
    recent_models: Arc<Mutex<RecentModels>>,
    config: HandlerConfig,
}

impl CommandHandler {
    pub fn new(
        local_backend: Arc<dyn InferenceBackend>,
        cloud_provider: Option<Arc<dyn CloudProvider>>,
        providers: Vec<ProviderEntry>,
        config: HandlerConfig,
    ) -> Self {
        let recent_models = recent_models_path()
            .map(|p| RecentModels::load(&p))
            .unwrap_or_default();
        Self {
            local_backend,
            cloud_provider,
            providers,
            conversations: Arc::new(Mutex::new(ConversationStore::default())),
            recent_models: Arc::new(Mutex::new(recent_models)),
            config,
        }
    }

    /// The last few (provider, model) pairs actually used, most recent first —
    /// lets IDE UIs offer a quick-pick list instead of the full live catalog.
    pub async fn recent_models(&self) -> Vec<tpt_anvil_providers::recent_models::RecentModel> {
        self.recent_models.lock().await.list().to_vec()
    }

    /// Pick which cloud provider to fall back to for this request: the
    /// cost-based router when enabled and providers are available, otherwise
    /// the single configured `cloud_provider` (pre-Router behavior).
    fn select_fallback_provider(
        &self,
        request: &CompletionRequest,
    ) -> Option<Arc<dyn CloudProvider>> {
        if self.config.router.enabled && !self.providers.is_empty() {
            let estimated_prompt_tokens: u32 = request
                .messages
                .iter()
                .map(|m| (m.content.len() / 4) as u32)
                .sum();
            let entry = router::select_provider(
                &self.providers,
                estimated_prompt_tokens,
                request.max_tokens,
                &self.config.router,
            );
            entry.map(|e| Arc::clone(&e.provider))
        } else {
            self.cloud_provider.clone()
        }
    }

    /// Run one generation attempt: stream from the local backend, falling
    /// back to the selected cloud provider if the local backend errors.
    /// Returns the full response text.
    async fn generate(
        &self,
        request: &CompletionRequest,
        tx: &mpsc::Sender<StreamChunk>,
    ) -> String {
        let (collect_tx, mut collect_rx) = mpsc::channel::<StreamChunk>(256);
        let backend = Arc::clone(&self.local_backend);
        let cloud = self.select_fallback_provider(request);
        let req_clone = request.clone();
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
            let _ = tx.send(chunk).await;
            if done {
                break;
            }
        }
        full_response
    }

    /// Run a slash command, streaming output tokens via `tx`.
    /// Returns the full response text, an optional diff patch, and — for
    /// commands that produce code and have verification enabled — the
    /// result of running the project's compiler/linter/tests against it.
    pub async fn run(
        &self,
        input: &str,
        ctx: &CodeContext,
        conversation_id: Option<&str>,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<(String, Option<DiffPatch>, Option<VerificationResult>)> {
        let (command, user_text) = Command::parse(input);
        info!("running command {:?} on {}", command, ctx.file_path);

        let mut messages =
            build_messages(command.as_str(), user_text, ctx, &self.config.smart_context);

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

        let mut request = CompletionRequest {
            messages: messages.clone(),
            model: None,
            max_tokens: 2048,
            temperature: 0.2,
            stream: true,
        };

        // Redact secrets before the request ever leaves this process, for
        // both cloud and local backends. Silent by design: log label+count
        // only, never the matched value.
        let redaction_hits = vault::redact_request(&mut request, &self.config.vault);
        if !redaction_hits.is_empty() {
            let summary: Vec<String> = redaction_hits
                .iter()
                .map(|h| format!("{}x {}", h.count, h.label))
                .collect();
            info!(
                "vault redacted secrets before sending request: {}",
                summary.join(", ")
            );
            vault::log_redactions(&redaction_hits, Some(command.as_str()));
        }

        let mut full_response = self.generate(&request, &tx).await;

        // Store in conversation history
        if let Some(conv_id) = conversation_id {
            let mut store = self.conversations.lock().await;
            let conv = store.get_or_create(conv_id);
            if let Some(last_user) = messages.last() {
                conv.push_user(last_user.content.clone());
            }
            conv.push_assistant(full_response.clone());
        }

        if !command.produces_code() {
            return Ok((full_response, None, None));
        }

        let Some(mut new_code) = extract_code_block(&full_response) else {
            return Ok((full_response, None, None));
        };
        let mut patch = DiffEngine::compute_diff(&ctx.content, &new_code, &ctx.file_path);

        if !self.config.verify.enabled {
            return Ok((full_response, Some(patch), None));
        }

        let mut verification = verify::verify_patch(
            &ctx.content,
            &new_code,
            &ctx.file_path,
            &self.config.project_root,
            &self.config.verify,
        )
        .await;

        // One bounded retry: feed the failure back to the model as an
        // additional turn and try again before giving up.
        let max_retries = self.config.verify.max_retries;
        let mut retries_used: u32 = 0;
        let mut retries_left = max_retries;
        while !verification.passed && retries_left > 0 {
            retries_left -= 1;
            retries_used += 1;
            let mut retry_messages = messages.clone();
            retry_messages.push(anvil_core::types::ChatMessage {
                role: Role::Assistant,
                content: full_response.clone(),
            });
            retry_messages.push(anvil_core::types::ChatMessage {
                role: Role::User,
                content: format!(
                    "That change failed verification with the following errors. Fix them and output the corrected code:\n\n{}",
                    verification.errors.join("\n\n")
                ),
            });
            let mut retry_request = CompletionRequest {
                messages: retry_messages,
                model: None,
                max_tokens: 2048,
                temperature: 0.2,
                stream: true,
            };
            vault::redact_request(&mut retry_request, &self.config.vault);

            full_response = self.generate(&retry_request, &tx).await;
            let Some(retried_code) = extract_code_block(&full_response) else {
                break;
            };
            new_code = retried_code;
            patch = DiffEngine::compute_diff(&ctx.content, &new_code, &ctx.file_path);
            verification = verify::verify_patch(
                &ctx.content,
                &new_code,
                &ctx.file_path,
                &self.config.project_root,
                &self.config.verify,
            )
            .await;
        }

        verification.retries_used = retries_used;
        verification.max_retries = max_retries;
        Ok((full_response, Some(patch), Some(verification)))
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
