// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::types::{ChatMessage, CodeContext, Role};

pub fn build_system_prompt(command: &str) -> String {
    match command {
        "/generate" => "You are an expert programmer. Generate clean, idiomatic code based on the user's description and the provided codebase context. Output only code unless asked for explanation. Use the same language and style as the surrounding code.".into(),
        "/test" => "You are an expert at writing unit tests. Given the selected code, generate comprehensive unit tests. Cover the happy path, edge cases, and error conditions. Use the same testing framework already in use in the project.".into(),
        "/explain" => "You are a clear and concise technical communicator. Explain the selected code in plain language. Describe what it does, how it works, and any important patterns or gotchas. Be precise but accessible.".into(),
        "/fix" => "You are an expert debugger. Analyze the selected code and any provided error messages. Identify the root cause of the bug and provide a fix. Show only the corrected code with a brief explanation of what was wrong.".into(),
        "/docs" => "You are a documentation expert. Generate clear, accurate docstrings and inline documentation for the selected code. Follow the conventions of the language (e.g. rustdoc, JSDoc, docstrings). Include parameter descriptions, return values, and examples where helpful.".into(),
        _ => "You are a helpful AI coding assistant. Answer the user's question about the provided code.".into(),
    }
}

pub fn assemble_context_message(ctx: &CodeContext) -> String {
    let mut parts = Vec::new();

    parts.push(format!("**File:** `{}`", ctx.file_path));
    parts.push(format!("**Language:** {}", ctx.language));

    if let Some(sel) = &ctx.selection {
        parts.push(format!(
            "**Selected code (lines {}-{}):**\n```{}\n{}\n```",
            sel.start_line, sel.end_line, ctx.language, ctx.content
        ));
    } else {
        parts.push(format!("**Current file content:**\n```{}\n{}\n```", ctx.language, ctx.content));
    }

    if !ctx.related_chunks.is_empty() {
        parts.push("**Related context from codebase:**".into());
        for chunk in ctx.related_chunks.iter().take(5) {
            parts.push(format!("`{}`: {}", chunk.file_path, chunk.content));
        }
    }

    parts.join("\n\n")
}

pub fn build_messages(command: &str, user_input: &str, ctx: &CodeContext) -> Vec<ChatMessage> {
    vec![
        ChatMessage { role: Role::System, content: build_system_prompt(command) },
        ChatMessage { role: Role::User, content: format!("{}\n\n{}", assemble_context_message(ctx), user_input) },
    ]
}
