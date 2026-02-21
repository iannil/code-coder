//! Zero Gateway - Main entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Initialize logging
    init_logging(&config.observability.log_level, &config.observability.log_format);

    tracing::info!("Zero Gateway v{}", env!("CARGO_PKG_VERSION"));

    // Start the gateway server
    zero_gateway::start_server(&config).await
}
