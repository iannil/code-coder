//! Red/Yellow Light Signal System.
//!
//! Implements the "Trim Weak" (斩弱) discipline with quantified sell triggers.
//!
//! # Signal Levels
//!
//! - **Red Light** (红灯): Mandatory sell - thesis invalidated or fundamental deterioration
//! - **Yellow Light** (黄灯): Deep review required - underperformance or technical breakdown
//! - **Green Light** (绿灯): Normal hold - continue monitoring
//!
//! # Philosophy
//!
//! "Cutting losses is not admitting defeat - it's freeing capital for better opportunities."
//! The red/yellow light system removes emotion from sell decisions by establishing
//! clear, pre-defined criteria.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::Position;

// ============================================================================
// Signal Types
// ============================================================================

/// Signal level indicating urgency of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalLevel {
    /// Red: Must sell unconditionally
    Red,
    /// Yellow: Must conduct deep review
    Yellow,
    /// Green: Normal hold, continue monitoring
    Green,
}

impl std::fmt::Display for SignalLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Red => write!(f, "红灯"),
            Self::Yellow => write!(f, "黄灯"),
            Self::Green => write!(f, "绿灯"),
        }
    }
}

/// Specific trigger that caused a signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalTrigger {
    // === Red Light Triggers ===
    /// Core investment thesis has been proven wrong
    ThesisInvalidated {
        original_thesis: String,
        invalidation_reason: String,
    },
    /// Key financial metrics show irreversible deterioration
    FundamentalDeterioration {
        metric: String,
        previous_value: f64,
        current_value: f64,
        threshold: f64,
    },
    /// Management integrity compromised (fraud, scandal)
    ManagementIntegrity {
        issue: String,
    },
    /// Industry structural decline (not cyclical)
    IndustryStructuralDecline {
        reason: String,
    },

    // === Yellow Light Triggers ===
    /// Consistently underperforming benchmark
    UnderperformingBenchmark {
        benchmark: String,
        period_months: u32,
        underperformance_pct: f64,
    },
    /// Technical breakdown (e.g., below key moving average)
    TechnicalBreakdown {
        indicator: String,
        current_value: f64,
        threshold_value: f64,
    },
    /// Time stop - held too long without positive return
    TimeStopLoss {
        holding_months: u32,
        max_months: u32,
        return_pct: f64,
    },
    /// Trailing stop triggered
    TrailingStopTriggered {
        high_price: f64,
        current_price: f64,
        drawdown_pct: f64,
        threshold_pct: f64,
    },
    /// Absolute loss threshold breached
    AbsoluteLossThreshold {
        entry_price: f64,
        current_price: f64,
        loss_pct: f64,
        threshold_pct: f64,
    },
    /// Valuation became excessive
    ValuationExcessive {
        metric: String,
        current_value: f64,
        historical_high: f64,
    },
    /// Dividend cut or suspension
    DividendCut {
        previous_dividend: f64,
        current_dividend: f64,
        cut_pct: f64,
    },
}

impl SignalTrigger {
    /// Get the signal level for this trigger.
    pub fn level(&self) -> SignalLevel {
        match self {
            Self::ThesisInvalidated { .. }
            | Self::FundamentalDeterioration { .. }
            | Self::ManagementIntegrity { .. }
            | Self::IndustryStructuralDecline { .. } => SignalLevel::Red,

            Self::UnderperformingBenchmark { .. }
            | Self::TechnicalBreakdown { .. }
            | Self::TimeStopLoss { .. }
            | Self::TrailingStopTriggered { .. }
            | Self::AbsoluteLossThreshold { .. }
            | Self::ValuationExcessive { .. }
            | Self::DividendCut { .. } => SignalLevel::Yellow,
        }
    }

    /// Get human-readable description of the trigger.
    pub fn description(&self) -> String {
        match self {
            Self::ThesisInvalidated {
                original_thesis,
                invalidation_reason,
            } => format!(
                "投资逻辑被证伪：原论点「{}」，证伪原因「{}」",
                original_thesis, invalidation_reason
            ),

            Self::FundamentalDeterioration {
                metric,
                previous_value,
                current_value,
                ..
            } => format!(
                "关键指标恶化：{} 从 {:.1} 降至 {:.1}",
                metric, previous_value, current_value
            ),

            Self::ManagementIntegrity { issue } => {
                format!("管理层诚信问题：{}", issue)
            }

            Self::IndustryStructuralDecline { reason } => {
                format!("行业结构性衰退：{}", reason)
            }

            Self::UnderperformingBenchmark {
                benchmark,
                period_months,
                underperformance_pct,
            } => format!(
                "持续跑输基准：相对 {} {}个月跑输 {:.1}%",
                benchmark, period_months, underperformance_pct
            ),

            Self::TechnicalBreakdown {
                indicator,
                current_value,
                threshold_value,
            } => format!(
                "技术止损：{} 当前值 {:.2} 低于阈值 {:.2}",
                indicator, current_value, threshold_value
            ),

            Self::TimeStopLoss {
                holding_months,
                max_months,
                return_pct,
            } => format!(
                "时间止损：持有 {} 个月（上限 {}个月），收益率 {:.1}%",
                holding_months, max_months, return_pct
            ),

            Self::TrailingStopTriggered {
                high_price,
                current_price,
                drawdown_pct,
                threshold_pct,
            } => format!(
                "移动止损：从高点 {:.2} 回撤至 {:.2}，跌幅 {:.1}%（阈值 {:.1}%）",
                high_price, current_price, drawdown_pct.abs(), threshold_pct.abs()
            ),

            Self::AbsoluteLossThreshold {
                entry_price,
                current_price,
                loss_pct,
                threshold_pct,
            } => format!(
                "绝对止损：从 {:.2} 跌至 {:.2}，亏损 {:.1}%（阈值 {:.1}%）",
                entry_price, current_price, loss_pct.abs(), threshold_pct.abs()
            ),

            Self::ValuationExcessive {
                metric,
                current_value,
                historical_high,
            } => format!(
                "估值过高：{} 当前 {:.1}，历史高点 {:.1}",
                metric, current_value, historical_high
            ),

            Self::DividendCut {
                previous_dividend,
                current_dividend,
                cut_pct,
            } => format!(
                "股息削减：从 {:.2} 降至 {:.2}，削减 {:.1}%",
                previous_dividend, current_dividend, cut_pct
            ),
        }
    }
}

/// Recommended action based on signal assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecommendedAction {
    /// Sell immediately
    SellImmediately,
    /// Conduct deep review before decision
    DeepReview,
    /// Consider trimming position
    ConsiderTrimming,
    /// Hold and monitor
    HoldAndMonitor,
    /// Consider adding (on dips)
    ConsiderAdding,
}

impl std::fmt::Display for RecommendedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SellImmediately => write!(f, "立即卖出"),
            Self::DeepReview => write!(f, "深度复盘"),
            Self::ConsiderTrimming => write!(f, "考虑减仓"),
            Self::HoldAndMonitor => write!(f, "持有观察"),
            Self::ConsiderAdding => write!(f, "考虑加仓"),
        }
    }
}

// ============================================================================
// Signal Assessment
// ============================================================================

/// Complete signal assessment for a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAssessment {
    /// Symbol being assessed
    pub symbol: String,
    /// Overall signal level (worst of all triggers)
    pub level: SignalLevel,
    /// All triggered signals
    pub triggers: Vec<SignalTrigger>,
    /// Recommended action
    pub recommended_action: RecommendedAction,
    /// Detailed reasoning
    pub reasoning: String,
    /// Assessment timestamp
    pub assessed_at: DateTime<Utc>,
}

impl SignalAssessment {
    /// Create a green light assessment (no triggers).
    pub fn green(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            level: SignalLevel::Green,
            triggers: Vec::new(),
            recommended_action: RecommendedAction::HoldAndMonitor,
            reasoning: "无触发条件，正常持有".to_string(),
            assessed_at: Utc::now(),
        }
    }

    /// Check if immediate action is required.
    pub fn requires_immediate_action(&self) -> bool {
        self.level == SignalLevel::Red
    }

    /// Check if review is needed.
    pub fn requires_review(&self) -> bool {
        matches!(self.level, SignalLevel::Red | SignalLevel::Yellow)
    }
}

// ============================================================================
// Signal Analyzer
// ============================================================================

/// Configuration for signal analysis.
#[derive(Debug, Clone)]
pub struct SignalConfig {
    /// Absolute loss threshold for yellow light (%)
    pub absolute_loss_threshold: f64,
    /// Trailing stop threshold (%)
    pub trailing_stop_threshold: f64,
    /// Time stop threshold (months)
    pub time_stop_months: u32,
    /// Underperformance threshold (% relative to benchmark)
    pub underperformance_threshold: f64,
    /// Underperformance period (months)
    pub underperformance_period_months: u32,
    /// Dividend cut threshold (%)
    pub dividend_cut_threshold: f64,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            absolute_loss_threshold: -25.0,
            trailing_stop_threshold: -15.0,
            time_stop_months: 24,
            underperformance_threshold: -10.0,
            underperformance_period_months: 12,
            dividend_cut_threshold: -20.0,
        }
    }
}

/// Signal analyzer for positions.
pub struct SignalAnalyzer {
    config: SignalConfig,
}

impl SignalAnalyzer {
    /// Create a new signal analyzer.
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    /// Analyze a position for signals.
    pub fn analyze(&self, position: &Position, context: &SignalContext) -> SignalAssessment {
        let mut triggers = Vec::new();

        // Check absolute loss
        let loss_pct = position.unrealized_pnl_pct();
        if loss_pct <= self.config.absolute_loss_threshold {
            triggers.push(SignalTrigger::AbsoluteLossThreshold {
                entry_price: position.entry_price,
                current_price: position.current_price,
                loss_pct,
                threshold_pct: self.config.absolute_loss_threshold,
            });
        }

        // Check trailing stop
        let drawdown = position.drawdown_from_high();
        if drawdown <= self.config.trailing_stop_threshold {
            triggers.push(SignalTrigger::TrailingStopTriggered {
                high_price: position.high_since_entry,
                current_price: position.current_price,
                drawdown_pct: drawdown,
                threshold_pct: self.config.trailing_stop_threshold,
            });
        }

        // Check time stop
        let holding_months = calculate_holding_months(position.entry_date);
        if holding_months >= self.config.time_stop_months && loss_pct < 0.0 {
            triggers.push(SignalTrigger::TimeStopLoss {
                holding_months,
                max_months: self.config.time_stop_months,
                return_pct: loss_pct,
            });
        }

        // Check benchmark underperformance
        if let Some(ref benchmark_data) = context.benchmark_performance {
            let relative_perf = loss_pct - benchmark_data.return_pct;
            if relative_perf <= self.config.underperformance_threshold
                && benchmark_data.period_months >= self.config.underperformance_period_months
            {
                triggers.push(SignalTrigger::UnderperformingBenchmark {
                    benchmark: benchmark_data.benchmark_name.clone(),
                    period_months: benchmark_data.period_months,
                    underperformance_pct: relative_perf,
                });
            }
        }

        // Check technical breakdown
        if let Some(ref technical) = context.technical_data {
            if position.current_price < technical.ma_200 {
                triggers.push(SignalTrigger::TechnicalBreakdown {
                    indicator: "200日均线".to_string(),
                    current_value: position.current_price,
                    threshold_value: technical.ma_200,
                });
            }
        }

        // Check dividend cut
        if let Some(ref dividend_data) = context.dividend_data {
            if dividend_data.current_dividend < dividend_data.previous_dividend {
                let cut_pct = ((dividend_data.current_dividend - dividend_data.previous_dividend)
                    / dividend_data.previous_dividend)
                    * 100.0;
                if cut_pct <= self.config.dividend_cut_threshold {
                    triggers.push(SignalTrigger::DividendCut {
                        previous_dividend: dividend_data.previous_dividend,
                        current_dividend: dividend_data.current_dividend,
                        cut_pct,
                    });
                }
            }
        }

        // Add any thesis invalidation from context
        if let Some(ref invalidation) = context.thesis_invalidation {
            triggers.push(SignalTrigger::ThesisInvalidated {
                original_thesis: position.investment_thesis.clone(),
                invalidation_reason: invalidation.clone(),
            });
        }

        // Determine overall level
        let level = if triggers.is_empty() {
            SignalLevel::Green
        } else {
            triggers
                .iter()
                .map(|t| t.level())
                .min_by_key(|l| match l {
                    SignalLevel::Red => 0,
                    SignalLevel::Yellow => 1,
                    SignalLevel::Green => 2,
                })
                .unwrap_or(SignalLevel::Green)
        };

        // Determine recommended action
        let recommended_action = match level {
            SignalLevel::Red => RecommendedAction::SellImmediately,
            SignalLevel::Yellow if triggers.len() >= 3 => RecommendedAction::ConsiderTrimming,
            SignalLevel::Yellow => RecommendedAction::DeepReview,
            SignalLevel::Green if loss_pct > 20.0 => RecommendedAction::ConsiderAdding,
            SignalLevel::Green => RecommendedAction::HoldAndMonitor,
        };

        // Generate reasoning
        let reasoning = if triggers.is_empty() {
            "无触发条件，正常持有。".to_string()
        } else {
            let trigger_descriptions: Vec<String> =
                triggers.iter().map(|t| t.description()).collect();
            format!(
                "触发 {} 信号，共 {} 项：\n{}",
                level,
                triggers.len(),
                trigger_descriptions.join("\n")
            )
        };

        SignalAssessment {
            symbol: position.symbol.clone(),
            level,
            triggers,
            recommended_action,
            reasoning,
            assessed_at: Utc::now(),
        }
    }

    /// Batch analyze multiple positions.
    pub fn analyze_batch(
        &self,
        positions: &[Position],
        contexts: &[SignalContext],
    ) -> Vec<SignalAssessment> {
        positions
            .iter()
            .zip(contexts.iter())
            .map(|(pos, ctx)| self.analyze(pos, ctx))
            .collect()
    }
}

impl Default for SignalAnalyzer {
    fn default() -> Self {
        Self::new(SignalConfig::default())
    }
}

// ============================================================================
// Context Data
// ============================================================================

/// Additional context for signal analysis.
#[derive(Debug, Clone, Default)]
pub struct SignalContext {
    /// Benchmark performance data
    pub benchmark_performance: Option<BenchmarkData>,
    /// Technical analysis data
    pub technical_data: Option<TechnicalData>,
    /// Dividend data
    pub dividend_data: Option<DividendData>,
    /// Manual thesis invalidation note
    pub thesis_invalidation: Option<String>,
}

/// Benchmark performance data.
#[derive(Debug, Clone)]
pub struct BenchmarkData {
    pub benchmark_name: String,
    pub return_pct: f64,
    pub period_months: u32,
}

/// Technical analysis data.
#[derive(Debug, Clone)]
pub struct TechnicalData {
    pub ma_200: f64,
    pub ma_50: f64,
    pub rsi: Option<f64>,
}

/// Dividend data.
#[derive(Debug, Clone)]
pub struct DividendData {
    pub previous_dividend: f64,
    pub current_dividend: f64,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_holding_months(entry_date: DateTime<Utc>) -> u32 {
    let duration = Utc::now() - entry_date;
    (duration.num_days() / 30) as u32
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_test_position(
        symbol: &str,
        entry: f64,
        current: f64,
        high: f64,
        entry_date: DateTime<Utc>,
    ) -> Position {
        use super::super::types::*;
        Position {
            symbol: symbol.to_string(),
            name: format!("Test {}", symbol),
            tier: PoolTier::Core,
            entry_price: entry,
            current_price: current,
            quantity: 100.0,
            weight: 10.0,
            entry_date,
            investment_thesis: "Test thesis".to_string(),
            key_metrics: vec![],
            stop_loss_triggers: StopLossTriggers::default(),
            high_since_entry: high,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_green_signal() {
        let analyzer = SignalAnalyzer::default();
        let position = make_test_position("TEST", 100.0, 110.0, 115.0, Utc::now());
        let context = SignalContext::default();

        let assessment = analyzer.analyze(&position, &context);

        assert_eq!(assessment.level, SignalLevel::Green);
        assert!(assessment.triggers.is_empty());
    }

    #[test]
    fn test_absolute_loss_trigger() {
        let analyzer = SignalAnalyzer::new(SignalConfig {
            absolute_loss_threshold: -20.0,
            ..Default::default()
        });

        // 30% loss
        let position = make_test_position("TEST", 100.0, 70.0, 100.0, Utc::now());
        let context = SignalContext::default();

        let assessment = analyzer.analyze(&position, &context);

        assert_eq!(assessment.level, SignalLevel::Yellow);
        assert!(assessment.triggers.iter().any(|t| {
            matches!(t, SignalTrigger::AbsoluteLossThreshold { .. })
        }));
    }

    #[test]
    fn test_trailing_stop_trigger() {
        let analyzer = SignalAnalyzer::new(SignalConfig {
            trailing_stop_threshold: -15.0,
            ..Default::default()
        });

        // 20% drawdown from high
        let position = make_test_position("TEST", 100.0, 80.0, 100.0, Utc::now());
        let context = SignalContext::default();

        let assessment = analyzer.analyze(&position, &context);

        assert!(assessment.triggers.iter().any(|t| {
            matches!(t, SignalTrigger::TrailingStopTriggered { .. })
        }));
    }

    #[test]
    fn test_time_stop_trigger() {
        let analyzer = SignalAnalyzer::new(SignalConfig {
            time_stop_months: 24,
            ..Default::default()
        });

        // Held for 30 months with negative return
        let entry_date = Utc::now() - Duration::days(30 * 30);
        let position = make_test_position("TEST", 100.0, 95.0, 110.0, entry_date);
        let context = SignalContext::default();

        let assessment = analyzer.analyze(&position, &context);

        assert!(assessment.triggers.iter().any(|t| {
            matches!(t, SignalTrigger::TimeStopLoss { .. })
        }));
    }

    #[test]
    fn test_thesis_invalidation_red_signal() {
        let analyzer = SignalAnalyzer::default();
        let position = make_test_position("TEST", 100.0, 100.0, 100.0, Utc::now());
        let context = SignalContext {
            thesis_invalidation: Some("Management fraud discovered".to_string()),
            ..Default::default()
        };

        let assessment = analyzer.analyze(&position, &context);

        assert_eq!(assessment.level, SignalLevel::Red);
        assert_eq!(assessment.recommended_action, RecommendedAction::SellImmediately);
    }

    #[test]
    fn test_technical_breakdown() {
        let analyzer = SignalAnalyzer::default();
        let position = make_test_position("TEST", 100.0, 95.0, 110.0, Utc::now());
        let context = SignalContext {
            technical_data: Some(TechnicalData {
                ma_200: 100.0, // Price below 200 MA
                ma_50: 98.0,
                rsi: Some(35.0),
            }),
            ..Default::default()
        };

        let assessment = analyzer.analyze(&position, &context);

        assert!(assessment.triggers.iter().any(|t| {
            matches!(t, SignalTrigger::TechnicalBreakdown { .. })
        }));
    }

    #[test]
    fn test_signal_trigger_descriptions() {
        let trigger = SignalTrigger::AbsoluteLossThreshold {
            entry_price: 100.0,
            current_price: 70.0,
            loss_pct: -30.0,
            threshold_pct: -25.0,
        };

        let desc = trigger.description();
        assert!(desc.contains("绝对止损"));
        assert!(desc.contains("100"));
        assert!(desc.contains("70"));
    }
}
