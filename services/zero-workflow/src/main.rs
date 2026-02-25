//! Zero Workflow - Main entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;
use zero_workflow::WorkflowService;

#[tokio::main]
async fn main() -> Result<()> {
    // Start timing immediately for cold-start measurement
    let startup_start = std::time::Instant::now();

    // Load configuration
    let config = Config::load()?;

    // Initialize logging
    init_logging(
        &config.observability.log_level,
        &config.observability.log_format,
    );

    tracing::info!("Zero Workflow v{}", env!("CARGO_PKG_VERSION"));

    // Start the workflow service
    let service = WorkflowService::new(config);

    // Log startup timing before entering main service loop
    let startup_duration = startup_start.elapsed();
    tracing::info!(
        duration_ms = startup_duration.as_millis() as u64,
        "Service initialized in {:?}",
        startup_duration
    );

    service.start().await
}
