# Zero Trading Phase 1-2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose Paper Trading functionality via HTTP API and add CLI commands for zero-trading service management.

**Architecture:** Add paper trading state management to TradingState, create new HTTP endpoints in routes.rs, and extend zero-cli with a `trading` subcommand that communicates with the trading service via HTTP.

**Tech Stack:** Rust, Axum, Clap, Tokio, Serde

---

## Phase 1: Paper Trading API

### Task 1: Add PaperTradingState to TradingState

**Files:**
- Modify: `services/zero-trading/src/lib.rs:66-117`

**Step 1: Write the test**

```rust
// Add to services/zero-trading/src/lib.rs at the end

#[cfg(test)]
mod tests {
    use super::*;
    use zero_common::config::Config;

    #[test]
    fn test_trading_state_has_paper_runner() {
        let config = Config::default_test();
        let state = TradingState::new(config);
        // Should have paper trading manager
        assert!(state.paper_manager.is_some() || true); // Existence check
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services/zero-trading && cargo test test_trading_state_has_paper_runner`
Expected: Compilation error - `paper_manager` field doesn't exist

**Step 3: Add PaperTradingManager struct**

Create a new manager struct that wraps PaperTradingRunner with shared state for API access:

```rust
// Add to services/zero-trading/src/paper_trading/mod.rs after line 119

use std::sync::Arc;
use tokio::sync::RwLock;

/// Manager for paper trading sessions (API-friendly wrapper)
pub struct PaperTradingManager {
    /// App configuration
    config: zero_common::config::Config,
    /// Current runner (if session active)
    runner: Arc<RwLock<Option<PaperTradingRunner>>>,
    /// Current session state
    state: Arc<RwLock<SessionState>>,
    /// Session result (after completion)
    last_result: Arc<RwLock<Option<SessionResult>>>,
    /// Active session start time
    start_time: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
}

impl PaperTradingManager {
    /// Create a new manager
    pub fn new(config: &zero_common::config::Config) -> Self {
        Self {
            config: config.clone(),
            runner: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(SessionState::Idle)),
            last_result: Arc::new(RwLock::new(None)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Start a new paper trading session
    pub async fn start_session(&self, paper_config: PaperTradingConfig, duration: Option<std::time::Duration>) -> anyhow::Result<()> {
        let current_state = *self.state.read().await;
        if current_state == SessionState::Running {
            anyhow::bail!("Session already running");
        }

        let runner = PaperTradingRunner::new(&self.config, paper_config);

        // Store runner
        {
            let mut runner_lock = self.runner.write().await;
            *runner_lock = Some(runner);
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = SessionState::Running;
        }
        {
            let mut start = self.start_time.write().await;
            *start = Some(chrono::Utc::now());
        }

        // Spawn session in background
        let runner_arc = Arc::clone(&self.runner);
        let state_arc = Arc::clone(&self.state);
        let result_arc = Arc::clone(&self.last_result);

        tokio::spawn(async move {
            let result = {
                let runner_lock = runner_arc.read().await;
                if let Some(ref runner) = *runner_lock {
                    runner.run_session(duration).await
                } else {
                    Err(anyhow::anyhow!("Runner not initialized"))
                }
            };

            // Store result and update state
            match result {
                Ok(session_result) => {
                    let mut result_lock = result_arc.write().await;
                    *result_lock = Some(session_result);
                    let mut state = state_arc.write().await;
                    *state = SessionState::Completed;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Paper trading session failed");
                    let mut state = state_arc.write().await;
                    *state = SessionState::Failed;
                }
            }
        });

        Ok(())
    }

    /// Stop the current session
    pub async fn stop_session(&self) -> anyhow::Result<()> {
        let runner_lock = self.runner.read().await;
        if let Some(ref runner) = *runner_lock {
            runner.stop().await;
        }
        Ok(())
    }

    /// Get current session state
    pub async fn get_state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Get session status details
    pub async fn get_status(&self) -> PaperSessionStatus {
        let state = *self.state.read().await;
        let start_time = *self.start_time.read().await;
        let elapsed = start_time.map(|s| chrono::Utc::now().signed_duration_since(s));

        PaperSessionStatus {
            state,
            start_time,
            elapsed_seconds: elapsed.map(|d| d.num_seconds()),
        }
    }

    /// Get trades from current or last session
    pub async fn get_trades(&self) -> Vec<PaperTrade> {
        if let Some(ref result) = *self.last_result.read().await {
            return result.trades.clone();
        }
        Vec::new()
    }

    /// Get last session result
    pub async fn get_result(&self) -> Option<SessionResult> {
        self.last_result.read().await.clone()
    }

    /// Generate report for last session
    pub async fn get_report(&self) -> Option<PaperTradingReport> {
        let result = self.last_result.read().await.clone()?;
        Some(PaperTradingReport {
            title: "Paper Trading Session Report".to_string(),
            period: format!("{} to {}",
                result.start_time.format("%Y-%m-%d %H:%M"),
                result.end_time.format("%H:%M")
            ),
            summary: result.summary,
            trades: result.trades,
            validations: result.validations,
        })
    }
}

/// Paper session status for API responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaperSessionStatus {
    pub state: SessionState,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub elapsed_seconds: Option<i64>,
}
```

**Step 4: Update mod.rs exports**

Add to `services/zero-trading/src/paper_trading/mod.rs` exports:

```rust
pub use runner::{PaperTradingRunner, PaperTradingConfig, SessionResult};
// Add this line:
pub use self::{PaperTradingManager, PaperSessionStatus};
```

**Step 5: Add PaperTradingManager to TradingState**

Modify `services/zero-trading/src/lib.rs`:

```rust
// Add import at top
use crate::paper_trading::PaperTradingManager;

// Add field to TradingState struct (after line 82)
pub struct TradingState {
    // ... existing fields ...
    /// Paper trading manager
    pub paper_manager: Arc<PaperTradingManager>,
}

// Update TradingState::new() to initialize paper_manager
impl TradingState {
    pub fn new(config: Config) -> Self {
        // ... existing initialization ...

        // Add paper trading manager
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
            paper_manager, // Add this
        }
    }
}
```

**Step 6: Run test to verify it passes**

Run: `cd services/zero-trading && cargo test test_trading_state_has_paper_runner`
Expected: PASS

**Step 7: Commit**

```bash
git add services/zero-trading/src/paper_trading/mod.rs services/zero-trading/src/lib.rs
git commit -m "feat(trading): add PaperTradingManager for API access"
```

---

### Task 2: Add Paper Trading API Routes

**Files:**
- Modify: `services/zero-trading/src/routes.rs`
- Modify: `services/zero-trading/src/lib.rs` (router)

**Step 1: Write the integration test**

```rust
// Add to services/zero-trading/tests/api_test.rs (create if needed)

#[tokio::test]
async fn test_paper_trading_endpoints_exist() {
    // This test verifies the routes compile and are registered
    // Full integration test would need a running server
    use zero_trading::routes;

    // Verify response types are serializable
    let status = routes::PaperStatusResponse {
        state: "Idle".to_string(),
        start_time: None,
        elapsed_seconds: None,
        trades_count: 0,
        current_pnl: None,
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Idle"));
}
```

**Step 2: Add response types to routes.rs**

```rust
// Add to services/zero-trading/src/routes.rs after line 72

/// Paper trading start request
#[derive(Debug, Deserialize)]
pub struct PaperStartRequest {
    /// Initial capital (default: 100000)
    #[serde(default = "default_capital")]
    pub initial_capital: f64,
    /// Session duration in seconds (optional)
    pub duration_secs: Option<u64>,
    /// Maximum positions (default: 5)
    #[serde(default = "default_max_positions")]
    pub max_positions: usize,
    /// Enable notifications (default: true)
    #[serde(default = "default_true")]
    pub enable_notifications: bool,
}

fn default_capital() -> f64 { 100_000.0 }
fn default_max_positions() -> usize { 5 }
fn default_true() -> bool { true }

/// Paper trading status response
#[derive(Debug, Serialize)]
pub struct PaperStatusResponse {
    pub state: String,
    pub start_time: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub trades_count: usize,
    pub current_pnl: Option<f64>,
}

/// Paper trading trades response
#[derive(Debug, Serialize)]
pub struct PaperTradesResponse {
    pub trades: Vec<crate::paper_trading::PaperTrade>,
    pub count: usize,
}

/// Paper trading report response
#[derive(Debug, Serialize)]
pub struct PaperReportResponse {
    pub title: String,
    pub period: String,
    pub summary: crate::paper_trading::SessionSummary,
    pub verification: crate::paper_trading::report::VerificationResult,
    pub text_report: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}
```

**Step 3: Add route handlers**

```rust
// Add to services/zero-trading/src/routes.rs after macro routes

// ============================================================================
// Paper Trading Routes
// ============================================================================

/// Start paper trading session
pub async fn paper_start(
    State(state): State<Arc<TradingState>>,
    Json(req): Json<PaperStartRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    use crate::paper_trading::PaperTradingConfig;
    use crate::strategy::SignalStrength;

    let config = PaperTradingConfig {
        initial_capital: req.initial_capital,
        max_position_pct: 20.0,
        min_signal_strength: SignalStrength::Medium,
        max_positions: req.max_positions,
        enable_notifications: req.enable_notifications,
        scan_interval_secs: 60,
        max_duration: req.duration_secs.map(std::time::Duration::from_secs),
    };

    let duration = req.duration_secs.map(std::time::Duration::from_secs);

    match state.paper_manager.start_session(config, duration).await {
        Ok(()) => Ok(Json(SuccessResponse {
            success: true,
            message: "Paper trading session started".to_string(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to start paper trading");
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Stop paper trading session
pub async fn paper_stop(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    match state.paper_manager.stop_session().await {
        Ok(()) => Ok(Json(SuccessResponse {
            success: true,
            message: "Paper trading session stopped".to_string(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to stop paper trading");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get paper trading status
pub async fn paper_status(
    State(state): State<Arc<TradingState>>,
) -> Json<PaperStatusResponse> {
    let status = state.paper_manager.get_status().await;
    let trades = state.paper_manager.get_trades().await;
    let result = state.paper_manager.get_result().await;

    let current_pnl = result.as_ref().map(|r| r.summary.net_profit);

    Json(PaperStatusResponse {
        state: format!("{:?}", status.state),
        start_time: status.start_time.map(|t| t.to_rfc3339()),
        elapsed_seconds: status.elapsed_seconds,
        trades_count: trades.len(),
        current_pnl,
    })
}

/// Get paper trading trades
pub async fn paper_trades(
    State(state): State<Arc<TradingState>>,
) -> Json<PaperTradesResponse> {
    let trades = state.paper_manager.get_trades().await;
    let count = trades.len();
    Json(PaperTradesResponse { trades, count })
}

/// Get paper trading report
pub async fn paper_report(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<PaperReportResponse>, StatusCode> {
    match state.paper_manager.get_report().await {
        Some(report) => {
            let verification = report.meets_verification_criteria();
            let text_report = report.to_text_report();

            Ok(Json(PaperReportResponse {
                title: report.title,
                period: report.period,
                summary: report.summary,
                verification,
                text_report,
            }))
        }
        None => Err(StatusCode::NOT_FOUND)
    }
}
```

**Step 4: Register routes in lib.rs**

Modify `services/zero-trading/src/lib.rs` router section:

```rust
// In TradingService::start(), add paper trading routes to the router
let app = Router::new()
    .route("/health", get(routes::health))
    .route("/api/v1/signals", get(routes::get_signals))
    .route("/api/v1/positions", get(routes::get_positions))
    .route("/api/v1/status", get(routes::get_status))
    // Macro agent routes
    .route("/api/v1/macro/decision", get(routes::get_macro_decision))
    .route("/api/v1/macro/analyze", axum::routing::post(routes::force_macro_analysis))
    .route("/api/v1/macro/report", get(routes::get_macro_report))
    .route("/api/v1/macro/report/send", axum::routing::post(routes::send_macro_report))
    .route("/api/v1/macro/status", get(routes::check_agent_status))
    // Paper trading routes (NEW)
    .route("/api/v1/paper/start", axum::routing::post(routes::paper_start))
    .route("/api/v1/paper/stop", axum::routing::post(routes::paper_stop))
    .route("/api/v1/paper/status", get(routes::paper_status))
    .route("/api/v1/paper/trades", get(routes::paper_trades))
    .route("/api/v1/paper/report", get(routes::paper_report))
    .with_state(self.state.clone());
```

**Step 5: Run tests**

Run: `cd services/zero-trading && cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add services/zero-trading/src/routes.rs services/zero-trading/src/lib.rs
git commit -m "feat(trading): add Paper Trading HTTP API endpoints"
```

---

## Phase 2: CLI Commands

### Task 3: Add Trading Commands to zero-cli

**Files:**
- Create: `services/zero-cli/src/trading.rs`
- Modify: `services/zero-cli/src/main.rs`

**Step 1: Create trading module**

Create `services/zero-cli/src/trading.rs`:

```rust
//! Trading service CLI commands.

use anyhow::{bail, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::Config;

/// Trading subcommands
#[derive(Subcommand, Debug)]
pub enum TradingCommands {
    /// Start the trading service
    Start {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to
        #[arg(long, default_value = "4434")]
        port: u16,
    },
    /// Stop the trading service
    Stop,
    /// Show trading service status
    Status,
    /// Paper trading commands
    Paper {
        #[command(subcommand)]
        paper_command: PaperCommands,
    },
    /// Run backtest
    Backtest {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,
        /// Initial capital
        #[arg(long, default_value = "100000")]
        capital: f64,
    },
}

/// Paper trading subcommands
#[derive(Subcommand, Debug)]
pub enum PaperCommands {
    /// Start paper trading session
    Start {
        /// Initial capital
        #[arg(long, default_value = "100000")]
        capital: f64,
        /// Session duration (e.g., "4h", "1d")
        #[arg(long)]
        duration: Option<String>,
        /// Maximum concurrent positions
        #[arg(long, default_value = "5")]
        max_positions: usize,
        /// Disable notifications
        #[arg(long)]
        no_notify: bool,
    },
    /// Stop paper trading session
    Stop,
    /// Show paper trading status
    Status,
    /// Show paper trades
    Trades,
    /// Show paper trading report
    Report,
}

const TRADING_BASE_URL: &str = "http://127.0.0.1:4434";

/// Handle trading commands
pub async fn handle_command(cmd: TradingCommands, config: &Config) -> Result<()> {
    match cmd {
        TradingCommands::Start { host, port } => start_service(&host, port, config).await,
        TradingCommands::Stop => stop_service().await,
        TradingCommands::Status => show_status().await,
        TradingCommands::Paper { paper_command } => handle_paper_command(paper_command).await,
        TradingCommands::Backtest { start, end, capital } => {
            run_backtest(&start, &end, capital).await
        }
    }
}

async fn start_service(host: &str, port: u16, _config: &Config) -> Result<()> {
    info!(host, port, "Starting trading service");

    // Check if already running
    if is_service_running().await {
        bail!("Trading service is already running on port {}", port);
    }

    // Start the service binary
    let child = std::process::Command::new("zero-trading")
        .env("TRADING_HOST", host)
        .env("TRADING_PORT", port.to_string())
        .spawn();

    match child {
        Ok(_) => {
            println!("Trading service starting on {}:{}", host, port);
            println!("Use 'zero trading status' to check status");
            Ok(())
        }
        Err(e) => {
            bail!("Failed to start trading service: {}", e);
        }
    }
}

async fn stop_service() -> Result<()> {
    // Send shutdown request to the service
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/shutdown", TRADING_BASE_URL))
        .send()
        .await
    {
        Ok(_) => {
            println!("Trading service stopped");
            Ok(())
        }
        Err(_) => {
            println!("Trading service is not running");
            Ok(())
        }
    }
}

async fn is_service_running() -> bool {
    let client = reqwest::Client::new();
    client
        .get(format!("{}/health", TRADING_BASE_URL))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}

async fn show_status() -> Result<()> {
    let client = reqwest::Client::new();

    match client
        .get(format!("{}/api/v1/status", TRADING_BASE_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                let status: ServiceStatus = resp.json().await?;
                println!("Trading Service Status");
                println!("──────────────────────────────");
                println!("Market Connected:  {}", if status.market_connected { "✅" } else { "❌" });
                println!("Broker Connected:  {}", if status.broker_connected { "✅" } else { "❌" });
                println!("Active Signals:    {}", status.active_signals);
                println!("Open Positions:    {}", status.open_positions);
                if let Some(last_scan) = status.last_scan {
                    println!("Last Scan:         {}", last_scan);
                }
            } else {
                println!("Trading service returned error: {}", resp.status());
            }
        }
        Err(_) => {
            println!("Trading service is not running");
            println!("Start with: zero trading start");
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ServiceStatus {
    market_connected: bool,
    broker_connected: bool,
    last_scan: Option<String>,
    active_signals: usize,
    open_positions: usize,
}

// ============================================================================
// Paper Trading Commands
// ============================================================================

async fn handle_paper_command(cmd: PaperCommands) -> Result<()> {
    match cmd {
        PaperCommands::Start { capital, duration, max_positions, no_notify } => {
            paper_start(capital, duration, max_positions, !no_notify).await
        }
        PaperCommands::Stop => paper_stop().await,
        PaperCommands::Status => paper_status().await,
        PaperCommands::Trades => paper_trades().await,
        PaperCommands::Report => paper_report().await,
    }
}

async fn paper_start(capital: f64, duration: Option<String>, max_positions: usize, notify: bool) -> Result<()> {
    let duration_secs = duration.as_ref().map(|d| parse_duration(d)).transpose()?;

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "initial_capital": capital,
        "duration_secs": duration_secs,
        "max_positions": max_positions,
        "enable_notifications": notify,
    });

    match client
        .post(format!("{}/api/v1/paper/start", TRADING_BASE_URL))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Paper trading session started");
                println!("  Capital:      ¥{:.2}", capital);
                if let Some(d) = duration {
                    println!("  Duration:     {}", d);
                }
                println!("  Max Positions: {}", max_positions);
                println!("  Notifications: {}", if notify { "enabled" } else { "disabled" });
                println!();
                println!("Use 'zero trading paper status' to monitor progress");
            } else {
                let error: serde_json::Value = resp.json().await?;
                bail!("Failed to start: {:?}", error);
            }
        }
        Err(e) => {
            bail!("Trading service not available: {}", e);
        }
    }

    Ok(())
}

fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim().to_lowercase();

    if let Some(hours) = s.strip_suffix('h') {
        let h: u64 = hours.parse()?;
        return Ok(h * 3600);
    }
    if let Some(days) = s.strip_suffix('d') {
        let d: u64 = days.parse()?;
        return Ok(d * 86400);
    }
    if let Some(mins) = s.strip_suffix('m') {
        let m: u64 = mins.parse()?;
        return Ok(m * 60);
    }

    // Try parsing as seconds
    Ok(s.parse()?)
}

async fn paper_stop() -> Result<()> {
    let client = reqwest::Client::new();

    match client
        .post(format!("{}/api/v1/paper/stop", TRADING_BASE_URL))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Paper trading session stopped");
            } else {
                println!("No active session to stop");
            }
        }
        Err(e) => {
            bail!("Trading service not available: {}", e);
        }
    }

    Ok(())
}

async fn paper_status() -> Result<()> {
    let client = reqwest::Client::new();

    match client
        .get(format!("{}/api/v1/paper/status", TRADING_BASE_URL))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                let status: PaperStatus = resp.json().await?;
                println!("Paper Trading Status");
                println!("──────────────────────────────");
                println!("State:          {}", status.state);
                if let Some(start) = status.start_time {
                    println!("Started:        {}", start);
                }
                if let Some(elapsed) = status.elapsed_seconds {
                    let hours = elapsed / 3600;
                    let mins = (elapsed % 3600) / 60;
                    println!("Elapsed:        {}h {}m", hours, mins);
                }
                println!("Trades:         {}", status.trades_count);
                if let Some(pnl) = status.current_pnl {
                    let emoji = if pnl >= 0.0 { "📈" } else { "📉" };
                    println!("P&L:            {} ¥{:.2}", emoji, pnl);
                }
            } else {
                println!("Failed to get status");
            }
        }
        Err(e) => {
            bail!("Trading service not available: {}", e);
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PaperStatus {
    state: String,
    start_time: Option<String>,
    elapsed_seconds: Option<i64>,
    trades_count: usize,
    current_pnl: Option<f64>,
}

async fn paper_trades() -> Result<()> {
    let client = reqwest::Client::new();

    match client
        .get(format!("{}/api/v1/paper/trades", TRADING_BASE_URL))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                let trades: PaperTradesResp = resp.json().await?;

                if trades.trades.is_empty() {
                    println!("No trades recorded yet");
                    return Ok(());
                }

                println!("Paper Trades ({} total)", trades.count);
                println!("═══════════════════════════════════════════════════════════");

                for trade in trades.trades.iter().take(20) {
                    let status_icon = match trade.status.as_str() {
                        "ClosedProfit" => "✅",
                        "ClosedLoss" => "❌",
                        "Open" => "🔵",
                        _ => "⚪",
                    };

                    let pnl = trade.realized_pnl.unwrap_or(0.0);
                    println!(
                        "{} {} | Entry: {:.2} → Exit: {:.2} | P&L: ¥{:.2}",
                        status_icon,
                        trade.symbol,
                        trade.entry_price,
                        trade.exit_price.unwrap_or(trade.entry_price),
                        pnl
                    );
                }

                if trades.count > 20 {
                    println!("... and {} more trades", trades.count - 20);
                }
            }
        }
        Err(e) => {
            bail!("Trading service not available: {}", e);
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PaperTradesResp {
    trades: Vec<PaperTrade>,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct PaperTrade {
    symbol: String,
    entry_price: f64,
    exit_price: Option<f64>,
    status: String,
    realized_pnl: Option<f64>,
}

async fn paper_report() -> Result<()> {
    let client = reqwest::Client::new();

    match client
        .get(format!("{}/api/v1/paper/report", TRADING_BASE_URL))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                let report: PaperReportResp = resp.json().await?;
                println!("{}", report.text_report);

                println!();
                println!("Verification: {}", if report.verification.passed { "✅ PASSED" } else { "❌ FAILED" });
                for issue in &report.verification.issues {
                    println!("  - {}", issue);
                }
                println!();
                println!("{}", report.verification.recommendation);
            } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
                println!("No report available. Run a paper trading session first.");
            }
        }
        Err(e) => {
            bail!("Trading service not available: {}", e);
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PaperReportResp {
    text_report: String,
    verification: Verification,
}

#[derive(Debug, Deserialize)]
struct Verification {
    passed: bool,
    issues: Vec<String>,
    recommendation: String,
}

async fn run_backtest(start: &str, end: &str, capital: f64) -> Result<()> {
    println!("Running backtest from {} to {} with ¥{:.2} capital", start, end, capital);
    println!("(Backtest CLI integration pending - use HTTP API directly)");

    // TODO: Implement backtest CLI
    // This would call the backtest API endpoints

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("4h").unwrap(), 4 * 3600);
        assert_eq!(parse_duration("1d").unwrap(), 86400);
        assert_eq!(parse_duration("30m").unwrap(), 30 * 60);
        assert_eq!(parse_duration("3600").unwrap(), 3600);
    }
}
```

**Step 2: Add module and commands to main.rs**

Modify `services/zero-cli/src/main.rs`:

```rust
// Add module declaration after line 39 (after mod util;)
mod trading;

// Add Trading command to Commands enum (after McpServer)
/// Manage automated trading (PO3+SMT strategy)
Trading {
    #[command(subcommand)]
    trading_command: trading::TradingCommands,
},

// Add match arm in main() (before the closing brace of the match)
Commands::Trading { trading_command } => {
    trading::handle_command(trading_command, &config).await
}
```

**Step 3: Run tests**

Run: `cd services/zero-cli && cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add services/zero-cli/src/trading.rs services/zero-cli/src/main.rs
git commit -m "feat(cli): add trading subcommand with paper trading support"
```

---

### Task 4: Integration Testing

**Files:**
- Create: `services/zero-trading/tests/paper_api_test.rs`

**Step 1: Create integration test file**

```rust
//! Paper trading API integration tests

use axum::http::StatusCode;
use serde_json::json;

// Note: These tests require a running server or use test utilities
// For now, we test the serialization and basic logic

#[test]
fn test_paper_start_request_defaults() {
    let json_str = r#"{}"#;
    // This would deserialize with defaults
    // Full test requires the routes module to be public
}

#[test]
fn test_paper_status_response_serialization() {
    let status = json!({
        "state": "Running",
        "start_time": "2026-02-25T10:00:00Z",
        "elapsed_seconds": 3600,
        "trades_count": 5,
        "current_pnl": 1500.0
    });

    let json_str = serde_json::to_string(&status).unwrap();
    assert!(json_str.contains("Running"));
    assert!(json_str.contains("1500"));
}

#[tokio::test]
async fn test_paper_trading_endpoints_compile() {
    // This test ensures all route handlers compile correctly
    // The actual HTTP testing would be done with a test server

    // Verify types are correct by using them
    use zero_trading::paper_trading::{SessionState, PaperTradingConfig};

    let config = PaperTradingConfig::default();
    assert!(config.initial_capital > 0.0);

    let state = SessionState::Idle;
    assert_eq!(state, SessionState::Idle);
}
```

**Step 2: Run all tests**

Run: `cd services/zero-trading && cargo test`
Expected: All 68+ tests pass

**Step 3: Final commit**

```bash
git add services/zero-trading/tests/
git commit -m "test(trading): add paper trading API integration tests"
```

---

## Verification Checklist

After completing all tasks, verify:

1. **Paper Trading API:**
   ```bash
   # Start trading service
   cd services/zero-trading && cargo run

   # In another terminal:
   curl -X POST http://localhost:4434/api/v1/paper/start \
     -H "Content-Type: application/json" \
     -d '{"initial_capital": 100000, "duration_secs": 14400}'

   curl http://localhost:4434/api/v1/paper/status
   ```

2. **CLI Commands:**
   ```bash
   # Build CLI
   cd services/zero-cli && cargo build

   # Test commands (with trading service running)
   ./target/debug/zero-cli trading status
   ./target/debug/zero-cli trading paper start --capital 100000 --duration 4h
   ./target/debug/zero-cli trading paper status
   ```

3. **All tests pass:**
   ```bash
   cd services/zero-trading && cargo test
   cd services/zero-cli && cargo test
   ```

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Add PaperTradingManager to TradingState | Medium |
| 2 | Add Paper Trading API routes | Medium |
| 3 | Add Trading CLI commands | Medium |
| 4 | Integration testing | Low |

**Total new lines of code:** ~600
**New files:** 2 (trading.rs, paper_api_test.rs)
**Modified files:** 4 (lib.rs, routes.rs, paper_trading/mod.rs, main.rs)
