//! Trading review and journaling system.
//!
//! Provides:
//! - Trading diary for recording trades and decisions
//! - Scheduled review reminders via ZeroBot cron
//! - Pattern analysis and insights extraction
//! - Integration with @trader and @decision agents

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Types
// ============================================================================

/// Trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeDirection {
    Long,
    Short,
}

/// Trade outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeOutcome {
    Win,
    Loss,
    BreakEven,
    Open,
}

/// Asset class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    Stock,
    Futures,
    Forex,
    Crypto,
    Options,
    Bond,
    Commodity,
    Other,
}

impl std::fmt::Display for AssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stock => write!(f, "股票"),
            Self::Futures => write!(f, "期货"),
            Self::Forex => write!(f, "外汇"),
            Self::Crypto => write!(f, "加密货币"),
            Self::Options => write!(f, "期权"),
            Self::Bond => write!(f, "债券"),
            Self::Commodity => write!(f, "商品"),
            Self::Other => write!(f, "其他"),
        }
    }
}

/// A single trade entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEntry {
    /// Unique trade ID
    pub id: String,
    /// Symbol/ticker
    pub symbol: String,
    /// Asset class
    pub asset_class: AssetClass,
    /// Trade direction
    pub direction: TradeDirection,
    /// Entry price
    pub entry_price: f64,
    /// Exit price (if closed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_price: Option<f64>,
    /// Quantity/size
    pub quantity: f64,
    /// Entry timestamp
    pub entry_time: DateTime<Utc>,
    /// Exit timestamp (if closed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_time: Option<DateTime<Utc>>,
    /// Stop loss price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<f64>,
    /// Take profit price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<f64>,
    /// Trade outcome
    pub outcome: TradeOutcome,
    /// Profit/Loss amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl: Option<f64>,
    /// Strategy or setup name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// Reasoning for entry
    pub entry_reason: String,
    /// Reasoning for exit (if closed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Screenshots or attachments
    #[serde(default)]
    pub attachments: Vec<String>,
    /// Custom notes
    #[serde(default)]
    pub notes: String,
}

impl TradeEntry {
    /// Calculate return percentage.
    pub fn return_percent(&self) -> Option<f64> {
        let exit = self.exit_price?;
        let multiplier = match self.direction {
            TradeDirection::Long => 1.0,
            TradeDirection::Short => -1.0,
        };
        Some(((exit - self.entry_price) / self.entry_price) * 100.0 * multiplier)
    }

    /// Calculate holding period in hours.
    pub fn holding_hours(&self) -> Option<f64> {
        let exit = self.exit_time?;
        let duration = exit.signed_duration_since(self.entry_time);
        Some(duration.num_minutes() as f64 / 60.0)
    }
}

/// A daily trading journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Date
    pub date: NaiveDate,
    /// Daily summary
    pub summary: String,
    /// Market conditions/context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_context: Option<String>,
    /// Emotional state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotional_state: Option<String>,
    /// Lessons learned
    #[serde(default)]
    pub lessons: Vec<String>,
    /// Goals for next day
    #[serde(default)]
    pub next_day_goals: Vec<String>,
    /// Mistakes made
    #[serde(default)]
    pub mistakes: Vec<String>,
    /// What went well
    #[serde(default)]
    pub wins: Vec<String>,
    /// Overall score (1-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Review period type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

/// A trading review/analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingReview {
    /// Review ID
    pub id: String,
    /// Review period type
    pub period: ReviewPeriod,
    /// Start date
    pub start_date: NaiveDate,
    /// End date
    pub end_date: NaiveDate,
    /// Summary statistics
    pub stats: ReviewStats,
    /// Analysis text (generated or manual)
    pub analysis: String,
    /// Patterns identified
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Areas for improvement
    #[serde(default)]
    pub improvements: Vec<String>,
    /// Goals for next period
    #[serde(default)]
    pub goals: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Review statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewStats {
    /// Total trades
    pub total_trades: usize,
    /// Winning trades
    pub winning_trades: usize,
    /// Losing trades
    pub losing_trades: usize,
    /// Win rate percentage
    pub win_rate: f64,
    /// Total P&L
    pub total_pnl: f64,
    /// Average win
    pub avg_win: f64,
    /// Average loss
    pub avg_loss: f64,
    /// Risk/reward ratio
    pub risk_reward_ratio: f64,
    /// Largest win
    pub largest_win: f64,
    /// Largest loss
    pub largest_loss: f64,
    /// Average holding time (hours)
    pub avg_holding_hours: f64,
    /// Most traded symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_traded_symbol: Option<String>,
    /// Best performing strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_strategy: Option<String>,
}

impl ReviewStats {
    /// Calculate from trades.
    pub fn from_trades(trades: &[TradeEntry]) -> Self {
        let closed_trades: Vec<_> = trades
            .iter()
            .filter(|t| t.outcome != TradeOutcome::Open)
            .collect();

        if closed_trades.is_empty() {
            return Self::default();
        }

        let wins: Vec<_> = closed_trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Win)
            .collect();
        let losses: Vec<_> = closed_trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Loss)
            .collect();

        let total = closed_trades.len();
        let win_count = wins.len();
        let loss_count = losses.len();

        let total_pnl: f64 = closed_trades.iter().filter_map(|t| t.pnl).sum();

        let avg_win = if win_count > 0 {
            wins.iter().filter_map(|t| t.pnl).sum::<f64>() / win_count as f64
        } else {
            0.0
        };

        let avg_loss = if loss_count > 0 {
            losses.iter().filter_map(|t| t.pnl).sum::<f64>().abs() / loss_count as f64
        } else {
            0.0
        };

        let risk_reward = if avg_loss > 0.0 {
            avg_win / avg_loss
        } else {
            0.0
        };

        let largest_win = closed_trades
            .iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p > 0.0)
            .fold(0.0_f64, f64::max);

        let largest_loss = closed_trades
            .iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p < 0.0)
            .fold(0.0_f64, f64::min);

        let avg_holding = {
            let holding_times: Vec<f64> = closed_trades
                .iter()
                .filter_map(|t| t.holding_hours())
                .collect();
            if holding_times.is_empty() {
                0.0
            } else {
                holding_times.iter().sum::<f64>() / holding_times.len() as f64
            }
        };

        // Find most traded symbol
        let mut symbol_counts: HashMap<&str, usize> = HashMap::new();
        for trade in &closed_trades {
            *symbol_counts.entry(&trade.symbol).or_insert(0) += 1;
        }
        let most_traded = symbol_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(sym, _)| sym.to_string());

        // Find best strategy
        let mut strategy_pnl: HashMap<&str, f64> = HashMap::new();
        for trade in &closed_trades {
            if let Some(ref strategy) = trade.strategy {
                *strategy_pnl.entry(strategy.as_str()).or_insert(0.0) +=
                    trade.pnl.unwrap_or(0.0);
            }
        }
        let best_strategy = strategy_pnl
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|(_, pnl)| *pnl > 0.0)
            .map(|(s, _)| s.to_string());

        Self {
            total_trades: total,
            winning_trades: win_count,
            losing_trades: loss_count,
            win_rate: (win_count as f64 / total as f64) * 100.0,
            total_pnl,
            avg_win,
            avg_loss,
            risk_reward_ratio: risk_reward,
            largest_win,
            largest_loss,
            avg_holding_hours: avg_holding,
            most_traded_symbol: most_traded,
            best_strategy,
        }
    }
}

/// Reminder schedule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderSchedule {
    /// Enable daily review reminders
    pub daily_enabled: bool,
    /// Time for daily reminder (HH:MM format)
    pub daily_time: String,
    /// Days for daily reminder (0=Sun, 6=Sat)
    #[serde(default = "default_weekdays")]
    pub daily_days: Vec<u8>,
    /// Enable weekly review reminders
    pub weekly_enabled: bool,
    /// Day and time for weekly reminder
    pub weekly_day_time: String, // e.g., "Sat 10:00"
    /// Enable monthly review reminders
    pub monthly_enabled: bool,
    /// Day of month for monthly reminder
    pub monthly_day: u8,
}

fn default_weekdays() -> Vec<u8> {
    vec![1, 2, 3, 4, 5] // Monday to Friday
}

impl Default for ReminderSchedule {
    fn default() -> Self {
        Self {
            daily_enabled: true,
            daily_time: "21:00".to_string(),
            daily_days: default_weekdays(),
            weekly_enabled: true,
            weekly_day_time: "Sat 10:00".to_string(),
            monthly_enabled: true,
            monthly_day: 1,
        }
    }
}

impl ReminderSchedule {
    /// Convert to cron expressions for scheduler registration.
    pub fn to_cron_expressions(&self) -> Vec<(String, ReviewPeriod, String)> {
        let mut crons = Vec::new();

        // Daily reminder - e.g., "0 21 * * 1-5" for weekdays at 21:00
        if self.daily_enabled {
            let (hour, minute) = self.parse_time(&self.daily_time);
            let days = self.format_days(&self.daily_days);
            let expr = format!("{minute} {hour} * * {days}");
            crons.push((expr, ReviewPeriod::Daily, "trading.review.daily".to_string()));
        }

        // Weekly reminder - e.g., "0 10 * * 6" for Saturday at 10:00
        if self.weekly_enabled {
            if let Some((day, hour, minute)) = self.parse_day_time(&self.weekly_day_time) {
                let expr = format!("{minute} {hour} * * {day}");
                crons.push((expr, ReviewPeriod::Weekly, "trading.review.weekly".to_string()));
            }
        }

        // Monthly reminder - e.g., "0 9 1 * *" for 1st of each month at 09:00
        if self.monthly_enabled {
            let expr = format!("0 9 {} * *", self.monthly_day);
            crons.push((expr, ReviewPeriod::Monthly, "trading.review.monthly".to_string()));
        }

        crons
    }

    fn parse_time(&self, time: &str) -> (u8, u8) {
        let parts: Vec<&str> = time.split(':').collect();
        let hour = parts.first().and_then(|h| h.parse().ok()).unwrap_or(21);
        let minute = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
        (hour, minute)
    }

    fn format_days(&self, days: &[u8]) -> String {
        if days.is_empty() {
            return "*".to_string();
        }
        days.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn parse_day_time(&self, day_time: &str) -> Option<(u8, u8, u8)> {
        // Format: "Sat 10:00" or "6 10:00"
        let parts: Vec<&str> = day_time.split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }

        let day = match parts[0].to_lowercase().as_str() {
            "sun" | "0" => 0,
            "mon" | "1" => 1,
            "tue" | "2" => 2,
            "wed" | "3" => 3,
            "thu" | "4" => 4,
            "fri" | "5" => 5,
            "sat" | "6" => 6,
            _ => return None,
        };

        let time_parts: Vec<&str> = parts[1].split(':').collect();
        let hour = time_parts.first()?.parse().ok()?;
        let minute = time_parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);

        Some((day, hour, minute))
    }
}

/// Notification channel configuration for trading reminders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel type (telegram, feishu, dingtalk, wecom, email)
    pub channel_type: String,
    /// Channel ID or recipient
    pub channel_id: String,
    /// Whether this channel is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Multi-channel notification configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// List of notification channels
    #[serde(default)]
    pub channels: Vec<NotificationChannel>,
    /// Include P&L summary in notifications
    #[serde(default = "default_true")]
    pub include_pnl_summary: bool,
    /// Include open positions count
    #[serde(default = "default_true")]
    pub include_open_positions: bool,
}

// ============================================================================
// Trading Review System
// ============================================================================

/// Trading review system.
pub struct TradingReviewSystem {
    /// HTTP client
    client: reqwest::Client,
    /// CodeCoder endpoint (for LLM analysis)
    codecoder_endpoint: String,
    /// Storage path for trading data
    #[allow(dead_code)] // Reserved for file-based persistence
    storage_path: PathBuf,
    /// Trades in memory
    trades: Arc<tokio::sync::RwLock<Vec<TradeEntry>>>,
    /// Journal entries in memory
    journals: Arc<tokio::sync::RwLock<HashMap<NaiveDate, JournalEntry>>>,
    /// Reviews
    reviews: Arc<tokio::sync::RwLock<Vec<TradingReview>>>,
    /// Channels endpoint for notifications
    channels_endpoint: Option<String>,
    /// Notification configuration (multi-channel)
    notification_config: NotificationConfig,
    /// Reminder schedule
    reminder_schedule: ReminderSchedule,
}

impl TradingReviewSystem {
    /// Create a new trading review system.
    pub fn new(codecoder_endpoint: impl Into<String>, storage_path: impl Into<PathBuf>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            codecoder_endpoint: codecoder_endpoint.into(),
            storage_path: storage_path.into(),
            trades: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            journals: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            reviews: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            channels_endpoint: None,
            notification_config: NotificationConfig::default(),
            reminder_schedule: ReminderSchedule::default(),
        }
    }

    /// Set channels endpoint.
    pub fn with_channels_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.channels_endpoint = Some(endpoint.into());
        self
    }

    /// Set notification configuration (multi-channel support).
    pub fn with_notification_config(mut self, config: NotificationConfig) -> Self {
        self.notification_config = config;
        self
    }

    /// Set reminder schedule.
    pub fn with_reminder_schedule(mut self, schedule: ReminderSchedule) -> Self {
        self.reminder_schedule = schedule;
        self
    }

    /// Add a notification channel.
    pub fn add_notification_channel(&mut self, channel_type: impl Into<String>, channel_id: impl Into<String>) {
        self.notification_config.channels.push(NotificationChannel {
            channel_type: channel_type.into(),
            channel_id: channel_id.into(),
            enabled: true,
        });
    }

    /// Get cron tasks for scheduler registration.
    pub fn get_cron_tasks(&self) -> Vec<zero_common::config::CronTask> {
        self.reminder_schedule
            .to_cron_expressions()
            .into_iter()
            .map(|(expr, _period, id)| zero_common::config::CronTask {
                id,
                expression: expr,
                command: "trading-review".to_string(),
                description: Some("Trading review reminder".to_string()),
            })
            .collect()
    }

    /// Record a new trade.
    pub async fn record_trade(&self, trade: TradeEntry) -> Result<()> {
        let mut trades = self.trades.write().await;
        trades.push(trade);
        Ok(())
    }

    /// Update an existing trade (e.g., close it).
    pub async fn update_trade(
        &self,
        trade_id: &str,
        exit_price: f64,
        exit_reason: Option<String>,
    ) -> Result<()> {
        let mut trades = self.trades.write().await;

        for trade in trades.iter_mut() {
            if trade.id == trade_id {
                trade.exit_price = Some(exit_price);
                trade.exit_time = Some(Utc::now());
                trade.exit_reason = exit_reason;

                // Calculate P&L
                let direction_multiplier = match trade.direction {
                    TradeDirection::Long => 1.0,
                    TradeDirection::Short => -1.0,
                };
                let pnl = (exit_price - trade.entry_price) * trade.quantity * direction_multiplier;
                trade.pnl = Some(pnl);

                // Determine outcome
                trade.outcome = if pnl > 0.0 {
                    TradeOutcome::Win
                } else if pnl < 0.0 {
                    TradeOutcome::Loss
                } else {
                    TradeOutcome::BreakEven
                };

                return Ok(());
            }
        }

        anyhow::bail!("Trade not found: {}", trade_id)
    }

    /// Get trades for a date range.
    pub async fn get_trades(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<TradeEntry> {
        let trades = self.trades.read().await;
        trades
            .iter()
            .filter(|t| {
                let entry_date = t.entry_time.date_naive();
                entry_date >= start && entry_date <= end
            })
            .cloned()
            .collect()
    }

    /// Save or update journal entry.
    pub async fn save_journal(&self, entry: JournalEntry) -> Result<()> {
        let mut journals = self.journals.write().await;
        journals.insert(entry.date, entry);
        Ok(())
    }

    /// Get journal entry for a date.
    pub async fn get_journal(&self, date: NaiveDate) -> Option<JournalEntry> {
        let journals = self.journals.read().await;
        journals.get(&date).cloned()
    }

    /// Generate a review for a period.
    pub async fn generate_review(&self, period: ReviewPeriod) -> Result<TradingReview> {
        let today = Utc::now().date_naive();

        let (start_date, end_date) = match period {
            ReviewPeriod::Daily => (today, today),
            ReviewPeriod::Weekly => {
                let start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
                (start, today)
            }
            ReviewPeriod::Monthly => {
                let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                    .unwrap_or(today);
                (start, today)
            }
            ReviewPeriod::Quarterly => {
                let quarter_start_month = ((today.month() - 1) / 3) * 3 + 1;
                let start = NaiveDate::from_ymd_opt(today.year(), quarter_start_month, 1)
                    .unwrap_or(today);
                (start, today)
            }
            ReviewPeriod::Yearly => {
                let start = NaiveDate::from_ymd_opt(today.year(), 1, 1)
                    .unwrap_or(today);
                (start, today)
            }
        };

        let trades = self.get_trades(start_date, end_date).await;
        let stats = ReviewStats::from_trades(&trades);

        // Generate analysis using LLM
        let analysis = self.generate_analysis(&trades, &stats, period).await?;

        let review = TradingReview {
            id: uuid::Uuid::new_v4().to_string(),
            period,
            start_date,
            end_date,
            stats,
            analysis: analysis.analysis,
            patterns: analysis.patterns,
            improvements: analysis.improvements,
            goals: analysis.goals,
            created_at: Utc::now(),
        };

        // Store review
        let mut reviews = self.reviews.write().await;
        reviews.push(review.clone());

        Ok(review)
    }

    /// Generate analysis using LLM.
    async fn generate_analysis(
        &self,
        trades: &[TradeEntry],
        stats: &ReviewStats,
        period: ReviewPeriod,
    ) -> Result<AnalysisResult> {
        if trades.is_empty() {
            return Ok(AnalysisResult {
                analysis: "本期无交易记录。".to_string(),
                patterns: vec![],
                improvements: vec!["开始记录交易以便后续分析".to_string()],
                goals: vec!["建立交易记录习惯".to_string()],
            });
        }

        // Format trades for LLM
        let trades_summary = trades
            .iter()
            .take(50) // Limit to recent 50 trades
            .map(|t| {
                format!(
                    "- {} {} {}: {} @ {} -> {} | PnL: {} | 原因: {}",
                    t.entry_time.format("%m-%d"),
                    match t.direction {
                        TradeDirection::Long => "做多",
                        TradeDirection::Short => "做空",
                    },
                    t.symbol,
                    t.quantity,
                    t.entry_price,
                    t.exit_price.map(|p| p.to_string()).unwrap_or_else(|| "持仓中".to_string()),
                    t.pnl.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "-".to_string()),
                    t.entry_reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let period_name = match period {
            ReviewPeriod::Daily => "日",
            ReviewPeriod::Weekly => "周",
            ReviewPeriod::Monthly => "月",
            ReviewPeriod::Quarterly => "季度",
            ReviewPeriod::Yearly => "年度",
        };

        let prompt = format!(
            r#"你是一位专业的交易复盘分析师。请根据以下交易数据，生成一份{period_name}度复盘报告。

## 统计数据
- 总交易次数: {}
- 胜率: {:.1}%
- 总盈亏: {:.2}
- 平均盈利: {:.2}
- 平均亏损: {:.2}
- 盈亏比: {:.2}
- 最大单笔盈利: {:.2}
- 最大单笔亏损: {:.2}
- 平均持仓时间: {:.1} 小时

## 交易记录
{trades_summary}

请按以下 JSON 格式输出分析结果：

```json
{{
  "analysis": "整体分析（150字以内，评估交易表现和心态）",
  "patterns": ["识别到的交易模式1", "识别到的交易模式2"],
  "improvements": ["需要改进的地方1", "需要改进的地方2"],
  "goals": ["下一周期目标1", "下一周期目标2"]
}}
```

注意：
1. 分析要具体，结合实际交易数据
2. 识别好的模式和不好的模式
3. 改进建议要可执行
4. 目标要SMART（具体、可衡量、可实现、相关、有时限）"#,
            stats.total_trades,
            stats.win_rate,
            stats.total_pnl,
            stats.avg_win,
            stats.avg_loss,
            stats.risk_reward_ratio,
            stats.largest_win,
            stats.largest_loss,
            stats.avg_holding_hours,
        );

        let url = format!("{}/api/v1/chat", self.codecoder_endpoint);
        let request = serde_json::json!({
            "message": prompt,
            "agent": "trader",
            "user_id": "trading-review",
            "channel": "review"
        });

        let response = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(120))
            .json(&request)
            .send()
            .await
            .context("Failed to call CodeCoder API")?;

        if !response.status().is_success() {
            // Return default analysis on API failure
            return Ok(AnalysisResult {
                analysis: format!(
                    "本期共交易 {} 次，胜率 {:.1}%，总盈亏 {:.2}。",
                    stats.total_trades, stats.win_rate, stats.total_pnl
                ),
                patterns: vec![],
                improvements: vec!["详细分析暂不可用".to_string()],
                goals: vec!["保持记录交易".to_string()],
            });
        }

        let resp: serde_json::Value = response.json().await?;
        let message = resp["message"].as_str().unwrap_or("");

        // Parse JSON from response
        self.parse_analysis_response(message)
    }

    /// Parse analysis response from LLM.
    fn parse_analysis_response(&self, response: &str) -> Result<AnalysisResult> {
        // Try to extract JSON from code blocks
        let json_str = if let Some(start) = response.find("```json") {
            let after_marker = &response[start + 7..];
            if let Some(end) = after_marker.find("```") {
                after_marker[..end].trim()
            } else {
                response
            }
        } else if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        Ok(serde_json::from_str(json_str).unwrap_or_else(|_| AnalysisResult {
            analysis: response.chars().take(500).collect(),
            patterns: vec![],
            improvements: vec![],
            goals: vec![],
        }))
    }

    /// Send review reminder to all configured channels.
    pub async fn send_reminder(&self, period: ReviewPeriod) -> Result<Vec<SendResult>> {
        let endpoint = self
            .channels_endpoint
            .as_ref()
            .context("Channels endpoint not configured")?;

        let enabled_channels: Vec<_> = self
            .notification_config
            .channels
            .iter()
            .filter(|c| c.enabled)
            .collect();

        if enabled_channels.is_empty() {
            anyhow::bail!("No notification channels configured");
        }

        let period_name = match period {
            ReviewPeriod::Daily => "日度",
            ReviewPeriod::Weekly => "周度",
            ReviewPeriod::Monthly => "月度",
            ReviewPeriod::Quarterly => "季度",
            ReviewPeriod::Yearly => "年度",
        };

        // Build message with optional P&L summary
        let mut message = format!(
            "📊 **交易复盘提醒**\n\n⏰ 是时候进行{}复盘了！\n",
            period_name
        );

        // Add P&L summary if configured
        if self.notification_config.include_pnl_summary {
            let today = Utc::now().date_naive();
            let (start, end) = match period {
                ReviewPeriod::Daily => (today, today),
                ReviewPeriod::Weekly => {
                    let start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
                    (start, today)
                }
                ReviewPeriod::Monthly => {
                    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
                    (start, today)
                }
                _ => (today - chrono::Duration::days(30), today),
            };

            let trades = self.get_trades(start, end).await;
            let stats = ReviewStats::from_trades(&trades);

            if stats.total_trades > 0 {
                message.push_str(&format!(
                    "\n📈 **本期概况**\n- 交易次数: {}\n- 胜率: {:.1}%\n- 总盈亏: {:.2}\n",
                    stats.total_trades, stats.win_rate, stats.total_pnl
                ));
            }
        }

        // Add open positions count if configured
        if self.notification_config.include_open_positions {
            let trades = self.trades.read().await;
            let open_count = trades.iter().filter(|t| t.outcome == TradeOutcome::Open).count();
            if open_count > 0 {
                message.push_str(&format!("\n⚠️ 当前持仓: {} 笔\n", open_count));
            }
        }

        message.push_str(&format!(
            "\n复盘检查清单：\n1. 回顾{}交易\n2. 记录盈亏和情绪\n3. 识别成功/失败模式\n4. 制定改进计划\n\n输入 `@trader 复盘` 开始AI辅助复盘",
            match period {
                ReviewPeriod::Daily => "今日",
                ReviewPeriod::Weekly => "本周",
                ReviewPeriod::Monthly => "本月",
                _ => "本期",
            }
        ));

        // Send to all enabled channels
        let mut results = Vec::new();
        let url = format!("{}/api/v1/send", endpoint);

        for channel in enabled_channels {
            let body = serde_json::json!({
                "channel_type": channel.channel_type,
                "channel_id": channel.channel_id,
                "content": {
                    "type": "markdown",
                    "text": message
                }
            });

            let result = match self.client.post(&url).json(&body).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        SendResult {
                            channel_type: channel.channel_type.clone(),
                            channel_id: channel.channel_id.clone(),
                            success: true,
                            error: None,
                        }
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        SendResult {
                            channel_type: channel.channel_type.clone(),
                            channel_id: channel.channel_id.clone(),
                            success: false,
                            error: Some(format!("{} - {}", status, body)),
                        }
                    }
                }
                Err(e) => SendResult {
                    channel_type: channel.channel_type.clone(),
                    channel_id: channel.channel_id.clone(),
                    success: false,
                    error: Some(e.to_string()),
                },
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Send custom notification to all channels.
    pub async fn send_notification(&self, title: &str, content: &str) -> Result<Vec<SendResult>> {
        let endpoint = self
            .channels_endpoint
            .as_ref()
            .context("Channels endpoint not configured")?;

        let enabled_channels: Vec<_> = self
            .notification_config
            .channels
            .iter()
            .filter(|c| c.enabled)
            .collect();

        if enabled_channels.is_empty() {
            anyhow::bail!("No notification channels configured");
        }

        let message = format!("📊 **{}**\n\n{}", title, content);
        let url = format!("{}/api/v1/send", endpoint);

        let mut results = Vec::new();
        for channel in enabled_channels {
            let body = serde_json::json!({
                "channel_type": channel.channel_type,
                "channel_id": channel.channel_id,
                "content": {
                    "type": "markdown",
                    "text": message
                }
            });

            let result = match self.client.post(&url).json(&body).send().await {
                Ok(response) => SendResult {
                    channel_type: channel.channel_type.clone(),
                    channel_id: channel.channel_id.clone(),
                    success: response.status().is_success(),
                    error: if response.status().is_success() {
                        None
                    } else {
                        Some(response.status().to_string())
                    },
                },
                Err(e) => SendResult {
                    channel_type: channel.channel_type.clone(),
                    channel_id: channel.channel_id.clone(),
                    success: false,
                    error: Some(e.to_string()),
                },
            };
            results.push(result);
        }

        Ok(results)
    }

    /// Get current P&L summary for today.
    pub async fn get_daily_summary(&self) -> DailySummary {
        let today = Utc::now().date_naive();
        let trades = self.get_trades(today, today).await;
        let stats = ReviewStats::from_trades(&trades);

        let all_trades = self.trades.read().await;
        let open_positions = all_trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Open)
            .count();

        DailySummary {
            date: today,
            trades_count: stats.total_trades,
            win_rate: stats.win_rate,
            total_pnl: stats.total_pnl,
            open_positions,
            journal_exists: self.journals.read().await.contains_key(&today),
        }
    }

    /// Get recent reviews.
    pub async fn get_reviews(&self, limit: usize) -> Vec<TradingReview> {
        let reviews = self.reviews.read().await;
        reviews.iter().rev().take(limit).cloned().collect()
    }
}

/// Analysis result from LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisResult {
    analysis: String,
    #[serde(default)]
    patterns: Vec<String>,
    #[serde(default)]
    improvements: Vec<String>,
    #[serde(default)]
    goals: Vec<String>,
}

/// Result of sending a notification to a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub channel_type: String,
    pub channel_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Daily trading summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: NaiveDate,
    pub trades_count: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub open_positions: usize,
    pub journal_exists: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_return_percent() {
        let mut trade = TradeEntry {
            id: "1".to_string(),
            symbol: "AAPL".to_string(),
            asset_class: AssetClass::Stock,
            direction: TradeDirection::Long,
            entry_price: 100.0,
            exit_price: Some(110.0),
            quantity: 10.0,
            entry_time: Utc::now(),
            exit_time: None,
            stop_loss: None,
            take_profit: None,
            outcome: TradeOutcome::Win,
            pnl: Some(100.0),
            strategy: None,
            entry_reason: "Test".to_string(),
            exit_reason: None,
            tags: vec![],
            attachments: vec![],
            notes: String::new(),
        };

        assert_eq!(trade.return_percent(), Some(10.0));

        // Test short trade
        trade.direction = TradeDirection::Short;
        trade.exit_price = Some(90.0);
        assert_eq!(trade.return_percent(), Some(10.0)); // 10% profit on short
    }

    #[test]
    fn test_review_stats_from_trades() {
        let trades = vec![
            TradeEntry {
                id: "1".to_string(),
                symbol: "AAPL".to_string(),
                asset_class: AssetClass::Stock,
                direction: TradeDirection::Long,
                entry_price: 100.0,
                exit_price: Some(110.0),
                quantity: 10.0,
                entry_time: Utc::now(),
                exit_time: Some(Utc::now()),
                stop_loss: None,
                take_profit: None,
                outcome: TradeOutcome::Win,
                pnl: Some(100.0),
                strategy: Some("Trend".to_string()),
                entry_reason: "Test".to_string(),
                exit_reason: None,
                tags: vec![],
                attachments: vec![],
                notes: String::new(),
            },
            TradeEntry {
                id: "2".to_string(),
                symbol: "GOOG".to_string(),
                asset_class: AssetClass::Stock,
                direction: TradeDirection::Long,
                entry_price: 100.0,
                exit_price: Some(95.0),
                quantity: 10.0,
                entry_time: Utc::now(),
                exit_time: Some(Utc::now()),
                stop_loss: None,
                take_profit: None,
                outcome: TradeOutcome::Loss,
                pnl: Some(-50.0),
                strategy: Some("Reversal".to_string()),
                entry_reason: "Test".to_string(),
                exit_reason: None,
                tags: vec![],
                attachments: vec![],
                notes: String::new(),
            },
        ];

        let stats = ReviewStats::from_trades(&trades);

        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.winning_trades, 1);
        assert_eq!(stats.losing_trades, 1);
        assert_eq!(stats.win_rate, 50.0);
        assert_eq!(stats.total_pnl, 50.0);
        assert_eq!(stats.avg_win, 100.0);
        assert_eq!(stats.avg_loss, 50.0);
        assert_eq!(stats.largest_win, 100.0);
        assert_eq!(stats.largest_loss, -50.0);
    }

    #[test]
    fn test_asset_class_display() {
        assert_eq!(AssetClass::Stock.to_string(), "股票");
        assert_eq!(AssetClass::Futures.to_string(), "期货");
        assert_eq!(AssetClass::Crypto.to_string(), "加密货币");
    }

    #[test]
    fn test_reminder_schedule_default() {
        let schedule = ReminderSchedule::default();
        assert!(schedule.daily_enabled);
        assert_eq!(schedule.daily_time, "21:00");
        assert_eq!(schedule.daily_days, vec![1, 2, 3, 4, 5]);
    }
}
