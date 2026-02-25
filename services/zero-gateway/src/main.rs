//! Zero Gateway - Main entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Start timing immediately for cold-start measurement
    let startup_start = std::time::Instant::now();

    // Load configuration
    let config = Config::load()?;

    // Initialize logging
    init_logging(&config.observability.log_level, &config.observability.log_format);

    tracing::info!("Zero Gateway v{}", env!("CARGO_PKG_VERSION"));

    // Log startup timing before entering main server loop
    let startup_duration = startup_start.elapsed();
    tracing::info!(
        duration_ms = startup_duration.as_millis() as u64,
        "Service initialized in {:?}",
        startup_duration
    );

    // Start the gateway server
    zero_gateway::start_server(&config).await
}
