//! Observer API Routes
//!
//! REST and SSE endpoints for the Observer Network.
//!
//! # Endpoints
//!
//! - `POST /api/v1/observer/start` - Start the observer network
//! - `POST /api/v1/observer/stop` - Stop the observer network
//! - `GET  /api/v1/observer/status` - Get network status
//! - `GET  /api/v1/observer/events` - SSE event stream
//! - `GET  /api/v1/observer/world-model` - Get current world model
//! - `GET  /api/v1/observer/consensus` - Get consensus snapshot
//! - `GET  /api/v1/observer/patterns` - Get active patterns
//! - `GET  /api/v1/observer/anomalies` - Get active anomalies
//! - `GET  /api/v1/observer/opportunities` - Get active opportunities
//! - `POST /api/v1/observer/ingest` - Ingest observation events
//! - `GET  /api/v1/observer/watchers` - List all watchers
//! - `GET  /api/v1/observer/watchers/:id` - Get watcher details
//! - `POST /api/v1/observer/watchers/:id/start` - Start a watcher
//! - `POST /api/v1/observer/watchers/:id/stop` - Stop a watcher

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as TokioStreamExt;

use crate::observer::{
    network::ObserverNetworkEvent, Anomaly, ConsensusSnapshot, EmergentPattern, Observation,
    Opportunity, WatcherMetrics, WatcherStatus, WorldModel,
};
use crate::unified_api::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Response Types
// ══════════════════════════════════════════════════════════════════════════════

/// Standard API response
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

/// Observer network status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserverStatus {
    pub running: bool,
    pub enabled: bool,
    pub stream_stats: StreamStatsResponse,
    pub consensus_confidence: f32,
    pub active_patterns: usize,
    pub active_anomalies: usize,
    pub active_opportunities: usize,
    pub has_world_model: bool,
}

/// Stream statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatsResponse {
    pub received: u64,
    pub processed: u64,
    pub dropped: u64,
    pub buffer_size: usize,
}

/// Ingest request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub observations: Vec<Observation>,
}

/// Ingest response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResponse {
    pub ingested: usize,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// Start the observer network
pub async fn start_observer(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<ObserverStatus>> {
    if let Some(ref observer) = state.observer {
        // Check if already running
        if observer.is_running().await {
            return Json(ApiResponse::err("Observer network is already running"));
        }

        // Start via the network
        // Note: We need to access the network through state somehow
        // For now, set running flag directly
        *observer.running.write().await = true;
        observer.consensus.start().await;

        let _ = observer.event_tx.send(ObserverNetworkEvent::Started);

        let status = get_observer_status_internal(&state).await;
        Json(ApiResponse::ok(status))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Stop the observer network
pub async fn stop_observer(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<ObserverStatus>> {
    if let Some(ref observer) = state.observer {
        if !observer.is_running().await {
            return Json(ApiResponse::err("Observer network is not running"));
        }

        *observer.running.write().await = false;
        observer.consensus.stop().await;

        let _ = observer.event_tx.send(ObserverNetworkEvent::Stopped);

        let status = get_observer_status_internal(&state).await;
        Json(ApiResponse::ok(status))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Get observer network status
pub async fn get_status(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<ObserverStatus>> {
    if state.observer.is_some() {
        let status = get_observer_status_internal(&state).await;
        Json(ApiResponse::ok(status))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// SSE event stream
pub async fn stream_events(
    State(state): State<Arc<UnifiedApiState>>,
) -> Sse<std::pin::Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>> {
    if let Some(ref observer) = state.observer {
        let rx = observer.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|result| {
            result.ok().map(|event| {
                let event_type = match &event {
                    ObserverNetworkEvent::Started => "started",
                    ObserverNetworkEvent::Stopped => "stopped",
                    ObserverNetworkEvent::ObservationReceived { .. } => "observation_received",
                    ObserverNetworkEvent::ConsensusUpdated { .. } => "consensus_updated",
                    ObserverNetworkEvent::GearSwitchRecommended { .. } => "gear_switch_recommended",
                    ObserverNetworkEvent::WorldModelUpdated { .. } => "world_model_updated",
                    ObserverNetworkEvent::WatcherStarted { .. } => "watcher_started",
                    ObserverNetworkEvent::WatcherStopped { .. } => "watcher_stopped",
                };

                Ok::<_, Infallible>(
                    Event::default()
                        .event(event_type)
                        .data(serde_json::to_string(&event).unwrap_or_default()),
                )
            })
        });

        Sse::new(Box::pin(stream) as std::pin::Pin<Box<dyn futures::Stream<Item = _> + Send>>)
            .keep_alive(KeepAlive::default())
    } else {
        // Return empty stream if observer not configured
        let stream = futures::stream::empty::<Result<Event, Infallible>>();
        Sse::new(Box::pin(stream) as std::pin::Pin<Box<dyn futures::Stream<Item = _> + Send>>)
            .keep_alive(KeepAlive::default())
    }
}

/// Get current world model
pub async fn get_world_model(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<Option<WorldModel>>> {
    if let Some(ref observer) = state.observer {
        let world_model = observer.consensus.get_world_model().await;
        Json(ApiResponse::ok(world_model))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Get consensus snapshot
pub async fn get_consensus(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<ConsensusSnapshot>> {
    if let Some(ref observer) = state.observer {
        let snapshot = observer.consensus.get_snapshot().await;
        Json(ApiResponse::ok(snapshot))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Get active patterns
pub async fn get_patterns(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<Vec<EmergentPattern>>> {
    if let Some(ref observer) = state.observer {
        let patterns = observer.consensus.get_patterns().await;
        Json(ApiResponse::ok(patterns))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Get active anomalies
pub async fn get_anomalies(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<Vec<Anomaly>>> {
    if let Some(ref observer) = state.observer {
        let anomalies = observer.consensus.get_anomalies().await;
        Json(ApiResponse::ok(anomalies))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Get active opportunities
pub async fn get_opportunities(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<Vec<Opportunity>>> {
    if let Some(ref observer) = state.observer {
        let opportunities = observer.consensus.get_opportunities().await;
        Json(ApiResponse::ok(opportunities))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

/// Ingest observations
pub async fn ingest_observations(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<IngestRequest>,
) -> Json<ApiResponse<IngestResponse>> {
    if let Some(ref observer) = state.observer {
        let count = request.observations.len();
        observer.consensus.add_observations(request.observations).await;

        Json(ApiResponse::ok(IngestResponse { ingested: count }))
    } else {
        Json(ApiResponse::err("Observer network not configured"))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal Helpers
// ══════════════════════════════════════════════════════════════════════════════

async fn get_observer_status_internal(state: &UnifiedApiState) -> ObserverStatus {
    if let Some(ref observer) = state.observer {
        let running = observer.is_running().await;
        let stream_stats = observer.stream.read().await.stats();
        let snapshot = observer.consensus.get_snapshot().await;

        ObserverStatus {
            running,
            enabled: true,
            stream_stats: StreamStatsResponse {
                received: stream_stats.received,
                processed: stream_stats.processed,
                dropped: stream_stats.dropped,
                buffer_size: stream_stats.buffer_size,
            },
            consensus_confidence: snapshot.confidence,
            active_patterns: snapshot.patterns.len(),
            active_anomalies: snapshot.anomalies.len(),
            active_opportunities: snapshot.opportunities.len(),
            has_world_model: snapshot.world_model.is_some(),
        }
    } else {
        ObserverStatus {
            running: false,
            enabled: false,
            stream_stats: StreamStatsResponse {
                received: 0,
                processed: 0,
                dropped: 0,
                buffer_size: 0,
            },
            consensus_confidence: 0.0,
            active_patterns: 0,
            active_anomalies: 0,
            active_opportunities: 0,
            has_world_model: false,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Endpoints
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherListResponse {
    pub watchers: Vec<WatcherStatus>,
    pub total: usize,
}

/// Watcher detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherDetailResponse {
    pub status: WatcherStatus,
    pub metrics: WatcherMetrics,
}

/// Stop watcher request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopWatcherRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// List all watchers
pub async fn list_watchers(
    State(state): State<Arc<UnifiedApiState>>,
) -> Json<ApiResponse<WatcherListResponse>> {
    if let Some(ref watcher_manager) = state.watcher_manager {
        let watchers = watcher_manager.read().await.get_all_statuses().await;
        let total = watchers.len();
        Json(ApiResponse::ok(WatcherListResponse { watchers, total }))
    } else {
        Json(ApiResponse::ok(WatcherListResponse {
            watchers: Vec::new(),
            total: 0,
        }))
    }
}

/// Get watcher details
pub async fn get_watcher(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<WatcherDetailResponse>> {
    if let Some(ref watcher_manager) = state.watcher_manager {
        let manager = watcher_manager.read().await;
        if let Some(status) = manager.get_watcher(&id).await {
            // Get metrics from the status
            let metrics = WatcherMetrics {
                observation_count: status.observation_count,
                error_count: status.error_count,
                avg_latency_ms: status.avg_latency_ms,
                last_observation: status.last_observation,
                error_rate: if status.observation_count > 0 {
                    status.error_count as f32 / status.observation_count as f32
                } else {
                    0.0
                },
            };
            Json(ApiResponse::ok(WatcherDetailResponse { status, metrics }))
        } else {
            Json(ApiResponse::err(format!("Watcher '{}' not found", id)))
        }
    } else {
        Json(ApiResponse::err("Watcher manager not configured"))
    }
}

/// Start a specific watcher
pub async fn start_watcher(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<WatcherStatus>> {
    if let Some(ref watcher_manager) = state.watcher_manager {
        let manager = watcher_manager.read().await;
        match manager.start_watcher(&id).await {
            Ok(true) => {
                if let Some(status) = manager.get_watcher(&id).await {
                    Json(ApiResponse::ok(status))
                } else {
                    Json(ApiResponse::err("Failed to get watcher status"))
                }
            }
            Ok(false) => Json(ApiResponse::err(format!("Watcher '{}' not found", id))),
            Err(e) => Json(ApiResponse::err(format!("Failed to start watcher: {}", e))),
        }
    } else {
        Json(ApiResponse::err("Watcher manager not configured"))
    }
}

/// Stop a specific watcher
pub async fn stop_watcher(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
    Json(request): Json<Option<StopWatcherRequest>>,
) -> Json<ApiResponse<WatcherStatus>> {
    if let Some(ref watcher_manager) = state.watcher_manager {
        let manager = watcher_manager.read().await;
        let reason = request.and_then(|r| r.reason);
        match manager.stop_watcher(&id, reason.as_deref()).await {
            Ok(true) => {
                if let Some(status) = manager.get_watcher(&id).await {
                    Json(ApiResponse::ok(status))
                } else {
                    Json(ApiResponse::err("Failed to get watcher status"))
                }
            }
            Ok(false) => Json(ApiResponse::err(format!("Watcher '{}' not found", id))),
            Err(e) => Json(ApiResponse::err(format!("Failed to stop watcher: {}", e))),
        }
    } else {
        Json(ApiResponse::err("Watcher manager not configured"))
    }
}

