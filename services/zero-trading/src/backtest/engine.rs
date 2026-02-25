//! Backtest engine for strategy simulation.

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

use crate::data::{Candle, Timeframe};
use crate::execution::{Position, OrderStatus};
use crate::strategy::{Po3Detector, SmtDetector, SignalDirection, SignalStrength, TradingSignal};
use super::metrics::{BacktestMetrics, TradeRecord};
use super::report::BacktestReport;

/// Backtest configuration
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// Initial capital
    pub initial_capital: f64,
    /// Risk per trade (percentage)
    pub risk_per_trade_pct: f64,
    /// Maximum positions
    pub max_positions: usize,
    /// Commission per trade (percentage)
    pub commission_pct: f64,
    /// Slippage (percentage)
    pub slippage_pct: f64,
    /// Minimum accumulation bars for PO3
    pub min_accumulation_bars: usize,
    /// Manipulation threshold (ATR multiple)
    pub manipulation_threshold: f64,
    /// Start date for backtest
    pub start_date: NaiveDate,
    /// End date for backtest
    pub end_date: NaiveDate,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100000.0,
            risk_per_trade_pct: 2.0,
            max_positions: 5,
            commission_pct: 0.03, // 0.03% (typical A-share)
            slippage_pct: 0.1,    // 0.1%
            min_accumulation_bars: 5,
            manipulation_threshold: 1.5,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        }
    }
}

/// Backtest result
#[derive(Debug, Clone)]
pub struct BacktestResult {
    /// Configuration used
    pub config: BacktestConfig,
    /// Performance metrics
    pub metrics: BacktestMetrics,
    /// Trade history
    pub trades: Vec<TradeRecord>,
    /// Daily equity curve
    pub equity_curve: Vec<(NaiveDate, f64)>,
    /// Generated report
    pub report: BacktestReport,
}

/// Virtual position for backtesting
#[derive(Debug, Clone)]
struct VirtualPosition {
    symbol: String,
    quantity: f64,
    entry_price: f64,
    entry_date: NaiveDate,
    stop_loss: f64,
    take_profit: f64,
    current_price: f64,
}

impl VirtualPosition {
    fn unrealized_pnl(&self) -> f64 {
        (self.current_price - self.entry_price) * self.quantity
    }

    fn pnl_percent(&self) -> f64 {
        ((self.current_price - self.entry_price) / self.entry_price) * 100.0
    }

    fn can_sell(&self, current_date: NaiveDate) -> bool {
        // T+1: can only sell the day after entry
        current_date > self.entry_date
    }

    fn should_stop_loss(&self) -> bool {
        self.current_price <= self.stop_loss
    }

    fn should_take_profit(&self) -> bool {
        self.current_price >= self.take_profit
    }
}

/// Backtest engine
pub struct BacktestEngine {
    config: BacktestConfig,
    po3_detector: Po3Detector,
    smt_detector: SmtDetector,
}

impl BacktestEngine {
    /// Create a new backtest engine
    pub fn new(config: BacktestConfig) -> Self {
        let po3_detector = Po3Detector::new(
            config.min_accumulation_bars,
            config.manipulation_threshold,
        );
        let smt_detector = SmtDetector::new();

        Self {
            config,
            po3_detector,
            smt_detector,
        }
    }

    /// Run backtest on historical data
    pub fn run(
        &self,
        primary_data: &[Candle],
        reference_data: &[Candle],
    ) -> Result<BacktestResult> {
        let mut capital = self.config.initial_capital;
        let mut positions: HashMap<String, VirtualPosition> = HashMap::new();
        let mut trades: Vec<TradeRecord> = Vec::new();
        let mut equity_curve: Vec<(NaiveDate, f64)> = Vec::new();
        let mut peak_equity = capital;
        let mut max_drawdown = 0.0;

        // Group data by date
        let daily_data = self.group_by_date(primary_data);
        let ref_daily_data = self.group_by_date(reference_data);

        // Iterate through each day
        for (date, day_candles) in &daily_data {
            if *date < self.config.start_date || *date > self.config.end_date {
                continue;
            }

            let last_candle = day_candles.last().ok_or_else(|| anyhow::anyhow!("No candles"))?;

            // Update position prices
            for pos in positions.values_mut() {
                pos.current_price = last_candle.close;
            }

            // Check for exits (T+1 compliant)
            let mut to_close: Vec<String> = Vec::new();
            for (symbol, pos) in &positions {
                if pos.can_sell(*date) {
                    if pos.should_stop_loss() || pos.should_take_profit() {
                        to_close.push(symbol.clone());
                    }
                }
            }

            // Execute exits
            for symbol in to_close {
                if let Some(pos) = positions.remove(&symbol) {
                    let exit_price = pos.current_price * (1.0 - self.config.slippage_pct / 100.0);
                    let gross_pnl = (exit_price - pos.entry_price) * pos.quantity;
                    let commission = (pos.entry_price + exit_price) * pos.quantity * (self.config.commission_pct / 100.0);
                    let net_pnl = gross_pnl - commission;

                    capital += pos.quantity * exit_price - commission;

                    let reason = if pos.should_stop_loss() {
                        "stop_loss"
                    } else {
                        "take_profit"
                    };

                    trades.push(TradeRecord {
                        symbol: pos.symbol.clone(),
                        entry_date: pos.entry_date,
                        entry_price: pos.entry_price,
                        exit_date: *date,
                        exit_price,
                        quantity: pos.quantity,
                        gross_pnl,
                        net_pnl,
                        commission,
                        exit_reason: reason.to_string(),
                    });
                }
            }

            // Check for new signals
            if positions.len() < self.config.max_positions {
                // Build lookback window for signal detection
                let lookback: Vec<Candle> = primary_data
                    .iter()
                    .filter(|c| c.timestamp.date_naive() <= *date)
                    .take(50)
                    .cloned()
                    .collect();

                let ref_lookback: Vec<Candle> = reference_data
                    .iter()
                    .filter(|c| c.timestamp.date_naive() <= *date)
                    .take(50)
                    .cloned()
                    .collect();

                if lookback.len() >= 20 && ref_lookback.len() >= 20 {
                    // Detect PO3 structure
                    if let Some(po3) = self.po3_detector.detect(&lookback) {
                        // Check SMT divergence
                        let smt = self.smt_detector.detect_divergence(&lookback, &ref_lookback);

                        // Only take long signals (short is "avoid" for T+1)
                        if po3.direction == SignalDirection::Long {
                            let signal_strength = self.calculate_strength(&po3, &smt);

                            if signal_strength >= SignalStrength::Medium {
                                // Calculate position size
                                let risk_amount = capital * (self.config.risk_per_trade_pct / 100.0);
                                let risk_per_share = (po3.ideal_entry - po3.stop_loss).abs();
                                let quantity = (risk_amount / risk_per_share).floor();

                                if quantity > 0.0 {
                                    let entry_price = po3.ideal_entry * (1.0 + self.config.slippage_pct / 100.0);
                                    let position_cost = quantity * entry_price;

                                    if position_cost <= capital {
                                        let symbol = last_candle.symbol.clone();
                                        capital -= position_cost;

                                        positions.insert(symbol.clone(), VirtualPosition {
                                            symbol,
                                            quantity,
                                            entry_price,
                                            entry_date: *date,
                                            stop_loss: po3.stop_loss,
                                            take_profit: po3.midpoint,
                                            current_price: entry_price,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Calculate total equity
            let positions_value: f64 = positions.values()
                .map(|p| p.quantity * p.current_price)
                .sum();
            let total_equity = capital + positions_value;

            // Track equity curve
            equity_curve.push((*date, total_equity));

            // Track max drawdown
            if total_equity > peak_equity {
                peak_equity = total_equity;
            }
            let drawdown = (peak_equity - total_equity) / peak_equity * 100.0;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }

        // Close remaining positions at end
        for (_, pos) in positions.drain() {
            let exit_price = pos.current_price;
            let gross_pnl = (exit_price - pos.entry_price) * pos.quantity;
            let commission = (pos.entry_price + exit_price) * pos.quantity * (self.config.commission_pct / 100.0);
            let net_pnl = gross_pnl - commission;

            capital += pos.quantity * exit_price - commission;

            trades.push(TradeRecord {
                symbol: pos.symbol,
                entry_date: pos.entry_date,
                entry_price: pos.entry_price,
                exit_date: self.config.end_date,
                exit_price,
                quantity: pos.quantity,
                gross_pnl,
                net_pnl,
                commission,
                exit_reason: "end_of_backtest".to_string(),
            });
        }

        // Calculate metrics
        let metrics = BacktestMetrics::from_trades(&trades, self.config.initial_capital, max_drawdown);
        let report = BacktestReport::generate(&self.config, &metrics, &trades, &equity_curve);

        Ok(BacktestResult {
            config: self.config.clone(),
            metrics,
            trades,
            equity_curve,
            report,
        })
    }

    /// Group candles by date
    fn group_by_date(&self, candles: &[Candle]) -> Vec<(NaiveDate, Vec<Candle>)> {
        let mut grouped: HashMap<NaiveDate, Vec<Candle>> = HashMap::new();

        for candle in candles {
            let date = candle.timestamp.date_naive();
            grouped.entry(date).or_default().push(candle.clone());
        }

        let mut result: Vec<(NaiveDate, Vec<Candle>)> = grouped.into_iter().collect();
        result.sort_by_key(|(date, _)| *date);
        result
    }

    /// Calculate signal strength
    fn calculate_strength(
        &self,
        po3: &crate::strategy::Po3Structure,
        smt: &Option<crate::strategy::SmtDivergence>,
    ) -> SignalStrength {
        let mut score = 0;

        // PO3 quality
        if po3.manipulation_clear {
            score += 1;
        }
        if po3.distribution_started {
            score += 1;
        }

        // SMT divergence
        if smt.is_some() {
            score += 2;
        }

        match score {
            0..=1 => SignalStrength::Weak,
            2 => SignalStrength::Medium,
            3 => SignalStrength::Strong,
            _ => SignalStrength::VeryStrong,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtest_config_default() {
        let config = BacktestConfig::default();
        assert!((config.initial_capital - 100000.0).abs() < 0.01);
        assert_eq!(config.max_positions, 5);
    }

    #[test]
    fn test_virtual_position_pnl() {
        let pos = VirtualPosition {
            symbol: "TEST".to_string(),
            quantity: 100.0,
            entry_price: 10.0,
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            stop_loss: 9.5,
            take_profit: 11.0,
            current_price: 10.5,
        };

        assert!((pos.unrealized_pnl() - 50.0).abs() < 0.01);
        assert!((pos.pnl_percent() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_virtual_position_t1_rule() {
        let pos = VirtualPosition {
            symbol: "TEST".to_string(),
            quantity: 100.0,
            entry_price: 10.0,
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            stop_loss: 9.5,
            take_profit: 11.0,
            current_price: 10.0,
        };

        // Cannot sell on entry day
        assert!(!pos.can_sell(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        // Can sell next day
        assert!(pos.can_sell(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
    }

    #[test]
    fn test_backtest_engine_creation() {
        let config = BacktestConfig::default();
        let engine = BacktestEngine::new(config);
        assert!(engine.config.initial_capital > 0.0);
    }
}
