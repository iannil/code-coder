//! Webhook routes for Zero Gateway.
//!
//! This module provides webhook endpoints for receiving external messages
//! and forwarding them to the LLM for processing.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Webhook state.
#[derive(Clone)]
pub struct WebhookState {
    /// CodeCoder API endpoint to forward requests to
    pub codecoder_endpoint: String,
    /// HTTP client for making requests
    client: Arc<reqwest::Client>,
}

impl WebhookState {
    /// Create a new webhook state.
    pub fn new(codecoder_endpoint: &str) -> Self {
        Self {
            codecoder_endpoint: codecoder_endpoint.to_string(),
            client: Arc::new(reqwest::Client::new()),
        }
    }
}

/// Incoming webhook request.
#[derive(Debug, Deserialize)]
pub struct WebhookRequest {
    /// The message to process
    pub message: String,
    /// Optional context or metadata
    #[serde(default)]
    pub context: Option<serde_json::Value>,
    /// Optional user identifier
    #[serde(default)]
    pub user_id: Option<String>,
    /// Optional conversation ID for multi-turn
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// Webhook response.
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    /// Whether the request was successful
    pub success: bool,
    /// The response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Conversation ID for continuing the conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct WebhookErrorResponse {
    pub error: String,
    pub code: String,
}

/// Build webhook routes.
pub fn webhook_routes(state: WebhookState) -> Router {
    Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/webhook/chat", post(webhook_chat_handler))
        .with_state(state)
}

/// Handle incoming webhook request.
async fn webhook_handler(
    State(state): State<WebhookState>,
    Json(request): Json<WebhookRequest>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<WebhookErrorResponse>)> {
    // Validate request
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(WebhookErrorResponse {
                error: "Message cannot be empty".into(),
                code: "INVALID_REQUEST".into(),
            }),
        ));
    }

    // Forward to CodeCoder API
    let response = forward_to_codecoder(&state, &request).await?;

    Ok(Json(response))
}

/// Handle chat-style webhook (more conversational).
async fn webhook_chat_handler(
    State(state): State<WebhookState>,
    Json(request): Json<WebhookRequest>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<WebhookErrorResponse>)> {
    // Validate request
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(WebhookErrorResponse {
                error: "Message cannot be empty".into(),
                code: "INVALID_REQUEST".into(),
            }),
        ));
    }

    // Forward to CodeCoder API with chat context
    let response = forward_to_codecoder(&state, &request).await?;

    Ok(Json(response))
}

/// Forward request to CodeCoder API.
async fn forward_to_codecoder(
    state: &WebhookState,
    request: &WebhookRequest,
) -> Result<WebhookResponse, (StatusCode, Json<WebhookErrorResponse>)> {
    // Build the request payload for CodeCoder
    let payload = serde_json::json!({
        "prompt": request.message,
        "context": request.context,
        "conversation_id": request.conversation_id,
    });

    // Make request to CodeCoder
    let url = format!("{}/api/v1/chat", state.codecoder_endpoint);

    let response = state
        .client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to connect to CodeCoder");
            (
                StatusCode::BAD_GATEWAY,
                Json(WebhookErrorResponse {
                    error: "Failed to connect to backend".into(),
                    code: "BACKEND_ERROR".into(),
                }),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %body, "CodeCoder returned error");

        return Err((
            StatusCode::BAD_GATEWAY,
            Json(WebhookErrorResponse {
                error: format!("Backend returned {}", status),
                code: "BACKEND_ERROR".into(),
            }),
        ));
    }

    // Parse response
    let codecoder_response: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to parse CodeCoder response");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(WebhookErrorResponse {
                error: "Failed to parse backend response".into(),
                code: "PARSE_ERROR".into(),
            }),
        )
    })?;

    // Extract response text
    let response_text = codecoder_response
        .get("response")
        .and_then(|v| v.as_str())
        .map(String::from);

    let conversation_id = codecoder_response
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(WebhookResponse {
        success: true,
        response: response_text,
        error: None,
        conversation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_state_new() {
        let state = WebhookState::new("http://127.0.0.1:4400");
        assert_eq!(state.codecoder_endpoint, "http://127.0.0.1:4400");
    }
}
