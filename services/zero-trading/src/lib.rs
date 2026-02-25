//! Zero Trading Library
//!
//! This library provides automated trading capabilities using the PO3 (Power of 3)
//! plus SMT (Smart Money Technique) divergence strategy, adapted for A-shares T+1 rules.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    zero-trading (Rust Service)                      │
//! │                           :4434                                      │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
//! │  │  Market Data    │  │  Strategy       │  │  Execution      │     │
//! │  │  Aggregator     │  │  Engine         │  │  Engine         │     │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Concepts
//!
//! ## PO3 (Power of 3)
//! - **Accumulation**: Sideways consolidation range
//! - **Manipulation**: False breakout to trap traders
//! - **Distribution**: True reversal toward the midpoint
//!
//! ## SMT Divergence
//! - Compare correlated pairs (e.g., CSI 300 vs CSI 500)
//! - Detect when one makes a new high/low but the other doesn't follow
//! - Strong reversal signal when combined with PO3
//!
//! ## T+1 Adaptation (A-shares)
//! - Cannot sell on the same day as purchase
//! - Entry decisions on Day 1, exit evaluation on Day 2
//! - Key time windows: 9:25 auction, 9:30-10:00 first hour

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod backtest;
pub mod broker;
pub mod data;
pub mod execution;
pub mod macro_agent;
pub mod macro_filter;
pub mod notification;
pub mod paper_trading;
pub mod routes;
pub mod strategy;

use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use zero_common::config::Config;

use crate::data::MarketDataAggregator;
use crate::execution::ExecutionEngine;
use crate::macro_agent::{MacroOrchestrator, MacroReportGenerator};
use crate::macro_filter::MacroFilter;
use crate::notification::NotificationClient;
use crate::paper_trading::PaperTradingManager;
use crate::strategy::StrategyEngine;

/// Trading service state
pub struct TradingState {
    /// Configuration
    pub config: Config,
    /// Market data aggregator
    pub data: Arc<MarketDataAggregator>,
    /// Strategy engine
    pub strategy: Arc<StrategyEngine>,
    /// Execution engine
    pub execution: Arc<RwLock<ExecutionEngine>>,
    /// Macro filter (rule engine)
    pub macro_filter: Arc<MacroFilter>,
    /// Macro orchestrator (hybrid mode coordinator)
    pub macro_orchestrator: Arc<MacroOrchestrator>,
    /// Notification client
    pub notification: Arc<NotificationClient>,
    /// Report generator
    pub report_generator: Arc<MacroReportGenerator>,
    /// Paper trading manager
    pub paper_manager: Arc<PaperTradingManager>,
}

impl TradingState {
    /// Create a new trading state
    pub fn new(config: Config) -> Self {
        let data = Arc::new(MarketDataAggregator::new(&config));
        let strategy = Arc::new(StrategyEngine::new(&config));
        let execution = Arc::new(RwLock::new(ExecutionEngine::new(&config)));
        let macro_filter = Arc::new(MacroFilter::new(&config));
        let notification = Arc::new(NotificationClient::new(&config));

        // Create macro orchestrator (coordinates rule engine + agent)
        let macro_orchestrator = Arc::new(macro_agent::create_orchestrator(
            &config,
            Arc::clone(&macro_filter),
        ));

        // Create report generator for scheduled macro reports
        let report_generator = Arc::new(macro_agent::create_report_generator(
            &config,
            Arc::clone(&notification),
        ));

        // Create paper trading manager
        let paper_manager = Arc::new(PaperTradingManager::new(&config));

        Self {
            config,
            data,
            strategy,
            execution,
            macro_filter,
            macro_orchestrator,
            notification,
            report_generator,
            paper_manager,
        }
    }
}

/// Main trading service
pub struct TradingService {
    state: Arc<TradingState>,
}

impl TradingService {
    /// Create a new trading service
    pub fn new(config: Config) -> Self {
        let state = Arc::new(TradingState::new(config));
        Self { state }
    }

    /// Start the trading service
    pub async fn start(self) -> Result<()> {
        let port = self
            .state
            .config
            .trading
            .as_ref()
            .map(|t| t.port)
            .unwrap_or(4434);

        let host = self
            .state
            .config
            .trading
            .as_ref()
            .map(|t| t.host.as_str())
            .unwrap_or("127.0.0.1");

        // Build HTTP routes
        let app = Router::new()
            .route("/health", get(routes::health))
            .route("/api/v1/signals", get(routes::get_signals))
            .route("/api/v1/positions", get(routes::get_positions))
            .route("/api/v1/status", get(routes::get_status))
            // Macro agent routes
            .route("/api/v1/macro/decision", get(routes::get_macro_decision))
            .route("/api/v1/macro/analyze", axum::routing::post(routes::force_macro_analysis))
            .route("/api/v1/macro/report", get(routes::generate_macro_report))
            .route("/api/v1/macro/report/send", axum::routing::post(routes::send_macro_report))
            .route("/api/v1/macro/status", get(routes::check_agent_status))
            // Paper trading routes
            .route("/api/v1/paper/start", axum::routing::post(routes::paper_start))
            .route("/api/v1/paper/stop", axum::routing::post(routes::paper_stop))
            .route("/api/v1/paper/status", get(routes::paper_status))
            .route("/api/v1/paper/trades", get(routes::paper_trades))
            .route("/api/v1/paper/report", get(routes::paper_report))
            .with_state(self.state.clone());

        // Start the market data updater
        let data_state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = data_state.data.start_updater().await {
                tracing::error!(error = %e, "Market data updater failed");
            }
        });

        // Start the strategy scanner
        let strategy_state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = run_strategy_scanner(strategy_state).await {
                tracing::error!(error = %e, "Strategy scanner failed");
            }
        });

        // Start the macro report scheduler (weekly/monthly reports)
        let report_state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = report_state.report_generator.start().await {
                tracing::error!(error = %e, "Macro report scheduler failed");
            }
        });

        // Start HTTP server
        let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
        tracing::info!(address = %addr, "Starting HTTP server");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Run the strategy scanner in a loop
async fn run_strategy_scanner(state: Arc<TradingState>) -> Result<()> {
    use chrono::{Local, Timelike};

    loop {
        let now = Local::now();
        let hour = now.hour();
        let minute = now.minute();

        // Only run during A-share trading hours (9:15-15:00 Beijing time)
        let is_trading_hours = (hour == 9 && minute >= 15)
            || (hour >= 10 && hour < 15)
            || (hour == 15 && minute == 0);

        if is_trading_hours {
            // Check macro conditions using the orchestrator (hybrid mode)
            let trading_recommended = state.macro_orchestrator.is_trading_recommended().await;

            if !trading_recommended {
                tracing::info!("Macro orchestrator recommends avoiding trading, skipping scan");
            } else {
                // Scan for signals
                match state.strategy.scan_for_signals(&state.data).await {
                    Ok(signals) => {
                        // Send notifications for new signals
                        for signal in &signals {
                            if let Err(e) = state.notification.send_signal(signal).await {
                                tracing::warn!(
                                    signal_id = %signal.id,
                                    error = %e,
                                    "Failed to send signal notification"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Signal scan failed");
                    }
                }
            }
        } else {
            tracing::debug!(hour, minute, "Outside trading hours, skipping scan");
        }

        // Sleep for 1 minute before next scan
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
