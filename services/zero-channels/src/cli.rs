//! CLI channel adapter for interactive terminal sessions.
//!
//! Provides a simple stdin/stdout based channel for local testing and development.

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelResult};
use async_trait::async_trait;
use tokio::io::{self, AsyncBufReadExt, BufReader};

/// CLI channel - stdin/stdout, always available, zero deps.
pub struct CliChannel;

impl CliChannel {
    /// Create a new CLI channel.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &'static str {
        "cli"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        match message.content {
            OutgoingContent::Text { text } | OutgoingContent::Markdown { text } => {
                println!("{text}");
            }
            OutgoingContent::Voice { .. } => {
                println!("[Voice message - audio playback not supported in CLI]");
            }
            OutgoingContent::Image { caption, .. } => {
                if let Some(cap) = caption {
                    println!("[Image: {cap}]");
                } else {
                    println!("[Image]");
                }
            }
            OutgoingContent::File { filename, .. } => {
                println!("[File: {filename}]");
            }
        }
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if line == "/quit" || line == "/exit" {
                break;
            }

            let msg = ChannelMessage {
                id: uuid::Uuid::new_v4().to_string(),
                channel_type: ChannelType::Cli,
                channel_id: "cli".to_string(),
                user_id: "user".to_string(),
                content: MessageContent::Text { text: line },
                attachments: vec![],
                metadata: std::collections::HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
            };

            callback(msg);
        }
        Ok(())
    }

    async fn health_check(&self) -> ChannelResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_channel_name() {
        assert_eq!(CliChannel::new().name(), "cli");
    }

    #[tokio::test]
    async fn cli_channel_init() {
        let mut ch = CliChannel::new();
        assert!(ch.init().await.is_ok());
    }

    #[tokio::test]
    async fn cli_channel_health_check() {
        let ch = CliChannel::new();
        assert!(ch.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn cli_channel_shutdown() {
        let ch = CliChannel::new();
        assert!(ch.shutdown().await.is_ok());
    }

    #[test]
    fn cli_channel_default() {
        let ch = CliChannel::default();
        assert_eq!(ch.name(), "cli");
    }
}
