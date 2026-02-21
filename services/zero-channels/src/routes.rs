//! HTTP routes for zero-channels webhook endpoints.
//!
//! Provides webhook handlers for channels that use push-based messaging:
//! - Telegram (webhook mode)
//! - Feishu (event subscriptions)
//! - Slack (Events API)
//! - Discord (interactions)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::feishu::{self, FeishuChannel};
use crate::message::ChannelMessage;

// ============================================================================
// State
// ============================================================================

/// Shared state for the channels HTTP server.
pub struct ChannelsState {
    /// Feishu channel instance (if configured)
    pub feishu: Option<Arc<FeishuChannel>>,
    /// Channel for forwarding incoming messages
    pub message_tx: mpsc::Sender<ChannelMessage>,
    /// CodeCoder API endpoint for forwarding
    pub codecoder_endpoint: String,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebhookResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    challenge: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ============================================================================
// Health Routes
// ============================================================================

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy",
        service: "zero-channels",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ready(State(state): State<Arc<ChannelsState>>) -> impl IntoResponse {
    // Check if message channel is still open
    if state.message_tx.is_closed() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "not_ready",
                service: "zero-channels",
                version: env!("CARGO_PKG_VERSION"),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ready",
            service: "zero-channels",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

// ============================================================================
// Feishu Webhook
// ============================================================================

async fn feishu_webhook(
    State(state): State<Arc<ChannelsState>>,
    body: String,
) -> impl IntoResponse {
    let Some(ref feishu) = state.feishu else {
        return (
            StatusCode::NOT_FOUND,
            Json(WebhookResponse {
                success: false,
                message: Some("Feishu channel not configured".to_string()),
                challenge: None,
            }),
        );
    };

    match feishu::process_event_callback(feishu, &body) {
        Ok((challenge_response, message)) => {
            // If there's a challenge, return it (URL verification)
            if let Some(challenge) = challenge_response {
                // Parse the challenge response to extract the challenge value
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&challenge) {
                    if let Some(c) = parsed.get("challenge").and_then(|v| v.as_str()) {
                        return (
                            StatusCode::OK,
                            Json(WebhookResponse {
                                success: true,
                                message: None,
                                challenge: Some(c.to_string()),
                            }),
                        );
                    }
                }
            }

            // If there's a message, forward it
            if let Some(msg) = message {
                if let Err(e) = state.message_tx.send(msg).await {
                    tracing::error!("Failed to forward Feishu message: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(WebhookResponse {
                            success: false,
                            message: Some(format!("Failed to forward message: {e}")),
                            challenge: None,
                        }),
                    );
                }
            }

            (
                StatusCode::OK,
                Json(WebhookResponse {
                    success: true,
                    message: None,
                    challenge: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!("Feishu webhook error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    success: false,
                    message: Some(e.to_string()),
                    challenge: None,
                }),
            )
        }
    }
}

// ============================================================================
// Telegram Webhook
// ============================================================================

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    chat: TelegramChat,
    from: Option<TelegramUser>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
    username: Option<String>,
}

async fn telegram_webhook(
    State(state): State<Arc<ChannelsState>>,
    Path(token): Path<String>,
    Json(update): Json<TelegramUpdate>,
) -> impl IntoResponse {
    tracing::debug!("Telegram webhook received update_id: {}", update.update_id);

    // Validate token (simple check - in production, compare against configured token)
    if token.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(WebhookResponse {
                success: false,
                message: Some("Invalid token".to_string()),
                challenge: None,
            }),
        );
    }

    // Process text messages
    if let Some(msg) = update.message {
        if let Some(text) = msg.text {
            let user_id = msg
                .from
                .map(|u| u.username.unwrap_or_else(|| u.id.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            let channel_msg = ChannelMessage {
                id: uuid::Uuid::new_v4().to_string(),
                channel_type: crate::message::ChannelType::Telegram,
                channel_id: msg.chat.id.to_string(),
                user_id,
                content: crate::message::MessageContent::Text { text },
                attachments: vec![],
                metadata: std::collections::HashMap::new(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
            };

            if let Err(e) = state.message_tx.send(channel_msg).await {
                tracing::error!("Failed to forward Telegram message: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(WebhookResponse {
                        success: false,
                        message: Some(format!("Failed to forward message: {e}")),
                        challenge: None,
                    }),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(WebhookResponse {
            success: true,
            message: None,
            challenge: None,
        }),
    )
}

// ============================================================================
// Generic Webhook (for custom integrations)
// ============================================================================

#[derive(Debug, Deserialize)]
struct GenericWebhookPayload {
    channel: String,
    user_id: String,
    message: String,
    #[serde(default)]
    metadata: std::collections::HashMap<String, serde_json::Value>,
}

async fn generic_webhook(
    State(state): State<Arc<ChannelsState>>,
    Json(payload): Json<GenericWebhookPayload>,
) -> impl IntoResponse {
    let channel_type = match payload.channel.to_lowercase().as_str() {
        "telegram" => crate::message::ChannelType::Telegram,
        "discord" => crate::message::ChannelType::Discord,
        "slack" => crate::message::ChannelType::Slack,
        "feishu" => crate::message::ChannelType::Feishu,
        "whatsapp" => crate::message::ChannelType::WhatsApp,
        "matrix" => crate::message::ChannelType::Matrix,
        "imessage" => crate::message::ChannelType::IMessage,
        "email" => crate::message::ChannelType::Email,
        _ => crate::message::ChannelType::Cli,
    };

    let msg = ChannelMessage {
        id: uuid::Uuid::new_v4().to_string(),
        channel_type,
        channel_id: payload.channel.clone(),
        user_id: payload.user_id,
        content: crate::message::MessageContent::Text {
            text: payload.message,
        },
        attachments: vec![],
        metadata: payload
            .metadata
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
    };

    if let Err(e) = state.message_tx.send(msg).await {
        tracing::error!("Failed to forward generic webhook message: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(WebhookResponse {
                success: false,
                message: Some(format!("Failed to forward message: {e}")),
                challenge: None,
            }),
        );
    }

    (
        StatusCode::OK,
        Json(WebhookResponse {
            success: true,
            message: None,
            challenge: None,
        }),
    )
}

// ============================================================================
// Router Builder
// ============================================================================

/// Build the channels HTTP router.
pub fn build_router(state: Arc<ChannelsState>) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health))
        .route("/ready", get(ready))
        // Channel webhooks
        .route("/webhook/feishu", post(feishu_webhook))
        .route("/webhook/telegram/:token", post(telegram_webhook))
        .route("/webhook/generic", post(generic_webhook))
        // Add state
        .with_state(state)
}

/// Create a channels state with a message receiver.
pub fn create_state(
    feishu: Option<Arc<FeishuChannel>>,
    codecoder_endpoint: String,
) -> (Arc<ChannelsState>, mpsc::Receiver<ChannelMessage>) {
    let (tx, rx) = mpsc::channel(100);

    let state = Arc::new(ChannelsState {
        feishu,
        message_tx: tx,
        codecoder_endpoint,
    });

    (state, rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn create_test_state() -> (Arc<ChannelsState>, mpsc::Receiver<ChannelMessage>) {
        create_state(None, "http://localhost:4400".to_string())
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let (state, _rx) = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_endpoint() {
        let (state, _rx) = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_feishu_webhook_not_configured() {
        let (state, _rx) = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/feishu")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_generic_webhook() {
        let (state, mut rx) = create_test_state();
        let app = build_router(state);

        let payload = serde_json::json!({
            "channel": "test",
            "user_id": "user123",
            "message": "Hello, World!"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/generic")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check that message was forwarded
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.user_id, "user123");
        if let crate::message::MessageContent::Text { text } = msg.content {
            assert_eq!(text, "Hello, World!");
        } else {
            panic!("Expected text message");
        }
    }

    #[tokio::test]
    async fn test_telegram_webhook() {
        let (state, mut rx) = create_test_state();
        let app = build_router(state);

        let payload = serde_json::json!({
            "update_id": 12345,
            "message": {
                "message_id": 1,
                "chat": { "id": 123456789 },
                "from": { "id": 987654321, "username": "testuser" },
                "text": "Hello from Telegram!"
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/telegram/test-token")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check that message was forwarded
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.user_id, "testuser");
        assert_eq!(msg.channel_id, "123456789");
        if let crate::message::MessageContent::Text { text } = msg.content {
            assert_eq!(text, "Hello from Telegram!");
        } else {
            panic!("Expected text message");
        }
    }
}
