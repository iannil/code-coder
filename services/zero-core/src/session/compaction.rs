//! Session compaction - context window management
//!
//! Provides two levels of compaction:
//! - **Prune**: Mark old tool outputs as compacted (fast, deterministic)
//! - **Compact**: Remove oldest messages entirely (fallback when prune is insufficient)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

// ============================================================================
// Prune Types - for marking old tool outputs as compacted
// ============================================================================

/// Configuration for pruning tool outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneConfig {
    /// Minimum tokens to prune before executing (default: 20_000)
    pub minimum: usize,
    /// Protected token threshold - keep this many recent tokens (default: 40_000)
    pub protect: usize,
    /// Tools to never prune (e.g., "skill")
    pub protected_tools: Vec<String>,
}

impl Default for PruneConfig {
    fn default() -> Self {
        Self {
            minimum: 20_000,
            protect: 40_000,
            protected_tools: vec!["skill".to_string()],
        }
    }
}

/// Reference to a message part that should be pruned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartReference {
    /// Message ID
    pub message_id: String,
    /// Part ID
    pub part_id: String,
    /// Tool name
    pub tool: String,
    /// Estimated tokens in this part
    pub tokens: usize,
}

/// Result of prune planning (which parts to mark as compacted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunePlan {
    /// Parts to mark as compacted
    pub parts_to_prune: Vec<PartReference>,
    /// Total tokens that will be pruned
    pub total_tokens_to_prune: usize,
    /// Whether the prune should be executed (pruned >= minimum)
    pub should_execute: bool,
}

/// Model token limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelLimit {
    /// Context window size (total tokens)
    pub context: usize,
    /// Maximum output tokens
    pub output: usize,
    /// Maximum input tokens (optional, derived from context - output)
    pub input: Option<usize>,
}

/// Current token usage from an assistant response
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens
    pub input: usize,
    /// Output tokens
    pub output: usize,
    /// Cache read tokens
    pub cache_read: usize,
    /// Cache write tokens
    pub cache_write: usize,
}

impl TokenUsage {
    /// Calculate total tokens used (input + cache read + output)
    pub fn total(&self) -> usize {
        self.input + self.cache_read + self.output
    }
}

/// Tool part info for prune computation (passed from TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPartInfo {
    /// Message ID
    pub message_id: String,
    /// Part ID
    pub part_id: String,
    /// Tool name
    pub tool: String,
    /// Tool status (completed, error, etc.)
    pub status: String,
    /// Whether already compacted
    pub compacted: bool,
    /// Output text (for token estimation)
    pub output: String,
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

    /// Check if token usage overflows the model's context limit
    ///
    /// Returns true if compaction is needed based on model limits.
    /// This is the "usable" tokens calculation from TypeScript.
    pub fn is_overflow(tokens: &TokenUsage, limit: &ModelLimit) -> bool {
        if limit.context == 0 {
            return false;
        }

        let count = tokens.total();

        // Calculate usable tokens: either explicit input limit or (context - output)
        // Cap output at a reasonable max (like SessionPrompt.OUTPUT_TOKEN_MAX = 32768)
        const OUTPUT_TOKEN_MAX: usize = 32768;
        let effective_output = std::cmp::min(limit.output, OUTPUT_TOKEN_MAX);
        let effective_output = if effective_output == 0 {
            OUTPUT_TOKEN_MAX
        } else {
            effective_output
        };

        let usable = limit.input.unwrap_or(limit.context.saturating_sub(effective_output));

        count > usable
    }

    /// Compute a prune plan for tool outputs
    ///
    /// Goes backwards through tool parts, protecting recent ones up to `protect` tokens,
    /// then marks older ones for pruning. Returns plan with parts to prune.
    ///
    /// The actual pruning (marking parts as compacted) is done by TypeScript.
    pub fn compute_prune_plan(
        tool_parts: &[ToolPartInfo],
        config: &PruneConfig,
        _is_summary_mode: bool,
    ) -> PrunePlan {
        let protected_tools: HashSet<&str> =
            config.protected_tools.iter().map(|s| s.as_str()).collect();

        let mut total_tool_tokens: usize = 0;
        let mut parts_to_prune: Vec<PartReference> = Vec::new();
        let mut total_tokens_to_prune: usize = 0;
        let _turns_seen = 0;

        // Process tool parts in reverse order (newest first)
        for part in tool_parts.iter().rev() {
            // Skip if already compacted
            if part.compacted {
                // If we encounter a compacted part, we've reached a boundary
                break;
            }

            // Skip non-completed tools
            if part.status != "completed" {
                continue;
            }

            // Skip protected tools
            if protected_tools.contains(part.tool.as_str()) {
                continue;
            }

            // Estimate tokens for this tool output
            let tokens = Self::estimate_tokens(&part.output);
            total_tool_tokens += tokens;

            // Keep recent tool outputs protected
            if total_tool_tokens <= config.protect {
                continue;
            }

            // This part is beyond the protection threshold - mark for pruning
            parts_to_prune.push(PartReference {
                message_id: part.message_id.clone(),
                part_id: part.part_id.clone(),
                tool: part.tool.clone(),
                tokens,
            });
            total_tokens_to_prune += tokens;
        }

        // Only execute if we have enough tokens to prune
        let should_execute = total_tokens_to_prune >= config.minimum;

        PrunePlan {
            parts_to_prune,
            total_tokens_to_prune,
            should_execute,
        }
    }

    /// Compute prune plan with message-level turn tracking
    ///
    /// This version tracks user message "turns" to skip the most recent 2 turns,
    /// matching the TypeScript behavior more closely.
    pub fn compute_prune_plan_with_turns(
        messages: &[MessageInfo],
        config: &PruneConfig,
    ) -> PrunePlan {
        let protected_tools: HashSet<&str> =
            config.protected_tools.iter().map(|s| s.as_str()).collect();

        let mut total_tool_tokens: usize = 0;
        let mut parts_to_prune: Vec<PartReference> = Vec::new();
        let mut total_tokens_to_prune: usize = 0;
        let mut turns_seen = 0;

        // Process messages in reverse order
        for msg in messages.iter().rev() {
            // Count user message turns
            if msg.role == "user" {
                turns_seen += 1;
            }

            // Skip the most recent 2 turns
            if turns_seen < 2 {
                continue;
            }

            // If we hit a summary message, stop processing
            if msg.is_summary {
                break;
            }

            // Process tool parts in this message (also in reverse)
            for part in msg.tool_parts.iter().rev() {
                // If we encounter a compacted part, stop
                if part.compacted {
                    // Return early with what we have
                    return PrunePlan {
                        parts_to_prune,
                        total_tokens_to_prune,
                        should_execute: total_tokens_to_prune >= config.minimum,
                    };
                }

                // Skip non-completed tools
                if part.status != "completed" {
                    continue;
                }

                // Skip protected tools
                if protected_tools.contains(part.tool.as_str()) {
                    continue;
                }

                // Estimate tokens
                let tokens = Self::estimate_tokens(&part.output);
                total_tool_tokens += tokens;

                // Keep recent outputs protected
                if total_tool_tokens <= config.protect {
                    continue;
                }

                // Mark for pruning
                parts_to_prune.push(PartReference {
                    message_id: part.message_id.clone(),
                    part_id: part.part_id.clone(),
                    tool: part.tool.clone(),
                    tokens,
                });
                total_tokens_to_prune += tokens;
            }
        }

        PrunePlan {
            parts_to_prune,
            total_tokens_to_prune,
            should_execute: total_tokens_to_prune >= config.minimum,
        }
    }
}

/// Message info for prune computation (passed from TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    /// Message ID
    pub message_id: String,
    /// Message role (user, assistant)
    pub role: String,
    /// Whether this is a summary message
    pub is_summary: bool,
    /// Tool parts in this message
    pub tool_parts: Vec<ToolPartInfo>,
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

    #[test]
    fn test_is_overflow() {
        let tokens = TokenUsage {
            input: 100_000,
            output: 5_000,
            cache_read: 10_000,
            cache_write: 0,
        };

        let limit = ModelLimit {
            context: 128_000,
            output: 8_192,
            input: None,
        };

        // Total = 115_000, usable = 128_000 - 8_192 = 119_808
        assert!(!Compactor::is_overflow(&tokens, &limit));

        let tokens_over = TokenUsage {
            input: 110_000,
            output: 5_000,
            cache_read: 10_000,
            cache_write: 0,
        };

        // Total = 125_000 > 119_808
        assert!(Compactor::is_overflow(&tokens_over, &limit));
    }

    #[test]
    fn test_is_overflow_zero_context() {
        let tokens = TokenUsage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 0,
            cache_write: 0,
        };

        let limit = ModelLimit {
            context: 0, // Unlimited
            output: 8_192,
            input: None,
        };

        // Should never overflow with zero context
        assert!(!Compactor::is_overflow(&tokens, &limit));
    }

    #[test]
    fn test_compute_prune_plan() {
        let tool_parts = vec![
            ToolPartInfo {
                message_id: "msg1".to_string(),
                part_id: "part1".to_string(),
                tool: "read".to_string(),
                status: "completed".to_string(),
                compacted: false,
                output: "a".repeat(50_000 * 4), // ~50k tokens
            },
            ToolPartInfo {
                message_id: "msg2".to_string(),
                part_id: "part2".to_string(),
                tool: "read".to_string(),
                status: "completed".to_string(),
                compacted: false,
                output: "b".repeat(30_000 * 4), // ~30k tokens
            },
            ToolPartInfo {
                message_id: "msg3".to_string(),
                part_id: "part3".to_string(),
                tool: "read".to_string(),
                status: "completed".to_string(),
                compacted: false,
                output: "c".repeat(10_000 * 4), // ~10k tokens (protected - within 40k)
            },
        ];

        let config = PruneConfig::default(); // minimum: 20k, protect: 40k

        let plan = Compactor::compute_prune_plan(&tool_parts, &config, false);

        // Should prune part1 and part2 (beyond 40k protection)
        // part3 (10k) is protected, then part2 (30k) brings us to 40k (protected)
        // part1 (50k) is beyond protection, so it should be pruned
        assert!(plan.should_execute);
        assert!(!plan.parts_to_prune.is_empty());
    }

    #[test]
    fn test_prune_protected_tools() {
        let tool_parts = vec![
            ToolPartInfo {
                message_id: "msg1".to_string(),
                part_id: "part1".to_string(),
                tool: "skill".to_string(), // Protected!
                status: "completed".to_string(),
                compacted: false,
                output: "a".repeat(100_000 * 4),
            },
        ];

        let config = PruneConfig::default();
        let plan = Compactor::compute_prune_plan(&tool_parts, &config, false);

        // Should not prune protected tools
        assert!(plan.parts_to_prune.is_empty());
        assert!(!plan.should_execute);
    }

    #[test]
    fn test_prune_stops_at_compacted() {
        let tool_parts = vec![
            ToolPartInfo {
                message_id: "msg1".to_string(),
                part_id: "part1".to_string(),
                tool: "read".to_string(),
                status: "completed".to_string(),
                compacted: true, // Already compacted!
                output: "old content".to_string(),
            },
            ToolPartInfo {
                message_id: "msg2".to_string(),
                part_id: "part2".to_string(),
                tool: "read".to_string(),
                status: "completed".to_string(),
                compacted: false,
                output: "b".repeat(100_000 * 4),
            },
        ];

        let config = PruneConfig::default();
        let plan = Compactor::compute_prune_plan(&tool_parts, &config, false);

        // Should stop at the compacted part and not process part1
        // part2 is newest, part1 is already compacted - stop there
        assert!(plan.parts_to_prune.is_empty() || !plan.parts_to_prune.iter().any(|p| p.part_id == "part1"));
    }
}
