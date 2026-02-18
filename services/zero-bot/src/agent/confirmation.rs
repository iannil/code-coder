//! Confirmation handler for interactive user approval
//!
//! When tools need user permission (e.g., file writes, shell commands),
//! this module handles sending confirmation requests to the appropriate
//! channel and waiting for user response.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex, RwLock};

/// Confirmation response type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResponse {
    /// Approve this single request
    Once,
    /// Approve and remember for future requests of this type
    Always,
    /// Reject the request
    Reject,
}

impl ConfirmationResponse {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Once | Self::Always)
    }

    pub fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }
}

/// Global notification sink for sending messages to channels
static NOTIFICATION_SINK: RwLock<Option<Arc<dyn NotificationSink>>> = RwLock::const_new(None);

/// Global confirmation registry for managing pending confirmations
static CONFIRMATION_REGISTRY: RwLock<Option<Arc<ConfirmationRegistry>>> = RwLock::const_new(None);

/// Default timeout for confirmation requests (2 minutes)
const DEFAULT_CONFIRMATION_TIMEOUT_SECS: u64 = 120;

/// Trait for sending notifications to channels
#[async_trait]
pub trait NotificationSink: Send + Sync {
    /// Send a simple text notification
    async fn send_notification(&self, channel: &str, user_id: &str, message: &str);

    /// Send a confirmation request with approve/reject buttons
    /// Returns the `message_id` for later reference
    async fn send_confirmation_request(
        &self,
        channel: &str,
        user_id: &str,
        request_id: &str,
        permission: &str,
        message: &str,
    ) -> anyhow::Result<()>;

    /// Update a confirmation message after user responds
    async fn update_confirmation_result(
        &self,
        channel: &str,
        user_id: &str,
        approved: bool,
        message: &str,
    ) -> anyhow::Result<()>;
}

/// Register a notification sink
pub async fn set_notification_sink(sink: Arc<dyn NotificationSink>) {
    let mut guard = NOTIFICATION_SINK.write().await;
    *guard = Some(sink);
}

/// Get the current notification sink
pub async fn get_notification_sink() -> Option<Arc<dyn NotificationSink>> {
    NOTIFICATION_SINK.read().await.clone()
}

/// Send a notification (if a sink is registered)
pub async fn notify(channel: &str, user_id: &str, message: &str) {
    if let Some(sink) = NOTIFICATION_SINK.read().await.as_ref() {
        sink.send_notification(channel, user_id, message).await;
    }
}

/// Initialize the global confirmation registry
pub async fn init_confirmation_registry() {
    let mut guard = CONFIRMATION_REGISTRY.write().await;
    if guard.is_none() {
        *guard = Some(Arc::new(ConfirmationRegistry::new()));
    }
}

/// Get the global confirmation registry
pub async fn get_confirmation_registry() -> Option<Arc<ConfirmationRegistry>> {
    CONFIRMATION_REGISTRY.read().await.clone()
}

/// Request confirmation from user and wait for response
///
/// This function:
/// 1. Registers the confirmation request in the global registry
/// 2. Sends an interactive message to the user via their channel
/// 3. Waits for the user to click approve/reject (with timeout)
/// 4. Returns the result
pub async fn request_confirmation_and_wait(
    channel: &str,
    user_id: &str,
    request_id: &str,
    permission: &str,
    message: &str,
    timeout_secs: Option<u64>,
) -> anyhow::Result<ConfirmationResponse> {
    let registry = get_confirmation_registry()
        .await
        .ok_or_else(|| anyhow::anyhow!("Confirmation registry not initialized"))?;

    let sink = get_notification_sink()
        .await
        .ok_or_else(|| anyhow::anyhow!("Notification sink not initialized"))?;

    // Register the pending confirmation
    let rx = registry
        .register(request_id.to_string(), permission.to_string(), message.to_string())
        .await;

    // Send the interactive confirmation request
    sink.send_confirmation_request(channel, user_id, request_id, permission, message)
        .await?;

    // Wait for user response with timeout
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_CONFIRMATION_TIMEOUT_SECS));

    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(response)) => {
            let result_msg = match response {
                ConfirmationResponse::Once => "✅ 已批准",
                ConfirmationResponse::Always => "✅ 已始终批准",
                ConfirmationResponse::Reject => "❌ 已拒绝",
            };
            // Update the message to show result
            let _ = sink.update_confirmation_result(channel, user_id, response.is_approved(), result_msg).await;
            Ok(response)
        }
        Ok(Err(_)) => {
            // Sender dropped (shouldn't happen)
            Err(anyhow::anyhow!("Confirmation channel closed unexpectedly"))
        }
        Err(_) => {
            // Timeout - clean up the pending confirmation
            registry.cleanup(request_id).await;
            let _ = sink.update_confirmation_result(channel, user_id, false, "⏱️ 已超时").await;
            Err(anyhow::anyhow!("Confirmation request timed out after {} seconds", timeout.as_secs()))
        }
    }
}

/// Respond to a confirmation from user callback (e.g., Telegram button click)
///
/// For backwards compatibility, use `handle_confirmation_response_with_type` for
/// full control over the response type.
pub async fn handle_confirmation_response(request_id: &str, approved: bool) -> bool {
    let response = if approved {
        ConfirmationResponse::Once
    } else {
        ConfirmationResponse::Reject
    };
    handle_confirmation_response_with_type(request_id, response).await
}

/// Respond to a confirmation with a specific response type
pub async fn handle_confirmation_response_with_type(request_id: &str, response: ConfirmationResponse) -> bool {
    if let Some(registry) = get_confirmation_registry().await {
        registry.respond(request_id, response).await
    } else {
        false
    }
}

/// Pending confirmation request
pub struct PendingConfirmation {
    pub request_id: String,
    pub permission: String,
    pub message: String,
    pub responder: oneshot::Sender<ConfirmationResponse>,
}

/// Confirmation handler trait - implement for each channel type
#[async_trait]
pub trait ConfirmationHandler: Send + Sync {
    /// Request confirmation from user
    /// Returns true if approved, false if rejected
    async fn request_confirmation(
        &self,
        sender_id: &str,
        request_id: &str,
        permission: &str,
        message: &str,
    ) -> anyhow::Result<bool>;

    /// Send a message to the user (for status updates)
    async fn send_message(&self, sender_id: &str, message: &str) -> anyhow::Result<()>;
}

/// Registry for pending confirmations
/// Used to match user responses to pending requests
#[derive(Default)]
pub struct ConfirmationRegistry {
    pending: Mutex<HashMap<String, PendingConfirmation>>,
}

impl ConfirmationRegistry {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Register a pending confirmation and get a receiver for the response
    pub async fn register(
        &self,
        request_id: String,
        permission: String,
        message: String,
    ) -> oneshot::Receiver<ConfirmationResponse> {
        let (tx, rx) = oneshot::channel();
        let pending = PendingConfirmation {
            request_id: request_id.clone(),
            permission,
            message,
            responder: tx,
        };
        self.pending.lock().await.insert(request_id, pending);
        rx
    }

    /// Respond to a pending confirmation
    pub async fn respond(&self, request_id: &str, response: ConfirmationResponse) -> bool {
        if let Some(pending) = self.pending.lock().await.remove(request_id) {
            let _ = pending.responder.send(response);
            true
        } else {
            false
        }
    }

    /// Get info about a pending confirmation
    pub async fn get_pending(&self, request_id: &str) -> Option<(String, String)> {
        self.pending
            .lock()
            .await
            .get(request_id)
            .map(|p| (p.permission.clone(), p.message.clone()))
    }

    /// List all pending confirmation IDs for a cleanup
    pub async fn list_pending(&self) -> Vec<String> {
        self.pending.lock().await.keys().cloned().collect()
    }

    /// Clean up a pending confirmation (e.g., on timeout)
    pub async fn cleanup(&self, request_id: &str) {
        self.pending.lock().await.remove(request_id);
    }

    /// Check if a request is pending
    pub async fn is_pending(&self, request_id: &str) -> bool {
        self.pending.lock().await.contains_key(request_id)
    }

    /// Get the count of pending confirmations
    pub async fn pending_count(&self) -> usize {
        self.pending.lock().await.len()
    }
}

/// Context passed to tools for interactive operations
#[derive(Clone)]
pub struct ToolContext {
    pub channel_name: String,
    pub sender_id: String,
    pub confirmation_handler: Option<Arc<dyn ConfirmationHandler>>,
}

impl ToolContext {
    pub fn new(channel_name: &str, sender_id: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
            sender_id: sender_id.to_string(),
            confirmation_handler: None,
        }
    }

    pub fn with_handler(mut self, handler: Arc<dyn ConfirmationHandler>) -> Self {
        self.confirmation_handler = Some(handler);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn confirmation_registry_register_and_respond() {
        let registry = ConfirmationRegistry::new();

        let rx = registry
            .register("req-1".to_string(), "shell".to_string(), "Run command?".to_string())
            .await;

        assert!(registry.is_pending("req-1").await);
        assert_eq!(registry.pending_count().await, 1);

        // Respond with approval
        let responded = registry.respond("req-1", ConfirmationResponse::Once).await;
        assert!(responded);

        // Check the response was received
        let result = rx.await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_approved());

        // Should no longer be pending
        assert!(!registry.is_pending("req-1").await);
        assert_eq!(registry.pending_count().await, 0);
    }

    #[tokio::test]
    async fn confirmation_registry_respond_reject() {
        let registry = ConfirmationRegistry::new();

        let rx = registry
            .register("req-2".to_string(), "edit".to_string(), "Edit file?".to_string())
            .await;

        // Respond with rejection
        registry.respond("req-2", ConfirmationResponse::Reject).await;

        let result = rx.await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_approved());
    }

    #[tokio::test]
    async fn confirmation_registry_respond_unknown() {
        let registry = ConfirmationRegistry::new();

        // Try to respond to unknown request
        let responded = registry.respond("unknown-req", ConfirmationResponse::Once).await;
        assert!(!responded);
    }

    #[tokio::test]
    async fn confirmation_registry_get_pending() {
        let registry = ConfirmationRegistry::new();

        registry
            .register("req-3".to_string(), "browser".to_string(), "Open browser?".to_string())
            .await;

        let pending = registry.get_pending("req-3").await;
        assert!(pending.is_some());
        let (permission, message) = pending.unwrap();
        assert_eq!(permission, "browser");
        assert_eq!(message, "Open browser?");

        // Unknown request
        assert!(registry.get_pending("unknown").await.is_none());
    }

    #[tokio::test]
    async fn confirmation_registry_list_pending() {
        let registry = ConfirmationRegistry::new();

        registry
            .register("req-a".to_string(), "p1".to_string(), "m1".to_string())
            .await;
        registry
            .register("req-b".to_string(), "p2".to_string(), "m2".to_string())
            .await;

        let list = registry.list_pending().await;
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"req-a".to_string()));
        assert!(list.contains(&"req-b".to_string()));
    }

    #[tokio::test]
    async fn confirmation_registry_cleanup() {
        let registry = ConfirmationRegistry::new();

        registry
            .register("req-clean".to_string(), "perm".to_string(), "msg".to_string())
            .await;

        assert!(registry.is_pending("req-clean").await);

        registry.cleanup("req-clean").await;

        assert!(!registry.is_pending("req-clean").await);
        assert_eq!(registry.pending_count().await, 0);
    }

    #[tokio::test]
    async fn confirmation_registry_multiple_concurrent() {
        let registry = Arc::new(ConfirmationRegistry::new());

        // Register multiple confirmations
        let rx1 = registry
            .register("concurrent-1".to_string(), "p1".to_string(), "m1".to_string())
            .await;
        let rx2 = registry
            .register("concurrent-2".to_string(), "p2".to_string(), "m2".to_string())
            .await;

        assert_eq!(registry.pending_count().await, 2);

        // Respond to them concurrently
        let reg1 = registry.clone();
        let reg2 = registry.clone();

        let (r1, r2) = tokio::join!(
            async move { reg1.respond("concurrent-1", ConfirmationResponse::Once).await },
            async move { reg2.respond("concurrent-2", ConfirmationResponse::Reject).await }
        );

        assert!(r1);
        assert!(r2);

        // Check results
        assert!(rx1.await.unwrap().is_approved());
        assert!(!rx2.await.unwrap().is_approved());
    }

    #[test]
    fn tool_context_creation() {
        let ctx = ToolContext::new("telegram", "user123");
        assert_eq!(ctx.channel_name, "telegram");
        assert_eq!(ctx.sender_id, "user123");
        assert!(ctx.confirmation_handler.is_none());
    }
}
