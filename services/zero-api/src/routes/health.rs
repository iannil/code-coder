//! Health check endpoint

use axum::Json;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Server start time for uptime calculation
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

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
        uptime_secs: START_TIME.elapsed().as_secs(),
    })
}
