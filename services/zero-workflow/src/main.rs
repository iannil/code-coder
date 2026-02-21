//! Zero Workflow - Main entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;
use zero_workflow::WorkflowService;

#[tokio::main]
async fn main() -> Result<()> {
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
    service.start().await
}
