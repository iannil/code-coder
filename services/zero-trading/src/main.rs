//! Zero Trading - Automated trading service for the Zero ecosystem.
//!
//! Implements the PO3 (Power of 3) + SMT (Smart Money Technique) divergence
//! strategy for A-shares market with T+1 adaptation.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;
use zero_trading::TradingService;

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

    tracing::info!("Zero Trading v{}", env!("CARGO_PKG_VERSION"));

    // Start the trading service
    let service = TradingService::new(config);

    // Log startup timing before entering main service loop
    let startup_duration = startup_start.elapsed();
    tracing::info!(
        duration_ms = startup_duration.as_millis() as u64,
        "Service initialized in {:?}",
        startup_duration
    );

    service.start().await
}
