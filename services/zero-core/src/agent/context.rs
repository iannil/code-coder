//! Tool context for interactive operations.
//!
//! Carries channel and sender information to tools for context-aware execution.

/// Context passed to tools for interactive operations.
///
/// This struct carries information about the originating channel and sender,
/// allowing tools to perform context-aware operations like requesting
/// user confirmation or sending progress updates.
///
/// Note: For confirmation handling, use the global `NotificationSink`
/// registered via `set_notification_sink()` in the `confirmation` module.
#[derive(Clone, Debug, Default)]
pub struct ToolContext {
    /// Name of the channel (e.g., "telegram", "discord", "cli")
    pub channel_name: String,
    /// Unique identifier for the sender
    pub sender_id: String,
}

impl ToolContext {
    /// Create a new tool context with channel and sender info.
    pub fn new(channel_name: &str, sender_id: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            sender_id: sender_id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_context_creation() {
        let ctx = ToolContext::new("telegram", "user123");
        assert_eq!(ctx.channel_name, "telegram");
        assert_eq!(ctx.sender_id, "user123");
    }

    #[test]
    fn tool_context_debug() {
        let ctx = ToolContext::new("cli", "local");
        let debug = format!("{ctx:?}");
        assert!(debug.contains("cli"));
        assert!(debug.contains("local"));
    }

    #[test]
    fn tool_context_default() {
        let ctx = ToolContext::default();
        assert!(ctx.channel_name.is_empty());
        assert!(ctx.sender_id.is_empty());
    }
}
