use async_trait::async_trait;

/// Source type of the message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageSource {
    /// Text message
    #[default]
    Text,
    /// Voice message (transcribed via STT)
    Voice,
    /// Document/file message
    Document,
}

/// A message received from or sent to a channel
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
    /// Source type of the message (text, voice, document)
    pub source: MessageSource,
}

/// Core channel trait — implement for any messaging platform
#[async_trait]
pub trait Channel: Send + Sync {
    /// Human-readable channel name
    fn name(&self) -> &str;

    /// Send a message through this channel
    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()>;

    /// Start listening for incoming messages (long-running)
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    /// Check if channel is healthy
    async fn health_check(&self) -> bool {
        true
    }
}
