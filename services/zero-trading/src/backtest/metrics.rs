//! Backtest performance metrics.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Trade record for backtest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Symbol traded
    pub symbol: String,
    /// Entry date
    pub entry_date: NaiveDate,
    /// Entry price
    pub entry_price: f64,
    /// Exit date
    pub exit_date: NaiveDate,
    /// Exit price
    pub exit_price: f64,
    /// Quantity
    pub quantity: f64,
    /// Gross P&L
    pub gross_pnl: f64,
    /// Net P&L (after commission)
    pub net_pnl: f64,
    /// Commission paid
    pub commission: f64,
    /// Exit reason (stop_loss, take_profit, end_of_backtest)
    pub exit_reason: String,
}

impl TradeRecord {
    /// Check if trade was profitable
    pub fn is_winner(&self) -> bool {
        self.net_pnl > 0.0
    }

    /// Get return percentage
    pub fn return_pct(&self) -> f64 {
        if self.entry_price > 0.0 {
            ((self.exit_price - self.entry_price) / self.entry_price) * 100.0
        } else {
            0.0
        }
    }

    /// Get holding period in days
    pub fn holding_days(&self) -> i64 {
        (self.exit_date - self.entry_date).num_days()
    }
}

/// Backtest performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestMetrics {
    /// Total number of trades
    pub total_trades: usize,
    /// Number of winning trades
    pub winning_trades: usize,
    /// Number of losing trades
    pub losing_trades: usize,
    /// Win rate (percentage)
    pub win_rate: f64,
    /// Gross profit
    pub gross_profit: f64,
    /// Gross loss
    pub gross_loss: f64,
    /// Net profit
    pub net_profit: f64,
    /// Total return (percentage)
    pub total_return_pct: f64,
    /// Average trade P&L
    pub avg_trade_pnl: f64,
    /// Average winning trade
    pub avg_winner: f64,
    /// Average losing trade
    pub avg_loser: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Maximum drawdown (percentage)
    pub max_drawdown_pct: f64,
    /// Average holding period (days)
    pub avg_holding_days: f64,
    /// Total commission paid
    pub total_commission: f64,
    /// Sharpe ratio (annualized, simplified)
    pub sharpe_ratio: f64,
    /// Expectancy (average profit per trade as % of risk)
    pub expectancy: f64,
}

impl BacktestMetrics {
    /// Calculate metrics from trade records
    pub fn from_trades(trades: &[TradeRecord], initial_capital: f64, max_drawdown: f64) -> Self {
        let total_trades = trades.len();

        if total_trades == 0 {
            return Self::empty();
        }

        let winning_trades: Vec<&TradeRecord> = trades.iter().filter(|t| t.is_winner()).collect();
        let losing_trades: Vec<&TradeRecord> = trades.iter().filter(|t| !t.is_winner()).collect();

        let winning_count = winning_trades.len();
        let losing_count = losing_trades.len();

        let gross_profit: f64 = winning_trades.iter().map(|t| t.net_pnl).sum();
        let gross_loss: f64 = losing_trades.iter().map(|t| t.net_pnl.abs()).sum();
        let net_profit: f64 = trades.iter().map(|t| t.net_pnl).sum();
        let total_commission: f64 = trades.iter().map(|t| t.commission).sum();

        let win_rate = if total_trades > 0 {
            (winning_count as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        let avg_trade_pnl = net_profit / total_trades as f64;

        let avg_winner = if winning_count > 0 {
            gross_profit / winning_count as f64
        } else {
            0.0
        };

        let avg_loser = if losing_count > 0 {
            gross_loss / losing_count as f64
        } else {
            0.0
        };

        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let total_return_pct = (net_profit / initial_capital) * 100.0;

        let avg_holding_days: f64 = trades.iter()
            .map(|t| t.holding_days() as f64)
            .sum::<f64>() / total_trades as f64;

        // Calculate Sharpe ratio (simplified, assuming risk-free rate = 0)
        let returns: Vec<f64> = trades.iter().map(|t| t.return_pct()).collect();
        let sharpe_ratio = Self::calculate_sharpe(&returns);

        // Calculate expectancy
        let expectancy = (win_rate / 100.0 * avg_winner) - ((1.0 - win_rate / 100.0) * avg_loser);

        Self {
            total_trades,
            winning_trades: winning_count,
            losing_trades: losing_count,
            win_rate,
            gross_profit,
            gross_loss,
            net_profit,
            total_return_pct,
            avg_trade_pnl,
            avg_winner,
            avg_loser,
            profit_factor,
            max_drawdown_pct: max_drawdown,
            avg_holding_days,
            total_commission,
            sharpe_ratio,
            expectancy,
        }
    }

    /// Calculate Sharpe ratio from returns
    fn calculate_sharpe(returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }

        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev > 0.0 {
            // Annualize assuming ~250 trading days
            (mean / std_dev) * (250.0_f64).sqrt()
        } else {
            0.0
        }
    }

    /// Create empty metrics
    fn empty() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            net_profit: 0.0,
            total_return_pct: 0.0,
            avg_trade_pnl: 0.0,
            avg_winner: 0.0,
            avg_loser: 0.0,
            profit_factor: 0.0,
            max_drawdown_pct: 0.0,
            avg_holding_days: 0.0,
            total_commission: 0.0,
            sharpe_ratio: 0.0,
            expectancy: 0.0,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_winning_trade() -> TradeRecord {
        TradeRecord {
            symbol: "TEST".to_string(),
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price: 10.0,
            exit_date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            exit_price: 11.0,
            quantity: 100.0,
            gross_pnl: 100.0,
            net_pnl: 97.0,
            commission: 3.0,
            exit_reason: "take_profit".to_string(),
        }
    }

    fn make_losing_trade() -> TradeRecord {
        TradeRecord {
            symbol: "TEST".to_string(),
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price: 10.0,
            exit_date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            exit_price: 9.5,
            quantity: 100.0,
            gross_pnl: -50.0,
            net_pnl: -53.0,
            commission: 3.0,
            exit_reason: "stop_loss".to_string(),
        }
    }

    #[test]
    fn test_trade_record_is_winner() {
        let winner = make_winning_trade();
        let loser = make_losing_trade();

        assert!(winner.is_winner());
        assert!(!loser.is_winner());
    }

    #[test]
    fn test_trade_record_return_pct() {
        let trade = make_winning_trade();
        assert!((trade.return_pct() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_trade_record_holding_days() {
        let trade = make_winning_trade();
        assert_eq!(trade.holding_days(), 1);
    }

    #[test]
    fn test_metrics_from_trades() {
        let trades = vec![
            make_winning_trade(),
            make_losing_trade(),
        ];

        let metrics = BacktestMetrics::from_trades(&trades, 100000.0, 5.0);

        assert_eq!(metrics.total_trades, 2);
        assert_eq!(metrics.winning_trades, 1);
        assert_eq!(metrics.losing_trades, 1);
        assert!((metrics.win_rate - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_empty() {
        let metrics = BacktestMetrics::from_trades(&[], 100000.0, 0.0);
        assert_eq!(metrics.total_trades, 0);
        assert!((metrics.win_rate).abs() < 0.01);
    }

    #[test]
    fn test_profit_factor() {
        let trades = vec![
            make_winning_trade(),
            make_losing_trade(),
        ];

        let metrics = BacktestMetrics::from_trades(&trades, 100000.0, 5.0);

        // profit_factor = gross_profit / gross_loss = 97 / 53 ≈ 1.83
        assert!(metrics.profit_factor > 1.0);
    }
}
