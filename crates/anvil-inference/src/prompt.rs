// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::types::{ChatMessage, Role};

/// Format a message list into a raw prompt string for models that don't accept chat format.
pub fn format_prompt(messages: &[ChatMessage], template: PromptTemplate) -> String {
    match template {
        PromptTemplate::ChatMl => format_chatml(messages),
        PromptTemplate::Llama3 => format_llama3(messages),
        PromptTemplate::Alpaca => format_alpaca(messages),
        PromptTemplate::Raw => messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n"),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PromptTemplate {
    ChatMl,
    Llama3,
    Alpaca,
    Raw,
}

impl PromptTemplate {
    pub fn from_model_id(model_id: &str) -> Self {
        let id = model_id.to_lowercase();
        if id.contains("llama-3") || id.contains("llama3") {
            PromptTemplate::Llama3
        } else if id.contains("alpaca") {
            PromptTemplate::Alpaca
        } else {
            PromptTemplate::ChatMl
        }
    }
}

fn format_chatml(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for msg in messages {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        out.push_str(&format!("<|im_start|>{role}\n{}\n<|im_end|>\n", msg.content));
    }
    out.push_str("<|im_start|>assistant\n");
    out
}

fn format_llama3(messages: &[ChatMessage]) -> String {
    let mut out = String::from("<|begin_of_text|>");
    for msg in messages {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        out.push_str(&format!(
            "<|start_header_id|>{role}<|end_header_id|>\n\n{}\n<|eot_id|>",
            msg.content
        ));
    }
    out.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
    out
}

fn format_alpaca(messages: &[ChatMessage]) -> String {
    let mut system = String::new();
    let mut instruction = String::new();
    for msg in messages {
        match msg.role {
            Role::System => system = msg.content.clone(),
            Role::User => instruction = msg.content.clone(),
            Role::Assistant => {}
        }
    }
    if system.is_empty() {
        format!("### Instruction:\n{instruction}\n\n### Response:\n")
    } else {
        format!("{system}\n\n### Instruction:\n{instruction}\n\n### Response:\n")
    }
}
