//! Token metering middleware for Zero Gateway.
//!
//! Intercepts proxy requests/responses to extract token usage from LLM API
//! responses and records them in the quota manager.

use crate::quota::QuotaManager;
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// Token Usage Types
// ============================================================================

/// Token usage information extracted from LLM responses.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Input/prompt tokens
    #[serde(alias = "prompt_tokens")]
    pub input_tokens: i64,
    /// Output/completion tokens
    #[serde(alias = "completion_tokens")]
    pub output_tokens: i64,
    /// Total tokens (may be calculated)
    #[serde(default)]
    pub total_tokens: i64,
    /// Model used (for cost calculation)
    #[serde(default)]
    pub model: Option<String>,
}

impl TokenUsage {
    /// Calculate total if not provided.
    pub fn total(&self) -> i64 {
        if self.total_tokens > 0 {
            self.total_tokens
        } else {
            self.input_tokens + self.output_tokens
        }
    }
}

/// Response wrapper that may contain usage info.
#[derive(Debug, Deserialize)]
struct LlmResponse {
    #[serde(default)]
    usage: Option<TokenUsage>,
}

// ============================================================================
// Metering State
// ============================================================================

/// State for the metering middleware.
#[derive(Clone)]
pub struct MeteringState {
    /// Quota manager for recording usage
    pub quota_manager: Arc<QuotaManager>,
}

impl MeteringState {
    /// Create a new metering state with the default database path.
    pub fn new() -> anyhow::Result<Self> {
        let db_path = Self::default_db_path();
        Self::with_db_path(&db_path)
    }

    /// Create a new metering state with a custom database path.
    pub fn with_db_path(db_path: &PathBuf) -> anyhow::Result<Self> {
        let quota_manager = QuotaManager::new(db_path)?;
        Ok(Self {
            quota_manager: Arc::new(quota_manager),
        })
    }

    fn default_db_path() -> PathBuf {
        let config_dir = zero_common::config::config_dir();
        config_dir.join("metering.db")
    }
}

impl Default for MeteringState {
    fn default() -> Self {
        Self::new().expect("Failed to initialize metering state")
    }
}

// ============================================================================
// Middleware
// ============================================================================

/// Metering middleware that records token usage.
///
/// This middleware:
/// 1. Checks quota before allowing the request
/// 2. Passes the request through to the next handler
/// 3. Extracts token usage from the response
/// 4. Records usage in the quota manager
pub async fn metering_middleware(
    State(state): State<MeteringState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Extract user ID from request (set by auth middleware)
    let user_id = request
        .extensions()
        .get::<crate::auth::AuthUser>()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Check quota before processing
    let within_quota = state
        .quota_manager
        .check_quota(&user_id)
        .unwrap_or(true);

    if !within_quota {
        tracing::warn!(user_id = %user_id, "Request rejected: quota exceeded");
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Pass through to the next handler
    let response = next.run(request).await;

    // Only process successful responses for metering
    if !response.status().is_success() {
        return Ok(response);
    }

    // Extract the response body to read usage
    let (parts, body) = response.into_parts();

    // Read the body
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to read response body");
            // Reconstruct response without metering
            return Ok(Response::from_parts(parts, Body::empty()));
        }
    };

    // Try to extract usage
    if let Some(usage) = extract_usage(&body_bytes) {
        tracing::debug!(
            user_id = %user_id,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            "Recording token usage"
        );

        if let Err(e) = state
            .quota_manager
            .record_usage(&user_id, usage.input_tokens, usage.output_tokens)
        {
            tracing::error!(error = %e, "Failed to record usage");
        }
    }

    // Reconstruct the response
    Ok(Response::from_parts(parts, Body::from(body_bytes)))
}

/// Extract token usage from response body.
fn extract_usage(body: &[u8]) -> Option<TokenUsage> {
    // Try to parse as JSON and extract usage
    let response: LlmResponse = serde_json::from_slice(body).ok()?;
    response.usage
}

// ============================================================================
// Usage Report Types
// ============================================================================

/// Usage report for a user.
#[derive(Debug, Clone, Serialize)]
pub struct UsageReport {
    pub user_id: String,
    pub period: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub requests: i64,
    pub limit_input: i64,
    pub limit_output: i64,
    pub percentage_used: f64,
}

impl MeteringState {
    /// Get usage report for a user.
    pub fn get_usage_report(&self, user_id: &str) -> anyhow::Result<UsageReport> {
        let usage = self.quota_manager.get_daily_usage(user_id)?;
        let limits = self.quota_manager.get_limits(user_id)?;

        let total = usage.input_tokens + usage.output_tokens;
        let limit_total = limits.daily_input_tokens + limits.daily_output_tokens;
        let percentage = if limit_total > 0 {
            (total as f64 / limit_total as f64) * 100.0
        } else {
            0.0
        };

        Ok(UsageReport {
            user_id: user_id.to_string(),
            period: "daily".to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: total,
            requests: usage.requests,
            limit_input: limits.daily_input_tokens,
            limit_output: limits.daily_output_tokens,
            percentage_used: percentage,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_usage_anthropic_format() {
        let body = r#"{
            "content": [{"type": "text", "text": "Hello"}],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let usage = extract_usage(body.as_bytes()).unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
    }

    #[test]
    fn test_extract_usage_openai_format() {
        let body = r#"{
            "choices": [{"message": {"content": "Hello"}}],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 25,
                "total_tokens": 40
            }
        }"#;

        let usage = extract_usage(body.as_bytes()).unwrap();
        assert_eq!(usage.input_tokens, 15);
        assert_eq!(usage.output_tokens, 25);
        assert_eq!(usage.total_tokens, 40);
    }

    #[test]
    fn test_extract_usage_no_usage() {
        let body = r#"{"message": "Hello"}"#;
        assert!(extract_usage(body.as_bytes()).is_none());
    }

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            total_tokens: 0,
            model: None,
        };
        assert_eq!(usage.total(), 30);

        let usage_with_total = TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            total_tokens: 35, // Pre-calculated
            model: None,
        };
        assert_eq!(usage_with_total.total(), 35);
    }
}
