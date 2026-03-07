//! zero-api: HTTP/WebSocket API service for CodeCoder
//!
//! This service exposes zero-core functionality via REST API and WebSocket:
//! - `/api/v1/session` - Session management
//! - `/api/v1/tools` - Tool execution
//! - `/api/v1/mcp` - MCP protocol endpoints
//! - `/ws` - WebSocket for real-time streaming

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;
mod state;

use state::AppState;

/// Default port for the API server
/// Port 4435 is part of the Rust microservices range (4430-4439)
/// - 4430: zero-gateway
/// - 4431: zero-channels
/// - 4432: zero-workflow
/// - 4433: zero-browser
/// - 4434: zero-trading
/// - 4435: zero-api
pub const DEFAULT_PORT: u16 = 4435;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Get port from environment or use default
    let port = std::env::var("ZERO_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    // Create app state
    let state = Arc::new(AppState::new()?);

    // Build router
    let app = create_router(state);

    // Create listener
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("zero-api server starting on {}", addr);
    tracing::info!("API endpoints:");
    tracing::info!("  - GET  /health");
    tracing::info!("  - POST /api/v1/tools/:tool");
    tracing::info!("  - GET  /api/v1/session");
    tracing::info!("  - POST /api/v1/session");
    tracing::info!("  - GET  /ws (WebSocket)");

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the main router with all routes
fn create_router(state: Arc<AppState>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(routes::health::health_check))
        // Tools API
        .route("/api/v1/tools/:tool", post(routes::tools::execute_tool))
        .route("/api/v1/tools", get(routes::tools::list_tools))
        // Session API
        .route("/api/v1/session", get(routes::session::get_session))
        .route("/api/v1/session", post(routes::session::create_session))
        .route("/api/v1/session/:id", get(routes::session::get_session_by_id))
        .route("/api/v1/session/:id/messages", get(routes::session::get_messages))
        .route("/api/v1/session/:id/messages", post(routes::session::add_message))
        // MCP API
        .route("/api/v1/mcp/tools", get(routes::mcp::list_tools))
        .route("/api/v1/mcp/call", post(routes::mcp::call_tool))
        // WebSocket
        .route("/ws", get(routes::ws::ws_handler))
        // Layers
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let state = Arc::new(AppState::new().unwrap());
        let _router = create_router(state);
        // Router should be created without panicking
    }
}
