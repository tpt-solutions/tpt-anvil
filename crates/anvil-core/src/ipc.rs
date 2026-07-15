// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use serde::{Deserialize, Serialize};

use crate::types::{CodeContext, CompletionRequest, DiffPatch};

pub const SOCKET_NAME: &str = "anvil.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params: serde_json::to_value(params).unwrap_or(serde_json::Value::Null),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn ok(id: u64, result: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
            error: None,
        }
    }

    pub fn err(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message: message.into() }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params: serde_json::to_value(params).unwrap_or(serde_json::Value::Null),
        }
    }
}

// --- RPC method parameter types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct SlashCommandParams {
    pub command: String,
    pub context: CodeContext,
    pub args: Option<String>,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawCompletionParams {
    pub request: CompletionRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyDiffParams {
    pub patch: DiffPatch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexProjectParams {
    pub root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub active_backend: String,
    pub active_model: Option<String>,
    pub index_status: String,
}

// --- Streaming notification payload ---
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamTokenNotification {
    pub conversation_id: String,
    pub delta: String,
    pub done: bool,
}
