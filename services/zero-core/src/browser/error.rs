//! Error types for zero-browser.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Browser service errors.
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Browser launch failed: {0}")]
    LaunchFailed(String),

    #[error("Navigation timeout: {url}")]
    NavigationTimeout { url: String },

    #[error("Element not found: {selector}")]
    ElementNotFound { selector: String },

    #[error("Network interception failed: {0}")]
    NetworkError(String),

    #[error("Pattern not found: {0}")]
    PatternNotFound(String),

    #[error("Pattern extraction failed: {reason}")]
    PatternExtractionFailed { reason: String },

    #[error("Replay failed: {pattern_id} - {reason}")]
    ReplayFailed { pattern_id: String, reason: String },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// API error response.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl IntoResponse for BrowserError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            BrowserError::SessionNotFound(_) => (StatusCode::NOT_FOUND, "SESSION_NOT_FOUND"),
            BrowserError::PatternNotFound(_) => (StatusCode::NOT_FOUND, "PATTERN_NOT_FOUND"),
            BrowserError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
            BrowserError::ElementNotFound { .. } => (StatusCode::NOT_FOUND, "ELEMENT_NOT_FOUND"),
            BrowserError::NavigationTimeout { .. } => {
                (StatusCode::GATEWAY_TIMEOUT, "NAVIGATION_TIMEOUT")
            }
            BrowserError::LaunchFailed(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "BROWSER_UNAVAILABLE")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = serde_json::json!({
            "success": false,
            "error": ApiError {
                code: code.to_string(),
                message: self.to_string(),
            }
        });

        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BrowserError::SessionNotFound("abc123".to_string());
        assert_eq!(err.to_string(), "Session not found: abc123");
    }

    #[test]
    fn test_error_into_response() {
        let err = BrowserError::InvalidRequest("missing url".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
