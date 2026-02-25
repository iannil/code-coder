//! zero-browser service entry point.

use anyhow::Result;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use zero_browser::{build_router, AppState};
use zero_common::config::Config;
use zero_common::logging::init_logging;

/// Default port for zero-browser service.
const DEFAULT_PORT: u16 = 4433;

/// Default session timeout in seconds (30 minutes).
const DEFAULT_SESSION_TIMEOUT_SECS: i64 = 1800;

#[tokio::main]
async fn main() -> Result<()> {
    let startup_start = std::time::Instant::now();

    let config = Config::load()?;
    init_logging(
        &config.observability.log_level,
        &config.observability.log_format,
    );

    tracing::info!("Zero Browser v{}", env!("CARGO_PKG_VERSION"));

    // Create application state
    let state = AppState::new(DEFAULT_SESSION_TIMEOUT_SECS);

    // Build router with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = build_router(state).layer(cors);

    // Bind to localhost on default port
    let addr = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT));

    let startup_duration = startup_start.elapsed();
    tracing::info!(
        duration_ms = startup_duration.as_millis() as u64,
        "Service initialized in {:?}",
        startup_duration
    );

    tracing::info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
