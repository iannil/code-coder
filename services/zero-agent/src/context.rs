//! Tool context for interactive operations.
//!
//! Carries channel and sender information to tools for context-aware execution.

use crate::confirmation::ConfirmationHandler;
use std::sync::Arc;

/// Context passed to tools for interactive operations.
///
/// This struct carries information about the originating channel and sender,
/// allowing tools to perform context-aware operations like requesting
/// user confirmation or sending progress updates.
#[derive(Clone)]
pub struct ToolContext {
    /// Name of the channel (e.g., "telegram", "discord", "cli")
    pub channel_name: String,
    /// Unique identifier for the sender
    pub sender_id: String,
    /// Optional handler for requesting user confirmation
    pub confirmation_handler: Option<Arc<dyn ConfirmationHandler>>,
}

impl ToolContext {
    /// Create a new tool context with channel and sender info.
    pub fn new(channel_name: &str, sender_id: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            sender_id: sender_id.to_string(),
            confirmation_handler: None,
        }
    }

    /// Add a confirmation handler to this context.
    pub fn with_handler(mut self, handler: Arc<dyn ConfirmationHandler>) -> Self {
        self.confirmation_handler = Some(handler);
        self
    }
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("channel_name", &self.channel_name)
            .field("sender_id", &self.sender_id)
            .field("has_confirmation_handler", &self.confirmation_handler.is_some())
            .finish()
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
        assert!(ctx.confirmation_handler.is_none());
    }

    #[test]
    fn tool_context_debug() {
        let ctx = ToolContext::new("cli", "local");
        let debug = format!("{ctx:?}");
        assert!(debug.contains("cli"));
        assert!(debug.contains("local"));
        assert!(debug.contains("has_confirmation_handler"));
    }
}
