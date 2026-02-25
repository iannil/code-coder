//! Backtest report generation.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::engine::BacktestConfig;
use super::metrics::{BacktestMetrics, TradeRecord};

/// Backtest report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestReport {
    /// Report title
    pub title: String,
    /// Test period
    pub period: String,
    /// Summary statistics
    pub summary: ReportSummary,
    /// Risk metrics
    pub risk_metrics: RiskMetrics,
    /// Trade statistics
    pub trade_stats: TradeStats,
    /// Text report (formatted)
    pub text_report: String,
}

/// Report summary section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub initial_capital: f64,
    pub final_capital: f64,
    pub net_profit: f64,
    pub total_return_pct: f64,
    pub total_trades: usize,
    pub win_rate: f64,
}

/// Risk metrics section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
}

/// Trade statistics section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeStats {
    pub avg_trade_pnl: f64,
    pub avg_winner: f64,
    pub avg_loser: f64,
    pub avg_holding_days: f64,
    pub total_commission: f64,
}

impl BacktestReport {
    /// Generate a report from backtest results
    pub fn generate(
        config: &BacktestConfig,
        metrics: &BacktestMetrics,
        trades: &[TradeRecord],
        equity_curve: &[(NaiveDate, f64)],
    ) -> Self {
        let final_capital = equity_curve.last()
            .map(|(_, equity)| *equity)
            .unwrap_or(config.initial_capital);

        let summary = ReportSummary {
            initial_capital: config.initial_capital,
            final_capital,
            net_profit: metrics.net_profit,
            total_return_pct: metrics.total_return_pct,
            total_trades: metrics.total_trades,
            win_rate: metrics.win_rate,
        };

        let risk_metrics = RiskMetrics {
            max_drawdown_pct: metrics.max_drawdown_pct,
            sharpe_ratio: metrics.sharpe_ratio,
            profit_factor: metrics.profit_factor,
            expectancy: metrics.expectancy,
        };

        let trade_stats = TradeStats {
            avg_trade_pnl: metrics.avg_trade_pnl,
            avg_winner: metrics.avg_winner,
            avg_loser: metrics.avg_loser,
            avg_holding_days: metrics.avg_holding_days,
            total_commission: metrics.total_commission,
        };

        let text_report = Self::format_text_report(config, &summary, &risk_metrics, &trade_stats, trades);

        Self {
            title: "PO3+SMT 策略回测报告".to_string(),
            period: format!("{} 至 {}", config.start_date, config.end_date),
            summary,
            risk_metrics,
            trade_stats,
            text_report,
        }
    }

    /// Format as text report
    fn format_text_report(
        config: &BacktestConfig,
        summary: &ReportSummary,
        risk: &RiskMetrics,
        stats: &TradeStats,
        trades: &[TradeRecord],
    ) -> String {
        let mut report = String::new();

        report.push_str("═══════════════════════════════════════════════════════════════\n");
        report.push_str("                    PO3+SMT 策略回测报告\n");
        report.push_str("═══════════════════════════════════════════════════════════════\n\n");

        // Test period
        report.push_str(&format!("📅 测试周期: {} 至 {}\n\n", config.start_date, config.end_date));

        // Summary
        report.push_str("📊 收益概要\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  初始资金:       ¥{:>12.2}\n", summary.initial_capital));
        report.push_str(&format!("  最终资金:       ¥{:>12.2}\n", summary.final_capital));
        report.push_str(&format!("  净利润:         ¥{:>12.2}\n", summary.net_profit));
        report.push_str(&format!("  总收益率:       {:>12.2}%\n", summary.total_return_pct));
        report.push_str(&format!("  总交易次数:     {:>12}\n", summary.total_trades));
        report.push_str(&format!("  胜率:           {:>12.2}%\n\n", summary.win_rate));

        // Risk metrics
        report.push_str("⚠️ 风险指标\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  最大回撤:       {:>12.2}%\n", risk.max_drawdown_pct));
        report.push_str(&format!("  夏普比率:       {:>12.2}\n", risk.sharpe_ratio));
        report.push_str(&format!("  盈利因子:       {:>12.2}\n", risk.profit_factor));
        report.push_str(&format!("  期望值:         ¥{:>12.2}\n\n", risk.expectancy));

        // Trade stats
        report.push_str("📈 交易统计\n");
        report.push_str("───────────────────────────────────────────────────────────────\n");
        report.push_str(&format!("  平均盈亏:       ¥{:>12.2}\n", stats.avg_trade_pnl));
        report.push_str(&format!("  平均盈利:       ¥{:>12.2}\n", stats.avg_winner));
        report.push_str(&format!("  平均亏损:       ¥{:>12.2}\n", stats.avg_loser));
        report.push_str(&format!("  平均持仓天数:   {:>12.1}\n", stats.avg_holding_days));
        report.push_str(&format!("  总手续费:       ¥{:>12.2}\n\n", stats.total_commission));

        // Recent trades
        if !trades.is_empty() {
            report.push_str("📝 最近交易\n");
            report.push_str("───────────────────────────────────────────────────────────────\n");

            let recent = trades.iter().rev().take(10);
            for trade in recent {
                let result = if trade.is_winner() { "✅" } else { "❌" };
                report.push_str(&format!(
                    "  {} {} | {} → {} | P&L: ¥{:.2} ({:.1}%)\n",
                    result,
                    trade.symbol,
                    trade.entry_date,
                    trade.exit_date,
                    trade.net_pnl,
                    trade.return_pct()
                ));
            }
        }

        report.push_str("\n═══════════════════════════════════════════════════════════════\n");

        report
    }

    /// Format as Telegram message
    pub fn to_telegram_message(&self) -> String {
        format!(
            "📊 *回测报告*\n\n\
            *测试周期:* {}\n\n\
            *收益概要*\n\
            净利润: ¥{:.2}\n\
            总收益率: {:.2}%\n\
            交易次数: {}\n\
            胜率: {:.2}%\n\n\
            *风险指标*\n\
            最大回撤: {:.2}%\n\
            夏普比率: {:.2}\n\
            盈利因子: {:.2}",
            self.period,
            self.summary.net_profit,
            self.summary.total_return_pct,
            self.summary.total_trades,
            self.summary.win_rate,
            self.risk_metrics.max_drawdown_pct,
            self.risk_metrics.sharpe_ratio,
            self.risk_metrics.profit_factor
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_summary_serialization() {
        let summary = ReportSummary {
            initial_capital: 100000.0,
            final_capital: 110000.0,
            net_profit: 10000.0,
            total_return_pct: 10.0,
            total_trades: 20,
            win_rate: 55.0,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("100000"));
    }

    #[test]
    fn test_risk_metrics_serialization() {
        let risk = RiskMetrics {
            max_drawdown_pct: 5.0,
            sharpe_ratio: 1.5,
            profit_factor: 2.0,
            expectancy: 100.0,
        };

        let json = serde_json::to_string(&risk).unwrap();
        assert!(json.contains("1.5"));
    }
}
