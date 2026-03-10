//! Notification sink for sending messages to channels.
//!
//! This module provides the `NotificationSink` trait that abstracts
//! notification delivery across different messaging channels (Telegram,
//! Discord, Slack, etc.).
//!
//! ## Usage
//!
//! Implementations of this trait are registered globally and used by
//! the confirmation system to send interactive confirmation requests
//! and status updates to users.

use async_trait::async_trait;

/// Trait for sending notifications to messaging channels.
///
/// This is the unified notification interface used across the Zero ecosystem.
/// Implementations handle channel-specific details like formatting, inline
/// keyboards (Telegram), reactions (Discord/Slack), etc.
///
/// # Example
///
/// ```ignore
/// use zero_common::NotificationSink;
/// use async_trait::async_trait;
///
/// struct MyNotificationSink;
///
/// #[async_trait]
/// impl NotificationSink for MyNotificationSink {
///     async fn send_notification(&self, channel: &str, user_id: &str, message: &str) {
///         println!("[{channel}] To {user_id}: {message}");
///     }
///
///     async fn send_confirmation_request(
///         &self, channel: &str, user_id: &str, request_id: &str,
///         permission: &str, message: &str,
///     ) -> anyhow::Result<()> {
///         // Send interactive message with approve/reject buttons
///         Ok(())
///     }
///
///     async fn update_confirmation_result(
///         &self, channel: &str, user_id: &str, approved: bool, message: &str,
///     ) -> anyhow::Result<()> {
///         // Update the confirmation message to show result
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait NotificationSink: Send + Sync {
    /// Send a simple text notification to a channel.
    ///
    /// # Arguments
    /// * `channel` - The channel type (e.g., "telegram", "discord", "slack")
    /// * `user_id` - The recipient's user ID within the channel
    /// * `message` - The message content to send
    async fn send_notification(&self, channel: &str, user_id: &str, message: &str);

    /// Send a confirmation request with approve/reject options.
    ///
    /// For channels that support interactive elements (like Telegram inline
    /// keyboards), this should render interactive buttons. For others, it
    /// should send a text-based prompt with instructions.
    ///
    /// # Arguments
    /// * `channel` - The channel type
    /// * `user_id` - The recipient's user ID
    /// * `request_id` - Unique identifier for this confirmation request
    /// * `permission` - The permission being requested (e.g., "shell", "edit")
    /// * `message` - Human-readable description of what's being requested
    async fn send_confirmation_request(
        &self,
        channel: &str,
        user_id: &str,
        request_id: &str,
        permission: &str,
        message: &str,
    ) -> anyhow::Result<()>;

    /// Update a confirmation message after the user responds.
    ///
    /// This is called after the user approves or rejects a confirmation
    /// request to update the message with the result.
    ///
    /// # Arguments
    /// * `channel` - The channel type
    /// * `user_id` - The user's ID
    /// * `approved` - Whether the request was approved
    /// * `message` - Status message to display (e.g., "✅ Approved")
    async fn update_confirmation_result(
        &self,
        channel: &str,
        user_id: &str,
        approved: bool,
        message: &str,
    ) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct MockNotificationSink {
        notification_count: AtomicUsize,
        confirmation_count: AtomicUsize,
        update_count: AtomicUsize,
    }

    impl MockNotificationSink {
        fn new() -> Self {
            Self {
                notification_count: AtomicUsize::new(0),
                confirmation_count: AtomicUsize::new(0),
                update_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl NotificationSink for MockNotificationSink {
        async fn send_notification(&self, _channel: &str, _user_id: &str, _message: &str) {
            self.notification_count.fetch_add(1, Ordering::SeqCst);
        }

        async fn send_confirmation_request(
            &self,
            _channel: &str,
            _user_id: &str,
            _request_id: &str,
            _permission: &str,
            _message: &str,
        ) -> anyhow::Result<()> {
            self.confirmation_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn update_confirmation_result(
            &self,
            _channel: &str,
            _user_id: &str,
            _approved: bool,
            _message: &str,
        ) -> anyhow::Result<()> {
            self.update_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_notification_sink_trait() {
        let sink = Arc::new(MockNotificationSink::new());

        sink.send_notification("telegram", "123", "Hello").await;
        assert_eq!(sink.notification_count.load(Ordering::SeqCst), 1);

        sink.send_confirmation_request("telegram", "123", "req-1", "shell", "Run ls?")
            .await
            .unwrap();
        assert_eq!(sink.confirmation_count.load(Ordering::SeqCst), 1);

        sink.update_confirmation_result("telegram", "123", true, "Approved")
            .await
            .unwrap();
        assert_eq!(sink.update_count.load(Ordering::SeqCst), 1);
    }
}
