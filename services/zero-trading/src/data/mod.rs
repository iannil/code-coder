//! Market data module for A-shares.
//!
//! Provides data fetching, caching, and aggregation for multiple timeframes.
//! Supports multiple data providers with automatic failover.
//!
//! # Data Sources
//! - **iTick** (Primary): REST API with minute-level data, 15 years history
//! - **Lixin** (Backup): 理杏仁 API, requires token, high-quality daily data
//!
//! # High-Frequency Economic Data
//! - **high_frequency**: Collection and storage of high-frequency macro indicators
//!   for predicting and validating official data releases

mod cache;
mod aggregator;
mod provider;
mod health;
mod router;
mod rate_limiter;
mod itick;
mod lixin;
pub mod high_frequency;
mod hf_scheduler;
pub mod local_storage;
pub mod sync;

pub use cache::DataCache;
pub use aggregator::MarketDataAggregator;
pub use provider::{DataProvider, DataCapabilities, ProviderError, ProviderInfo, StockInfo, FinancialStatementData};
pub use provider::{ValuationMetrics, ValuationMetricName, StatisticsGranularity, ValuationStatistics, ValuationStatisticsSet};
pub use health::{HealthMonitor, HealthMonitorConfig, ProviderHealth};
pub use router::{DataProviderRouter, RouterConfig};
pub use rate_limiter::{RateLimiter, SharedRateLimiter, shared_limiter};
pub use itick::ITickAdapter;
pub use lixin::LixinAdapter;
pub use high_frequency::{
    CollectorConfig as HighFrequencyConfig,
    CollectionReport,
    HighFrequencyCollector,
    HighFrequencyDataSource,
    MockDataSource as MockHighFrequencySource,
};
pub use hf_scheduler::{
    HighFrequencyScheduler,
    HfScheduledTask,
    HfSchedulerState,
};
pub use local_storage::{
    LocalStorage,
    LocalStorageConfig,
    LocalStorageStats,
    SyncMetadata,
    SyncStatus,
};
pub use sync::{DataSynchronizer, SyncConfig, SyncReport};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Core Data Types
// ============================================================================

/// Timeframe for K-line data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    /// 1-minute candles
    M1,
    /// 5-minute candles
    M5,
    /// 15-minute candles
    M15,
    /// 30-minute candles
    M30,
    /// 1-hour candles
    H1,
    /// 4-hour candles
    H4,
    /// Daily candles
    Daily,
    /// Weekly candles
    Weekly,
}

impl Timeframe {
    /// Parse from string (e.g., "D", "H4", "1H", "Daily")
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "1M" | "M1" => Some(Self::M1),
            "5M" | "M5" => Some(Self::M5),
            "15M" | "M15" => Some(Self::M15),
            "30M" | "M30" => Some(Self::M30),
            "1H" | "H1" | "60M" => Some(Self::H1),
            "4H" | "H4" | "240M" => Some(Self::H4),
            "D" | "DAILY" | "1D" => Some(Self::Daily),
            "W" | "WEEKLY" | "1W" => Some(Self::Weekly),
            _ => None,
        }
    }

    /// Convert to API frequency string (for data providers)
    pub fn to_api_freq(&self) -> &'static str {
        match self {
            Self::M1 => "1min",
            Self::M5 => "5min",
            Self::M15 => "15min",
            Self::M30 => "30min",
            Self::H1 => "60min",
            Self::H4 => "240min",
            Self::Daily => "D",
            Self::Weekly => "W",
        }
    }

    /// Get the number of minutes per candle
    pub fn minutes(&self) -> u32 {
        match self {
            Self::M1 => 1,
            Self::M5 => 5,
            Self::M15 => 15,
            Self::M30 => 30,
            Self::H1 => 60,
            Self::H4 => 240,
            Self::Daily => 60 * 4, // 4 hours of trading
            Self::Weekly => 60 * 4 * 5, // 5 trading days
        }
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::M1 => write!(f, "1M"),
            Self::M5 => write!(f, "5M"),
            Self::M15 => write!(f, "15M"),
            Self::M30 => write!(f, "30M"),
            Self::H1 => write!(f, "1H"),
            Self::H4 => write!(f, "4H"),
            Self::Daily => write!(f, "D"),
            Self::Weekly => write!(f, "W"),
        }
    }
}

/// A single candlestick (OHLCV)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Symbol/ticker
    pub symbol: String,
    /// Timeframe
    pub timeframe: Timeframe,
    /// Candle open time
    pub timestamp: DateTime<Utc>,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Close price
    pub close: f64,
    /// Volume
    pub volume: f64,
    /// Amount (turnover in currency)
    #[serde(default)]
    pub amount: f64,
}

impl Candle {
    /// Check if this is a bullish candle
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if this is a bearish candle
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Get the candle body size
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get the upper wick size
    pub fn upper_wick(&self) -> f64 {
        self.high - self.close.max(self.open)
    }

    /// Get the lower wick size
    pub fn lower_wick(&self) -> f64 {
        self.close.min(self.open) - self.low
    }

    /// Get the full range (high - low)
    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    /// Get the midpoint price
    pub fn midpoint(&self) -> f64 {
        (self.high + self.low) / 2.0
    }
}

/// Auction data from pre-market session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionData {
    /// Symbol/ticker
    pub symbol: String,
    /// Date
    pub date: NaiveDate,
    /// Expected opening price from auction
    pub expected_price: f64,
    /// Auction volume
    pub volume: f64,
    /// Price change from previous close
    pub change_percent: f64,
    /// Timestamp of the data
    pub timestamp: DateTime<Utc>,
}

/// Real-time quote data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// Symbol/ticker
    pub symbol: String,
    /// Last price
    pub last: f64,
    /// Bid price
    pub bid: f64,
    /// Ask price
    pub ask: f64,
    /// Bid volume
    pub bid_volume: f64,
    /// Ask volume
    pub ask_volume: f64,
    /// Day's volume
    pub volume: f64,
    /// Day's high
    pub high: f64,
    /// Day's low
    pub low: f64,
    /// Previous close
    pub prev_close: f64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl Quote {
    /// Get the change from previous close
    pub fn change(&self) -> f64 {
        self.last - self.prev_close
    }

    /// Get the change percentage from previous close
    pub fn change_percent(&self) -> f64 {
        if self.prev_close > 0.0 {
            ((self.last - self.prev_close) / self.prev_close) * 100.0
        } else {
            0.0
        }
    }

    /// Get the spread (ask - bid)
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }
}

// ============================================================================
// Symbol Information
// ============================================================================

/// Stock/ETF information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// Symbol code (e.g., "000001.SZ")
    pub symbol: String,
    /// Name (e.g., "平安银行")
    pub name: String,
    /// Exchange (SZ, SH, BJ)
    pub exchange: String,
    /// Listing date
    #[serde(default)]
    pub list_date: Option<NaiveDate>,
    /// Industry
    #[serde(default)]
    pub industry: Option<String>,
    /// Whether trading is suspended
    #[serde(default)]
    pub is_suspended: bool,
}

/// SMT pair configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtPair {
    /// Primary symbol
    pub primary: String,
    /// Reference symbol
    pub reference: String,
    /// Pair name for display
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
}

impl SmtPair {
    /// Create a new SMT pair
    pub fn new(primary: impl Into<String>, reference: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            reference: reference.into(),
            name: name.into(),
            description: None,
        }
    }
}

/// Default SMT pairs for A-shares
pub fn default_smt_pairs() -> Vec<SmtPair> {
    vec![
        SmtPair {
            primary: "000300.SH".to_string(), // CSI 300 Index
            reference: "000905.SH".to_string(), // CSI 500 Index
            name: "沪深300 vs 中证500".to_string(),
            description: Some("大小盘轮动指标".to_string()),
        },
        SmtPair {
            primary: "000016.SH".to_string(), // SSE 50 Index
            reference: "000688.SH".to_string(), // STAR 50 Index
            name: "上证50 vs 科创50".to_string(),
            description: Some("蓝筹vs成长指标".to_string()),
        },
        SmtPair {
            primary: "512880.SH".to_string(), // Securities ETF
            reference: "512800.SH".to_string(), // Bank ETF
            name: "券商ETF vs 银行ETF".to_string(),
            description: Some("金融板块领先指标".to_string()),
        },
    ]
}

/// Default tracked symbols for market data updates.
/// Includes major indices and popular ETFs for A-shares.
pub fn default_tracked_symbols() -> Vec<String> {
    vec![
        // Major Indices
        "000001.SH".to_string(), // Shanghai Composite
        "000300.SH".to_string(), // CSI 300
        "000905.SH".to_string(), // CSI 500
        "000016.SH".to_string(), // SSE 50
        "000688.SH".to_string(), // STAR 50
        "399001.SZ".to_string(), // Shenzhen Component
        "399006.SZ".to_string(), // ChiNext
        // Popular ETFs
        "512880.SH".to_string(), // Securities ETF
        "512800.SH".to_string(), // Bank ETF
        "510300.SH".to_string(), // CSI 300 ETF
        "159915.SZ".to_string(), // CSI 500 ETF
    ]
}

// ============================================================================
// Index Overview for Daily Reports
// ============================================================================

/// Overview of multiple indices for daily macro reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexOverview {
    /// Index data for each tracked symbol
    pub indices: Vec<IndexData>,
    /// Timestamp when data was fetched
    pub as_of: DateTime<Utc>,
}

/// Single index data point for daily reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexData {
    /// Index symbol (e.g., "000300.SH")
    pub symbol: String,
    /// Index name (e.g., "沪深300")
    pub name: String,
    /// Current/latest close price
    pub close: f64,
    /// Daily change percentage
    pub change_pct: f64,
    /// Trading volume
    pub volume: f64,
    /// 5-day moving average (optional)
    pub ma5: Option<f64>,
    /// 20-day moving average (optional)
    pub ma20: Option<f64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_to_api_freq() {
        assert_eq!(Timeframe::Daily.to_api_freq(), "D");
        assert_eq!(Timeframe::H1.to_api_freq(), "60min");
        assert_eq!(Timeframe::H4.to_api_freq(), "240min");
    }

    #[test]
    fn test_candle_helpers() {
        let candle = Candle {
            symbol: "000001.SZ".to_string(),
            timeframe: Timeframe::Daily,
            timestamp: Utc::now(),
            open: 10.0,
            high: 12.0,
            low: 9.5,
            close: 11.0,
            volume: 1000000.0,
            amount: 10500000.0,
        };

        assert!(candle.is_bullish());
        assert!(!candle.is_bearish());
        assert!((candle.body_size() - 1.0).abs() < 0.001);
        assert!((candle.range() - 2.5).abs() < 0.001);
        assert!((candle.midpoint() - 10.75).abs() < 0.001);
    }

    #[test]
    fn test_quote_helpers() {
        let quote = Quote {
            symbol: "000001.SZ".to_string(),
            last: 10.5,
            bid: 10.49,
            ask: 10.51,
            bid_volume: 1000.0,
            ask_volume: 500.0,
            volume: 1000000.0,
            high: 11.0,
            low: 10.0,
            prev_close: 10.0,
            timestamp: Utc::now(),
        };

        assert!((quote.change() - 0.5).abs() < 0.001);
        assert!((quote.change_percent() - 5.0).abs() < 0.001);
        assert!((quote.spread() - 0.02).abs() < 0.001);
    }

    #[test]
    fn test_default_smt_pairs() {
        let pairs = default_smt_pairs();
        assert!(!pairs.is_empty());
        assert!(pairs.iter().any(|p| p.primary == "000300.SH"));
    }
}
