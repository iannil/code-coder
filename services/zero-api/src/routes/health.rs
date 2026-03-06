//! Health check endpoint

use axum::Json;
use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

/// Handle GET /health
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0, // TODO: Track actual uptime
    })
}
