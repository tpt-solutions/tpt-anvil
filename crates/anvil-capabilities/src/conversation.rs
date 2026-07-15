// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::collections::HashMap;

use anvil_core::types::{ChatMessage, Role};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), messages: Vec::new() }
    }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage { role: Role::User, content: content.into() });
    }

    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage { role: Role::Assistant, content: content.into() });
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.messages.insert(0, ChatMessage { role: Role::System, content: system.into() });
        self
    }
}

#[derive(Default)]
pub struct ConversationStore {
    conversations: HashMap<String, Conversation>,
}

impl ConversationStore {
    pub fn get_or_create(&mut self, id: &str) -> &mut Conversation {
        self.conversations.entry(id.to_string()).or_insert_with(|| Conversation::new(id))
    }

    pub fn get(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    pub fn remove(&mut self, id: &str) {
        self.conversations.remove(id);
    }
}
