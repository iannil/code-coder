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
pub mod circuit_breaker;
pub mod data;
pub mod execution;
pub mod r#loop;
pub mod macro_agent;
pub mod macro_filter;
pub mod notification;
pub mod paper_trading;
pub mod portfolio;
pub mod routes;
pub mod scheduler;
pub mod screener;
pub mod session;
pub mod strategy;
pub mod task_scheduler;
pub mod valuation;
pub mod value;

use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use zero_common::config::Config;

use crate::data::{LixinAdapter, MarketDataAggregator};
use crate::execution::ExecutionEngine;
use crate::macro_agent::{MacroOrchestrator, MacroReportGenerator};
use crate::macro_filter::MacroFilter;
use crate::notification::NotificationClient;
use crate::paper_trading::PaperTradingManager;
use crate::portfolio::{DipAnalyzer, PoolsConfig, PoolsManager, SignalAnalyzer};
use crate::scheduler::TradingScheduler;
use crate::screener::{ScreenerConfig, ScreenerScheduler};
use crate::session::TradingSessionManager;
use crate::strategy::StrategyEngine;
use crate::task_scheduler::TaskScheduler;
use crate::valuation::ValuationAnalyzer;
use crate::value::ValueAnalyzer;

/// Type alias for the concrete screener scheduler (uses LixinAdapter for fundamental data)
pub type ScreenerSchedulerImpl = ScreenerScheduler<LixinAdapter>;

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
    /// Session manager
    pub session_manager: Arc<TradingSessionManager>,
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
    /// Value analyzer (printing machine checklist, cash flow DNA)
    pub value_analyzer: Arc<ValueAnalyzer>,
    /// Valuation analyzer (PE/PB/DY three-dimensional)
    pub valuation_analyzer: Arc<ValuationAnalyzer>,
    /// Portfolio pools manager
    pub portfolio_manager: Arc<PoolsManager>,
    /// Signal analyzer (red/yellow light system)
    pub signal_analyzer: Arc<SignalAnalyzer>,
    /// Dip analyzer (golden pit vs value trap)
    pub dip_analyzer: Arc<DipAnalyzer>,
    /// Screener scheduler (requires Lixin API token)
    pub screener_scheduler: Option<Arc<ScreenerSchedulerImpl>>,
}

impl TradingState {
    /// Create a new trading state
    pub fn new(config: Config) -> Self {
        let data = Arc::new(MarketDataAggregator::new(&config));
        let strategy = Arc::new(StrategyEngine::new(&config));
        let execution = Arc::new(RwLock::new(ExecutionEngine::new(&config)));

        // Share local storage between data aggregator and macro filter
        let local_storage = data.local_storage();
        let macro_filter = Arc::new(MacroFilter::with_local_storage(&config, local_storage.clone()));
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

        // Create session manager with default config
        let db_path = config
            .trading
            .as_ref()
            .and_then(|t| t.local_storage.as_ref())
            .and_then(|ls| ls.db_path.as_ref())
            .map(|p| {
                if p.starts_with("~/") {
                    dirs::home_dir()
                        .map(|h| h.join(&p[2..]))
                        .unwrap_or_else(|| std::path::PathBuf::from(p))
                } else {
                    std::path::PathBuf::from(p)
                }
            })
            .unwrap_or_else(|| dirs::home_dir()
                .map(|h| h.join(".codecoder/financial.db"))
                .unwrap_or_else(|| std::path::PathBuf::from("financial.db")));

        let session_manager = Arc::new(
            TradingSessionManager::new(
                db_path,
                Arc::clone(&data),
                Arc::clone(&strategy),
                Arc::clone(&execution),
            ).expect("Failed to create session manager")
        );

        // Create value analysis components (with shared local storage for caching)
        let value_analyzer = Arc::new(ValueAnalyzer::with_local_storage(&config, local_storage.clone()));
        let valuation_analyzer = Arc::new(ValuationAnalyzer::with_local_storage(local_storage));

        // Create portfolio management components
        let portfolio_config = PoolsConfig::default();
        let portfolio_manager = Arc::new(PoolsManager::new(portfolio_config));
        let signal_analyzer = Arc::new(SignalAnalyzer::default());
        let dip_analyzer = Arc::new(DipAnalyzer::default());

        // Create screener scheduler (requires Lixin API token and local storage for fundamental data)
        let screener_scheduler = config.lixin_token().and_then(|token| {
            let local_storage_for_screener = data.local_storage()?;
            let lixin_provider = Arc::new(LixinAdapter::new(token));

            // Parse screener config from JSON value, or use defaults
            let screener_config: ScreenerConfig = config
                .trading
                .as_ref()
                .and_then(|t| t.screener.as_ref())
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            Some(Arc::new(ScreenerScheduler::new(
                screener_config,
                Arc::new(config.clone()),
                lixin_provider,
                local_storage_for_screener,
            )))
        });

        if screener_scheduler.is_some() {
            tracing::info!("Screener scheduler initialized with Lixin data provider");
        } else {
            tracing::warn!("Screener scheduler not available: Lixin API token not configured");
        }

        Self {
            config,
            data,
            strategy,
            execution,
            session_manager,
            macro_filter,
            macro_orchestrator,
            notification,
            report_generator,
            paper_manager,
            value_analyzer,
            valuation_analyzer,
            portfolio_manager,
            signal_analyzer,
            dip_analyzer,
            screener_scheduler,
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
            // Value analysis routes
            .route("/api/v1/value/analyze", axum::routing::post(routes::value_analyze))
            .route("/api/v1/valuation/analyze", axum::routing::post(routes::valuation_analyze))
            // Portfolio management routes
            .route("/api/v1/portfolio/summary", get(routes::portfolio_summary))
            .route("/api/v1/portfolio/positions", get(routes::portfolio_positions))
            .route("/api/v1/portfolio/signals", get(routes::portfolio_signals))
            .route("/api/v1/portfolio/dip-assessment", axum::routing::post(routes::dip_assessment))
            // Market data sync routes
            .route("/api/v1/data/sync", axum::routing::post(routes::data_sync))
            .route("/api/v1/data/stats", get(routes::data_stats))
            .route("/api/v1/data/symbols", get(routes::data_symbols))
            .route("/api/v1/data/symbols/add", axum::routing::post(routes::add_symbols))
            // Screener routes
            .route("/api/v1/screener/status", get(routes::screener_status))
            .route("/api/v1/screener/run", axum::routing::post(routes::screener_run))
            .route("/api/v1/screener/results", get(routes::screener_results))
            .route("/api/v1/screener/history", get(routes::screener_history))
            .route("/api/v1/screener/sync", axum::routing::post(routes::screener_sync))
            .with_state(self.state.clone());

        // Initialize market data aggregator (register providers)
        if let Err(e) = self.state.data.initialize(&self.state.config).await {
            tracing::error!(error = %e, "Failed to initialize market data aggregator");
        }

        // Start the market data updater
        let data_state = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = data_state.data.start_updater().await {
                tracing::error!(error = %e, "Market data updater failed");
            }
        });

        // Start the preparation task runner (24/7 operation)
        let prep_state = self.state.clone();
        tokio::spawn(async move {
            run_preparation_tasks(prep_state).await;
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

        // Start the notification retry background task
        let notification_client = Arc::clone(&self.state.notification);
        tokio::spawn(async move {
            notification_client.start_retry_task().await;
        });

        // Start the trading session scheduler (if enabled in config)
        if let Some(trading_config) = &self.state.config.trading {
            if let Some(schedule_config) = &trading_config.schedule {
                if schedule_config.enabled {
                    match TradingScheduler::new(
                        schedule_config.clone(),
                        Arc::clone(&self.state.session_manager),
                    ) {
                        Ok(scheduler) => {
                            let session_manager = Arc::clone(&self.state.session_manager);
                            tokio::spawn(async move {
                                if let Err(e) = scheduler.run().await {
                                    tracing::error!(error = %e, "Trading session scheduler failed");
                                }
                                // Ensure session is stopped when scheduler exits
                                if let Err(e) = session_manager.stop_session().await {
                                    tracing::warn!(error = %e, "Failed to stop session after scheduler exit");
                                }
                            });
                            tracing::info!(
                                start = %schedule_config.session_start,
                                stop = %schedule_config.session_stop,
                                "Trading session scheduler started"
                            );
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to create trading scheduler");
                        }
                    }
                } else {
                    tracing::info!("Trading session scheduler disabled in config");
                }
            }
        }

        // Start the screener scheduler (if available and enabled)
        if let Some(screener) = &self.state.screener_scheduler {
            if screener.config().enabled {
                let screener_clone = Arc::clone(screener);
                tokio::spawn(async move {
                    run_screener_scheduler(screener_clone).await;
                });
                tracing::info!(
                    scan_cron = %screener.config().schedule_cron,
                    sync_cron = %screener.config().data_sync_cron,
                    "Screener scheduler started"
                );
            } else {
                tracing::info!("Screener scheduler disabled in config");
            }
        }

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
                        // Fetch macro environment for enhanced notifications
                        let macro_env = state.macro_filter.get_environment().await.ok();

                        // Send enhanced notifications for new signals
                        for signal in &signals {
                            if let Err(e) = state
                                .notification
                                .send_trading_signal_with_recommendation(signal, macro_env.as_ref())
                                .await
                            {
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

/// Run preparation tasks (24/7 operation).
///
/// Preparation tasks include:
/// - Data preloading: Cache historical data for all tracked symbols
/// - Parameter precomputation: Calculate technical indicators
/// - Macro analysis: Background updates for macro economic data
///
/// These tasks run more frequently during non-trading hours and
/// less frequently during trading hours to avoid competing with execution tasks.
async fn run_preparation_tasks(state: Arc<TradingState>) {
    // Get preparation task config from trading config
    let prep_config = state
        .config
        .trading
        .as_ref()
        .and_then(|t| t.preparation_tasks.clone())
        .unwrap_or_default();

    if !prep_config.enabled {
        tracing::info!("Preparation tasks disabled in config");
        return;
    }

    // Create task scheduler
    let scheduler = TaskScheduler::new(
        prep_config,
        Arc::clone(&state.data),
        Arc::clone(&state.strategy),
    );

    tracing::info!(
        data_interval_secs = scheduler.prep_config.data_preload_interval_secs,
        param_interval_secs = scheduler.prep_config.parameter_precompute_interval_secs,
        macro_interval_secs = scheduler.prep_config.macro_analysis_interval_secs,
        "Preparation task runner started"
    );

    loop {
        let is_trading_hours = TaskScheduler::is_trading_hours();

        // Run all preparation tasks
        if let Err(e) = scheduler.run_preparation_tasks().await {
            tracing::warn!(error = %e, "Preparation tasks failed");
        }

        // Calculate sleep interval based on trading hours
        let sleep_secs = scheduler.get_prep_interval_secs(is_trading_hours);

        tracing::debug!(
            is_trading_hours,
            sleep_secs,
            "Preparation tasks cycle completed, sleeping"
        );

        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }
}

/// Run the screener scheduler based on cron configuration.
///
/// Monitors two cron schedules:
/// - `schedule_cron`: Daily market scan (default: weekdays 18:00)
/// - `data_sync_cron`: Weekly data synchronization (default: Sunday 20:00)
///
/// Uses the `cron` crate for proper cron expression parsing to support
/// user-customized schedules beyond the defaults.
async fn run_screener_scheduler(scheduler: Arc<ScreenerSchedulerImpl>) {
    use chrono::{Datelike, Timelike};
    use cron::Schedule;
    use std::str::FromStr;

    let config = scheduler.config();
    let scan_cron = &config.schedule_cron;
    let sync_cron = &config.data_sync_cron;

    // Parse cron expressions (prepend seconds field for cron crate compatibility)
    let scan_schedule = match Schedule::from_str(&format!("0 {}", scan_cron)) {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::error!(
                cron = %scan_cron,
                error = %e,
                "Failed to parse scan cron expression, scan scheduling disabled"
            );
            None
        }
    };

    let sync_schedule = match Schedule::from_str(&format!("0 {}", sync_cron)) {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::error!(
                cron = %sync_cron,
                error = %e,
                "Failed to parse sync cron expression, sync scheduling disabled"
            );
            None
        }
    };

    if scan_schedule.is_none() && sync_schedule.is_none() {
        tracing::warn!("Both cron schedules failed to parse, screener scheduler exiting");
        return;
    }

    tracing::info!(
        scan_cron = %scan_cron,
        sync_cron = %sync_cron,
        "Screener scheduler running with cron schedules"
    );

    // Track last execution times to prevent duplicate runs
    let mut last_scan_minute: Option<(u32, u32, u32)> = None; // (hour, minute, day)
    let mut last_sync_minute: Option<(u32, u32, u32)> = None;

    loop {
        let now = chrono::Local::now();
        let hour = now.hour();
        let minute = now.minute();
        let day = now.ordinal();

        // Check scan schedule
        if let Some(ref schedule) = scan_schedule {
            if should_run_now(schedule, now) {
                let current = (hour, minute, day);
                if last_scan_minute != Some(current) {
                    last_scan_minute = Some(current);
                    tracing::info!("Triggering scheduled market scan");
                    if let Err(e) = scheduler.trigger_quick_scan().await {
                        tracing::error!(error = %e, "Scheduled market scan failed");
                    }
                }
            }
        }

        // Check sync schedule
        if let Some(ref schedule) = sync_schedule {
            if should_run_now(schedule, now) {
                let current = (hour, minute, day);
                if last_sync_minute != Some(current) {
                    last_sync_minute = Some(current);
                    tracing::info!("Triggering scheduled data sync");
                    if let Err(e) = scheduler.trigger_sync().await {
                        tracing::error!(error = %e, "Scheduled data sync failed");
                    }
                }
            }
        }

        // Sleep for 30 seconds before next check
        // Using 30s instead of 60s to reduce chance of missing a minute boundary
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

/// Check if the current time matches the cron schedule.
fn should_run_now(schedule: &cron::Schedule, now: chrono::DateTime<chrono::Local>) -> bool {
    use chrono::Timelike;

    // Get the upcoming scheduled time
    if let Some(next) = schedule.upcoming(chrono::Local).next() {
        // If the next scheduled time is within 1 minute, we should run
        // This handles the case where we check mid-minute
        let diff = next.signed_duration_since(now);
        if diff.num_seconds() <= 60 && diff.num_seconds() >= 0 {
            return true;
        }

        // Also check if we're currently in the scheduled minute
        // by comparing hour, minute, and day-of-week/day-of-month
        if next.hour() == now.hour() && next.minute() == now.minute() {
            return true;
        }
    }

    false
}
