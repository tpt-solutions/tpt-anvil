// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::sync::Arc;

use anvil_core::{
    AnvilError, Result,
    types::{CodeContext, CompletionRequest, DiffPatch, StreamChunk},
};
use anvil_inference::backend::InferenceBackend;
use anvil_providers::provider::CloudProvider;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::{
    context::build_messages,
    conversation::ConversationStore,
    diff::{DiffEngine, extract_code_block},
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

pub struct CommandHandler {
    local_backend: Arc<dyn InferenceBackend>,
    cloud_provider: Option<Arc<dyn CloudProvider>>,
    conversations: Arc<Mutex<ConversationStore>>,
}

impl CommandHandler {
    pub fn new(
        local_backend: Arc<dyn InferenceBackend>,
        cloud_provider: Option<Arc<dyn CloudProvider>>,
    ) -> Self {
        Self {
            local_backend,
            cloud_provider,
            conversations: Arc::new(Mutex::new(ConversationStore::default())),
        }
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

        // Collect streamed tokens
        let (collect_tx, mut collect_rx) = mpsc::channel::<StreamChunk>(256);
        let backend = Arc::clone(&self.local_backend);
        let req_clone = request.clone();
        let forward_tx = tx.clone();
        let collect_tx_clone = collect_tx.clone();

        tokio::spawn(async move {
            let _ = backend.stream(&req_clone, collect_tx_clone).await;
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
            Command::Generate | Command::Fix => {
                extract_code_block(&full_response).map(|new_code| {
                    DiffEngine::compute_diff(&ctx.content, &new_code, &ctx.file_path)
                })
            }
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
            ("/generate a REST endpoint", Command::Generate, "a REST endpoint"),
            ("/test", Command::Test, ""),
            ("/explain focus on concurrency", Command::Explain, "focus on concurrency"),
            ("/fix TypeError: cannot read property", Command::Fix, "TypeError: cannot read property"),
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
