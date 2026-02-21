//! Outbound message routing for zero-channels.
//!
//! Provides a unified interface for routing responses from CodeCoder back to
//! the originating IM channel (Telegram, Feishu, etc.).

use crate::feishu::FeishuChannel;
use crate::message::{ChannelMessage, ChannelType, OutgoingContent, OutgoingMessage};
use crate::telegram::TelegramChannel;
use crate::traits::Channel;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// OutboundRouter
// ============================================================================

/// Routes outgoing messages to the appropriate channel.
///
/// The router maintains channel instances and dispatches messages based on
/// the channel type specified in the outgoing message.
pub struct OutboundRouter {
    /// Telegram channel instance
    telegram: Option<Arc<TelegramChannel>>,
    /// Feishu channel instance
    feishu: Option<Arc<FeishuChannel>>,
    /// Pending responses keyed by request message ID
    pending: Arc<RwLock<HashMap<String, PendingResponse>>>,
}

/// Tracks a pending response with routing information.
#[derive(Debug, Clone)]
pub struct PendingResponse {
    /// Original message that triggered the request
    pub original_message: ChannelMessage,
    /// Timestamp when the request was made
    pub requested_at: i64,
}

/// Result of a send operation.
#[derive(Debug)]
pub struct SendResult {
    /// Whether the send was successful
    pub success: bool,
    /// Message ID returned by the channel (if any)
    pub message_id: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl OutboundRouter {
    /// Create a new outbound router.
    pub fn new() -> Self {
        Self {
            telegram: None,
            feishu: None,
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the Telegram channel instance.
    pub fn with_telegram(mut self, channel: Arc<TelegramChannel>) -> Self {
        self.telegram = Some(channel);
        self
    }

    /// Set the Feishu channel instance.
    pub fn with_feishu(mut self, channel: Arc<FeishuChannel>) -> Self {
        self.feishu = Some(channel);
        self
    }

    /// Register a pending response for a message.
    ///
    /// This is called when a message is received and forwarded to CodeCoder,
    /// so we know where to route the response.
    pub async fn register_pending(&self, message: ChannelMessage) {
        let entry = PendingResponse {
            original_message: message.clone(),
            requested_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        };

        let mut pending = self.pending.write().await;
        pending.insert(message.id.clone(), entry);

        tracing::debug!(
            message_id = %message.id,
            channel_type = ?message.channel_type,
            "Registered pending response"
        );
    }

    /// Remove and return a pending response.
    pub async fn take_pending(&self, message_id: &str) -> Option<PendingResponse> {
        let mut pending = self.pending.write().await;
        pending.remove(message_id)
    }

    /// Send a response to a specific channel.
    pub async fn send(&self, message: OutgoingMessage) -> SendResult {
        match message.channel_type {
            ChannelType::Telegram => self.send_telegram(message).await,
            ChannelType::Feishu => self.send_feishu(message).await,
            _ => SendResult {
                success: false,
                message_id: None,
                error: Some(format!("Unsupported channel type: {:?}", message.channel_type)),
            },
        }
    }

    /// Send a response to the original sender of a message.
    ///
    /// This is the primary method for routing CodeCoder responses.
    pub async fn respond(&self, original_message_id: &str, content: OutgoingContent) -> SendResult {
        let pending = {
            let pending = self.pending.read().await;
            pending.get(original_message_id).cloned()
        };

        let Some(pending) = pending else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some(format!("No pending response for message: {}", original_message_id)),
            };
        };

        let outgoing = OutgoingMessage {
            channel_type: pending.original_message.channel_type,
            channel_id: pending.original_message.channel_id.clone(),
            reply_to: Some(original_message_id.to_string()),
            content,
        };

        let result = self.send(outgoing).await;

        // Remove the pending entry on success
        if result.success {
            let mut pending_map = self.pending.write().await;
            pending_map.remove(original_message_id);
        }

        result
    }

    /// Send a message directly to a channel (without pending lookup).
    ///
    /// Use this for proactive messages, not responses.
    pub async fn send_direct(
        &self,
        channel_type: ChannelType,
        channel_id: String,
        content: OutgoingContent,
    ) -> SendResult {
        let outgoing = OutgoingMessage {
            channel_type,
            channel_id,
            reply_to: None,
            content,
        };

        self.send(outgoing).await
    }

    async fn send_telegram(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref telegram) = self.telegram else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("Telegram channel not configured".to_string()),
            };
        };

        match telegram.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send Telegram message");
                SendResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    async fn send_feishu(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref feishu) = self.feishu else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("Feishu channel not configured".to_string()),
            };
        };

        match feishu.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send Feishu message");
                SendResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Clean up stale pending responses older than the given TTL.
    pub async fn cleanup_stale(&self, ttl_ms: i64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let mut pending = self.pending.write().await;
        let before_count = pending.len();

        pending.retain(|_, entry| now - entry.requested_at < ttl_ms);

        let removed = before_count - pending.len();
        if removed > 0 {
            tracing::info!(removed = removed, "Cleaned up stale pending responses");
        }
    }

    /// Get the number of pending responses.
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }
}

impl Default for OutboundRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;

    fn create_test_message() -> ChannelMessage {
        ChannelMessage {
            id: "test-123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456789".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: std::collections::HashMap::new(),
            timestamp: 1234567890000,
        }
    }

    #[tokio::test]
    async fn test_register_pending() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg.clone()).await;

        assert_eq!(router.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_take_pending() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg.clone()).await;
        let taken = router.take_pending("test-123").await;

        assert!(taken.is_some());
        assert_eq!(router.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_cleanup_stale() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg).await;

        // With a very short TTL, the message should be cleaned up
        router.cleanup_stale(0).await;

        assert_eq!(router.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_send_without_channel() {
        let router = OutboundRouter::new();

        let result = router
            .send_direct(
                ChannelType::Telegram,
                "123".into(),
                OutgoingContent::Text {
                    text: "Test".into(),
                },
            )
            .await;

        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
