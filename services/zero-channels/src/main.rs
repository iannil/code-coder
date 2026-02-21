//! Zero Channels - Main entry point.

use anyhow::Result;
use zero_channels::start_server;
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Initialize logging
    init_logging(
        &config.observability.log_level,
        &config.observability.log_format,
    );

    tracing::info!("Zero Channels v{}", env!("CARGO_PKG_VERSION"));

    // Start the HTTP server
    start_server(&config).await
}
