//! HTTP routes for zero-channels webhook endpoints.
//!
//! Provides webhook handlers for channels that use push-based messaging:
//! - Telegram (webhook mode)
//! - Feishu (event subscriptions)
//! - WeChat Work (企业微信) event callbacks
//! - DingTalk (钉钉) outgoing robot webhooks
//! - WhatsApp (Meta Cloud API webhooks)
//! - Slack (Events API)
//! - Discord (interactions)

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::capture_bridge::CaptureBridge;
use crate::dingtalk::{self, DingTalkChannel};
use crate::feishu::{self, FeishuChannel};
use crate::message::{ChannelMessage, ChannelType, OutgoingContent};
use crate::outbound::OutboundRouter;
use crate::wecom::{self, WeComChannel};
use crate::whatsapp::WhatsAppChannel;

// ============================================================================
// State
// ============================================================================

/// Shared state for the channels HTTP server.
pub struct ChannelsState {
    /// Feishu channel instance (if configured)
    pub feishu: Option<Arc<FeishuChannel>>,
    /// WeChat Work channel instance (if configured)
    pub wecom: Option<Arc<WeComChannel>>,
    /// DingTalk channel instance (if configured)
    pub dingtalk: Option<Arc<DingTalkChannel>>,
    /// WhatsApp channel instance (if configured)
    pub whatsapp: Option<Arc<WhatsAppChannel>>,
    /// WhatsApp app secret for webhook signature verification
    pub whatsapp_app_secret: Option<Arc<str>>,
    /// Channel for forwarding incoming messages
    pub message_tx: mpsc::Sender<ChannelMessage>,
    /// CodeCoder API endpoint for forwarding
    pub codecoder_endpoint: String,
    /// Outbound router for sending messages
    pub outbound: Option<Arc<OutboundRouter>>,
    /// Capture bridge for asset capture (if configured)
    pub capture: Option<Arc<CaptureBridge>>,
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
                trace_id: zero_common::logging::generate_trace_id(),
                span_id: zero_common::logging::generate_span_id(),
                parent_span_id: None,
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
// WeChat Work Webhook
// ============================================================================

#[derive(Debug, Deserialize)]
struct WeComQueryParams {
    msg_signature: String,
    timestamp: String,
    nonce: String,
    echostr: Option<String>,
}

/// WeChat Work URL verification handler (GET request).
async fn wecom_verify(
    State(state): State<Arc<ChannelsState>>,
    Query(params): Query<WeComQueryParams>,
) -> impl IntoResponse {
    let Some(ref wecom) = state.wecom else {
        return (StatusCode::NOT_FOUND, "WeChat Work channel not configured".to_string());
    };

    let Some(echostr) = params.echostr else {
        return (StatusCode::BAD_REQUEST, "Missing echostr parameter".to_string());
    };

    match wecom.verify_url(&params.msg_signature, &params.timestamp, &params.nonce, &echostr) {
        Ok(decrypted) => (StatusCode::OK, decrypted),
        Err(e) => {
            tracing::error!("WeChat Work URL verification failed: {}", e);
            (StatusCode::FORBIDDEN, format!("Verification failed: {e}"))
        }
    }
}

/// WeChat Work message callback handler (POST request).
async fn wecom_webhook(
    State(state): State<Arc<ChannelsState>>,
    Query(params): Query<WeComQueryParams>,
    body: String,
) -> impl IntoResponse {
    let Some(ref wecom) = state.wecom else {
        return (
            StatusCode::NOT_FOUND,
            Json(WebhookResponse {
                success: false,
                message: Some("WeChat Work channel not configured".to_string()),
                challenge: None,
            }),
        );
    };

    match wecom::process_event_callback(
        wecom,
        &params.msg_signature,
        &params.timestamp,
        &params.nonce,
        &body,
    ) {
        Ok((_, message)) => {
            if let Some(msg) = message {
                if let Err(e) = state.message_tx.send(msg).await {
                    tracing::error!("Failed to forward WeChat Work message: {}", e);
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
            tracing::error!("WeChat Work webhook error: {}", e);
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
// DingTalk Webhook
// ============================================================================

#[derive(Debug, Deserialize)]
struct DingTalkQueryParams {
    timestamp: Option<String>,
    sign: Option<String>,
}

async fn dingtalk_webhook(
    State(state): State<Arc<ChannelsState>>,
    Query(params): Query<DingTalkQueryParams>,
    body: String,
) -> impl IntoResponse {
    let Some(ref dingtalk) = state.dingtalk else {
        return (
            StatusCode::NOT_FOUND,
            Json(WebhookResponse {
                success: false,
                message: Some("DingTalk channel not configured".to_string()),
                challenge: None,
            }),
        );
    };

    match dingtalk::process_outgoing_callback(
        dingtalk,
        params.timestamp.as_deref(),
        params.sign.as_deref(),
        &body,
    ) {
        Ok((_, message)) => {
            if let Some(msg) = message {
                if let Err(e) = state.message_tx.send(msg).await {
                    tracing::error!("Failed to forward DingTalk message: {}", e);
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
            tracing::error!("DingTalk webhook error: {}", e);
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
// WhatsApp Webhook
// ============================================================================

/// WhatsApp verification query params (Meta webhook verification)
#[derive(Debug, Deserialize)]
struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

/// Verify WhatsApp webhook signature (X-Hub-Signature-256).
/// Returns true if the signature is valid, false otherwise.
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// GET /webhook/whatsapp — Meta webhook verification
async fn whatsapp_verify(
    State(state): State<Arc<ChannelsState>>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> impl IntoResponse {
    let Some(ref whatsapp) = state.whatsapp else {
        return (StatusCode::NOT_FOUND, "WhatsApp channel not configured".to_string());
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches = params
        .verify_token
        .as_deref()
        .is_some_and(|t| {
            let expected = whatsapp.verify_token();
            t.len() == expected.len() && t.as_bytes().iter().zip(expected.as_bytes()).all(|(a, b)| a == b)
        });

    if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(challenge) = params.challenge {
            tracing::info!("WhatsApp webhook verified successfully");
            return (StatusCode::OK, challenge);
        }
        return (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string());
    }

    tracing::warn!("WhatsApp webhook verification failed — token mismatch");
    (StatusCode::FORBIDDEN, "Forbidden".to_string())
}

/// POST /webhook/whatsapp — incoming message webhook
async fn whatsapp_webhook(
    State(state): State<Arc<ChannelsState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref whatsapp) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(WebhookResponse {
                success: false,
                message: Some("WhatsApp channel not configured".to_string()),
                challenge: None,
            }),
        );
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = state.whatsapp_app_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_whatsapp_signature(app_secret, &body, signature) {
            tracing::warn!(
                "WhatsApp webhook signature verification failed (signature: {})",
                if signature.is_empty() { "missing" } else { "invalid" }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(WebhookResponse {
                    success: false,
                    message: Some("Invalid signature".to_string()),
                    challenge: None,
                }),
            );
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(WebhookResponse {
                success: false,
                message: Some("Invalid JSON payload".to_string()),
                challenge: None,
            }),
        );
    };

    // Parse messages from the webhook payload
    let messages = whatsapp.parse_webhook_payload(&payload);

    if messages.is_empty() {
        // Acknowledge the webhook even if no messages (could be status updates)
        return (
            StatusCode::OK,
            Json(WebhookResponse {
                success: true,
                message: None,
                challenge: None,
            }),
        );
    }

    // Forward messages to the message processor
    for msg in messages {
        tracing::info!(
            "WhatsApp message from {}: {}",
            msg.user_id,
            msg.text().unwrap_or_default().chars().take(50).collect::<String>()
        );

        if let Err(e) = state.message_tx.send(msg).await {
            tracing::error!("Failed to forward WhatsApp message: {}", e);
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
        "wecom" => crate::message::ChannelType::WeCom,
        "dingtalk" => crate::message::ChannelType::DingTalk,
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
        trace_id: zero_common::logging::generate_trace_id(),
        span_id: zero_common::logging::generate_span_id(),
        parent_span_id: None,
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
// Send API (for outbound messages)
// ============================================================================

#[derive(Debug, Deserialize)]
struct SendRequest {
    /// Channel type: feishu, wecom, dingtalk, telegram
    channel_type: String,
    /// Channel ID (group chat ID or user ID)
    channel_id: String,
    /// Message content
    content: SendContent,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SendContent {
    /// Plain text message
    Text { text: String },
    /// Markdown message
    Markdown { text: String },
}

#[derive(Debug, Serialize)]
struct SendResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn send_message(
    State(state): State<Arc<ChannelsState>>,
    Json(req): Json<SendRequest>,
) -> impl IntoResponse {
    let Some(ref outbound) = state.outbound else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(SendResponse {
                success: false,
                message_id: None,
                error: Some("Outbound router not configured".to_string()),
            }),
        );
    };

    let channel_type = match req.channel_type.to_lowercase().as_str() {
        "telegram" => ChannelType::Telegram,
        "feishu" => ChannelType::Feishu,
        "wecom" => ChannelType::WeCom,
        "dingtalk" => ChannelType::DingTalk,
        "discord" => ChannelType::Discord,
        "slack" => ChannelType::Slack,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SendResponse {
                    success: false,
                    message_id: None,
                    error: Some(format!("Unsupported channel type: {}", req.channel_type)),
                }),
            );
        }
    };

    let content = match req.content {
        SendContent::Text { text } => OutgoingContent::Text { text },
        SendContent::Markdown { text } => OutgoingContent::Markdown { text },
    };

    let result = outbound.send_direct(channel_type, req.channel_id, content).await;

    if result.success {
        (
            StatusCode::OK,
            Json(SendResponse {
                success: true,
                message_id: result.message_id,
                error: None,
            }),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SendResponse {
                success: false,
                message_id: None,
                error: result.error,
            }),
        )
    }
}

// ============================================================================
// Capture API
// ============================================================================

/// Request to capture content from a URL.
#[derive(Debug, Deserialize)]
struct CaptureRequest {
    /// URL to capture
    url: String,
    /// Optional additional tags
    #[serde(default)]
    tags: Vec<String>,
    /// Destination platform (feishu_docs, notion)
    #[serde(default)]
    destination: Option<String>,
}

/// Response from capture operations.
#[derive(Debug, Serialize)]
struct CaptureResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<crate::capture_bridge::CapturedAsset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Query parameters for capture history.
#[derive(Debug, Deserialize)]
struct HistoryQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    20
}

/// Response for history listing.
#[derive(Debug, Serialize)]
struct HistoryResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Vec<crate::capture_bridge::CapturedAsset>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Request to save an asset to a new destination.
#[derive(Debug, Deserialize)]
struct SaveRequest {
    /// Destination platform (feishu_docs, notion)
    destination: String,
}

/// Response from save operations.
#[derive(Debug, Serialize)]
struct SaveResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<crate::capture_bridge::SavedLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Capture content from a URL.
async fn capture_url(
    State(state): State<Arc<ChannelsState>>,
    Json(req): Json<CaptureRequest>,
) -> impl IntoResponse {
    let Some(ref capture) = state.capture else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(CaptureResponse {
                success: false,
                data: None,
                error: Some("Capture service not configured".to_string()),
            }),
        );
    };

    match capture
        .capture_url(&req.url, Some(req.tags), req.destination.as_deref())
        .await
    {
        Ok(asset) => (
            StatusCode::OK,
            Json(CaptureResponse {
                success: true,
                data: Some(asset),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CaptureResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Get capture history.
async fn capture_history(
    State(state): State<Arc<ChannelsState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let Some(ref capture) = state.capture else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HistoryResponse {
                success: false,
                data: None,
                total: None,
                error: Some("Capture service not configured".to_string()),
            }),
        );
    };

    let assets = capture.get_history(query.limit, query.offset).await;
    let total = assets.len();

    (
        StatusCode::OK,
        Json(HistoryResponse {
            success: true,
            data: Some(assets),
            total: Some(total),
            error: None,
        }),
    )
}

/// Get a specific captured asset.
async fn get_asset(
    State(state): State<Arc<ChannelsState>>,
    Path(asset_id): Path<String>,
) -> impl IntoResponse {
    let Some(ref capture) = state.capture else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(CaptureResponse {
                success: false,
                data: None,
                error: Some("Capture service not configured".to_string()),
            }),
        );
    };

    match capture.get_asset(&asset_id).await {
        Some(asset) => (
            StatusCode::OK,
            Json(CaptureResponse {
                success: true,
                data: Some(asset),
                error: None,
            }),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(CaptureResponse {
                success: false,
                data: None,
                error: Some(format!("Asset not found: {}", asset_id)),
            }),
        ),
    }
}

/// Save an asset to a new destination.
async fn save_asset(
    State(state): State<Arc<ChannelsState>>,
    Path(asset_id): Path<String>,
    Json(req): Json<SaveRequest>,
) -> impl IntoResponse {
    let Some(ref capture) = state.capture else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(SaveResponse {
                success: false,
                data: None,
                error: Some("Capture service not configured".to_string()),
            }),
        );
    };

    match capture.save_to_destination(&asset_id, &req.destination).await {
        Ok(location) => (
            StatusCode::OK,
            Json(SaveResponse {
                success: true,
                data: Some(location),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SaveResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
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
        .route("/webhook/wecom", get(wecom_verify).post(wecom_webhook))
        .route("/webhook/dingtalk", post(dingtalk_webhook))
        .route("/webhook/whatsapp", get(whatsapp_verify).post(whatsapp_webhook))
        .route("/webhook/generic", post(generic_webhook))
        // Send API
        .route("/api/v1/send", post(send_message))
        // Capture API
        .route("/api/v1/capture", post(capture_url))
        .route("/api/v1/capture/history", get(capture_history))
        .route("/api/v1/capture/:asset_id", get(get_asset))
        .route("/api/v1/capture/:asset_id/save", post(save_asset))
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
        wecom: None,
        dingtalk: None,
        whatsapp: None,
        whatsapp_app_secret: None,
        message_tx: tx,
        codecoder_endpoint,
        outbound: None,
        capture: None,
    });

    (state, rx)
}

/// Create a channels state with extended channel support.
pub fn create_state_extended(
    feishu: Option<Arc<FeishuChannel>>,
    wecom: Option<Arc<WeComChannel>>,
    dingtalk: Option<Arc<DingTalkChannel>>,
    codecoder_endpoint: String,
) -> (Arc<ChannelsState>, mpsc::Receiver<ChannelMessage>) {
    let (tx, rx) = mpsc::channel(100);

    let state = Arc::new(ChannelsState {
        feishu,
        wecom,
        dingtalk,
        whatsapp: None,
        whatsapp_app_secret: None,
        message_tx: tx,
        codecoder_endpoint,
        outbound: None,
        capture: None,
    });

    (state, rx)
}

/// Create a channels state with outbound router.
pub fn create_state_with_outbound(
    feishu: Option<Arc<FeishuChannel>>,
    wecom: Option<Arc<WeComChannel>>,
    dingtalk: Option<Arc<DingTalkChannel>>,
    outbound: Arc<OutboundRouter>,
    codecoder_endpoint: String,
) -> (Arc<ChannelsState>, mpsc::Receiver<ChannelMessage>) {
    let (tx, rx) = mpsc::channel(100);

    let state = Arc::new(ChannelsState {
        feishu,
        wecom,
        dingtalk,
        whatsapp: None,
        whatsapp_app_secret: None,
        message_tx: tx,
        codecoder_endpoint,
        outbound: Some(outbound),
        capture: None,
    });

    (state, rx)
}

/// Create a channels state with capture support.
pub fn create_state_with_capture(
    feishu: Option<Arc<FeishuChannel>>,
    wecom: Option<Arc<WeComChannel>>,
    dingtalk: Option<Arc<DingTalkChannel>>,
    outbound: Arc<OutboundRouter>,
    capture: Arc<CaptureBridge>,
    codecoder_endpoint: String,
) -> (Arc<ChannelsState>, mpsc::Receiver<ChannelMessage>) {
    let (tx, rx) = mpsc::channel(100);

    let state = Arc::new(ChannelsState {
        feishu,
        wecom,
        dingtalk,
        whatsapp: None,
        whatsapp_app_secret: None,
        message_tx: tx,
        codecoder_endpoint,
        outbound: Some(outbound),
        capture: Some(capture),
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
        create_state_extended(None, None, None, "http://localhost:4400".to_string())
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
