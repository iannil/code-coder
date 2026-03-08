//! Outbound message routing for zero-channels.
//!
//! Provides a unified interface for routing responses from CodeCoder back to
//! the originating IM channel (Telegram, Feishu, WeChat Work, DingTalk, WhatsApp, Email, etc.).

use super::dingtalk::DingTalkChannel;
use super::email::EmailChannel;
use super::feishu::FeishuChannel;
use super::message::{ChannelMessage, ChannelType, OutgoingContent, OutgoingMessage};
use super::telegram::TelegramChannel;
use super::traits::Channel;
use super::wecom::WeComChannel;
use super::whatsapp::WhatsAppChannel;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use zero_common::logging::{generate_span_id, LifecycleEventType, RequestContext};

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
    /// WeChat Work channel instance
    wecom: Option<Arc<WeComChannel>>,
    /// DingTalk channel instance
    dingtalk: Option<Arc<DingTalkChannel>>,
    /// WhatsApp channel instance
    whatsapp: Option<Arc<WhatsAppChannel>>,
    /// Email channel instance
    email: Option<Arc<EmailChannel>>,
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
    /// Tracing context for this request
    pub trace_context: Option<(String, String)>, // (trace_id, span_id)
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
            wecom: None,
            dingtalk: None,
            whatsapp: None,
            email: None,
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

    /// Set the WeChat Work channel instance.
    pub fn with_wecom(mut self, channel: Arc<WeComChannel>) -> Self {
        self.wecom = Some(channel);
        self
    }

    /// Set the DingTalk channel instance.
    pub fn with_dingtalk(mut self, channel: Arc<DingTalkChannel>) -> Self {
        self.dingtalk = Some(channel);
        self
    }

    /// Set the WhatsApp channel instance.
    pub fn with_whatsapp(mut self, channel: Arc<WhatsAppChannel>) -> Self {
        self.whatsapp = Some(channel);
        self
    }

    /// Set the Email channel instance.
    pub fn with_email(mut self, channel: Arc<EmailChannel>) -> Self {
        self.email = Some(channel);
        self
    }

    /// Register a pending response for a message.
    ///
    /// This is called when a message is received and forwarded to CodeCoder,
    /// so we know where to route the response.
    pub async fn register_pending(&self, message: ChannelMessage) {
        // Extract tracing context from the message
        let trace_context = if message.has_tracing() {
            Some((message.trace_id.clone(), message.span_id.clone()))
        } else {
            None
        };

        let entry = PendingResponse {
            original_message: message.clone(),
            requested_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            trace_context,
        };

        let mut pending = self.pending.write().await;
        pending.insert(message.id.clone(), entry);

        tracing::debug!(
            message_id = %message.id,
            channel_type = ?message.channel_type,
            trace_id = %message.trace_id,
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
            ChannelType::WeCom => self.send_wecom(message).await,
            ChannelType::DingTalk => self.send_dingtalk(message).await,
            ChannelType::WhatsApp => self.send_whatsapp(message).await,
            ChannelType::Email => self.send_email(message).await,
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
        let start = Instant::now();

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

        // Create tracing context if available
        let ctx = pending.trace_context.as_ref().map(|(trace_id, parent_span_id)| {
            RequestContext {
                trace_id: trace_id.clone(),
                span_id: generate_span_id(),
                parent_span_id: Some(parent_span_id.clone()),
                service: "zero-channels".to_string(),
                user_id: Some(pending.original_message.user_id.clone()),
                baggage: HashMap::new(),
            }
        });

        if let Some(ref ctx) = ctx {
            ctx.log_event(
                LifecycleEventType::FunctionStart,
                serde_json::json!({
                    "function": "OutboundRouter::respond",
                    "channel": pending.original_message.channel_type.as_str(),
                    "message_id": original_message_id,
                }),
            );
        }

        let outgoing = OutgoingMessage {
            channel_type: pending.original_message.channel_type,
            channel_id: pending.original_message.channel_id.clone(),
            reply_to: Some(original_message_id.to_string()),
            content,
        };

        let result = self.send(outgoing).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        if let Some(ref ctx) = ctx {
            ctx.log_event(
                LifecycleEventType::FunctionEnd,
                serde_json::json!({
                    "function": "OutboundRouter::respond",
                    "duration_ms": duration_ms,
                    "success": result.success,
                    "error": result.error,
                }),
            );
        }

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

    async fn send_wecom(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref wecom) = self.wecom else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("WeChat Work channel not configured".to_string()),
            };
        };

        match wecom.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send WeChat Work message");
                SendResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    async fn send_dingtalk(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref dingtalk) = self.dingtalk else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("DingTalk channel not configured".to_string()),
            };
        };

        match dingtalk.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send DingTalk message");
                SendResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    async fn send_whatsapp(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref whatsapp) = self.whatsapp else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("WhatsApp channel not configured".to_string()),
            };
        };

        match whatsapp.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send WhatsApp message");
                SendResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    async fn send_email(&self, message: OutgoingMessage) -> SendResult {
        let Some(ref email) = self.email else {
            return SendResult {
                success: false,
                message_id: None,
                error: Some("Email channel not configured".to_string()),
            };
        };

        match email.send(message).await {
            Ok(msg_id) => SendResult {
                success: true,
                message_id: Some(msg_id),
                error: None,
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to send Email message");
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
    use crate::channels::message::MessageContent;

    // ────────────────────────────────────────────────────────────────────────────
    // Helper Functions
    // ────────────────────────────────────────────────────────────────────────────

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
            trace_id: "trace-abc-123".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        }
    }

    fn create_test_message_with_id(id: &str) -> ChannelMessage {
        ChannelMessage {
            id: id.into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456789".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: std::collections::HashMap::new(),
            timestamp: 1234567890000,
            trace_id: format!("trace-{}", id),
            span_id: format!("span-{}", id),
            parent_span_id: None,
        }
    }

    fn create_test_message_no_tracing() -> ChannelMessage {
        ChannelMessage {
            id: "test-no-trace".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456789".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: std::collections::HashMap::new(),
            timestamp: 1234567890000,
            trace_id: String::new(),
            span_id: String::new(),
            parent_span_id: None,
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // OutboundRouter Creation Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_new_router() {
        let router = OutboundRouter::new();
        assert_eq!(router.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_default_router() {
        let router = OutboundRouter::default();
        assert_eq!(router.pending_count().await, 0);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Pending Registration Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_register_pending() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg.clone()).await;

        assert_eq!(router.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_register_pending_with_tracing() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg.clone()).await;

        let pending = router.take_pending("test-123").await.unwrap();
        assert!(pending.trace_context.is_some());

        let (trace_id, span_id) = pending.trace_context.unwrap();
        assert_eq!(trace_id, "trace-abc-123");
        assert_eq!(span_id, "span-xyz");
    }

    #[tokio::test]
    async fn test_register_pending_without_tracing() {
        let router = OutboundRouter::new();
        let msg = create_test_message_no_tracing();

        router.register_pending(msg.clone()).await;

        let pending = router.take_pending("test-no-trace").await.unwrap();
        assert!(pending.trace_context.is_none());
    }

    #[tokio::test]
    async fn test_register_multiple_pending() {
        let router = OutboundRouter::new();

        router.register_pending(create_test_message_with_id("msg-1")).await;
        router.register_pending(create_test_message_with_id("msg-2")).await;
        router.register_pending(create_test_message_with_id("msg-3")).await;

        assert_eq!(router.pending_count().await, 3);
    }

    #[tokio::test]
    async fn test_register_pending_overwrites_same_id() {
        let router = OutboundRouter::new();

        let mut msg1 = create_test_message();
        msg1.user_id = "user-first".into();
        router.register_pending(msg1).await;

        let mut msg2 = create_test_message();
        msg2.user_id = "user-second".into();
        router.register_pending(msg2).await;

        // Should still have only 1 pending (overwritten)
        assert_eq!(router.pending_count().await, 1);

        let pending = router.take_pending("test-123").await.unwrap();
        assert_eq!(pending.original_message.user_id, "user-second");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Take Pending Tests
    // ────────────────────────────────────────────────────────────────────────────

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
    async fn test_take_pending_nonexistent() {
        let router = OutboundRouter::new();

        let taken = router.take_pending("nonexistent").await;
        assert!(taken.is_none());
    }

    #[tokio::test]
    async fn test_take_pending_twice() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg).await;

        let first = router.take_pending("test-123").await;
        let second = router.take_pending("test-123").await;

        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_take_specific_pending() {
        let router = OutboundRouter::new();

        router.register_pending(create_test_message_with_id("msg-1")).await;
        router.register_pending(create_test_message_with_id("msg-2")).await;
        router.register_pending(create_test_message_with_id("msg-3")).await;

        let taken = router.take_pending("msg-2").await;
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().original_message.id, "msg-2");

        // Other messages should still be pending
        assert_eq!(router.pending_count().await, 2);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Cleanup Stale Tests
    // ────────────────────────────────────────────────────────────────────────────

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
    async fn test_cleanup_stale_preserves_fresh() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg).await;

        // With a long TTL, the message should not be cleaned up
        router.cleanup_stale(60_000_000).await; // 1000 minutes

        assert_eq!(router.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_cleanup_stale_empty_router() {
        let router = OutboundRouter::new();

        // Should not panic on empty router
        router.cleanup_stale(0).await;
        assert_eq!(router.pending_count().await, 0);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Send Tests
    // ────────────────────────────────────────────────────────────────────────────

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
        assert!(result.error.unwrap().contains("not configured"));
    }

    #[tokio::test]
    async fn test_send_unsupported_channel() {
        let router = OutboundRouter::new();

        // CLI and other channels are not supported for outbound
        let msg = OutgoingMessage {
            channel_type: ChannelType::Cli,
            channel_id: "123".into(),
            reply_to: None,
            content: OutgoingContent::Text {
                text: "Test".into(),
            },
        };

        let result = router.send(msg).await;

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Unsupported channel type"));
    }

    #[tokio::test]
    async fn test_send_all_channel_types_unconfigured() {
        let router = OutboundRouter::new();

        // Test each channel type returns proper error when not configured
        let channel_types = [
            (ChannelType::Telegram, "Telegram"),
            (ChannelType::Feishu, "Feishu"),
            (ChannelType::WeCom, "WeChat Work"),
            (ChannelType::DingTalk, "DingTalk"),
            (ChannelType::WhatsApp, "WhatsApp"),
            (ChannelType::Email, "Email"),
        ];

        for (channel_type, name) in channel_types {
            let result = router
                .send_direct(
                    channel_type,
                    "123".into(),
                    OutgoingContent::Text { text: "Test".into() },
                )
                .await;

            assert!(!result.success, "Expected failure for {}", name);
            assert!(
                result.error.as_ref().unwrap().contains("not configured"),
                "Expected 'not configured' error for {}, got: {:?}",
                name,
                result.error
            );
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Respond Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_respond_without_pending() {
        let router = OutboundRouter::new();

        let result = router
            .respond(
                "nonexistent-msg",
                OutgoingContent::Text {
                    text: "Response".into(),
                },
            )
            .await;

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("No pending response"));
    }

    #[tokio::test]
    async fn test_respond_with_pending_no_channel() {
        let router = OutboundRouter::new();
        let msg = create_test_message();

        router.register_pending(msg).await;

        let result = router
            .respond(
                "test-123",
                OutgoingContent::Text {
                    text: "Response".into(),
                },
            )
            .await;

        // Should fail because no Telegram channel is configured
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not configured"));

        // Pending should NOT be removed on failure
        // (respond only removes on success)
        assert_eq!(router.pending_count().await, 1);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // PendingResponse Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_pending_response_timestamp() {
        let router = OutboundRouter::new();

        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        router.register_pending(create_test_message()).await;

        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let pending = router.take_pending("test-123").await.unwrap();

        assert!(pending.requested_at >= before);
        assert!(pending.requested_at <= after);
    }

    #[tokio::test]
    async fn test_pending_response_preserves_original() {
        let router = OutboundRouter::new();
        let mut msg = create_test_message();
        msg.metadata.insert("custom_key".into(), "custom_value".into());

        router.register_pending(msg).await;

        let pending = router.take_pending("test-123").await.unwrap();

        assert_eq!(pending.original_message.id, "test-123");
        assert_eq!(pending.original_message.channel_type, ChannelType::Telegram);
        assert_eq!(pending.original_message.channel_id, "456789");
        assert_eq!(
            pending.original_message.metadata.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Concurrent Access Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_concurrent_register() {
        let router = Arc::new(OutboundRouter::new());
        let mut handles = vec![];

        for i in 0..10 {
            let router = router.clone();
            handles.push(tokio::spawn(async move {
                router.register_pending(create_test_message_with_id(&format!("msg-{}", i))).await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(router.pending_count().await, 10);
    }

    #[tokio::test]
    async fn test_concurrent_take() {
        let router = Arc::new(OutboundRouter::new());

        // Register a single message
        router.register_pending(create_test_message()).await;

        // Try to take it from multiple concurrent tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let router = router.clone();
            handles.push(tokio::spawn(async move {
                router.take_pending("test-123").await
            }));
        }

        let results: Vec<_> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Exactly one should succeed
        let successes = results.iter().filter(|r| r.is_some()).count();
        assert_eq!(successes, 1);
    }
}
