//! Paper trading session runner.

use anyhow::Result;
use chrono::{DateTime, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn};
use zero_common::config::Config;

use super::{PaperTrade, PaperTradeStatus, SessionState, TradeDirection};
use super::validator::{SignalValidator, ValidationResult};
use super::report::{PaperTradingReport, SessionSummary};
use crate::data::MarketDataAggregator;
use crate::strategy::{StrategyEngine, TradingSignal, SignalDirection, SignalStrength};
use crate::notification::NotificationClient;

/// Paper trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTradingConfig {
    /// Initial paper capital
    pub initial_capital: f64,
    /// Maximum position size (percentage of capital)
    pub max_position_pct: f64,
    /// Minimum signal strength to trade
    pub min_signal_strength: SignalStrength,
    /// Maximum concurrent positions
    pub max_positions: usize,
    /// Enable Telegram notifications
    pub enable_notifications: bool,
    /// Scan interval in seconds
    pub scan_interval_secs: u64,
    /// Session duration limit (None = unlimited)
    pub max_duration: Option<Duration>,
}

impl Default for PaperTradingConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            max_position_pct: 20.0,
            min_signal_strength: SignalStrength::Medium,
            max_positions: 5,
            enable_notifications: true,
            scan_interval_secs: 60,
            max_duration: None,
        }
    }
}

/// Paper trading session result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Session end time
    pub end_time: DateTime<Utc>,
    /// Final state
    pub final_state: SessionState,
    /// Summary statistics
    pub summary: SessionSummary,
    /// All trades executed
    pub trades: Vec<PaperTrade>,
    /// Signals seen
    pub signals_seen: usize,
    /// Signals traded
    pub signals_traded: usize,
    /// Validation results
    pub validations: Vec<ValidationResult>,
}

/// Paper trading session runner
pub struct PaperTradingRunner {
    /// Configuration
    config: PaperTradingConfig,
    /// Market data aggregator
    data: Arc<MarketDataAggregator>,
    /// Strategy engine
    strategy: Arc<StrategyEngine>,
    /// Signal validator
    validator: SignalValidator,
    /// Notification client
    notification: Option<Arc<NotificationClient>>,
    /// Current session state
    state: Arc<RwLock<SessionState>>,
    /// Current capital
    capital: Arc<RwLock<f64>>,
    /// Open trades
    trades: Arc<RwLock<HashMap<String, PaperTrade>>>,
    /// Closed trades
    closed_trades: Arc<RwLock<Vec<PaperTrade>>>,
    /// Signals seen
    signals_seen: Arc<RwLock<Vec<TradingSignal>>>,
    /// Validation results
    validations: Arc<RwLock<Vec<ValidationResult>>>,
}

impl PaperTradingRunner {
    /// Create a new paper trading runner
    pub fn new(app_config: &Config, config: PaperTradingConfig) -> Self {
        let data = Arc::new(MarketDataAggregator::new(app_config));
        let strategy = Arc::new(StrategyEngine::new(app_config));
        let notification = if config.enable_notifications {
            Some(Arc::new(NotificationClient::new(app_config)))
        } else {
            None
        };

        Self {
            config: config.clone(),
            data,
            strategy,
            validator: SignalValidator::new(),
            notification,
            state: Arc::new(RwLock::new(SessionState::Idle)),
            capital: Arc::new(RwLock::new(config.initial_capital)),
            trades: Arc::new(RwLock::new(HashMap::new())),
            closed_trades: Arc::new(RwLock::new(Vec::new())),
            signals_seen: Arc::new(RwLock::new(Vec::new())),
            validations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Run a paper trading session
    pub async fn run_session(&self, duration: Option<Duration>) -> Result<SessionResult> {
        let start_time = Utc::now();
        let max_duration = duration.or(self.config.max_duration);

        info!(
            initial_capital = self.config.initial_capital,
            max_positions = self.config.max_positions,
            "Starting paper trading session"
        );

        // Set state to running
        {
            let mut state = self.state.write().await;
            *state = SessionState::Running;
        }

        // Send start notification
        if let Some(ref notifier) = self.notification {
            let _ = notifier.send_alert(
                "Paper Trading Started",
                &format!(
                    "Session started with ¥{:.0} capital",
                    self.config.initial_capital
                ),
            ).await;
        }

        // Run the main loop
        let mut scan_interval = interval(Duration::from_secs(self.config.scan_interval_secs));
        let session_start = std::time::Instant::now();

        loop {
            scan_interval.tick().await;

            // Check session duration
            if let Some(max) = max_duration {
                if session_start.elapsed() >= max {
                    info!("Session duration limit reached");
                    break;
                }
            }

            // Check if state changed externally
            let current_state = *self.state.read().await;
            if current_state != SessionState::Running {
                break;
            }

            // Check if within trading hours
            if !self.is_within_trading_hours() {
                continue;
            }

            // Scan for signals
            if let Err(e) = self.scan_and_trade().await {
                warn!(error = %e, "Scan cycle failed");
            }

            // Update open positions
            if let Err(e) = self.update_positions().await {
                warn!(error = %e, "Position update failed");
            }
        }

        // Close all remaining positions
        self.close_all_positions().await?;

        // Build result
        let end_time = Utc::now();
        let final_state = *self.state.read().await;
        let closed = self.closed_trades.read().await.clone();
        let signals = self.signals_seen.read().await.clone();
        let validations = self.validations.read().await.clone();
        let capital = *self.capital.read().await;

        let summary = SessionSummary::calculate(
            self.config.initial_capital,
            capital,
            &closed,
            start_time,
            end_time,
        );

        // Send completion notification
        if let Some(ref notifier) = self.notification {
            let report = PaperTradingReport {
                title: "Paper Trading Session Complete".to_string(),
                period: format!("{} to {}", start_time.format("%Y-%m-%d %H:%M"), end_time.format("%H:%M")),
                summary: summary.clone(),
                trades: closed.clone(),
                validations: validations.clone(),
            };
            let _ = notifier.send_alert("Paper Trading", &report.to_telegram_message()).await;
        }

        Ok(SessionResult {
            start_time,
            end_time,
            final_state,
            summary,
            trades: closed,
            signals_seen: signals.len(),
            signals_traded: self.closed_trades.read().await.len(),
            validations,
        })
    }

    /// Stop the current session
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = SessionState::Completed;
    }

    /// Check if within A-share trading hours
    fn is_within_trading_hours(&self) -> bool {
        let now = Local::now();
        let hour = now.hour();
        let minute = now.minute();

        // Morning session: 9:30 - 11:30
        // Afternoon session: 13:00 - 15:00
        (hour == 9 && minute >= 30) ||
        (hour == 10) ||
        (hour == 11 && minute <= 30) ||
        (hour == 13) ||
        (hour == 14) ||
        (hour == 15 && minute == 0)
    }

    /// Scan for signals and execute paper trades
    async fn scan_and_trade(&self) -> Result<()> {
        // Get signals from strategy engine
        let signals = self.strategy.scan_for_signals(&self.data).await?;

        for signal in signals {
            // Record signal
            {
                let mut seen = self.signals_seen.write().await;
                seen.push(signal.clone());
            }

            // Validate signal
            let validation = self.validator.validate(&signal);
            {
                let mut validations = self.validations.write().await;
                validations.push(validation.clone());
            }

            // Skip if validation failed
            if !validation.is_valid {
                info!(
                    signal_id = %signal.id,
                    reason = %validation.reason,
                    "Signal skipped: validation failed"
                );
                continue;
            }

            // Skip if signal too weak
            if signal.strength < self.config.min_signal_strength {
                info!(
                    signal_id = %signal.id,
                    strength = ?signal.strength,
                    "Signal skipped: too weak"
                );
                continue;
            }

            // Only trade long signals (T+1 constraint)
            if signal.direction != SignalDirection::Long {
                continue;
            }

            // Check if we can open new position
            let trades = self.trades.read().await;
            if trades.len() >= self.config.max_positions {
                info!("Maximum positions reached, skipping signal");
                continue;
            }
            if trades.contains_key(&signal.symbol) {
                info!(symbol = %signal.symbol, "Already have position, skipping");
                continue;
            }
            drop(trades);

            // Execute paper trade
            self.execute_paper_trade(&signal).await?;
        }

        Ok(())
    }

    /// Execute a paper trade
    async fn execute_paper_trade(&self, signal: &TradingSignal) -> Result<()> {
        let mut capital = self.capital.write().await;

        // Calculate position size
        let max_position = *capital * (self.config.max_position_pct / 100.0);
        let risk_amount = (signal.entry_price - signal.stop_loss).abs();
        let risk_per_trade = *capital * 0.02; // 2% risk per trade
        let quantity = (risk_per_trade / risk_amount).floor().max(100.0);
        let position_value = quantity * signal.entry_price;

        if position_value > max_position {
            warn!(
                symbol = %signal.symbol,
                position_value,
                max_position,
                "Position size exceeds limit"
            );
            return Ok(());
        }

        if position_value > *capital {
            warn!(
                symbol = %signal.symbol,
                position_value,
                capital = *capital,
                "Insufficient capital"
            );
            return Ok(());
        }

        // Create paper trade
        let trade = PaperTrade {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: signal.symbol.clone(),
            direction: TradeDirection::Long,
            entry_price: signal.entry_price,
            exit_price: None,
            quantity,
            entry_time: Utc::now(),
            exit_time: None,
            signal_id: signal.id.clone(),
            status: PaperTradeStatus::Open,
            realized_pnl: None,
        };

        // Deduct capital
        *capital -= position_value;

        // Store trade
        {
            let mut trades = self.trades.write().await;
            trades.insert(signal.symbol.clone(), trade.clone());
        }

        info!(
            symbol = %signal.symbol,
            quantity,
            entry_price = signal.entry_price,
            position_value,
            "[PAPER] Opened position"
        );

        // Send notification
        if let Some(ref notifier) = self.notification {
            let _ = notifier.send_signal(signal).await;
        }

        Ok(())
    }

    /// Update open positions with current prices
    async fn update_positions(&self) -> Result<()> {
        let trades = self.trades.read().await;
        let symbols: Vec<String> = trades.keys().cloned().collect();
        drop(trades);

        for symbol in symbols {
            // Get current quote
            let quote = match self.data.get_latest_quote(&symbol).await {
                Ok(q) => q,
                Err(_) => continue,
            };

            // Check if we need to close the position
            let should_close_stop_loss: Option<(PaperTrade, f64)>;
            let should_close_take_profit: Option<(PaperTrade, f64)>;

            {
                let trades = self.trades.read().await;
                if let Some(trade) = trades.get(&symbol) {
                    // Check stop loss
                    if quote.close <= trade.entry_price * 0.95 {
                        should_close_stop_loss = Some((trade.clone(), quote.close));
                        should_close_take_profit = None;
                    } else if quote.close >= trade.entry_price * 1.05 {
                        // Check take profit (5% profit target)
                        should_close_stop_loss = None;
                        should_close_take_profit = Some((trade.clone(), quote.close));
                    } else {
                        should_close_stop_loss = None;
                        should_close_take_profit = None;
                    }
                } else {
                    should_close_stop_loss = None;
                    should_close_take_profit = None;
                }
            }

            // Handle stop loss
            if let Some((mut trade, exit_price)) = should_close_stop_loss {
                trade.close(exit_price);

                let mut trades = self.trades.write().await;
                trades.remove(&symbol);
                drop(trades);

                let mut capital = self.capital.write().await;
                *capital += trade.quantity * exit_price;
                drop(capital);

                let mut closed = self.closed_trades.write().await;
                closed.push(trade.clone());

                info!(
                    symbol = %symbol,
                    exit_price,
                    pnl = trade.realized_pnl,
                    "[PAPER] Stop loss hit"
                );
            }

            // Handle take profit
            if let Some((mut trade, exit_price)) = should_close_take_profit {
                trade.close(exit_price);

                let mut trades = self.trades.write().await;
                trades.remove(&symbol);
                drop(trades);

                let mut capital = self.capital.write().await;
                *capital += trade.quantity * exit_price;
                drop(capital);

                let mut closed = self.closed_trades.write().await;
                closed.push(trade.clone());

                info!(
                    symbol = %symbol,
                    exit_price,
                    pnl = trade.realized_pnl,
                    "[PAPER] Take profit hit"
                );
            }
        }

        Ok(())
    }

    /// Close all remaining positions
    async fn close_all_positions(&self) -> Result<()> {
        let mut trades = self.trades.write().await;
        let symbols: Vec<String> = trades.keys().cloned().collect();

        for symbol in symbols {
            if let Some(mut trade) = trades.remove(&symbol) {
                // Get current price
                let price = self.data.get_latest_quote(&symbol)
                    .await
                    .map(|q| q.close)
                    .unwrap_or(trade.entry_price);

                trade.close(price);

                let mut capital = self.capital.write().await;
                *capital += trade.quantity * price;

                let mut closed = self.closed_trades.write().await;
                closed.push(trade);
            }
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_trading_config_default() {
        let config = PaperTradingConfig::default();
        assert!((config.initial_capital - 100_000.0).abs() < 0.01);
        assert_eq!(config.max_positions, 5);
    }

    #[test]
    fn test_session_result_serialization() {
        let result = SessionResult {
            start_time: Utc::now(),
            end_time: Utc::now(),
            final_state: SessionState::Completed,
            summary: SessionSummary {
                initial_capital: 100_000.0,
                final_capital: 105_000.0,
                net_profit: 5_000.0,
                total_return_pct: 5.0,
                total_trades: 10,
                winning_trades: 6,
                losing_trades: 4,
                win_rate: 60.0,
                avg_win: 1500.0,
                avg_loss: -750.0,
                profit_factor: 2.0,
                max_drawdown_pct: 3.0,
            },
            trades: vec![],
            signals_seen: 15,
            signals_traded: 10,
            validations: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("105000"));
    }
}
