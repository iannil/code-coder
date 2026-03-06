//! Session compaction - context window management

use serde::{Deserialize, Serialize};
use super::message::{Message, MessageRole};

/// Compaction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Remove oldest messages first
    RemoveOldest,
    /// Summarize older messages
    Summarize,
    /// Hybrid: summarize then remove
    Hybrid,
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Result of compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    /// Number of messages before compaction
    pub messages_before: usize,
    /// Number of messages after compaction
    pub messages_after: usize,
    /// Tokens before compaction
    pub tokens_before: usize,
    /// Tokens after compaction
    pub tokens_after: usize,
    /// Summary generated (if any)
    pub summary: Option<String>,
}

/// Compactor for managing context window
pub struct Compactor {
    /// Maximum tokens allowed
    max_tokens: usize,
    /// Target tokens after compaction
    target_tokens: usize,
    /// Strategy to use
    strategy: CompactionStrategy,
}

impl Default for Compactor {
    fn default() -> Self {
        Self::new(128_000, 100_000)
    }
}

impl Compactor {
    /// Create a new compactor with token limits
    pub fn new(max_tokens: usize, target_tokens: usize) -> Self {
        Self {
            max_tokens,
            target_tokens,
            strategy: CompactionStrategy::default(),
        }
    }

    /// Set the compaction strategy
    pub fn with_strategy(mut self, strategy: CompactionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Check if compaction is needed
    pub fn needs_compaction(&self, messages: &[Message]) -> bool {
        let total_tokens: usize = messages.iter().filter_map(|m| m.tokens).sum();
        total_tokens > self.max_tokens
    }

    /// Compact messages to fit within token limit
    pub fn compact(&self, messages: &mut Vec<Message>) -> CompactionResult {
        let tokens_before: usize = messages.iter().filter_map(|m| m.tokens).sum();
        let messages_before = messages.len();

        if !self.needs_compaction(messages) {
            return CompactionResult {
                messages_before,
                messages_after: messages_before,
                tokens_before,
                tokens_after: tokens_before,
                summary: None,
            };
        }

        match self.strategy {
            CompactionStrategy::RemoveOldest => {
                self.remove_oldest(messages);
            }
            CompactionStrategy::Summarize => {
                // Summarization requires LLM call - not implemented here
                // In practice, this would be handled by the caller
                self.remove_oldest(messages);
            }
            CompactionStrategy::Hybrid => {
                self.remove_oldest(messages);
            }
        }

        let tokens_after: usize = messages.iter().filter_map(|m| m.tokens).sum();

        CompactionResult {
            messages_before,
            messages_after: messages.len(),
            tokens_before,
            tokens_after,
            summary: None,
        }
    }

    /// Remove oldest messages until under target
    fn remove_oldest(&self, messages: &mut Vec<Message>) {
        // Find the split point - keep system messages and recent history
        let mut keep_from = 0;
        let mut running_tokens: usize = 0;

        // Calculate tokens from the end
        for (i, msg) in messages.iter().rev().enumerate() {
            let tokens = msg.tokens.unwrap_or(msg.content.len() / 4);
            if running_tokens + tokens > self.target_tokens {
                keep_from = messages.len() - i;
                break;
            }
            running_tokens += tokens;
        }

        // Always keep system messages at the start
        let system_messages: Vec<Message> = messages
            .iter()
            .take(keep_from)
            .filter(|m| m.role == MessageRole::System)
            .cloned()
            .collect();

        let kept_messages: Vec<Message> = messages.drain(keep_from..).collect();

        messages.clear();
        messages.extend(system_messages);
        messages.extend(kept_messages);
    }

    /// Estimate tokens for a string
    pub fn estimate_tokens(text: &str) -> usize {
        // Use the unified tokenizer
        crate::memory::tokenizer::estimate_tokens(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_messages(count: usize, tokens_each: usize) -> Vec<Message> {
        (0..count)
            .map(|i| {
                let mut msg = Message::user(format!("Message {}", i));
                msg.tokens = Some(tokens_each);
                msg
            })
            .collect()
    }

    #[test]
    fn test_needs_compaction() {
        let compactor = Compactor::new(1000, 800);

        let small_messages = create_test_messages(5, 100);
        assert!(!compactor.needs_compaction(&small_messages));

        let large_messages = create_test_messages(20, 100);
        assert!(compactor.needs_compaction(&large_messages));
    }

    #[test]
    fn test_compact_removes_oldest() {
        let compactor = Compactor::new(1000, 500);
        let mut messages = create_test_messages(20, 100);

        let result = compactor.compact(&mut messages);

        assert!(result.tokens_after <= 1000);
        assert!(messages.len() < 20);
    }

    #[test]
    fn test_compact_preserves_system_messages() {
        let compactor = Compactor::new(500, 300);
        let mut messages = vec![
            Message::system("System prompt"),
            Message::user("User 1"),
            Message::assistant("Assistant 1"),
            Message::user("User 2"),
            Message::assistant("Assistant 2"),
        ];

        for msg in &mut messages {
            msg.tokens = Some(100);
        }

        compactor.compact(&mut messages);

        // System message should be preserved
        assert!(messages.iter().any(|m| m.role == MessageRole::System));
    }
}
