// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Integration tests for cloud providers against a mock HTTP server (wiremock).
//!
//! These exercise the request/response wiring, streaming (SSE) parsing, and
//! error handling without contacting any real provider API.

use anvil_core::types::{BackendKind, ChatMessage, CompletionRequest, Role, StreamChunk};
use tpt_anvil_providers::provider::CloudProvider;
use tpt_anvil_providers::{
    anthropic::AnthropicProvider, custom::CustomProvider, openai::OpenAiProvider,
};
use tokio::sync::mpsc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn user_request(prompt: &str) -> CompletionRequest {
    CompletionRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: prompt.to_string(),
        }],
        model: None,
        max_tokens: 128,
        temperature: 0.0,
        stream: false,
    }
}

#[tokio::test]
async fn openai_complete_parses_response() {
    let server = MockServer::start().await;

    let body = serde_json::json!({
        "id": "chatcmpl-1",
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hello from mock"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 3, "total_tokens": 13}
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let provider =
        OpenAiProvider::with_base_url("test-key", "gpt-4o-mini", server.uri(), BackendKind::OpenAi);

    let resp = provider.complete(&user_request("hi")).await.unwrap();
    assert_eq!(resp.content, "hello from mock");
    assert_eq!(resp.model, "gpt-4o-mini");
    let usage = resp.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 3);
    assert_eq!(usage.total_tokens, 13);
}

#[tokio::test]
async fn openai_error_status_is_surfaced() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad key"))
        .mount(&server)
        .await;

    let provider =
        OpenAiProvider::with_base_url("nope", "gpt-4o", server.uri(), BackendKind::OpenAi);
    let err = provider.complete(&user_request("hi")).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("401"), "expected 401 in error, got: {msg}");
}

#[tokio::test]
async fn openai_stream_parses_sse_chunks() {
    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":null}]}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;

    let provider = OpenAiProvider::with_base_url("k", "gpt-4o", server.uri(), BackendKind::OpenAi);
    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    let mut req = user_request("hi");
    req.stream = true;
    provider.stream(&req, tx).await.unwrap();

    let mut assembled = String::new();
    let mut saw_done = false;
    while let Some(chunk) = rx.recv().await {
        assembled.push_str(&chunk.delta);
        if chunk.done {
            saw_done = true;
        }
    }
    assert_eq!(assembled, "Hello");
    assert!(saw_done, "stream should signal done");
}

#[tokio::test]
async fn custom_provider_uses_openai_shape() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "model": "local-model",
        "choices": [{"message": {"role": "assistant", "content": "custom ok"}}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let provider = CustomProvider::new("k", "local-model", server.uri());
    let resp = provider.complete(&user_request("hi")).await.unwrap();
    assert_eq!(resp.content, "custom ok");
}

#[tokio::test]
async fn anthropic_complete_parses_response() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "id": "msg_1",
        "model": "claude-sonnet-5",
        "content": [{"type": "text", "text": "anthropic mock reply"}],
        "usage": {"input_tokens": 12, "output_tokens": 4}
    });
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "ak"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_base_url("ak", "claude-sonnet-5", server.uri());
    let resp = provider.complete(&user_request("hi")).await.unwrap();
    assert_eq!(resp.content, "anthropic mock reply");
    let usage = resp.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 4);
    assert_eq!(usage.total_tokens, 16);
}

#[tokio::test]
async fn anthropic_stream_parses_events() {
    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\" there\"}}\n\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_base_url("ak", "claude-sonnet-5", server.uri());
    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    provider.stream(&user_request("hi"), tx).await.unwrap();

    let mut assembled = String::new();
    let mut saw_done = false;
    while let Some(chunk) = rx.recv().await {
        assembled.push_str(&chunk.delta);
        if chunk.done {
            saw_done = true;
        }
    }
    assert_eq!(assembled, "Hi there");
    assert!(saw_done);
}
