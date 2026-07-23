// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::types::{ChatMessage, CompletionRequest, Role};

/// Format a message list into a raw prompt string for models that don't accept chat format.
pub fn format_prompt(messages: &[ChatMessage], template: PromptTemplate) -> String {
    match template {
        PromptTemplate::ChatMl => format_chatml(messages),
        PromptTemplate::Llama3 => format_llama3(messages),
        PromptTemplate::Alpaca => format_alpaca(messages),
        PromptTemplate::Raw => messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
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
        out.push_str(&format!(
            "<|im_start|>{role}\n{}\n<|im_end|>\n",
            msg.content
        ));
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

/// Build a single prompt string from a `CompletionRequest`, auto-detecting
/// the appropriate chat template from the model name.
pub fn apply_chat_template(request: &CompletionRequest) -> String {
    let model_id = request.model.as_deref().unwrap_or("deepseek-coder:6.7b");
    let template = PromptTemplate::from_model_id(model_id);
    format_prompt(&request.messages, template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_core::types::{ChatMessage, Role};

    fn msg(role: Role, content: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: content.to_string(),
        }
    }

    #[test]
    fn from_model_id_selects_llama3() {
        assert!(matches!(
            PromptTemplate::from_model_id("llama-3.1-8b"),
            PromptTemplate::Llama3
        ));
        assert!(matches!(
            PromptTemplate::from_model_id("Meta-Llama3"),
            PromptTemplate::Llama3
        ));
    }

    #[test]
    fn from_model_id_selects_alpaca() {
        assert!(matches!(
            PromptTemplate::from_model_id("alpaca-7b"),
            PromptTemplate::Alpaca
        ));
    }

    #[test]
    fn from_model_id_defaults_to_chatml() {
        assert!(matches!(
            PromptTemplate::from_model_id("deepseek-coder:6.7b"),
            PromptTemplate::ChatMl
        ));
        assert!(matches!(
            PromptTemplate::from_model_id("qwen2.5-coder:7b"),
            PromptTemplate::ChatMl
        ));
    }

    #[test]
    fn chatml_format_contains_tokens() {
        let messages = vec![
            msg(Role::System, "You are helpful."),
            msg(Role::User, "Hello!"),
        ];
        let out = format_prompt(&messages, PromptTemplate::ChatMl);
        assert!(out.contains("<|im_start|>system"));
        assert!(out.contains("<|im_start|>user"));
        assert!(out.contains("<|im_start|>assistant"));
        assert!(out.contains("You are helpful."));
        assert!(out.contains("Hello!"));
    }

    #[test]
    fn llama3_format_contains_header_tokens() {
        let messages = vec![msg(Role::User, "Explain Rust lifetimes.")];
        let out = format_prompt(&messages, PromptTemplate::Llama3);
        assert!(out.contains("<|begin_of_text|>"));
        assert!(out.contains("<|start_header_id|>user<|end_header_id|>"));
        assert!(out.contains("Explain Rust lifetimes."));
    }

    #[test]
    fn alpaca_format_with_system() {
        let messages = vec![
            msg(Role::System, "Be concise."),
            msg(Role::User, "What is 2+2?"),
        ];
        let out = format_prompt(&messages, PromptTemplate::Alpaca);
        assert!(out.contains("Be concise."));
        assert!(out.contains("### Instruction:"));
        assert!(out.contains("### Response:"));
    }
}
