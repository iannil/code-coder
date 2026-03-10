//! Zero Server - Unified HTTP/WebSocket API server
//!
//! This module provides HTTP/WebSocket endpoints for CodeCoder:
//! - `/api/v1/session` - Session management
//! - `/api/v1/tools` - Tool execution
//! - `/api/v1/mcp` - MCP protocol endpoints
//! - `/ws` - WebSocket for real-time streaming
//!
//! Originally from `zero-server` crate, now merged into `zero-cli`.

pub mod api;

pub use api::{create_router, AppState, DEFAULT_PORT};
pub use api::routes::{health, mcp, session, tools, ws};

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use zero_core::common::config::Config;

/// Default ports for each service (legacy, for backward compatibility)
/// Note: Unified mode uses DAEMON_PORT (4402) with path prefixes
pub const DEFAULT_GATEWAY_PORT: u16 = 4430;
pub const DEFAULT_CHANNELS_PORT: u16 = 4431;
pub const DEFAULT_WORKFLOW_PORT: u16 = 4432;
pub const DEFAULT_API_PORT: u16 = 4435;

/// Unified daemon port (preferred)
pub const DAEMON_PORT: u16 = 4402;

/// Start the unified Zero Server
pub async fn start_server(config: &Config, unified: bool) -> Result<()> {
    if unified {
        start_unified(config).await
    } else {
        start_multi_port(config).await
    }
}

/// Start all services on separate ports (backward compatible mode).
async fn start_multi_port(config: &Config) -> Result<()> {
    let bind_addr: std::net::IpAddr = config.bind_address().parse()?;

    // Create handles for each service
    let gateway_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = zero_hub::gateway::start_server(&config).await {
                tracing::error!(service = "gateway", error = %e, "Gateway service failed");
            }
        })
    };

    let channels_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = zero_hub::channels::start_server(&config).await {
                tracing::error!(service = "channels", error = %e, "Channels service failed");
            }
        })
    };

    let workflow_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            let service = zero_hub::workflow::WorkflowService::new(config);
            if let Err(e) = service.start().await {
                tracing::error!(service = "workflow", error = %e, "Workflow service failed");
            }
        })
    };

    let api_handle = {
        let bind_addr = bind_addr;
        tokio::spawn(async move {
            if let Err(e) = start_api_server(bind_addr).await {
                tracing::error!(service = "api", error = %e, "API service failed");
            }
        })
    };

    tracing::info!(
        gateway_port = config.gateway_port(),
        channels_port = config.channels_port(),
        workflow_port = config.workflow_port(),
        api_port = DEFAULT_API_PORT,
        "All services started"
    );

    // Wait for shutdown signal
    shutdown_signal().await;

    tracing::info!("Shutting down Zero Server...");

    gateway_handle.abort();
    channels_handle.abort();
    workflow_handle.abort();
    api_handle.abort();

    Ok(())
}

/// Start all services on a single port with path prefixes.
async fn start_unified(config: &Config) -> Result<()> {
    let bind_addr: std::net::IpAddr = config.bind_address().parse()?;
    let port = config.gateway_port();
    let addr = SocketAddr::from((bind_addr, port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create API state
    let api_state = Arc::new(AppState::new()?);

    // Build unified router with path prefixes
    let router = Router::new()
        .nest("/gateway", zero_hub::gateway::build_router(config))
        .nest("/channels", build_channels_router(config))
        .nest("/workflow", build_workflow_router(config))
        .nest("/api", create_router(api_state))
        .layer(cors);

    tracing::info!(
        port = port,
        "Starting unified Zero Server on single port"
    );
    tracing::info!("Routes:");
    tracing::info!("  /gateway/* → Authentication, routing, quotas");
    tracing::info!("  /channels/* → IM channel adapters");
    tracing::info!("  /workflow/* → Webhooks, cron, workflows");
    tracing::info!("  /api/* → HTTP/WebSocket API");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Build channels router without starting background tasks.
fn build_channels_router(config: &Config) -> Router {
    let (router, _rx, _outbound, _email, _telegram, _tx) =
        zero_hub::channels::build_channels_router(config);
    router
}

/// Build workflow router without starting background tasks.
fn build_workflow_router(config: &Config) -> Router {
    let service = zero_hub::workflow::WorkflowService::new(config.clone());
    service.build_router()
}

/// Start the standalone API server.
async fn start_api_server(bind_addr: std::net::IpAddr) -> Result<()> {
    let port = std::env::var("ZERO_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_API_PORT);

    let addr = SocketAddr::from((bind_addr, port));
    let state = Arc::new(AppState::new()?);
    let router = create_router(state);

    tracing::info!(port = port, "Starting Zero API server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ports() {
        assert_eq!(DEFAULT_GATEWAY_PORT, 4430);
        assert_eq!(DEFAULT_CHANNELS_PORT, 4431);
        assert_eq!(DEFAULT_WORKFLOW_PORT, 4432);
        assert_eq!(DEFAULT_API_PORT, 4435);
    }
}
