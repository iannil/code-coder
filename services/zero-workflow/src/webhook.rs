//! Webhook handling for Zero Workflow.
//!
//! Handles incoming webhooks from Git platforms and triggers automated code reviews
//! and ticket automation.

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;

use crate::github::PullRequestEvent;
use crate::gitlab::MergeRequestEvent;
use crate::review_bridge::ReviewBridge;
use crate::ticket_bridge::TicketBridge;

type HmacSha256 = Hmac<Sha256>;

/// Webhook handler state.
#[derive(Clone)]
pub struct WebhookState {
    /// Shared secret for signature verification
    pub secret: Option<Arc<String>>,
    /// GitHub webhook secret
    pub github_secret: Option<Arc<String>>,
    /// GitLab webhook token
    pub gitlab_token: Option<Arc<String>>,
    /// Review bridge (optional, for code review)
    pub review_bridge: Option<Arc<ReviewBridge>>,
    /// Ticket bridge (optional, for ticket automation)
    pub ticket_bridge: Option<Arc<TicketBridge>>,
}

impl WebhookState {
    /// Create a new webhook state without review bridge.
    pub fn new(
        secret: Option<Arc<String>>,
        github_secret: Option<Arc<String>>,
        gitlab_token: Option<Arc<String>>,
    ) -> Self {
        Self {
            secret,
            github_secret,
            gitlab_token,
            review_bridge: None,
            ticket_bridge: None,
        }
    }

    /// Set the review bridge.
    pub fn with_review_bridge(mut self, bridge: Arc<ReviewBridge>) -> Self {
        self.review_bridge = Some(bridge);
        self
    }

    /// Set the ticket bridge.
    pub fn with_ticket_bridge(mut self, bridge: Arc<TicketBridge>) -> Self {
        self.ticket_bridge = Some(bridge);
        self
    }
}

/// Webhook event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Event type (push, pull_request, etc.)
    pub event_type: String,
    /// Source (github, gitlab, custom)
    pub source: String,
    /// Event payload
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: i64,
}

/// Webhook response.
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_url: Option<String>,
}

/// Build webhook routes.
pub fn webhook_routes(state: WebhookState) -> Router {
    Router::new()
        .route("/webhook", post(generic_webhook))
        .route("/webhook/github", post(github_webhook))
        .route("/webhook/gitlab", post(gitlab_webhook))
        .with_state(state)
}

/// Generic webhook handler.
async fn generic_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Verify signature if secret is configured
    if let Some(ref secret) = state.secret {
        let signature = headers
            .get("X-Webhook-Signature")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !verify_signature(secret, &body, signature) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Parse payload
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let _event = WebhookEvent {
        event_type: "custom".into(),
        source: "webhook".into(),
        payload,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    tracing::info!(event_type = "custom", "Received webhook event");

    Ok(Json(WebhookResponse {
        status: "ok".into(),
        message: "Event received".into(),
        review_url: None,
    }))
}

/// GitHub webhook handler.
async fn github_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Verify GitHub signature
    if let Some(ref secret) = state.github_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // GitHub uses "sha256=..." format
        let signature = signature
            .strip_prefix("sha256=")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !verify_signature(secret, &body, signature) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Get event type
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Parse payload
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    tracing::info!(event_type = %event_type, "Received GitHub webhook event");

    // Handle pull_request events for code review
    if event_type == "pull_request" {
        if let Some(ref bridge) = state.review_bridge {
            // Parse as PR event
            match serde_json::from_value::<PullRequestEvent>(payload.clone()) {
                Ok(pr_event) => {
                    if pr_event.should_review() {
                        let bridge = bridge.clone();
                        let pr_event_clone = pr_event.clone();

                        // Process review in background
                        tokio::spawn(async move {
                            match bridge.process_github_pr(&pr_event_clone).await {
                                Ok(result) => {
                                    tracing::info!(
                                        pr = pr_event_clone.pr_number(),
                                        "Code review completed: {:?}",
                                        result
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        pr = pr_event_clone.pr_number(),
                                        error = %e,
                                        "Code review failed"
                                    );
                                }
                            }
                        });

                        return Ok(Json(WebhookResponse {
                            status: "ok".into(),
                            message: format!(
                                "Pull request #{} received, code review triggered",
                                pr_event.pr_number()
                            ),
                            review_url: None,
                        }));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to parse pull_request event");
                }
            }
        }
    }

    Ok(Json(WebhookResponse {
        status: "ok".into(),
        message: format!("GitHub {} event received", event_type),
        review_url: None,
    }))
}

/// GitLab webhook handler.
async fn gitlab_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Verify GitLab token
    if let Some(ref token) = state.gitlab_token {
        let provided_token = headers
            .get("X-Gitlab-Token")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if provided_token != token.as_str() {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Get event type
    let event_type = headers
        .get("X-Gitlab-Event")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Parse payload
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    tracing::info!(event_type = %event_type, "Received GitLab webhook event");

    // Handle merge_request events for code review
    if event_type == "Merge Request Hook" {
        if let Some(ref bridge) = state.review_bridge {
            // Parse as MR event
            match serde_json::from_value::<MergeRequestEvent>(payload.clone()) {
                Ok(mr_event) => {
                    if mr_event.should_review() {
                        let bridge = bridge.clone();
                        let mr_event_clone = mr_event.clone();

                        // Process review in background
                        tokio::spawn(async move {
                            match bridge.process_gitlab_mr(&mr_event_clone).await {
                                Ok(result) => {
                                    tracing::info!(
                                        mr = mr_event_clone.mr_iid(),
                                        "Code review completed: {:?}",
                                        result
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        mr = mr_event_clone.mr_iid(),
                                        error = %e,
                                        "Code review failed"
                                    );
                                }
                            }
                        });

                        return Ok(Json(WebhookResponse {
                            status: "ok".into(),
                            message: format!(
                                "Merge request !{} received, code review triggered",
                                mr_event.mr_iid()
                            ),
                            review_url: None,
                        }));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to parse merge_request event");
                }
            }
        }
    }

    Ok(Json(WebhookResponse {
        status: "ok".into(),
        message: format!("GitLab {} event received", event_type),
        review_url: None,
    }))
}

/// Verify HMAC-SHA256 signature.
fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);

    let expected = hex::encode(mac.finalize().into_bytes());
    expected.eq_ignore_ascii_case(signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature() {
        let secret = "test-secret";
        let body = b"test body";

        // Generate correct signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(secret, body, &signature));
        assert!(!verify_signature(secret, body, "invalid"));
    }

    #[test]
    fn test_webhook_state_creation() {
        let state = WebhookState::new(
            Some(Arc::new("secret".into())),
            Some(Arc::new("github-secret".into())),
            None,
        );

        assert!(state.secret.is_some());
        assert!(state.github_secret.is_some());
        assert!(state.gitlab_token.is_none());
        assert!(state.review_bridge.is_none());
        assert!(state.ticket_bridge.is_none());
    }
}
