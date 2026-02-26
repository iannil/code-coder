//! iTick API adapter for A-share market data.
//!
//! # API Documentation
//! <https://docs.itick.org/en/rest-api/stocks/stock-kline>
//!
//! # Features
//! - Real-time quotes via REST and WebSocket
//! - Minute/Daily K-line data (15 years history)
//! - Level-2 order book (10 levels)
//! - Free tier: 5 requests/second (300/minute)
//!
//! # Coverage
//! - A-shares (SH/SZ)
//! - Hong Kong stocks
//! - US stocks
//!
//! # Rate Limits
//! - Free tier: 5 requests/second
//! - Proactive rate limiting enabled to avoid 429 errors
//! - Recommended as primary data source

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use super::provider::{DataCapabilities, DataProvider, ProviderError};
use super::rate_limiter::{RateLimiter, SharedRateLimiter};
use super::{Candle, Timeframe};

// ============================================================================
// Constants
// ============================================================================

/// iTick API base URL
const ITICK_API_BASE: &str = "https://api.itick.org";

/// Stock K-line endpoint
const KLINE_ENDPOINT: &str = "/stock/kline";

/// Stock quote endpoint
#[allow(dead_code)] // Reserved for real-time quote fetching
const QUOTE_ENDPOINT: &str = "/stock/quote";

/// Default rate limit: 5 requests per second = 300 per minute
const DEFAULT_RATE_LIMIT_RPM: u32 = 300;

/// Retry delay after rate limit error (seconds)
const RATE_LIMIT_RETRY_SECS: u64 = 2;

// ============================================================================
// Symbol Mapping
// ============================================================================

/// Convert standard symbol format to iTick format.
///
/// Standard: "000001.SZ" -> iTick: region="SZ", code="000001"
/// Standard: "600000.SH" -> iTick: region="SH", code="600000"
fn parse_symbol(symbol: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = symbol.split('.').collect();
    if parts.len() != 2 {
        return None;
    }

    let code = parts[0].to_string();
    let region = parts[1].to_uppercase();

    // Validate region
    match region.as_str() {
        "SZ" | "SH" | "BJ" => Some((region, code)),
        _ => None,
    }
}

/// Determine if a symbol is an index
#[allow(dead_code)] // Reserved for index-specific logic
fn is_index_symbol(symbol: &str) -> bool {
    let code = symbol.split('.').next().unwrap_or("");

    // Shanghai indices starting with 000
    if symbol.ends_with(".SH") && code.starts_with("000") {
        return true;
    }

    // Shenzhen indices starting with 399
    if symbol.ends_with(".SZ") && code.starts_with("399") {
        return true;
    }

    false
}

// ============================================================================
// Timeframe Mapping
// ============================================================================

/// Convert Timeframe to iTick kType parameter
///
/// kType values:
/// - 1: 1 minute
/// - 2: 5 minutes
/// - 3: 15 minutes
/// - 4: 30 minutes
/// - 5: 1 hour
/// - 6: 2 hours
/// - 7: 4 hours
/// - 8: 1 day
/// - 9: 1 week
/// - 10: 1 month
fn timeframe_to_ktype(tf: Timeframe) -> i32 {
    match tf {
        Timeframe::M1 => 1,
        Timeframe::M5 => 2,
        Timeframe::M15 => 3,
        Timeframe::M30 => 4,
        Timeframe::H1 => 5,
        Timeframe::H4 => 7,
        Timeframe::Daily => 8,
        Timeframe::Weekly => 9,
    }
}

/// Convert kType to Timeframe
fn ktype_to_timeframe(ktype: i32) -> Timeframe {
    match ktype {
        1 => Timeframe::M1,
        2 => Timeframe::M5,
        3 => Timeframe::M15,
        4 => Timeframe::M30,
        5 => Timeframe::H1,
        7 => Timeframe::H4,
        8 => Timeframe::Daily,
        9 => Timeframe::Weekly,
        _ => Timeframe::Daily,
    }
}

// ============================================================================
// iTick Adapter
// ============================================================================

/// iTick adapter for A-share market data.
///
/// Provides real-time and historical market data through the iTick API.
/// Supports minute-level data, making it ideal for intraday strategies.
///
/// Rate limiting is applied proactively to avoid hitting API limits.
pub struct ITickAdapter {
    /// API key (token)
    api_key: String,
    /// HTTP client
    client: reqwest::Client,
    /// Priority level
    priority: u8,
    /// Rate limiter for proactive throttling
    rate_limiter: SharedRateLimiter,
}

impl ITickAdapter {
    /// Create a new iTick adapter with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_priority(api_key, 1) // Default to highest priority
    }

    /// Create with custom priority
    pub fn with_priority(api_key: impl Into<String>, priority: u8) -> Self {
        Self::with_rate_limit(api_key, priority, DEFAULT_RATE_LIMIT_RPM)
    }

    /// Create with custom priority and rate limit
    pub fn with_rate_limit(api_key: impl Into<String>, priority: u8, rate_limit_rpm: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let rate_limiter = Arc::new(RateLimiter::new("itick", rate_limit_rpm));

        Self {
            api_key: api_key.into(),
            client,
            priority,
            rate_limiter,
        }
    }

    /// Create from config
    pub fn from_config(config: &zero_common::config::Config) -> Option<Self> {
        let api_key = config.itick_api_key()?;

        // Check if itick is enabled in data_sources config
        let (priority, rate_limit_rpm) = config
            .trading
            .as_ref()
            .and_then(|t| t.data_sources.as_ref())
            .and_then(|ds| {
                ds.sources
                    .iter()
                    .find(|s| s.provider == "itick" && s.enabled)
                    .map(|s| {
                        let rpm = s
                            .config
                            .as_ref()
                            .and_then(|c| c.get("rate_limit_rpm"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32)
                            .unwrap_or(DEFAULT_RATE_LIMIT_RPM);
                        (s.priority, rpm)
                    })
            })
            .unwrap_or((1, DEFAULT_RATE_LIMIT_RPM));

        Some(Self::with_rate_limit(api_key, priority, rate_limit_rpm))
    }

    /// Fetch K-line data from iTick API
    async fn fetch_kline(
        &self,
        symbol: &str,
        ktype: i32,
        limit: Option<usize>,
        start_time: Option<i64>, // Unix timestamp in milliseconds
        end_time: Option<i64>,   // Unix timestamp in milliseconds
    ) -> Result<Vec<Candle>, ProviderError> {
        let (region, code) = parse_symbol(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        // Build query parameters
        let mut url = format!(
            "{}{}?region={}&code={}",
            ITICK_API_BASE, KLINE_ENDPOINT, region, code
        );

        url.push_str(&format!("&kType={}", ktype));

        if let Some(limit_val) = limit {
            url.push_str(&format!("&limit={}", limit_val.min(1000))); // API max is 1000
        }

        if let Some(ts) = start_time {
            url.push_str(&format!("&st={}", ts));
        }

        if let Some(ts) = end_time {
            url.push_str(&format!("&et={}", ts));
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await;

        debug!(url = %url, symbol = symbol, "Fetching kline from iTick");

        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .header("token", &self.api_key)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Network("Request timeout".into())
                } else if e.is_connect() {
                    ProviderError::Network("Connection failed".into())
                } else {
                    ProviderError::Network(e.to_string())
                }
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProviderError::Auth("Invalid API token".into()));
        }

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ProviderError::RateLimited {
                retry_after_secs: Some(RATE_LIMIT_RETRY_SECS),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Internal(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let result: ITickResponse<Vec<ITickKline>> = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        if result.code != 0 {
            let msg = result.msg.unwrap_or_else(|| "Unknown error".to_string());

            if msg.contains("token") || msg.contains("auth") || msg.contains("认证") {
                return Err(ProviderError::Auth(msg));
            }
            if msg.contains("limit") || msg.contains("频率") || msg.contains("rate") {
                return Err(ProviderError::RateLimited {
                    retry_after_secs: Some(RATE_LIMIT_RETRY_SECS),
                });
            }

            return Err(ProviderError::Internal(msg));
        }

        let data = result.data.unwrap_or_default();
        let timeframe = ktype_to_timeframe(ktype);
        self.parse_klines(symbol, timeframe, &data)
    }

    /// Parse iTick K-line data into Candles
    fn parse_klines(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        data: &[ITickKline],
    ) -> Result<Vec<Candle>, ProviderError> {
        let mut candles = Vec::with_capacity(data.len());

        for item in data {
            // Convert millisecond timestamp to DateTime
            let timestamp = Utc
                .timestamp_millis_opt(item.t)
                .single()
                .ok_or_else(|| ProviderError::Internal(format!("Invalid timestamp: {}", item.t)))?;

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe,
                timestamp,
                open: item.o,
                high: item.h,
                low: item.l,
                close: item.c,
                volume: item.v,
                amount: item.tu.unwrap_or(0.0),
            });
        }

        // Sort by timestamp ascending
        candles.sort_by_key(|c| c.timestamp);

        Ok(candles)
    }
}

// ============================================================================
// DataProvider Implementation
// ============================================================================

#[async_trait]
impl DataProvider for ITickAdapter {
    fn name(&self) -> &'static str {
        "itick"
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
                Timeframe::H4,
                Timeframe::Daily,
                Timeframe::Weekly,
            ],
            realtime_quotes: true,
            auction_data: false,
            index_data: true,
            etf_data: true,
            max_history_days: Some(365 * 15), // 15 years of history
            rate_limit_rpm: Some(300),        // 5/sec = 300/min
        }
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: fetch 1 daily candle for a well-known stock
        let candles = self
            .fetch_kline("000001.SZ", 8, Some(1), None, None)
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
        // Convert dates to timestamps
        let start_ts = start_date.map(|d| {
            d.and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis()
        });
        let end_ts = end_date.map(|d| {
            d.and_hms_opt(23, 59, 59)
                .unwrap()
                .and_utc()
                .timestamp_millis()
        });

        self.fetch_kline(symbol, 8, limit, start_ts, end_ts).await
    }

    async fn get_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let ktype = timeframe_to_ktype(timeframe);

        if ktype >= 8 {
            return Err(ProviderError::InvalidRequest(
                "Use get_daily_candles for daily/weekly data".into(),
            ));
        }

        let start_ts = start_time.map(|dt| dt.timestamp_millis());
        let end_ts = end_time.map(|dt| dt.timestamp_millis());

        self.fetch_kline(symbol, ktype, None, start_ts, end_ts)
            .await
    }

    async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        // Index symbols use the same API
        self.get_daily_candles(symbol, start_date, end_date, None)
            .await
    }
}

// ============================================================================
// API Response Types
// ============================================================================

/// iTick API response wrapper
#[derive(Debug, Deserialize)]
struct ITickResponse<T> {
    /// Response code (0 = success)
    code: i32,
    /// Error message (if any)
    msg: Option<String>,
    /// Response data
    data: Option<T>,
}

/// K-line data from iTick
///
/// Field names are abbreviated in the API:
/// - o: open
/// - h: high
/// - l: low
/// - c: close
/// - v: volume
/// - tu: transaction amount (turnover)
/// - t: timestamp (milliseconds)
#[derive(Debug, Deserialize)]
struct ITickKline {
    /// Opening price
    o: f64,
    /// Highest price
    h: f64,
    /// Lowest price
    l: f64,
    /// Closing price
    c: f64,
    /// Trading volume
    v: f64,
    /// Transaction amount (turnover)
    #[serde(default)]
    tu: Option<f64>,
    /// Timestamp in milliseconds
    t: i64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol() {
        assert_eq!(
            parse_symbol("000001.SZ"),
            Some(("SZ".to_string(), "000001".to_string()))
        );
        assert_eq!(
            parse_symbol("600000.SH"),
            Some(("SH".to_string(), "600000".to_string()))
        );
        assert_eq!(
            parse_symbol("000300.SH"),
            Some(("SH".to_string(), "000300".to_string()))
        );
        assert_eq!(parse_symbol("INVALID"), None);
        assert_eq!(parse_symbol("000001"), None);
    }

    #[test]
    fn test_is_index_symbol() {
        assert!(is_index_symbol("000001.SH")); // 上证指数
        assert!(is_index_symbol("000300.SH")); // 沪深300
        assert!(is_index_symbol("399001.SZ")); // 深证成指
        assert!(is_index_symbol("399006.SZ")); // 创业板指

        assert!(!is_index_symbol("000001.SZ")); // 平安银行
        assert!(!is_index_symbol("600000.SH")); // 浦发银行
    }

    #[test]
    fn test_timeframe_to_ktype() {
        assert_eq!(timeframe_to_ktype(Timeframe::M1), 1);
        assert_eq!(timeframe_to_ktype(Timeframe::M5), 2);
        assert_eq!(timeframe_to_ktype(Timeframe::M15), 3);
        assert_eq!(timeframe_to_ktype(Timeframe::M30), 4);
        assert_eq!(timeframe_to_ktype(Timeframe::H1), 5);
        assert_eq!(timeframe_to_ktype(Timeframe::H4), 7);
        assert_eq!(timeframe_to_ktype(Timeframe::Daily), 8);
        assert_eq!(timeframe_to_ktype(Timeframe::Weekly), 9);
    }

    #[test]
    fn test_ktype_to_timeframe() {
        assert_eq!(ktype_to_timeframe(1), Timeframe::M1);
        assert_eq!(ktype_to_timeframe(2), Timeframe::M5);
        assert_eq!(ktype_to_timeframe(8), Timeframe::Daily);
        assert_eq!(ktype_to_timeframe(9), Timeframe::Weekly);
        assert_eq!(ktype_to_timeframe(99), Timeframe::Daily); // Unknown defaults to Daily
    }

    #[test]
    fn test_capabilities() {
        let adapter = ITickAdapter::new("test-token");
        let caps = adapter.capabilities();

        assert!(caps.supports_timeframe(Timeframe::Daily));
        assert!(caps.supports_timeframe(Timeframe::Weekly));
        assert!(caps.supports_timeframe(Timeframe::M5));
        assert!(caps.supports_timeframe(Timeframe::H4));
        assert!(caps.realtime_quotes);
        assert!(caps.index_data);
        assert_eq!(caps.rate_limit_rpm, Some(300));
        assert_eq!(caps.max_history_days, Some(365 * 15));
    }

    #[test]
    fn test_provider_info() {
        let adapter = ITickAdapter::with_priority("test-token", 5);
        assert_eq!(adapter.name(), "itick");
        assert_eq!(adapter.priority(), 5);
    }

    // Integration tests require network access and valid token
    // Run with: ITICK_API_KEY=xxx cargo test --features integration -- --ignored

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_health_check() {
        let api_key = std::env::var("ITICK_API_KEY").expect("ITICK_API_KEY not set");
        let adapter = ITickAdapter::new(api_key);
        let result = adapter.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_fetch_daily_candles() {
        let api_key = std::env::var("ITICK_API_KEY").expect("ITICK_API_KEY not set");
        let adapter = ITickAdapter::new(api_key);

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
    #[ignore = "requires valid API token"]
    async fn test_fetch_minute_candles() {
        let api_key = std::env::var("ITICK_API_KEY").expect("ITICK_API_KEY not set");
        let adapter = ITickAdapter::new(api_key);

        let candles = adapter
            .get_minute_candles("000001.SZ", Timeframe::M5, None, None)
            .await
            .unwrap();

        assert!(!candles.is_empty());
        assert_eq!(candles[0].timeframe, Timeframe::M5);
    }
}
