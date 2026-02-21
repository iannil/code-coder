//! CodeCoder bridge for zero-channels.
//!
//! Handles the complete message flow:
//! 1. Receive ChannelMessage from webhook/polling
//! 2. Forward to CodeCoder API
//! 3. Route response back to original channel

use crate::message::{ChannelMessage, MessageContent, OutgoingContent};
use crate::outbound::OutboundRouter;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

// ============================================================================
// CodeCoder API Types
// ============================================================================

/// Request to CodeCoder chat API.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// User message content
    pub message: String,
    /// Optional conversation ID for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Optional agent to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// User identifier
    pub user_id: String,
    /// Channel type for context
    pub channel: String,
}

/// Response from CodeCoder chat API.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Response message content
    pub message: String,
    /// Conversation ID for follow-ups
    pub conversation_id: Option<String>,
    /// Agent used
    pub agent: Option<String>,
    /// Token usage information
    pub usage: Option<TokenUsage>,
}

/// Token usage from LLM.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

/// Error response from CodeCoder.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}

// ============================================================================
// Bridge
// ============================================================================

/// Bridge between IM channels and CodeCoder.
pub struct CodeCoderBridge {
    /// HTTP client for API calls
    client: reqwest::Client,
    /// CodeCoder API endpoint
    endpoint: String,
    /// Outbound router for sending responses
    router: Arc<OutboundRouter>,
    /// Request timeout
    timeout: Duration,
}

impl CodeCoderBridge {
    /// Create a new bridge.
    pub fn new(endpoint: impl Into<String>, router: Arc<OutboundRouter>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // LLM calls can be slow
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            endpoint: endpoint.into(),
            router,
            timeout: Duration::from_secs(300),
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Process an incoming message and send response.
    ///
    /// This is the main entry point for the bridge.
    pub async fn process(&self, message: ChannelMessage) -> Result<()> {
        // Register the message for response routing
        self.router.register_pending(message.clone()).await;

        // Extract text content
        let text = match &message.content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Voice { .. } => {
                // Voice should have been transcribed before reaching here
                return Err(anyhow::anyhow!("Voice messages should be transcribed first"));
            }
            MessageContent::Image { caption, .. } => {
                caption.clone().unwrap_or_else(|| "[Image received]".to_string())
            }
            MessageContent::File { filename, .. } => {
                format!("[File received: {}]", filename)
            }
            MessageContent::Location { latitude, longitude, title } => {
                format!(
                    "[Location: {} at {}, {}]",
                    title.as_deref().unwrap_or("Unknown"),
                    latitude,
                    longitude
                )
            }
        };

        // Build the request
        let request = ChatRequest {
            message: text,
            conversation_id: message.metadata.get("conversation_id").cloned(),
            agent: message.metadata.get("agent").cloned(),
            user_id: message.user_id.clone(),
            channel: message.channel_type.as_str().to_string(),
        };

        // Send to CodeCoder
        let response = self.call_codecoder(&request).await;

        // Route the response
        match response {
            Ok(resp) => {
                let content = OutgoingContent::Markdown { text: resp.message };
                let result = self.router.respond(&message.id, content).await;

                if !result.success {
                    tracing::error!(
                        message_id = %message.id,
                        error = ?result.error,
                        "Failed to send response"
                    );
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "CodeCoder API call failed");

                // Send error message to user
                let error_content = OutgoingContent::Text {
                    text: format!("Sorry, I encountered an error: {}", e),
                };
                let _ = self.router.respond(&message.id, error_content).await;
            }
        }

        Ok(())
    }

    /// Call the CodeCoder API.
    async fn call_codecoder(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/v1/chat", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            user_id = %request.user_id,
            "Calling CodeCoder API"
        );

        let response = self
            .client
            .post(&url)
            .json(request)
            .timeout(self.timeout)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as error response
            if let Ok(error) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(anyhow::anyhow!("CodeCoder error: {}", error.error));
            }

            return Err(anyhow::anyhow!(
                "CodeCoder API returned {}: {}",
                status,
                error_text
            ));
        }

        let chat_response: ChatResponse = response.json().await?;

        tracing::debug!(
            conversation_id = ?chat_response.conversation_id,
            agent = ?chat_response.agent,
            usage = ?chat_response.usage,
            "CodeCoder response received"
        );

        Ok(chat_response)
    }

    /// Start a background processor that handles messages from a channel.
    pub fn spawn_processor(
        bridge: Arc<Self>,
        mut rx: mpsc::Receiver<ChannelMessage>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!("CodeCoder bridge processor started");

            while let Some(message) = rx.recv().await {
                let bridge = bridge.clone();

                // Process each message in its own task
                tokio::spawn(async move {
                    if let Err(e) = bridge.process(message).await {
                        tracing::error!(error = %e, "Failed to process message");
                    }
                });
            }

            tracing::info!("CodeCoder bridge processor stopped");
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ChannelType;
    use std::collections::HashMap;

    fn create_test_message() -> ChannelMessage {
        ChannelMessage {
            id: "test-msg-1".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "123456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello, CodeCoder!".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
        }
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            message: "Hello".into(),
            conversation_id: Some("conv-1".into()),
            agent: None,
            user_id: "user1".into(),
            channel: "telegram".into(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"message\":\"Hello\""));
        assert!(json.contains("\"conversation_id\":\"conv-1\""));
        assert!(!json.contains("\"agent\"")); // Should be skipped when None
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "message": "Hello back!",
            "conversation_id": "conv-1",
            "agent": "general",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20,
                "total_tokens": 30
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message, "Hello back!");
        assert_eq!(response.conversation_id, Some("conv-1".into()));
        assert!(response.usage.is_some());

        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
    }

    #[test]
    fn test_bridge_creation() {
        let router = Arc::new(OutboundRouter::new());
        let bridge = CodeCoderBridge::new("http://localhost:4400", router);
        assert_eq!(bridge.endpoint, "http://localhost:4400");
    }
}
