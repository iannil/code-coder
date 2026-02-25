//! Paper trading report generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{PaperTrade, PaperTradeStatus};
use super::validator::ValidationResult;

/// Paper trading session report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTradingReport {
    /// Report title
    pub title: String,
    /// Test period
    pub period: String,
    /// Summary statistics
    pub summary: SessionSummary,
    /// All trades
    pub trades: Vec<PaperTrade>,
    /// Validation results
    pub validations: Vec<ValidationResult>,
}

/// Session summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Starting capital
    pub initial_capital: f64,
    /// Ending capital
    pub final_capital: f64,
    /// Net profit/loss
    pub net_profit: f64,
    /// Total return percentage
    pub total_return_pct: f64,
    /// Total number of trades
    pub total_trades: usize,
    /// Winning trades
    pub winning_trades: usize,
    /// Losing trades
    pub losing_trades: usize,
    /// Win rate percentage
    pub win_rate: f64,
    /// Average winning trade
    pub avg_win: f64,
    /// Average losing trade
    pub avg_loss: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
}

impl SessionSummary {
    /// Calculate summary from trades
    pub fn calculate(
        initial_capital: f64,
        final_capital: f64,
        trades: &[PaperTrade],
        _start_time: DateTime<Utc>,
        _end_time: DateTime<Utc>,
    ) -> Self {
        let total_trades = trades.len();

        let winners: Vec<_> = trades.iter()
            .filter(|t| t.status == PaperTradeStatus::ClosedProfit)
            .collect();
        let losers: Vec<_> = trades.iter()
            .filter(|t| t.status == PaperTradeStatus::ClosedLoss)
            .collect();

        let winning_trades = winners.len();
        let losing_trades = losers.len();

        let gross_profit: f64 = winners.iter()
            .filter_map(|t| t.realized_pnl)
            .filter(|p| *p > 0.0)
            .sum();
        let gross_loss: f64 = losers.iter()
            .filter_map(|t| t.realized_pnl)
            .filter(|p| *p < 0.0)
            .map(|p| p.abs())
            .sum();

        let avg_win = if winning_trades > 0 {
            gross_profit / winning_trades as f64
        } else {
            0.0
        };

        let avg_loss = if losing_trades > 0 {
            -(gross_loss / losing_trades as f64)
        } else {
            0.0
        };

        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
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

        let net_profit = final_capital - initial_capital;
        let total_return_pct = (net_profit / initial_capital) * 100.0;

        // Calculate max drawdown
        let max_drawdown_pct = Self::calculate_max_drawdown(initial_capital, trades);

        Self {
            initial_capital,
            final_capital,
            net_profit,
            total_return_pct,
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown_pct,
        }
    }

    /// Calculate maximum drawdown percentage
    fn calculate_max_drawdown(initial_capital: f64, trades: &[PaperTrade]) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let mut equity = initial_capital;
        let mut peak = equity;
        let mut max_drawdown = 0.0f64;

        for trade in trades {
            if let Some(pnl) = trade.realized_pnl {
                equity += pnl;
                if equity > peak {
                    peak = equity;
                }
                let drawdown = (peak - equity) / peak * 100.0;
                max_drawdown = max_drawdown.max(drawdown);
            }
        }

        max_drawdown
    }
}

impl PaperTradingReport {
    /// Generate text format report
    pub fn to_text_report(&self) -> String {
        let mut report = String::new();

        report.push_str("═══════════════════════════════════════════════════════════════\n");
        report.push_str(&format!("                    {}\n", self.title));
        report.push_str("═══════════════════════════════════════════════════════════════\n\n");

        // Test period
        report.push_str(&format!("📅 测试周期: {}\n\n", self.period));

        // Summary
        report.push_str("📊 收益概要\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  初始资金:       ¥{:>12.2}\n", self.summary.initial_capital));
        report.push_str(&format!("  最终资金:       ¥{:>12.2}\n", self.summary.final_capital));
        report.push_str(&format!("  净利润:         ¥{:>12.2}\n", self.summary.net_profit));
        report.push_str(&format!("  总收益率:       {:>12.2}%\n", self.summary.total_return_pct));
        report.push_str(&format!("  总交易次数:     {:>12}\n", self.summary.total_trades));
        report.push_str(&format!("  胜率:           {:>12.2}%\n\n", self.summary.win_rate));

        // Risk metrics
        report.push_str("⚠️ 风险指标\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  最大回撤:       {:>12.2}%\n", self.summary.max_drawdown_pct));
        report.push_str(&format!("  盈利因子:       {:>12.2}\n", self.summary.profit_factor));
        report.push_str(&format!("  盈利交易:       {:>12}\n", self.summary.winning_trades));
        report.push_str(&format!("  亏损交易:       {:>12}\n\n", self.summary.losing_trades));

        // Trade stats
        report.push_str("📈 交易统计\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  平均盈利:       ¥{:>12.2}\n", self.summary.avg_win));
        report.push_str(&format!("  平均亏损:       ¥{:>12.2}\n\n", self.summary.avg_loss));

        // Recent trades
        if !self.trades.is_empty() {
            report.push_str("📝 交易记录\n");
            report.push_str("───────────────────────────────────────────────────────────────\n");

            for trade in self.trades.iter().take(10) {
                let status_icon = match trade.status {
                    PaperTradeStatus::ClosedProfit => "✅",
                    PaperTradeStatus::ClosedLoss => "❌",
                    PaperTradeStatus::Open => "🔵",
                    PaperTradeStatus::Cancelled => "⚪",
                };
                let pnl = trade.realized_pnl.unwrap_or(0.0);
                report.push_str(&format!(
                    "  {} {} | 入场:{:.2} → 出场:{:.2} | P&L: ¥{:.2}\n",
                    status_icon,
                    trade.symbol,
                    trade.entry_price,
                    trade.exit_price.unwrap_or(trade.entry_price),
                    pnl
                ));
            }
        }

        // Validation summary
        if !self.validations.is_empty() {
            let passed = self.validations.iter().filter(|v| v.is_valid).count();
            let total = self.validations.len();
            let pass_rate = (passed as f64 / total as f64) * 100.0;

            report.push_str("\n🔍 信号验证\n");
            report.push_str("───────────────────────────────────────────────────────────────\n");
            report.push_str(&format!("  信号总数:       {:>12}\n", total));
            report.push_str(&format!("  通过验证:       {:>12}\n", passed));
            report.push_str(&format!("  验证通过率:     {:>12.2}%\n", pass_rate));
        }

        report.push_str("\n═══════════════════════════════════════════════════════════════\n");

        report
    }

    /// Generate Telegram format message
    pub fn to_telegram_message(&self) -> String {
        let profit_emoji = if self.summary.net_profit >= 0.0 { "📈" } else { "📉" };

        format!(
            "{} *模拟交易报告*\n\n\
            *周期:* {}\n\n\
            *收益概要*\n\
            净利润: ¥{:.2}\n\
            收益率: {:.2}%\n\
            交易次数: {}\n\
            胜率: {:.2}%\n\n\
            *风险指标*\n\
            最大回撤: {:.2}%\n\
            盈利因子: {:.2}\n\n\
            ✅ 盈利: {} 笔 | ❌ 亏损: {} 笔",
            profit_emoji,
            self.period,
            self.summary.net_profit,
            self.summary.total_return_pct,
            self.summary.total_trades,
            self.summary.win_rate,
            self.summary.max_drawdown_pct,
            self.summary.profit_factor,
            self.summary.winning_trades,
            self.summary.losing_trades
        )
    }

    /// Check if session meets verification criteria
    pub fn meets_verification_criteria(&self) -> VerificationResult {
        let mut passed = true;
        let mut issues = Vec::new();

        // Check 1: Positive return or minimal loss
        if self.summary.total_return_pct < -5.0 {
            passed = false;
            issues.push(format!(
                "Total return {:.2}% below threshold (-5%)",
                self.summary.total_return_pct
            ));
        }

        // Check 2: Win rate above 40%
        if self.summary.total_trades >= 5 && self.summary.win_rate < 40.0 {
            passed = false;
            issues.push(format!(
                "Win rate {:.2}% below minimum (40%)",
                self.summary.win_rate
            ));
        }

        // Check 3: Maximum drawdown within limits
        if self.summary.max_drawdown_pct > 15.0 {
            passed = false;
            issues.push(format!(
                "Max drawdown {:.2}% exceeds limit (15%)",
                self.summary.max_drawdown_pct
            ));
        }

        // Check 4: Profit factor above 1.0 (if enough trades)
        if self.summary.total_trades >= 5 && self.summary.profit_factor < 1.0 {
            passed = false;
            issues.push(format!(
                "Profit factor {:.2} below 1.0",
                self.summary.profit_factor
            ));
        }

        // Check 5: Minimum trade count
        if self.summary.total_trades < 3 {
            passed = false;
            issues.push(format!(
                "Only {} trades, minimum 3 required",
                self.summary.total_trades
            ));
        }

        VerificationResult {
            passed,
            issues,
            recommendation: if passed {
                "Strategy meets verification criteria. Ready for cautious live trading."
                    .to_string()
            } else {
                "Strategy needs improvement before live trading. Review the issues above."
                    .to_string()
            },
        }
    }
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether verification passed
    pub passed: bool,
    /// Issues found
    pub issues: Vec<String>,
    /// Recommendation
    pub recommendation: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_summary_calculation() {
        let trades = vec![
            PaperTrade {
                id: "1".to_string(),
                symbol: "000001.SZ".to_string(),
                direction: super::super::TradeDirection::Long,
                entry_price: 10.0,
                exit_price: Some(11.0),
                quantity: 100.0,
                entry_time: Utc::now(),
                exit_time: Some(Utc::now()),
                signal_id: "s1".to_string(),
                status: PaperTradeStatus::ClosedProfit,
                realized_pnl: Some(100.0),
            },
            PaperTrade {
                id: "2".to_string(),
                symbol: "000002.SZ".to_string(),
                direction: super::super::TradeDirection::Long,
                entry_price: 10.0,
                exit_price: Some(9.5),
                quantity: 100.0,
                entry_time: Utc::now(),
                exit_time: Some(Utc::now()),
                signal_id: "s2".to_string(),
                status: PaperTradeStatus::ClosedLoss,
                realized_pnl: Some(-50.0),
            },
        ];

        let summary = SessionSummary::calculate(
            100_000.0,
            100_050.0,
            &trades,
            Utc::now(),
            Utc::now(),
        );

        assert_eq!(summary.total_trades, 2);
        assert_eq!(summary.winning_trades, 1);
        assert_eq!(summary.losing_trades, 1);
        assert!((summary.win_rate - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_verification_criteria() {
        let report = PaperTradingReport {
            title: "Test".to_string(),
            period: "Test period".to_string(),
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
            validations: vec![],
        };

        let result = report.meets_verification_criteria();
        assert!(result.passed);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_verification_failure() {
        let report = PaperTradingReport {
            title: "Test".to_string(),
            period: "Test period".to_string(),
            summary: SessionSummary {
                initial_capital: 100_000.0,
                final_capital: 85_000.0,
                net_profit: -15_000.0,
                total_return_pct: -15.0,
                total_trades: 10,
                winning_trades: 2,
                losing_trades: 8,
                win_rate: 20.0,
                avg_win: 500.0,
                avg_loss: -2000.0,
                profit_factor: 0.25,
                max_drawdown_pct: 20.0,
            },
            trades: vec![],
            validations: vec![],
        };

        let result = report.meets_verification_criteria();
        assert!(!result.passed);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_telegram_message() {
        let report = PaperTradingReport {
            title: "Test".to_string(),
            period: "2026-02-25".to_string(),
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
            validations: vec![],
        };

        let msg = report.to_telegram_message();
        assert!(msg.contains("模拟交易报告"));
        assert!(msg.contains("5000"));
    }
}
