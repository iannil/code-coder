//! Data provider abstraction for multi-source market data.
//!
//! Defines the `DataProvider` trait that all data sources must implement,
//! enabling automatic failover and health-based routing.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{Candle, Timeframe};

// Import and re-export valuation types from lixin for convenience
pub use super::lixin::{
    ValuationMetrics,
    ValuationMetricName,
    StatisticsGranularity,
    ValuationStatistics,
    ValuationStatisticsSet,
};

// ============================================================================
// Provider Capabilities
// ============================================================================

/// Capabilities supported by a data provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataCapabilities {
    /// Supported timeframes for K-line data
    pub timeframes: Vec<Timeframe>,
    /// Supports real-time quotes
    pub realtime_quotes: bool,
    /// Supports auction data
    pub auction_data: bool,
    /// Supports index data
    pub index_data: bool,
    /// Supports ETF data
    pub etf_data: bool,
    /// Maximum history days available
    pub max_history_days: Option<u32>,
    /// Rate limit (requests per minute)
    pub rate_limit_rpm: Option<u32>,
}

impl DataCapabilities {
    /// Check if a timeframe is supported
    pub fn supports_timeframe(&self, tf: Timeframe) -> bool {
        self.timeframes.contains(&tf)
    }

    /// Create capabilities for daily-only providers
    pub fn daily_only() -> Self {
        Self {
            timeframes: vec![Timeframe::Daily, Timeframe::Weekly],
            ..Default::default()
        }
    }

    /// Create full capabilities (all timeframes)
    pub fn full() -> Self {
        Self {
            timeframes: vec![
                Timeframe::M1,
                Timeframe::M5,
                Timeframe::M15,
                Timeframe::M30,
                Timeframe::H1,
                Timeframe::H4,
                Timeframe::Daily,
                Timeframe::Weekly,
            ],
            realtime_quotes: true,
            auction_data: true,
            index_data: true,
            etf_data: true,
            max_history_days: None,
            rate_limit_rpm: None,
        }
    }
}

// ============================================================================
// Provider Error
// ============================================================================

/// Errors specific to data providers.
#[derive(Debug, Clone)]
pub enum ProviderError {
    /// Network error (connection failed, timeout)
    Network(String),
    /// Authentication error (invalid token, expired)
    Auth(String),
    /// Rate limit exceeded
    RateLimited { retry_after_secs: Option<u64> },
    /// Data not available for the requested symbol/timeframe
    DataNotAvailable(String),
    /// Provider is temporarily unavailable
    Unavailable(String),
    /// Invalid request parameters
    InvalidRequest(String),
    /// Internal provider error
    Internal(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Auth(msg) => write!(f, "Authentication error: {}", msg),
            Self::RateLimited { retry_after_secs } => {
                write!(f, "Rate limited")?;
                if let Some(secs) = retry_after_secs {
                    write!(f, ", retry after {} seconds", secs)?;
                }
                Ok(())
            }
            Self::DataNotAvailable(msg) => write!(f, "Data not available: {}", msg),
            Self::Unavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

impl ProviderError {
    /// Check if the error is recoverable (worth retrying)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::RateLimited { .. } | Self::Unavailable(_)
        )
    }

    /// Check if this error should trigger a failover to another provider
    pub fn should_failover(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Auth(_) | Self::Unavailable(_) | Self::Internal(_)
        )
    }
}

// ============================================================================
// Data Provider Trait
// ============================================================================

// ============================================================================
// Stock Info for Screener
// ============================================================================

/// Basic stock information for screener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    /// Stock code (e.g., "000001")
    pub code: String,
    /// Stock name (e.g., "平安银行")
    pub name: String,
    /// Exchange (SZ, SH, BJ)
    pub exchange: String,
    /// Industry classification
    pub industry: Option<String>,
    /// Listing date
    pub list_date: Option<NaiveDate>,
    /// Whether trading is suspended
    pub is_suspended: bool,
    /// Whether it's a ST stock
    pub is_st: bool,
    /// Market cap in billion yuan
    pub market_cap: Option<f64>,
}

impl StockInfo {
    /// Get full symbol with exchange suffix (e.g., "000001.SZ")
    pub fn symbol(&self) -> String {
        format!("{}.{}", self.code, self.exchange)
    }

    /// Calculate days since listing
    pub fn listing_days(&self) -> Option<i64> {
        self.list_date.map(|d| {
            let today = chrono::Local::now().date_naive();
            (today - d).num_days()
        })
    }
}

/// Financial statement data from provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialStatementData {
    /// Stock symbol
    pub symbol: String,
    /// Report period end date (e.g., "2024-12-31")
    pub period_end: NaiveDate,
    /// Report type (annual, semi-annual, quarterly)
    pub report_type: String,

    // === Income Statement ===
    /// Total revenue
    pub revenue: Option<f64>,
    /// Gross profit
    pub gross_profit: Option<f64>,
    /// Operating income
    pub operating_income: Option<f64>,
    /// Net income attributable to parent
    pub net_income: Option<f64>,
    /// Interest expense
    pub interest_expense: Option<f64>,

    // === Balance Sheet ===
    /// Total assets
    pub total_assets: Option<f64>,
    /// Total equity attributable to parent
    pub total_equity: Option<f64>,
    /// Total liabilities
    pub total_liabilities: Option<f64>,
    /// Cash and cash equivalents
    pub cash: Option<f64>,
    /// Total debt (short + long term)
    pub total_debt: Option<f64>,
    /// Total shares outstanding
    pub shares_outstanding: Option<f64>,

    // === Cash Flow Statement ===
    /// Operating cash flow
    pub operating_cash_flow: Option<f64>,
    /// Investing cash flow
    pub investing_cash_flow: Option<f64>,
    /// Financing cash flow
    pub financing_cash_flow: Option<f64>,
    /// Capital expenditure (usually negative)
    pub capex: Option<f64>,

    // === Derived Metrics (pre-calculated by provider) ===
    /// ROE (Return on Equity) %
    pub roe: Option<f64>,
    /// ROA (Return on Assets) %
    pub roa: Option<f64>,
    /// Gross margin %
    pub gross_margin: Option<f64>,
    /// Net margin %
    pub net_margin: Option<f64>,
    /// Debt to equity ratio
    pub debt_to_equity: Option<f64>,
    /// Current ratio
    pub current_ratio: Option<f64>,
    /// PE ratio (TTM)
    pub pe_ttm: Option<f64>,
    /// PB ratio
    pub pb: Option<f64>,
    /// Dividend yield %
    pub dividend_yield: Option<f64>,
}

impl FinancialStatementData {
    /// Calculate free cash flow if not provided
    pub fn free_cash_flow(&self) -> Option<f64> {
        match (self.operating_cash_flow, self.capex) {
            (Some(ocf), Some(capex)) => Some(ocf + capex), // capex is usually negative
            _ => None,
        }
    }

    /// Check if this is a healthy cash flow pattern (OCF+, ICF-, FCF-)
    pub fn is_cash_cow_pattern(&self) -> bool {
        match (
            self.operating_cash_flow,
            self.investing_cash_flow,
            self.financing_cash_flow,
        ) {
            (Some(ocf), Some(icf), Some(fcf)) => ocf > 0.0 && icf < 0.0 && fcf < 0.0,
            _ => false,
        }
    }
}

/// Trait for market data providers.
///
/// All data sources (iTick, Lixin, etc.) implement this trait
/// to provide a unified interface for the data router.
#[async_trait]
pub trait DataProvider: Send + Sync {
    /// Get the provider name (e.g., "itick", "lixin")
    fn name(&self) -> &'static str;

    /// Get the provider priority (lower = higher priority)
    fn priority(&self) -> u8;

    /// Get the provider's capabilities
    fn capabilities(&self) -> DataCapabilities;

    /// Check if the provider is healthy and available.
    ///
    /// This is used by the health monitor to track provider status.
    /// Should be a lightweight check (e.g., simple API ping).
    async fn health_check(&self) -> Result<(), ProviderError>;

    /// Fetch daily candles for a symbol.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "000001.SZ")
    /// * `start_date` - Optional start date filter
    /// * `end_date` - Optional end date filter
    /// * `limit` - Optional maximum number of candles
    async fn get_daily_candles(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError>;

    /// Fetch minute/hourly candles for a symbol.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "000001.SZ")
    /// * `timeframe` - Candle timeframe (M1, M5, M15, M30, H1, H4)
    /// * `start_time` - Optional start time filter
    /// * `end_time` - Optional end time filter
    async fn get_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError>;

    /// Fetch index daily data (CSI 300, CSI 500, etc.)
    ///
    /// Default implementation calls get_daily_candles.
    async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        self.get_daily_candles(symbol, start_date, end_date, None)
            .await
    }

    // ========================================================================
    // Stock Screener Methods (Optional - for full market scanning)
    // ========================================================================

    /// List all stocks in the market.
    ///
    /// Returns basic information for all stocks. Used for full market scanning.
    /// Default implementation returns "not supported" error.
    async fn list_all_stocks(&self) -> Result<Vec<StockInfo>, ProviderError> {
        Err(ProviderError::DataNotAvailable(format!(
            "{} does not support stock listing",
            self.name()
        )))
    }

    /// Get financial statement data for a single stock.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "000001.SZ")
    /// * `period_end` - Optional specific period end date. If None, returns latest.
    async fn get_financial_data(
        &self,
        symbol: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<FinancialStatementData, ProviderError> {
        let _ = (symbol, period_end);
        Err(ProviderError::DataNotAvailable(format!(
            "{} does not support financial data",
            self.name()
        )))
    }

    /// Batch get financial data for multiple stocks.
    ///
    /// More efficient than calling get_financial_data() in a loop.
    /// Default implementation falls back to sequential calls.
    ///
    /// # Arguments
    /// * `symbols` - List of stock symbols
    /// * `period_end` - Optional specific period end date
    async fn batch_get_financial_data(
        &self,
        symbols: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<FinancialStatementData>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match self.get_financial_data(symbol, period_end).await {
                Ok(data) => results.push(data),
                Err(ProviderError::DataNotAvailable(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }

    // ========================================================================
    // Valuation Metrics Methods
    // ========================================================================

    /// Get valuation metrics for a single stock.
    ///
    /// Returns current valuation metrics like PE, PB, PS, dividend yield,
    /// market cap, and margin trading data.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "000001.SZ")
    /// * `date` - Optional specific date. If None, returns latest.
    ///
    /// Default implementation returns "not supported" error.
    async fn get_valuation_metrics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<ValuationMetrics, ProviderError> {
        let _ = (symbol, date);
        Err(ProviderError::DataNotAvailable(format!(
            "{} does not support valuation metrics",
            self.name()
        )))
    }

    /// Batch get valuation metrics for multiple stocks.
    ///
    /// More efficient than calling get_valuation_metrics() in a loop.
    /// Default implementation falls back to sequential calls.
    ///
    /// # Arguments
    /// * `symbols` - List of stock symbols
    /// * `date` - Optional specific date
    async fn batch_get_valuation_metrics(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match self.get_valuation_metrics(symbol, date).await {
                Ok(data) => results.push(data),
                Err(ProviderError::DataNotAvailable(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }

    /// Get valuation statistics with historical percentiles.
    ///
    /// Returns historical percentile data for valuation metrics,
    /// useful for determining if a stock is historically cheap or expensive.
    ///
    /// # Arguments
    /// * `symbol` - Stock symbol (e.g., "000001.SZ")
    /// * `metrics` - List of metrics to get statistics for
    /// * `granularities` - List of time granularities (e.g., 3Y, 5Y, since listing)
    /// * `date` - Optional specific date. If None, returns latest.
    ///
    /// Default implementation returns "not supported" error.
    async fn get_valuation_statistics(
        &self,
        symbol: &str,
        metrics: &[ValuationMetricName],
        granularities: &[StatisticsGranularity],
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatisticsSet, ProviderError> {
        let _ = (symbol, metrics, granularities, date);
        Err(ProviderError::DataNotAvailable(format!(
            "{} does not support valuation statistics",
            self.name()
        )))
    }
}

// ============================================================================
// Provider Info (for monitoring/debugging)
// ============================================================================

/// Information about a data provider for monitoring purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider name
    pub name: String,
    /// Priority level
    pub priority: u8,
    /// Current health status
    pub healthy: bool,
    /// Capabilities summary
    pub capabilities: DataCapabilities,
    /// Last successful request timestamp
    pub last_success: Option<DateTime<Utc>>,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Total requests made
    pub request_count: u64,
    /// Total errors encountered
    pub error_count: u64,
}

impl ProviderInfo {
    /// Create new provider info from a provider
    pub fn from_provider<P: DataProvider + ?Sized>(provider: &P, healthy: bool) -> Self {
        Self {
            name: provider.name().to_string(),
            priority: provider.priority(),
            healthy,
            capabilities: provider.capabilities(),
            last_success: None,
            last_error: None,
            request_count: 0,
            error_count: 0,
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
    fn test_capabilities_daily_only() {
        let caps = DataCapabilities::daily_only();
        assert!(caps.supports_timeframe(Timeframe::Daily));
        assert!(caps.supports_timeframe(Timeframe::Weekly));
        assert!(!caps.supports_timeframe(Timeframe::H1));
        assert!(!caps.realtime_quotes);
    }

    #[test]
    fn test_capabilities_full() {
        let caps = DataCapabilities::full();
        assert!(caps.supports_timeframe(Timeframe::Daily));
        assert!(caps.supports_timeframe(Timeframe::H1));
        assert!(caps.supports_timeframe(Timeframe::M5));
        assert!(caps.realtime_quotes);
        assert!(caps.index_data);
    }

    #[test]
    fn test_provider_error_recoverable() {
        assert!(ProviderError::Network("timeout".into()).is_recoverable());
        assert!(ProviderError::RateLimited { retry_after_secs: Some(60) }.is_recoverable());
        assert!(ProviderError::Unavailable("maintenance".into()).is_recoverable());
        assert!(!ProviderError::Auth("invalid token".into()).is_recoverable());
        assert!(!ProviderError::DataNotAvailable("no data".into()).is_recoverable());
    }

    #[test]
    fn test_provider_error_failover() {
        assert!(ProviderError::Network("timeout".into()).should_failover());
        assert!(ProviderError::Auth("invalid token".into()).should_failover());
        assert!(ProviderError::Unavailable("down".into()).should_failover());
        assert!(!ProviderError::RateLimited { retry_after_secs: None }.should_failover());
        assert!(!ProviderError::DataNotAvailable("no data".into()).should_failover());
    }

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::RateLimited {
            retry_after_secs: Some(30),
        };
        assert!(err.to_string().contains("30 seconds"));

        let err = ProviderError::Network("connection refused".into());
        assert!(err.to_string().contains("connection refused"));
    }
}
