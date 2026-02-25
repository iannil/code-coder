//! zero-browser service entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    init_logging(&config.observability.log_level, &config.observability.log_format);

    tracing::info!("Zero Browser v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Service starting on port 4433");

    // TODO: Start HTTP server
    Ok(())
}
