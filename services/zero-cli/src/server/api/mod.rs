//! zero-api: HTTP/WebSocket API service for CodeCoder
//!
//! This module exposes zero-core functionality via REST API and WebSocket:
//! - `/api/v1/session` - Session management
//! - `/api/v1/tools` - Tool execution
//! - `/api/v1/mcp` - MCP protocol endpoints
//! - `/ws` - WebSocket for real-time streaming

pub mod routes;
pub mod state;

pub use routes::{health, mcp, session, tools, ws};
pub use state::AppState;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

/// Default port for the API server
/// Port 4435 is part of the Rust microservices range (4430-4439)
pub const DEFAULT_PORT: u16 = 4435;

/// Create the main router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
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
