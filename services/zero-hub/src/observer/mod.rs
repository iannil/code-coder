//! Zero Observer - High-performance observation infrastructure for the Observer Network.
//!
//! This module provides the Rust-based foundation for the Observer Network,
//! handling high-throughput, deterministic operations:
//!
//! - **Event Streaming**: High-performance event buffering, routing, and aggregation
//! - **Three Dials Control**: Deterministic threshold-based control (observe/decide/act)
//! - **Gear Presets**: State machine for preset modes (P/N/D/S/M)
//! - **Scheduling**: Observation task scheduling (integrates with workflow scheduler)
//! - **Health Checks**: System health monitoring with rule-based evaluation
//! - **Storage**: Event persistence for history and replay
//!
//! ## Architecture
//!
//! The Observer module follows the project principle: high-deterministic tasks use Rust,
//! while high-uncertainty tasks (pattern recognition, anomaly interpretation) use TypeScript/LLM.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      TypeScript Layer (packages/ccode)                       │
//! │                        High Uncertainty - LLM Required                       │
//! │  • Pattern Detection • Anomaly Interpretation • Decision Generation         │
//! └──────────────────────────────────┬──────────────────────────────────────────┘
//!                                    │ HTTP/gRPC API
//!                                    ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Rust Layer (services/zero-hub/observer)                 │
//! │                        High Determinism - Performance Critical               │
//! │  • Event Stream Buffering • Three Dials Logic • Gear State Machine          │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zero_hub::observer::{Dial, ThreeDials, Gear, ObserverStream};
//!
//! // Create three dials with preset gear
//! let dials = ThreeDials::from_gear(Gear::D);
//! assert_eq!(dials.observe.value, 70);
//! assert_eq!(dials.decide.value, 60);
//! assert_eq!(dials.act.value, 40);
//!
//! // Check action thresholds
//! if dials.should_observe() { /* start observation */ }
//! if dials.should_decide_autonomously() { /* make decision */ }
//! if dials.should_act_immediately() { /* execute action */ }
//! ```

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod dial;
pub mod gear;
pub mod stream;
pub mod health;
pub mod storage;

// Re-export primary types
pub use dial::{Dial, DialMode, ThreeDials};
pub use gear::{Gear, GearTransition};
pub use stream::{ObserverEvent, ObserverStream, StreamConfig, StreamStats};
pub use health::{HealthCheck, HealthStatus, HealthReport};
pub use storage::{ObserverStorage, StoredEvent};

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Observer module state shared across routes.
#[derive(Clone)]
pub struct ObserverState {
    /// Current three-dial configuration
    pub dials: Arc<RwLock<ThreeDials>>,
    /// Current gear preset
    pub gear: Arc<RwLock<Gear>>,
    /// Event stream
    pub stream: Arc<RwLock<ObserverStream>>,
    /// Health checker
    pub health: Arc<HealthCheck>,
    /// Event storage
    pub storage: Arc<ObserverStorage>,
}

impl ObserverState {
    /// Create a new observer state with default configuration.
    pub fn new() -> Self {
        let gear = Gear::D; // Default to Drive mode
        let dials = ThreeDials::from_gear(gear);

        Self {
            dials: Arc::new(RwLock::new(dials)),
            gear: Arc::new(RwLock::new(gear)),
            stream: Arc::new(RwLock::new(ObserverStream::new(StreamConfig::default()))),
            health: Arc::new(HealthCheck::new()),
            storage: Arc::new(ObserverStorage::new()),
        }
    }

    /// Create a new observer state with custom gear.
    pub fn with_gear(gear: Gear) -> Self {
        let dials = ThreeDials::from_gear(gear);

        Self {
            dials: Arc::new(RwLock::new(dials)),
            gear: Arc::new(RwLock::new(gear)),
            stream: Arc::new(RwLock::new(ObserverStream::new(StreamConfig::default()))),
            health: Arc::new(HealthCheck::new()),
            storage: Arc::new(ObserverStorage::new()),
        }
    }
}

impl Default for ObserverState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// API Types
// ============================================================================

/// Request to set a specific gear.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetGearRequest {
    /// Gear preset (P, N, D, S, M)
    pub gear: String,
    /// Custom dial values for Manual (M) mode
    pub custom_dials: Option<DialValues>,
}

/// Dial values for custom configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialValues {
    pub observe: u8,
    pub decide: u8,
    pub act: u8,
}

/// Request to set a single dial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDialRequest {
    /// Dial name: "observe", "decide", or "act"
    pub dial: String,
    /// Value 0-100
    pub value: u8,
}

/// Observer state response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverStateResponse {
    pub gear: String,
    pub dials: DialValues,
    pub health: HealthReport,
    pub stream_stats: StreamStats,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

// ============================================================================
// Routes
// ============================================================================

/// Build the observer router with all routes.
pub fn build_router(state: ObserverState) -> Router {
    Router::new()
        .route("/state", get(get_state))
        .route("/gear", post(set_gear))
        .route("/dial", post(set_dial))
        .route("/health", get(get_health))
        .route("/stream/stats", get(get_stream_stats))
        .with_state(state)
}

/// Get current observer state.
async fn get_state(
    State(state): State<ObserverState>,
) -> Json<ApiResponse<ObserverStateResponse>> {
    let dials = state.dials.read().await;
    let gear = state.gear.read().await;
    let stream = state.stream.read().await;

    let response = ObserverStateResponse {
        gear: gear.to_string(),
        dials: DialValues {
            observe: dials.observe.value,
            decide: dials.decide.value,
            act: dials.act.value,
        },
        health: state.health.report(),
        stream_stats: stream.stats(),
    };

    Json(ApiResponse::ok(response))
}

/// Set gear preset.
async fn set_gear(
    State(state): State<ObserverState>,
    Json(request): Json<SetGearRequest>,
) -> Json<ApiResponse<DialValues>> {
    let new_gear = match request.gear.to_uppercase().as_str() {
        "P" | "PARK" => Gear::P,
        "N" | "NEUTRAL" => Gear::N,
        "D" | "DRIVE" => Gear::D,
        "S" | "SPORT" => Gear::S,
        "M" | "MANUAL" => Gear::M,
        _ => return Json(ApiResponse::err(format!("Invalid gear: {}", request.gear))),
    };

    let new_dials = if new_gear == Gear::M {
        // Manual mode requires custom dial values
        match request.custom_dials {
            Some(custom) => ThreeDials::custom(custom.observe, custom.decide, custom.act),
            None => ThreeDials::from_gear(Gear::D), // Default to D values if not specified
        }
    } else {
        ThreeDials::from_gear(new_gear)
    };

    // Update state
    {
        let mut gear = state.gear.write().await;
        *gear = new_gear;
    }
    {
        let mut dials = state.dials.write().await;
        *dials = new_dials.clone();
    }

    tracing::info!(
        gear = %new_gear,
        observe = new_dials.observe.value,
        decide = new_dials.decide.value,
        act = new_dials.act.value,
        "Gear changed"
    );

    Json(ApiResponse::ok(DialValues {
        observe: new_dials.observe.value,
        decide: new_dials.decide.value,
        act: new_dials.act.value,
    }))
}

/// Set individual dial value.
async fn set_dial(
    State(state): State<ObserverState>,
    Json(request): Json<SetDialRequest>,
) -> Json<ApiResponse<DialValues>> {
    if request.value > 100 {
        return Json(ApiResponse::err("Dial value must be 0-100"));
    }

    let mut dials = state.dials.write().await;

    match request.dial.to_lowercase().as_str() {
        "observe" => dials.observe.value = request.value,
        "decide" => dials.decide.value = request.value,
        "act" => dials.act.value = request.value,
        _ => return Json(ApiResponse::err(format!("Invalid dial: {}", request.dial))),
    }

    // Switch to Manual gear when individual dial is adjusted
    {
        let mut gear = state.gear.write().await;
        *gear = Gear::M;
    }

    tracing::info!(
        dial = %request.dial,
        value = request.value,
        "Dial adjusted (switched to Manual mode)"
    );

    Json(ApiResponse::ok(DialValues {
        observe: dials.observe.value,
        decide: dials.decide.value,
        act: dials.act.value,
    }))
}

/// Get health status.
async fn get_health(State(state): State<ObserverState>) -> Json<ApiResponse<HealthReport>> {
    Json(ApiResponse::ok(state.health.report()))
}

/// Get stream statistics.
async fn get_stream_stats(State(state): State<ObserverState>) -> Json<ApiResponse<StreamStats>> {
    let stream = state.stream.read().await;
    Json(ApiResponse::ok(stream.stats()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_state_default() {
        let state = ObserverState::new();

        // Default gear is D (Drive)
        let gear = state.gear.try_read().unwrap();
        assert_eq!(*gear, Gear::D);

        // Default dials for D gear
        let dials = state.dials.try_read().unwrap();
        assert_eq!(dials.observe.value, 70);
        assert_eq!(dials.decide.value, 60);
        assert_eq!(dials.act.value, 40);
    }

    #[test]
    fn test_observer_state_with_gear() {
        let state = ObserverState::with_gear(Gear::S);

        let gear = state.gear.try_read().unwrap();
        assert_eq!(*gear, Gear::S);

        let dials = state.dials.try_read().unwrap();
        assert_eq!(dials.observe.value, 90);
        assert_eq!(dials.decide.value, 80);
        assert_eq!(dials.act.value, 70);
    }
}
