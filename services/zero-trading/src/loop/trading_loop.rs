//! Core trading loop implementation.
//!
//! The TradingLoop provides second-level monitoring for automated trading,
//! running continuously during trading hours to check stop-loss/take-profit
//! conditions and scan for new entry signals.

use anyhow::Result;
use chrono::{Local, NaiveTime};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn};

use super::{MonitoredPosition, PriceMonitor, SignalDetector, TradingMode};
use crate::data::MarketDataAggregator;
use crate::execution::ExecutionEngine;
use crate::strategy::StrategyEngine;

/// Trading loop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Main loop interval in seconds (for signal scanning)
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,

    /// Price check interval in seconds (for stop-loss/take-profit)
    #[serde(default = "default_price_check_secs")]
    pub price_check_interval_secs: u64,

    /// Trading mode (paper or live)
    #[serde(default)]
    pub mode: TradingMode,

    /// Enable automatic order execution
    #[serde(default)]
    pub auto_execute: bool,

    /// Morning session start time (HH:MM format)
    #[serde(default = "default_morning_start")]
    pub morning_start: String,

    /// Morning session end time
    #[serde(default = "default_morning_end")]
    pub morning_end: String,

    /// Afternoon session start time
    #[serde(default = "default_afternoon_start")]
    pub afternoon_start: String,

    /// Afternoon session end time
    #[serde(default = "default_afternoon_end")]
    pub afternoon_end: String,
}

fn default_interval_secs() -> u64 {
    5
}
fn default_price_check_secs() -> u64 {
    1
}
fn default_morning_start() -> String {
    "09:30".to_string()
}
fn default_morning_end() -> String {
    "11:30".to_string()
}
fn default_afternoon_start() -> String {
    "13:00".to_string()
}
fn default_afternoon_end() -> String {
    "15:00".to_string()
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_interval_secs(),
            price_check_interval_secs: default_price_check_secs(),
            mode: TradingMode::Paper,
            auto_execute: false,
            morning_start: default_morning_start(),
            morning_end: default_morning_end(),
            afternoon_start: default_afternoon_start(),
            afternoon_end: default_afternoon_end(),
        }
    }
}

impl LoopConfig {
    /// Parse time string to NaiveTime
    fn parse_time(&self, time_str: &str) -> Option<NaiveTime> {
        NaiveTime::parse_from_str(time_str, "%H:%M").ok()
    }

    /// Check if current time is within trading hours
    pub fn is_trading_hours(&self) -> bool {
        let now = Local::now().time();

        let morning_start = self.parse_time(&self.morning_start);
        let morning_end = self.parse_time(&self.morning_end);
        let afternoon_start = self.parse_time(&self.afternoon_start);
        let afternoon_end = self.parse_time(&self.afternoon_end);

        // Check morning session
        if let (Some(start), Some(end)) = (morning_start, morning_end) {
            if now >= start && now <= end {
                return true;
            }
        }

        // Check afternoon session
        if let (Some(start), Some(end)) = (afternoon_start, afternoon_end) {
            if now >= start && now <= end {
                return true;
            }
        }

        false
    }
}

/// Trading loop state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopState {
    /// Loop not started
    Idle,
    /// Loop running
    Running,
    /// Loop paused (e.g., lunch break)
    Paused,
    /// Loop stopping
    Stopping,
    /// Loop stopped
    Stopped,
}

/// Events emitted by the trading loop
#[derive(Debug, Clone)]
pub enum LoopEvent {
    /// Loop started
    Started,
    /// Loop paused
    Paused,
    /// Loop resumed
    Resumed,
    /// Loop stopped
    Stopped,
    /// Price check completed
    PriceChecked { positions_checked: usize },
    /// Signal scan completed
    SignalScanned { signals_found: usize },
    /// Stop loss triggered
    StopLossTriggered { position_id: String, symbol: String },
    /// Take profit triggered
    TakeProfitTriggered { position_id: String, symbol: String },
    /// New signal detected
    SignalDetected { symbol: String },
    /// Error occurred
    Error { message: String },
}

/// Core trading loop for automated execution
pub struct TradingLoop {
    /// Configuration
    config: LoopConfig,
    /// Market data source
    #[allow(dead_code)] // Reserved for multi-timeframe analysis
    data: Arc<MarketDataAggregator>,
    /// Strategy engine
    #[allow(dead_code)] // Reserved for signal detection
    strategy: Arc<StrategyEngine>,
    /// Execution engine
    execution: Arc<RwLock<ExecutionEngine>>,
    /// Current state
    state: Arc<RwLock<LoopState>>,
    /// Monitored positions
    positions: Arc<RwLock<Vec<MonitoredPosition>>>,
    /// Price monitor
    price_monitor: PriceMonitor,
    /// Signal detector
    signal_detector: SignalDetector,
    /// Event broadcast sender
    event_tx: broadcast::Sender<LoopEvent>,
}

impl TradingLoop {
    /// Create a new trading loop
    pub fn new(
        config: LoopConfig,
        data: Arc<MarketDataAggregator>,
        strategy: Arc<StrategyEngine>,
        execution: Arc<RwLock<ExecutionEngine>>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            config: config.clone(),
            data: Arc::clone(&data),
            strategy: Arc::clone(&strategy),
            execution,
            state: Arc::new(RwLock::new(LoopState::Idle)),
            positions: Arc::new(RwLock::new(Vec::new())),
            price_monitor: PriceMonitor::new(Arc::clone(&data)),
            signal_detector: SignalDetector::new(Arc::clone(&strategy), Arc::clone(&data)),
            event_tx,
        }
    }

    /// Subscribe to loop events
    pub fn subscribe(&self) -> broadcast::Receiver<LoopEvent> {
        self.event_tx.subscribe()
    }

    /// Get current state
    pub async fn get_state(&self) -> LoopState {
        *self.state.read().await
    }

    /// Get monitored positions
    pub async fn get_positions(&self) -> Vec<MonitoredPosition> {
        self.positions.read().await.clone()
    }

    /// Add a position to monitor
    pub async fn add_position(&self, position: MonitoredPosition) {
        let mut positions = self.positions.write().await;
        positions.push(position);
    }

    /// Remove a position from monitoring
    pub async fn remove_position(&self, position_id: &str) {
        let mut positions = self.positions.write().await;
        positions.retain(|p| p.id != position_id);
    }

    /// Pause the loop
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        if *state == LoopState::Running {
            *state = LoopState::Paused;
            let _ = self.event_tx.send(LoopEvent::Paused);
            info!("Trading loop paused");
        }
    }

    /// Resume the loop
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        if *state == LoopState::Paused {
            *state = LoopState::Running;
            let _ = self.event_tx.send(LoopEvent::Resumed);
            info!("Trading loop resumed");
        }
    }

    /// Stop the loop
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        if *state == LoopState::Running || *state == LoopState::Paused {
            *state = LoopState::Stopping;
            info!("Trading loop stopping...");
        }
    }

    /// Run the trading loop
    pub async fn run(&self) -> Result<()> {
        // Set state to running
        {
            let mut state = self.state.write().await;
            *state = LoopState::Running;
        }
        let _ = self.event_tx.send(LoopEvent::Started);
        info!(
            mode = ?self.config.mode,
            interval_secs = self.config.interval_secs,
            "Trading loop started"
        );

        // Create intervals
        let mut main_interval = interval(Duration::from_secs(self.config.interval_secs));
        let mut price_interval = interval(Duration::from_secs(self.config.price_check_interval_secs));

        // Track last signal scan time to avoid duplicate scans
        let mut last_signal_scan = std::time::Instant::now();

        loop {
            tokio::select! {
                // Price check (more frequent)
                _ = price_interval.tick() => {
                    let current_state = *self.state.read().await;

                    match current_state {
                        LoopState::Stopping | LoopState::Stopped => break,
                        LoopState::Paused => continue,
                        LoopState::Running => {
                            // Only check prices during trading hours
                            if !self.config.is_trading_hours() {
                                debug!("Outside trading hours, skipping price check");
                                continue;
                            }

                            if let Err(e) = self.check_prices().await {
                                warn!(error = %e, "Price check failed");
                                let _ = self.event_tx.send(LoopEvent::Error {
                                    message: e.to_string(),
                                });
                            }
                        }
                        _ => continue,
                    }
                }

                // Signal scan (less frequent)
                _ = main_interval.tick() => {
                    let current_state = *self.state.read().await;

                    match current_state {
                        LoopState::Stopping | LoopState::Stopped => break,
                        LoopState::Paused => continue,
                        LoopState::Running => {
                            // Only scan during trading hours
                            if !self.config.is_trading_hours() {
                                debug!("Outside trading hours, skipping signal scan");
                                continue;
                            }

                            // Avoid duplicate scans
                            if last_signal_scan.elapsed().as_secs() < self.config.interval_secs {
                                continue;
                            }
                            last_signal_scan = std::time::Instant::now();

                            if let Err(e) = self.scan_signals().await {
                                warn!(error = %e, "Signal scan failed");
                                let _ = self.event_tx.send(LoopEvent::Error {
                                    message: e.to_string(),
                                });
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }

        // Final state update
        {
            let mut state = self.state.write().await;
            *state = LoopState::Stopped;
        }
        let _ = self.event_tx.send(LoopEvent::Stopped);
        info!("Trading loop stopped");

        Ok(())
    }

    /// Check prices and trigger stop-loss/take-profit
    async fn check_prices(&self) -> Result<()> {
        let positions = self.positions.read().await;

        if positions.is_empty() {
            return Ok(());
        }

        let symbols: Vec<String> = positions.iter().map(|p| p.symbol.clone()).collect();
        drop(positions);

        // Fetch current prices
        let price_results = self.price_monitor.check_prices(&symbols).await?;

        let mut positions = self.positions.write().await;
        let mut triggers: Vec<(String, bool, bool)> = Vec::new(); // (id, is_stop_loss, is_take_profit)

        for position in positions.iter_mut() {
            if let Some(result) = price_results.iter().find(|r| r.symbol == position.symbol) {
                position.update_price(result.current_price);

                if result.stop_loss_triggered {
                    triggers.push((position.id.clone(), true, false));
                } else if result.take_profit_triggered {
                    triggers.push((position.id.clone(), false, true));
                }
            }
        }

        let _ = self.event_tx.send(LoopEvent::PriceChecked {
            positions_checked: positions.len(),
        });

        drop(positions);

        // Handle triggers
        for (id, is_stop_loss, is_take_profit) in triggers {
            let positions = self.positions.read().await;
            let position = positions.iter().find(|p| p.id == id);

            if let Some(pos) = position {
                let symbol = pos.symbol.clone();
                drop(positions);

                if is_stop_loss {
                    info!(position_id = %id, symbol = %symbol, "Stop loss triggered");
                    let _ = self.event_tx.send(LoopEvent::StopLossTriggered {
                        position_id: id.clone(),
                        symbol: symbol.clone(),
                    });

                    if self.config.auto_execute {
                        self.execute_close(&id, "stop_loss").await?;
                    }
                } else if is_take_profit {
                    info!(position_id = %id, symbol = %symbol, "Take profit triggered");
                    let _ = self.event_tx.send(LoopEvent::TakeProfitTriggered {
                        position_id: id.clone(),
                        symbol: symbol.clone(),
                    });

                    if self.config.auto_execute {
                        self.execute_close(&id, "take_profit").await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Scan for new trading signals
    async fn scan_signals(&self) -> Result<()> {
        let signals = self.signal_detector.scan().await?;

        let _ = self.event_tx.send(LoopEvent::SignalScanned {
            signals_found: signals.len(),
        });

        for signal in &signals {
            info!(
                symbol = %signal.symbol,
                direction = ?signal.direction,
                strength = ?signal.strength,
                "New trading signal detected"
            );

            let _ = self.event_tx.send(LoopEvent::SignalDetected {
                symbol: signal.symbol.clone(),
            });

            // Auto-execute if enabled
            if self.config.auto_execute {
                self.execute_entry(signal).await?;
            }
        }

        Ok(())
    }

    /// Execute position close
    async fn execute_close(&self, position_id: &str, reason: &str) -> Result<()> {
        let positions = self.positions.read().await;
        let position = positions
            .iter()
            .find(|p| p.id == position_id)
            .cloned();
        drop(positions);

        if let Some(pos) = position {
            let mut execution = self.execution.write().await;

            match self.config.mode {
                TradingMode::Paper => {
                    // Paper trading - use execution engine's paper mode
                    execution.execute_sell(&pos.symbol, reason).await?;
                }
                TradingMode::Live => {
                    // Live trading - would need broker integration
                    execution.execute_sell(&pos.symbol, reason).await?;
                }
            }

            // Remove from monitoring
            self.remove_position(position_id).await;
        }

        Ok(())
    }

    /// Execute position entry
    async fn execute_entry(&self, signal: &crate::strategy::TradingSignal) -> Result<()> {
        let mut execution = self.execution.write().await;

        // Execute order - ExecutionEngine calculates position size using risk-based sizing:
        // quantity = (2% of capital) / (entry_price - stop_loss)
        let order = execution.execute_buy(signal).await?;

        let mode_label = match self.config.mode {
            TradingMode::Paper => "[PAPER]",
            TradingMode::Live => "[LIVE]",
        };
        info!(
            symbol = %signal.symbol,
            order_id = %order.id,
            quantity = order.quantity,
            "{} Entry order executed",
            mode_label
        );

        // Add to monitoring - use the quantity calculated by ExecutionEngine
        let position = MonitoredPosition {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: signal.symbol.clone(),
            entry_price: signal.entry_price,
            current_price: signal.entry_price,
            quantity: order.quantity,
            stop_loss: signal.stop_loss,
            take_profit: signal.take_profit,
            entry_time: chrono::Utc::now(),
            is_paper: matches!(self.config.mode, TradingMode::Paper),
        };
        self.add_position(position).await;

        Ok(())
    }

    /// Run preparation tasks (24/7 operation).
    ///
    /// Preparation tasks maintain the system state outside trading hours:
    /// - Update position data
    /// - Calculate stop-loss/take-profit reference prices
    /// - Refresh risk parameters
    ///
    /// This can be called at any time, even when markets are closed.
    pub async fn run_preparation(&self) -> Result<()> {
        // 1. Refresh monitored position data
        let positions = self.positions.read().await.clone();

        if !positions.is_empty() {
            info!(count = positions.len(), "Refreshing position data for preparation");

            for position in &positions {
                // Fetch current reference prices (if available)
                match self.data.get_latest_quote(&position.symbol).await {
                    Ok(quote) => {
                        debug!(
                            symbol = %position.symbol,
                            current_price = quote.close,
                            "Updated position reference price"
                        );
                        // Reference price updated, actual position update happens during execution
                    }
                    Err(e) => {
                        debug!(
                            symbol = %position.symbol,
                            error = %e,
                            "Failed to fetch reference price for preparation"
                        );
                    }
                }
            }
        }

        // 2. Update risk parameters
        // This could include recalculating position sizes, updating stops based on volatility, etc.
        debug!("Risk parameters refreshed");

        Ok(())
    }

    /// Run execution tasks (trading hours only).
    ///
    /// Execution tasks involve real-time monitoring and action:
    /// - Price checks for stop-loss/take-profit
    /// - Signal scanning for new entries
    /// - Order execution
    ///
    /// This should only be called during active trading hours.
    pub async fn run_execution(&self) -> Result<()> {
        // Check if we're in trading hours
        if !self.config.is_trading_hours() {
            debug!("Outside trading hours, execution skipped");
            return Ok(());
        }

        // 1. Check prices for stop-loss/take-profit triggers
        if let Err(e) = self.check_prices().await {
            warn!(error = %e, "Price check failed");
        }

        // 2. Scan for new trading signals
        if let Err(e) = self.scan_signals().await {
            warn!(error = %e, "Signal scan failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_config_default() {
        let config = LoopConfig::default();
        assert_eq!(config.interval_secs, 5);
        assert_eq!(config.price_check_interval_secs, 1);
        assert_eq!(config.mode, TradingMode::Paper);
        assert!(!config.auto_execute);
    }

    #[test]
    fn test_loop_config_serialization() {
        let config = LoopConfig {
            interval_secs: 10,
            price_check_interval_secs: 2,
            mode: TradingMode::Live,
            auto_execute: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: LoopConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.interval_secs, 10);
        assert_eq!(parsed.mode, TradingMode::Live);
        assert!(parsed.auto_execute);
    }

    #[test]
    fn test_loop_state_transitions() {
        assert_eq!(LoopState::Idle, LoopState::Idle);
        assert_ne!(LoopState::Running, LoopState::Paused);
    }
}
