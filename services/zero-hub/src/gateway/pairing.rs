//! Pairing authentication routes for Zero Gateway.
//!
//! This module provides pairing-based authentication, which allows devices
//! to authenticate using a one-time pairing code displayed on the server.
//!
//! The pairing flow:
//! 1. Server generates a pairing code and displays it
//! 2. Client sends the pairing code via POST /pair
//! 3. Server validates and returns a session token
//! 4. Token is stored and used for subsequent requests

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use zero_common::security::pairing::PairingGuard;

/// Pairing state shared across requests.
#[derive(Clone)]
pub struct PairingState {
    guard: Arc<RwLock<PairingGuard>>,
}

impl PairingState {
    /// Create a new pairing state.
    pub fn new(require_pairing: bool, existing_tokens: &[String]) -> Self {
        Self {
            guard: Arc::new(RwLock::new(PairingGuard::new(require_pairing, existing_tokens))),
        }
    }

    /// Check if a request is authenticated (has valid paired token).
    pub async fn is_authenticated(&self, headers: &HeaderMap) -> bool {
        // Check Authorization header for Bearer token
        if let Some(auth) = headers.get(header::AUTHORIZATION) {
            if let Ok(auth_str) = auth.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    let guard = self.guard.read().await;
                    return guard.is_authenticated(token);
                }
            }
        }

        // Check X-Pairing-Token header
        if let Some(token) = headers.get("X-Pairing-Token") {
            if let Ok(token_str) = token.to_str() {
                let guard = self.guard.read().await;
                return guard.is_authenticated(token_str);
            }
        }

        false
    }

    /// Get the current pairing code.
    pub async fn get_code(&self) -> Option<String> {
        let guard = self.guard.read().await;
        guard.pairing_code()
    }

    /// Try to pair with a code.
    /// Returns Ok(Some(token)) on success, Ok(None) on invalid code,
    /// Err(lockout_secs) if locked out due to brute force.
    pub async fn try_pair(&self, code: &str) -> Result<Option<String>, u64> {
        let guard = self.guard.read().await;
        guard.try_pair(code)
    }

    /// Check if pairing is required.
    pub async fn require_pairing(&self) -> bool {
        let guard = self.guard.read().await;
        guard.require_pairing()
    }

    /// Check if already paired (has at least one token).
    pub async fn is_paired(&self) -> bool {
        let guard = self.guard.read().await;
        guard.is_paired()
    }

    /// Get all paired token hashes (for persisting to config).
    pub async fn tokens(&self) -> Vec<String> {
        let guard = self.guard.read().await;
        guard.tokens()
    }
}

/// Pairing request body.
#[derive(Debug, Deserialize)]
pub struct PairingRequest {
    /// The pairing code from the server display
    pub code: String,
}

/// Pairing response.
#[derive(Debug, Serialize)]
pub struct PairingResponse {
    /// Whether pairing was successful
    pub success: bool,
    /// Session token (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Lockout seconds remaining (if locked out)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockout_secs: Option<u64>,
}

/// Pairing status response.
#[derive(Debug, Serialize)]
pub struct PairingStatusResponse {
    /// Whether the server requires pairing
    pub pairing_required: bool,
    /// Whether the current request is authenticated
    pub is_authenticated: bool,
    /// Whether the server is already paired (has tokens)
    pub is_paired: bool,
    /// Current pairing code (only shown if not authenticated and not yet paired)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing_code: Option<String>,
}

/// Build pairing routes.
pub fn pairing_routes(state: PairingState) -> Router {
    Router::new()
        .route("/pair", post(pair_handler))
        .route("/pair/status", get(pair_status_handler))
        .with_state(state)
}

/// Handle pairing request.
async fn pair_handler(
    State(state): State<PairingState>,
    headers: HeaderMap,
    Json(request): Json<PairingRequest>,
) -> Result<Json<PairingResponse>, (StatusCode, Json<PairingResponse>)> {
    // Check if already authenticated
    if state.is_authenticated(&headers).await {
        return Ok(Json(PairingResponse {
            success: true,
            token: None, // Already authenticated
            error: None,
            lockout_secs: None,
        }));
    }

    // Try to pair
    match state.try_pair(&request.code).await {
        Ok(Some(token)) => Ok(Json(PairingResponse {
            success: true,
            token: Some(token),
            error: None,
            lockout_secs: None,
        })),
        Ok(None) => Err((
            StatusCode::UNAUTHORIZED,
            Json(PairingResponse {
                success: false,
                token: None,
                error: Some("Invalid or expired pairing code".into()),
                lockout_secs: None,
            }),
        )),
        Err(lockout_secs) => Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(PairingResponse {
                success: false,
                token: None,
                error: Some(format!(
                    "Too many failed attempts. Try again in {} seconds",
                    lockout_secs
                )),
                lockout_secs: Some(lockout_secs),
            }),
        )),
    }
}

/// Handle pairing status check.
async fn pair_status_handler(
    State(state): State<PairingState>,
    headers: HeaderMap,
) -> Json<PairingStatusResponse> {
    let is_authenticated = state.is_authenticated(&headers).await;
    let is_paired = state.is_paired().await;
    let pairing_required = state.require_pairing().await;

    // Only show pairing code if:
    // - Not authenticated
    // - Not yet paired (no existing tokens)
    // In production, you'd also want to check if the request is from localhost
    let pairing_code = if !is_authenticated && !is_paired {
        state.get_code().await
    } else {
        None
    };

    Json(PairingStatusResponse {
        pairing_required,
        is_authenticated,
        is_paired,
        pairing_code,
    })
}

/// Middleware to check pairing authentication.
pub async fn pairing_middleware(
    State(state): State<PairingState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // Check if authenticated
    if !state.is_authenticated(&headers).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pairing_state_new_with_pairing_required() {
        let state = PairingState::new(true, &[]);
        let code = state.get_code().await;
        assert!(code.is_some());
        assert_eq!(code.unwrap().len(), 6); // 6-digit code
    }

    #[tokio::test]
    async fn pairing_state_new_without_pairing() {
        let state = PairingState::new(false, &[]);
        let code = state.get_code().await;
        assert!(code.is_none()); // No code when pairing not required
    }

    #[tokio::test]
    async fn pairing_flow() {
        let state = PairingState::new(true, &[]);

        // Get current code
        let code = state.get_code().await.unwrap();

        // Try pair with correct code
        let result = state.try_pair(&code).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn pairing_invalid_code() {
        let state = PairingState::new(true, &[]);

        // Try pair with incorrect code
        let result = state.try_pair("000000").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
