//! Confirmation handler for interactive user approval
//!
//! When tools need user permission (e.g., file writes, shell commands),
//! this module handles sending confirmation requests to the appropriate
//! channel and waiting for user response.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};

/// Global notification sink for sending messages to channels
static NOTIFICATION_SINK: RwLock<Option<Arc<dyn NotificationSink>>> = RwLock::const_new(None);

/// Trait for sending notifications to channels
#[async_trait]
pub trait NotificationSink: Send + Sync {
    async fn send_notification(&self, channel: &str, user_id: &str, message: &str);
}

/// Register a notification sink
pub async fn set_notification_sink(sink: Arc<dyn NotificationSink>) {
    let mut guard = NOTIFICATION_SINK.write().await;
    *guard = Some(sink);
}

/// Send a notification (if a sink is registered)
pub async fn notify(channel: &str, user_id: &str, message: &str) {
    if let Some(sink) = NOTIFICATION_SINK.read().await.as_ref() {
        sink.send_notification(channel, user_id, message).await;
    }
}

/// Pending confirmation request
pub struct PendingConfirmation {
    pub request_id: String,
    pub permission: String,
    pub message: String,
    pub responder: oneshot::Sender<bool>,
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
    ) -> oneshot::Receiver<bool> {
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
    pub async fn respond(&self, request_id: &str, approved: bool) -> bool {
        if let Some(pending) = self.pending.lock().await.remove(request_id) {
            let _ = pending.responder.send(approved);
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
