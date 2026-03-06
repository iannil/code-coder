//! Message types and storage

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool result
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::Tool => write!(f, "tool"),
        }
    }
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,

    /// Message role
    pub role: MessageRole,

    /// Message content
    pub content: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Token count (estimated)
    pub tokens: Option<usize>,

    /// Associated tool call ID (for tool messages)
    pub tool_call_id: Option<String>,

    /// Tool name (for tool messages)
    pub tool_name: Option<String>,

    /// Whether this message has been compacted
    #[serde(default)]
    pub compacted: bool,

    /// Metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Message {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            tokens: None,
            tool_call_id: None,
            tool_name: None,
            compacted: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tokens: None,
            tool_call_id: None,
            tool_name: None,
            compacted: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
            tokens: None,
            tool_call_id: None,
            tool_name: None,
            compacted: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a new tool message
    pub fn tool(tool_call_id: impl Into<String>, tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Tool,
            content: content.into(),
            timestamp: Utc::now(),
            tokens: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_name: Some(tool_name.into()),
            compacted: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Estimate token count for this message
    pub fn estimate_tokens(&mut self) {
        // Simple estimation: ~4 characters per token
        self.tokens = Some(self.content.len() / 4 + 1);
    }
}

/// In-memory message store
#[derive(Debug, Default)]
pub struct MessageStore {
    messages: Vec<Message>,
}

impl MessageStore {
    /// Create a new empty message store
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the store
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get mutable reference to all messages
    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    /// Get the last N messages
    pub fn last_n(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Get total token count (estimated)
    pub fn total_tokens(&self) -> usize {
        self.messages
            .iter()
            .filter_map(|m| m.tokens)
            .sum()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get message count
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_store() {
        let mut store = MessageStore::new();

        store.push(Message::user("Hello"));
        store.push(Message::assistant("Hi!"));

        assert_eq!(store.len(), 2);
        assert_eq!(store.messages()[0].role, MessageRole::User);
        assert_eq!(store.messages()[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_token_estimation() {
        let mut msg = Message::user("Hello, world!");
        msg.estimate_tokens();
        assert!(msg.tokens.is_some());
        assert!(msg.tokens.unwrap() > 0);
    }
}
