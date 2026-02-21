//! Channel traits for zero-cli.
//!
//! This module provides local trait definitions that are dyn-compatible
//! for use within zero-cli's routing and orchestration logic.
//!
//! The canonical channel implementations are in `zero-channels`, but they
//! use a generic listen method that isn't dyn-compatible. This module
//! provides adapters to bridge the gap.

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Message source for determining response format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageSource {
    #[default]
    Text,
    Voice,
}

/// Message content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Voice { audio_bytes: Vec<u8>, format: String },
}

impl MessageContent {
    pub fn text(&self) -> String {
        match self {
            Self::Text(t) => t.clone(),
            Self::Voice { .. } => "[Voice message]".to_string(),
        }
    }

    /// Check if this is a text message
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text())
    }
}

/// A message from a channel
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub channel_type: String,
    pub sender_id: String,
    pub content: String, // Simplified to String for local use
    pub message_id: Option<String>,
    pub reply_to: Option<String>,
    pub source: MessageSource,
}

impl ChannelMessage {
    pub fn new(channel_type: &str, sender_id: &str, content: impl Into<String>) -> Self {
        Self {
            channel_type: channel_type.to_string(),
            sender_id: sender_id.to_string(),
            content: content.into(),
            message_id: None,
            reply_to: None,
            source: MessageSource::Text,
        }
    }
}

/// Convert from zero-channels ChannelMessage to local ChannelMessage
impl From<zero_channels::ChannelMessage> for ChannelMessage {
    fn from(msg: zero_channels::ChannelMessage) -> Self {
        // Extract values before moving fields
        let channel_type = msg.channel_type.as_str().to_string();
        let content = msg.text().unwrap_or_default().to_string();
        let source = if msg.is_voice() {
            MessageSource::Voice
        } else {
            MessageSource::Text
        };

        Self {
            channel_type,
            sender_id: msg.user_id,
            content,
            message_id: Some(msg.id),
            reply_to: None,
            source,
        }
    }
}

/// Channel trait for messaging integrations (zero-cli local version).
///
/// This trait is dyn-compatible, allowing channels to be stored as `dyn Channel`.
/// The canonical implementations in `zero-channels` are not dyn-compatible
/// due to their generic `listen` method.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Returns the channel name
    fn name(&self) -> &str;

    /// Send a message to a recipient
    async fn send(&self, message: &str, recipient: &str) -> Result<()>;

    /// Start listening for messages
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;

    /// Perform a health check
    async fn health_check(&self) -> bool {
        true
    }
}

// ============================================================================
// Adapters: Wrap zero-channels implementations to implement local Channel trait
// ============================================================================

/// Wrapper for zero_channels::CliChannel that implements the local Channel trait.
pub struct CliChannelAdapter {
    inner: zero_channels::CliChannel,
}

impl CliChannelAdapter {
    pub fn new() -> Self {
        Self {
            inner: zero_channels::CliChannel::new(),
        }
    }
}

impl Default for CliChannelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Channel for CliChannelAdapter {
    fn name(&self) -> &str {
        "cli"
    }

    async fn send(&self, message: &str, _recipient: &str) -> Result<()> {
        // For CLI, just print to stdout
        println!("{message}");
        Ok(())
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        use zero_channels::Channel as ZeroChannel;

        // Create a callback that converts and sends messages
        let callback = move |msg: zero_channels::ChannelMessage| {
            let local_msg: ChannelMessage = msg.into();
            // Use blocking_send since we're in a sync callback context
            let _ = tx.blocking_send(local_msg);
        };

        self.inner
            .listen(callback)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
