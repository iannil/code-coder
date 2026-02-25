//! Channel traits for implementing channel adapters.

use crate::message::{ChannelMessage, OutgoingMessage};
use async_trait::async_trait;

/// Result type for channel operations.
pub type ChannelResult<T> = Result<T, ChannelError>;

/// Channel error type.
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Channel not ready")]
    NotReady,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Channel adapter trait.
///
/// Implement this trait to add support for a new messaging channel.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Get the channel name.
    fn name(&self) -> &'static str;

    /// Initialize the channel (connect, authenticate, etc.).
    async fn init(&mut self) -> ChannelResult<()>;

    /// Send a message to the channel.
    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String>;

    /// Start listening for incoming messages.
    ///
    /// This should spawn a background task that processes incoming messages
    /// and calls the provided callback.
    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static;

    /// Check if the channel is healthy.
    async fn health_check(&self) -> ChannelResult<()>;

    /// Shutdown the channel gracefully.
    async fn shutdown(&self) -> ChannelResult<()>;
}

/// Callback for processing incoming messages.
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message.
    async fn handle(&self, message: ChannelMessage) -> ChannelResult<Option<OutgoingMessage>>;
}

/// Simple echo handler for testing.
pub struct EchoHandler;

#[async_trait]
impl MessageHandler for EchoHandler {
    async fn handle(&self, message: ChannelMessage) -> ChannelResult<Option<OutgoingMessage>> {
        let text = message.text().map(|s| s.to_string());
        if let Some(text) = text {
            Ok(Some(OutgoingMessage {
                channel_type: message.channel_type,
                channel_id: message.channel_id,
                reply_to: Some(message.id),
                content: crate::message::OutgoingContent::Text {
                    text: format!("Echo: {text}"),
                },
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ChannelType, MessageContent};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_echo_handler() {
        let handler = EchoHandler;
        let message = ChannelMessage {
            id: "1".into(),
            channel_type: ChannelType::Cli,
            channel_id: "test".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 0,
            trace_id: "test-trace".into(),
            span_id: "test-span".into(),
            parent_span_id: None,
        };

        let response = handler.handle(message).await.unwrap();
        assert!(response.is_some());
    }
}
