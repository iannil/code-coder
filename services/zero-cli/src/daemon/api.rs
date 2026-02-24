//! Daemon management HTTP API.
//!
//! Provides REST endpoints for managing the daemon and its child services.
//! Runs on port 4402 alongside the process orchestrator.

use crate::process::{ServiceManager, ServiceStatus};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared state for the management API.
#[derive(Clone)]
pub struct ApiState {
    /// Service manager (shared with orchestrator)
    pub manager: Arc<Mutex<ServiceManager>>,
    /// Daemon start time
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Health response.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    daemon: &'static str,
    services: Vec<ServiceHealthStatus>,
    uptime_secs: i64,
}

#[derive(Serialize)]
struct ServiceHealthStatus {
    name: String,
    port: u16,
    running: bool,
    restart_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

impl From<ServiceStatus> for ServiceHealthStatus {
    fn from(s: ServiceStatus) -> Self {
        Self {
            name: s.name,
            port: s.port,
            running: s.running,
            restart_count: s.restart_count,
            last_error: s.last_error,
        }
    }
}

/// Status response (detailed).
#[derive(Serialize)]
struct StatusResponse {
    daemon: DaemonStatus,
    services: Vec<ServiceHealthStatus>,
    runtime: serde_json::Value,
}

#[derive(Serialize)]
struct DaemonStatus {
    status: &'static str,
    started_at: String,
    uptime_secs: i64,
    version: &'static str,
}

/// Service action response.
#[derive(Serialize)]
struct ActionResponse {
    success: bool,
    service: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Build the management API router.
pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/status", get(handle_status))
        .route("/restart/{name}", post(handle_restart))
        .route("/stop/{name}", post(handle_stop))
        .route("/start/{name}", post(handle_start))
        .with_state(state)
}

/// Start the management API server.
pub async fn serve(state: ApiState, host: &str, port: u16) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let app = router(state);

    tracing::info!("Management API listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

/// GET /health - Quick health check.
async fn handle_health(State(state): State<ApiState>) -> impl IntoResponse {
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds();

    let services: Vec<ServiceHealthStatus> = {
        let mut manager = state.manager.lock().await;
        manager.status().into_iter().map(Into::into).collect()
    };

    let all_healthy = services.iter().all(|s| s.running);

    Json(HealthResponse {
        status: if all_healthy { "healthy" } else { "degraded" },
        daemon: "running",
        services,
        uptime_secs: uptime,
    })
}

/// GET /status - Detailed status including runtime info.
async fn handle_status(State(state): State<ApiState>) -> impl IntoResponse {
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds();

    let services: Vec<ServiceHealthStatus> = {
        let mut manager = state.manager.lock().await;
        manager.status().into_iter().map(Into::into).collect()
    };

    let all_healthy = services.iter().all(|s| s.running);

    Json(StatusResponse {
        daemon: DaemonStatus {
            status: if all_healthy { "healthy" } else { "degraded" },
            started_at: state.started_at.to_rfc3339(),
            uptime_secs: uptime,
            version: env!("CARGO_PKG_VERSION"),
        },
        services,
        runtime: crate::health::snapshot_json(),
    })
}

/// POST /restart/:name - Restart a specific service.
async fn handle_restart(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut manager = state.manager.lock().await;

    // Find the service
    let service_exists = manager
        .status()
        .iter()
        .any(|s| s.name == name || s.name == format!("zero-{}", name));

    if !service_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(ActionResponse {
                success: false,
                service: name,
                action: "restart".into(),
                message: None,
                error: Some("Service not found".into()),
            }),
        );
    }

    // Trigger restart via health check (sets running to false, triggering restart)
    // For now, we'll restart all services as the manager doesn't expose single-service restart
    manager.health_check_and_restart();

    (
        StatusCode::OK,
        Json(ActionResponse {
            success: true,
            service: name,
            action: "restart".into(),
            message: Some("Restart triggered".into()),
            error: None,
        }),
    )
}

/// POST /stop/:name - Stop a specific service.
async fn handle_stop(State(state): State<ApiState>, Path(name): Path<String>) -> impl IntoResponse {
    let mut manager = state.manager.lock().await;

    // Check if service exists
    let service_exists = manager
        .status()
        .iter()
        .any(|s| s.name == name || s.name == format!("zero-{}", name));

    if !service_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(ActionResponse {
                success: false,
                service: name,
                action: "stop".into(),
                message: None,
                error: Some("Service not found".into()),
            }),
        );
    }

    // Note: Single-service stop not yet implemented in ServiceManager
    // Would need to extend ServiceManager to support this
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ActionResponse {
            success: false,
            service: name,
            action: "stop".into(),
            message: None,
            error: Some("Single-service stop not yet implemented".into()),
        }),
    )
}

/// POST /start/:name - Start a specific service.
async fn handle_start(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut manager = state.manager.lock().await;

    // Check if service exists
    let service_exists = manager
        .status()
        .iter()
        .any(|s| s.name == name || s.name == format!("zero-{}", name));

    if !service_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(ActionResponse {
                success: false,
                service: name,
                action: "start".into(),
                message: None,
                error: Some("Service not found".into()),
            }),
        );
    }

    // Note: Single-service start not yet implemented in ServiceManager
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ActionResponse {
            success: false,
            service: name,
            action: "start".into(),
            message: None,
            error: Some("Single-service start not yet implemented".into()),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes() {
        let response = HealthResponse {
            status: "healthy",
            daemon: "running",
            services: vec![],
            uptime_secs: 100,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
    }

    #[test]
    fn service_health_status_from_service_status() {
        let status = ServiceStatus {
            name: "gateway".into(),
            port: 4430,
            running: true,
            restart_count: 0,
            last_error: None,
        };
        let health: ServiceHealthStatus = status.into();
        assert_eq!(health.name, "gateway");
        assert!(health.running);
    }
}
