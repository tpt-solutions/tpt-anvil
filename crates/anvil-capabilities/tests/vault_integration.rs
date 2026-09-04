// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Integration test: a spy CloudProvider asserts that a seeded fake API key
//! is never delivered after Vault redaction runs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anvil_capabilities::vault::{self, VaultConfig};
use anvil_core::types::{ChatMessage, CompletionRequest, Role};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tpt_anvil_providers::provider::CloudProvider;
use tpt_anvil_providers::types::{CompletionResponse, ModelInfo, StreamChunk, TokenUsage};

/// Spy provider that records every request it receives and flags whether
/// the secret ever leaked through.
struct SpyProvider {
    saw_secret: AtomicBool,
}

const SECRET_PREFIX: &str = "ghp_";

#[async_trait]
impl CloudProvider for SpyProvider {
    fn name(&self) -> &str {
        "spy"
    }

    fn default_model(&self) -> &str {
        "spy-model"
    }

    async fn list_models(&self) -> tpt_anvil_providers::types::Result<Vec<ModelInfo>> {
        Ok(vec![])
    }

    async fn complete(
        &self,
        request: &tpt_anvil_providers::types::CompletionRequest,
    ) -> tpt_anvil_providers::types::Result<CompletionResponse> {
        for msg in &request.messages {
            if msg.content.contains(SECRET_PREFIX) {
                self.saw_secret.store(true, Ordering::SeqCst);
            }
        }
        Ok(CompletionResponse {
            content: "ok".into(),
            model: "spy-model".into(),
            usage: Some(TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 1,
                total_tokens: 1,
            }),
        })
    }

    async fn stream(
        &self,
        request: &tpt_anvil_providers::types::CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> tpt_anvil_providers::types::Result<()> {
        for msg in &request.messages {
            if msg.content.contains(SECRET_PREFIX) {
                self.saw_secret.store(true, Ordering::SeqCst);
            }
        }
        let _ = tx
            .send(StreamChunk {
                delta: "ok".into(),
                done: true,
            })
            .await;
        Ok(())
    }

    async fn count_tokens(&self, _text: &str) -> tpt_anvil_providers::types::Result<u32> {
        Ok(0)
    }
}

#[tokio::test]
async fn vault_prevents_secret_from_reaching_provider() {
    let spy = Arc::new(SpyProvider {
        saw_secret: AtomicBool::new(false),
    });

    let config = VaultConfig::default();
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz1234567890";

    let mut request = CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: format!("Please use this token: {secret} in the code"),
        }],
        model: None,
        max_tokens: 256,
        temperature: 0.0,
        stream: false,
    };

    let hits = vault::redact_request(&mut request, &config);
    assert!(!hits.is_empty(), "vault should have detected the secret");
    assert!(
        !request.messages[0].content.contains("ghp_"),
        "vault should have redacted the GitHub PAT"
    );

    // Simulate what commands.rs does: convert to provider request and call
    let provider_req = tpt_anvil_providers::types::CompletionRequest {
        messages: request
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
        model: request.model.clone(),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        stream: request.stream,
    };

    let _ = spy.complete(&provider_req).await;

    assert!(
        !spy.saw_secret.load(Ordering::SeqCst),
        "the secret must never reach the provider"
    );
}

#[tokio::test]
async fn vault_streams_without_leaking_secret() {
    let spy = Arc::new(SpyProvider {
        saw_secret: AtomicBool::new(false),
    });

    let config = VaultConfig::default();
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz1234567890";

    let mut request = CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: format!("Embed this token: {secret}"),
        }],
        model: None,
        max_tokens: 256,
        temperature: 0.0,
        stream: true,
    };

    vault::redact_request(&mut request, &config);

    let provider_req = tpt_anvil_providers::types::CompletionRequest {
        messages: request
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
        model: request.model.clone(),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        stream: request.stream,
    };

    let (tx, _rx) = mpsc::channel(64);
    let _ = spy.stream(&provider_req, tx).await;

    assert!(
        !spy.saw_secret.load(Ordering::SeqCst),
        "the secret must never reach the provider via stream"
    );
}

#[tokio::test]
async fn vault_disabled_passes_secret_through() {
    let spy = Arc::new(SpyProvider {
        saw_secret: AtomicBool::new(false),
    });

    let config = VaultConfig {
        enabled: false,
        ..Default::default()
    };

    let mut request = CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "Key is ghp_abcdefghijklmnopqrstuvwxyz1234567890".into(),
        }],
        model: None,
        max_tokens: 256,
        temperature: 0.0,
        stream: false,
    };

    vault::redact_request(&mut request, &config);

    let provider_req = tpt_anvil_providers::types::CompletionRequest {
        messages: request
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
        model: request.model.clone(),
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        stream: request.stream,
    };

    let _ = spy.complete(&provider_req).await;

    assert!(
        spy.saw_secret.load(Ordering::SeqCst),
        "disabled vault should let the secret through"
    );
}
