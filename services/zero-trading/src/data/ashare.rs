//! Ashare adapter for A-share market data.
//!
//! This adapter implements the DataProvider trait to fetch market data
//! using the same data sources as the mpquant/Ashare Python library.
//!
//! Primary data source: eastmoney APIs (免费、无限制)
//!
//! # Data Sources
//! - Daily K-line: push2his.eastmoney.com
//! - Minute K-line: push2his.eastmoney.com
//! - Real-time quotes: push2.eastmoney.com
//!
//! # Advantages over Tushare
//! - No API key required
//! - No rate limits
//! - Free for all users

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, warn};

use super::provider::{DataCapabilities, DataProvider, ProviderError};
use super::{Candle, Timeframe};

// ============================================================================
// Constants
// ============================================================================

/// Eastmoney historical data API
const EASTMONEY_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

/// Eastmoney real-time API
const EASTMONEY_QUOTE_URL: &str = "https://push2.eastmoney.com/api/qt/stock/get";

// ============================================================================
// Symbol Mapping
// ============================================================================

/// Convert standard symbol format to eastmoney format.
///
/// Standard: "000001.SZ" -> Eastmoney: "0.000001" (SZ) or "1.600000" (SH)
fn to_eastmoney_code(symbol: &str) -> Option<String> {
    let parts: Vec<&str> = symbol.split('.').collect();
    if parts.len() != 2 {
        return None;
    }

    let code = parts[0];
    let exchange = parts[1].to_uppercase();

    let market = match exchange.as_str() {
        "SZ" => "0",
        "SH" => "1",
        "BJ" => "0", // Beijing exchange uses SZ market code
        _ => return None,
    };

    Some(format!("{}.{}", market, code))
}

/// Convert standard symbol to secid format (used in some APIs)
fn to_secid(symbol: &str) -> Option<String> {
    to_eastmoney_code(symbol)
}

/// Convert eastmoney market code back to exchange suffix
fn market_to_exchange(market: &str) -> &'static str {
    match market {
        "0" => "SZ",
        "1" => "SH",
        _ => "SZ",
    }
}

// ============================================================================
// Timeframe Mapping
// ============================================================================

/// Convert Timeframe to eastmoney klt parameter
fn timeframe_to_klt(tf: Timeframe) -> i32 {
    match tf {
        Timeframe::M1 => 1,
        Timeframe::M5 => 5,
        Timeframe::M15 => 15,
        Timeframe::M30 => 30,
        Timeframe::H1 => 60,
        Timeframe::H4 => 240, // 4-hour not directly supported, use 240min
        Timeframe::Daily => 101,
        Timeframe::Weekly => 102,
    }
}

// ============================================================================
// Ashare Adapter
// ============================================================================

/// Ashare adapter for A-share market data.
///
/// Uses eastmoney APIs directly (same as mpquant/Ashare Python library).
pub struct AshareAdapter {
    /// HTTP client
    client: reqwest::Client,
    /// Priority level
    priority: u8,
}

impl AshareAdapter {
    /// Create a new Ashare adapter
    pub fn new() -> Self {
        Self::with_priority(1) // Default to highest priority
    }

    /// Create with custom priority
    pub fn with_priority(priority: u8) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client, priority }
    }

    /// Create from config
    pub fn from_config(config: &zero_common::config::Config) -> Option<Self> {
        // Check if ashare is enabled in data_sources config
        let trading = config.trading.as_ref()?;
        let data_sources = trading.data_sources.as_ref()?;

        let entry = data_sources
            .sources
            .iter()
            .find(|s| s.provider == "ashare" && s.enabled)?;

        Some(Self::with_priority(entry.priority))
    }

    /// Fetch K-line data from eastmoney API
    async fn fetch_kline(
        &self,
        symbol: &str,
        klt: i32, // 1=1min, 5=5min, etc.
        fqt: i32, // 0=不复权, 1=前复权, 2=后复权
        limit: Option<usize>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let secid =
            to_secid(symbol).ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let limit_str = limit.unwrap_or(1000).to_string();
        let start_str = start_date.map(|d| d.format("%Y%m%d").to_string());
        let end_str = end_date.map(|d| d.format("%Y%m%d").to_string());

        // Build URL with query params
        let url = format!(
            "{}?secid={}&klt={}&fqt={}&lmt={}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61{}{}",
            EASTMONEY_KLINE_URL,
            secid,
            klt,
            fqt,
            limit_str,
            start_str.as_ref().map(|s| format!("&beg={}", s)).unwrap_or_default(),
            end_str.as_ref().map(|s| format!("&end={}", s)).unwrap_or_default(),
        );

        debug!(url = %url, symbol = symbol, "Fetching kline from eastmoney");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(ProviderError::Network(format!("HTTP {}", status)));
        }

        let data: EastmoneyKlineResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        if data.rc != 0 {
            return Err(ProviderError::Internal(format!(
                "Eastmoney API error: rc={}",
                data.rc
            )));
        }

        let klines = data
            .data
            .and_then(|d| d.klines)
            .unwrap_or_default();

        let timeframe = match klt {
            1 => Timeframe::M1,
            5 => Timeframe::M5,
            15 => Timeframe::M15,
            30 => Timeframe::M30,
            60 => Timeframe::H1,
            240 => Timeframe::H4,
            101 => Timeframe::Daily,
            102 => Timeframe::Weekly,
            _ => Timeframe::Daily,
        };

        let candles = self.parse_klines(symbol, timeframe, &klines)?;
        Ok(candles)
    }

    /// Parse eastmoney kline strings into Candles
    fn parse_klines(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        klines: &[String],
    ) -> Result<Vec<Candle>, ProviderError> {
        let mut candles = Vec::with_capacity(klines.len());

        for line in klines {
            // Format: "2024-01-02,10.50,10.80,10.40,10.70,1000000,10500000,..."
            // Fields: date,open,close,high,low,volume,amount,...
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                warn!(line = line, "Invalid kline format, skipping");
                continue;
            }

            let timestamp = parse_eastmoney_datetime(parts[0], timeframe)?;
            let open = parts[1]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse open: {}", e)))?;
            let close = parts[2]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse close: {}", e)))?;
            let high = parts[3]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse high: {}", e)))?;
            let low = parts[4]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse low: {}", e)))?;
            let volume = parts[5]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse volume: {}", e)))?;
            let amount = parts[6]
                .parse::<f64>()
                .map_err(|e| ProviderError::Internal(format!("Failed to parse amount: {}", e)))?;

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe,
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                amount,
            });
        }

        // Sort by timestamp ascending
        candles.sort_by_key(|c| c.timestamp);

        Ok(candles)
    }
}

/// Parse eastmoney datetime string to UTC DateTime
fn parse_eastmoney_datetime(
    s: &str,
    timeframe: Timeframe,
) -> Result<DateTime<Utc>, ProviderError> {
    // Daily format: "2024-01-02"
    // Minute format: "2024-01-02 09:30"
    let dt = if s.len() == 10 {
        // Daily format
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| ProviderError::Internal(format!("Failed to parse date: {}", e)))?;
        // Use 15:00 (market close) as timestamp for daily candles
        date.and_hms_opt(15, 0, 0).unwrap()
    } else {
        // Minute format: "2024-01-02 09:30"
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
            .or_else(|_| NaiveDateTime::parse_from_str(&format!("{}:00", s), "%Y-%m-%d %H:%M:%S"))
            .map_err(|e| ProviderError::Internal(format!("Failed to parse datetime: {}", e)))?
    };

    // Convert to UTC (China is UTC+8)
    Ok(dt.and_utc() - chrono::Duration::hours(8))
}

impl Default for AshareAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DataProvider Implementation
// ============================================================================

#[async_trait]
impl DataProvider for AshareAdapter {
    fn name(&self) -> &'static str {
        "ashare"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn capabilities(&self) -> DataCapabilities {
        DataCapabilities {
            timeframes: vec![
                Timeframe::M1,
                Timeframe::M5,
                Timeframe::M15,
                Timeframe::M30,
                Timeframe::H1,
                Timeframe::Daily,
                Timeframe::Weekly,
            ],
            realtime_quotes: true,
            auction_data: false, // Not implemented yet
            index_data: true,
            etf_data: true,
            max_history_days: Some(365 * 5), // ~5 years typical
            rate_limit_rpm: None,            // No rate limit
        }
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: fetch 1 daily candle for a well-known stock
        let candles = self
            .fetch_kline("000001.SZ", 101, 1, Some(1), None, None)
            .await?;

        if candles.is_empty() {
            return Err(ProviderError::Unavailable(
                "Health check returned no data".into(),
            ));
        }

        Ok(())
    }

    async fn get_daily_candles(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        // klt=101 for daily, fqt=1 for forward-adjusted prices
        self.fetch_kline(symbol, 101, 1, limit, start_date, end_date)
            .await
    }

    async fn get_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let klt = timeframe_to_klt(timeframe);

        if klt >= 101 {
            return Err(ProviderError::InvalidRequest(
                "Use get_daily_candles for daily/weekly data".into(),
            ));
        }

        // Convert DateTime to NaiveDate for the API
        let start_date = start_time.map(|dt| dt.date_naive());
        let end_date = end_time.map(|dt| dt.date_naive());

        self.fetch_kline(symbol, klt, 1, None, start_date, end_date)
            .await
    }

    async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        // Index symbols use the same API
        self.fetch_kline(symbol, 101, 0, None, start_date, end_date)
            .await
    }
}

// ============================================================================
// Eastmoney API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct EastmoneyKlineResponse {
    /// Return code (0 = success)
    rc: i32,
    /// Return message
    #[allow(dead_code)]
    rt: Option<i32>,
    /// Data
    data: Option<EastmoneyKlineData>,
}

#[derive(Debug, Deserialize)]
struct EastmoneyKlineData {
    /// Stock code
    #[allow(dead_code)]
    code: Option<String>,
    /// Market code
    #[allow(dead_code)]
    market: Option<i32>,
    /// Stock name
    #[allow(dead_code)]
    name: Option<String>,
    /// K-line data as strings
    klines: Option<Vec<String>>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_to_eastmoney_code() {
        assert_eq!(to_eastmoney_code("000001.SZ"), Some("0.000001".to_string()));
        assert_eq!(to_eastmoney_code("600000.SH"), Some("1.600000".to_string()));
        assert_eq!(to_eastmoney_code("INVALID"), None);
    }

    #[test]
    fn test_timeframe_to_klt() {
        assert_eq!(timeframe_to_klt(Timeframe::M1), 1);
        assert_eq!(timeframe_to_klt(Timeframe::M5), 5);
        assert_eq!(timeframe_to_klt(Timeframe::H1), 60);
        assert_eq!(timeframe_to_klt(Timeframe::Daily), 101);
        assert_eq!(timeframe_to_klt(Timeframe::Weekly), 102);
    }

    #[test]
    fn test_parse_eastmoney_datetime_daily() {
        let dt = parse_eastmoney_datetime("2024-01-02", Timeframe::Daily).unwrap();
        // 15:00 CST = 07:00 UTC
        assert_eq!(dt.hour(), 7);
    }

    #[test]
    fn test_parse_eastmoney_datetime_minute() {
        let dt = parse_eastmoney_datetime("2024-01-02 09:30", Timeframe::M1).unwrap();
        // 09:30 CST = 01:30 UTC
        assert_eq!(dt.hour(), 1);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_capabilities() {
        let adapter = AshareAdapter::new();
        let caps = adapter.capabilities();
        assert!(caps.supports_timeframe(Timeframe::Daily));
        assert!(caps.supports_timeframe(Timeframe::M5));
        assert!(caps.rate_limit_rpm.is_none()); // No rate limit
    }

    // Integration tests require network access
    // Run with: cargo test --features integration -- --ignored

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_fetch_daily_candles() {
        let adapter = AshareAdapter::new();
        let candles = adapter
            .get_daily_candles("000001.SZ", None, None, Some(10))
            .await
            .unwrap();

        assert!(!candles.is_empty());
        assert!(candles.len() <= 10);
        assert_eq!(candles[0].symbol, "000001.SZ");
        assert_eq!(candles[0].timeframe, Timeframe::Daily);
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_health_check() {
        let adapter = AshareAdapter::new();
        let result = adapter.health_check().await;
        assert!(result.is_ok());
    }
}
