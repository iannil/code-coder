//! 理杏仁 (Lixinger) API adapter for A-share market data.
//!
//! # API Documentation
//! https://www.lixinger.com/open/api
//!
//! # Features
//! - High-quality fundamental data
//! - Daily K-line data for stocks and indices
//! - Requires API token (paid service)
//!
//! # Rate Limits
//! - Varies by subscription tier
//! - Recommended as backup data source

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

use super::provider::{DataCapabilities, DataProvider, ProviderError};
use super::{Candle, Timeframe};

// ============================================================================
// Constants
// ============================================================================

/// Lixinger API base URL
const LIXIN_API_BASE: &str = "https://open.lixinger.com/api";

/// Stock daily candlestick endpoint
const STOCK_DAILY_ENDPOINT: &str = "/a/stock/fs/daily-candlestick";

/// Index daily candlestick endpoint
const INDEX_DAILY_ENDPOINT: &str = "/a/index/fs/daily-candlestick";

// ============================================================================
// Symbol Mapping
// ============================================================================

/// Convert standard symbol format to Lixinger format.
///
/// Standard: "000001.SZ" -> Lixinger: "000001" (without exchange suffix for stocks)
/// Index: "000300.SH" -> Lixinger: "000300"
fn to_lixin_code(symbol: &str) -> Option<String> {
    let parts: Vec<&str> = symbol.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    Some(parts[0].to_string())
}

/// Determine if a symbol is an index
fn is_index_symbol(symbol: &str) -> bool {
    // Common A-share index patterns:
    // 000001.SH (上证指数), 000300.SH (沪深300), 000905.SH (中证500)
    // 399001.SZ (深证成指), 399006.SZ (创业板指)
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
// Lixin Adapter
// ============================================================================

/// Lixinger (理杏仁) adapter for A-share market data.
///
/// Provides high-quality financial data through the Lixinger Open API.
/// Requires a paid API token.
pub struct LixinAdapter {
    /// API token
    token: String,
    /// HTTP client
    client: reqwest::Client,
    /// Priority level
    priority: u8,
}

impl LixinAdapter {
    /// Create a new Lixin adapter with token
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_priority(token, 2) // Default priority 2 (after Ashare)
    }

    /// Create with custom priority
    pub fn with_priority(token: impl Into<String>, priority: u8) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            token: token.into(),
            client,
            priority,
        }
    }

    /// Create from config
    pub fn from_config(config: &zero_common::config::Config) -> Option<Self> {
        let trading = config.trading.as_ref()?;
        let token = trading.lixin_token.as_ref()?;

        // Check if lixin is enabled in data_sources config
        let priority = trading
            .data_sources
            .as_ref()
            .and_then(|ds| {
                ds.sources
                    .iter()
                    .find(|s| s.provider == "lixin" && s.enabled)
                    .map(|s| s.priority)
            })
            .unwrap_or(2);

        Some(Self::with_priority(token.clone(), priority))
    }

    /// Call the Lixinger API
    async fn call_api<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        request: &LixinRequest,
    ) -> Result<LixinResponse<T>, ProviderError> {
        let url = format!("{}{}", LIXIN_API_BASE, endpoint);

        debug!(url = %url, "Calling Lixinger API");

        let response = self
            .client
            .post(&url)
            .json(request)
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
                retry_after_secs: Some(60),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Internal(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let result: LixinResponse<T> = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        if result.code != 0 {
            let msg = result.message.unwrap_or_else(|| "Unknown error".to_string());

            if msg.contains("token") || msg.contains("认证") || msg.contains("授权") {
                return Err(ProviderError::Auth(msg));
            }
            if msg.contains("频率") || msg.contains("限制") {
                return Err(ProviderError::RateLimited {
                    retry_after_secs: Some(60),
                });
            }

            return Err(ProviderError::Internal(msg));
        }

        Ok(result)
    }

    /// Fetch daily candlestick data for a stock
    async fn fetch_stock_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinRequest {
            token: self.token.clone(),
            stock_codes: Some(vec![stock_code]),
            index_codes: None,
            start_date: start_date.map(|d| d.format("%Y-%m-%d").to_string()),
            end_date: end_date.map(|d| d.format("%Y-%m-%d").to_string()),
            limit: limit.map(|l| l as i32),
        };

        let response: LixinResponse<Vec<LixinCandlestick>> = self
            .call_api(STOCK_DAILY_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        self.parse_candlesticks(symbol, &data)
    }

    /// Fetch daily candlestick data for an index
    async fn fetch_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let index_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinRequest {
            token: self.token.clone(),
            stock_codes: None,
            index_codes: Some(vec![index_code]),
            start_date: start_date.map(|d| d.format("%Y-%m-%d").to_string()),
            end_date: end_date.map(|d| d.format("%Y-%m-%d").to_string()),
            limit: None,
        };

        let response: LixinResponse<Vec<LixinCandlestick>> = self
            .call_api(INDEX_DAILY_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        self.parse_candlesticks(symbol, &data)
    }

    /// Parse Lixinger candlestick data into Candles
    fn parse_candlesticks(
        &self,
        symbol: &str,
        data: &[LixinCandlestick],
    ) -> Result<Vec<Candle>, ProviderError> {
        let mut candles = Vec::with_capacity(data.len());

        for item in data {
            let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                .map_err(|e| ProviderError::Internal(format!("Failed to parse date: {}", e)))?;

            // Use 15:00 (market close) as timestamp
            let timestamp = date.and_hms_opt(15, 0, 0).unwrap().and_utc();

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp,
                open: item.open,
                high: item.high,
                low: item.low,
                close: item.close,
                volume: item.volume.unwrap_or(0.0),
                amount: item.amount.unwrap_or(0.0),
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
impl DataProvider for LixinAdapter {
    fn name(&self) -> &'static str {
        "lixin"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn capabilities(&self) -> DataCapabilities {
        DataCapabilities {
            timeframes: vec![Timeframe::Daily, Timeframe::Weekly],
            realtime_quotes: false,
            auction_data: false,
            index_data: true,
            etf_data: true,
            max_history_days: Some(365 * 20), // ~20 years of history
            rate_limit_rpm: Some(100),        // Conservative estimate
        }
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Simple health check: fetch 1 day of data for a well-known stock
        let today = Utc::now().date_naive();
        let start = today - chrono::Duration::days(7);

        let request = LixinRequest {
            token: self.token.clone(),
            stock_codes: Some(vec!["000001".to_string()]), // 平安银行
            index_codes: None,
            start_date: Some(start.format("%Y-%m-%d").to_string()),
            end_date: Some(today.format("%Y-%m-%d").to_string()),
            limit: Some(1),
        };

        let _: LixinResponse<Vec<LixinCandlestick>> = self
            .call_api(STOCK_DAILY_ENDPOINT, &request)
            .await?;

        Ok(())
    }

    async fn get_daily_candles(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        if is_index_symbol(symbol) {
            self.fetch_index_daily(symbol, start_date, end_date).await
        } else {
            self.fetch_stock_daily(symbol, start_date, end_date, limit).await
        }
    }

    async fn get_minute_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        // Lixinger doesn't provide minute-level data in their standard API
        Err(ProviderError::DataNotAvailable(
            "Lixinger does not support minute-level data".into(),
        ))
    }

    async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        self.fetch_index_daily(symbol, start_date, end_date).await
    }
}

// ============================================================================
// API Types
// ============================================================================

/// Lixinger API request
#[derive(Debug, Serialize)]
struct LixinRequest {
    /// API token
    token: String,
    /// Stock codes (for stock endpoints)
    #[serde(rename = "stockCodes", skip_serializing_if = "Option::is_none")]
    stock_codes: Option<Vec<String>>,
    /// Index codes (for index endpoints)
    #[serde(rename = "indexCodes", skip_serializing_if = "Option::is_none")]
    index_codes: Option<Vec<String>>,
    /// Start date (YYYY-MM-DD)
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    /// End date (YYYY-MM-DD)
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
    /// Limit number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i32>,
}

/// Lixinger API response wrapper
#[derive(Debug, Deserialize)]
struct LixinResponse<T> {
    /// Response code (0 = success)
    code: i32,
    /// Error message (if any)
    message: Option<String>,
    /// Response data
    data: Option<T>,
}

/// Candlestick data from Lixinger
#[derive(Debug, Deserialize)]
struct LixinCandlestick {
    /// Date (YYYY-MM-DD)
    date: String,
    /// Opening price
    open: f64,
    /// Highest price
    high: f64,
    /// Lowest price
    low: f64,
    /// Closing price
    close: f64,
    /// Trading volume
    #[serde(default)]
    volume: Option<f64>,
    /// Trading amount
    #[serde(default)]
    amount: Option<f64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_lixin_code() {
        assert_eq!(to_lixin_code("000001.SZ"), Some("000001".to_string()));
        assert_eq!(to_lixin_code("600000.SH"), Some("600000".to_string()));
        assert_eq!(to_lixin_code("000300.SH"), Some("000300".to_string()));
        assert_eq!(to_lixin_code("INVALID"), None);
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
    fn test_capabilities() {
        let adapter = LixinAdapter::new("test-token");
        let caps = adapter.capabilities();

        assert!(caps.supports_timeframe(Timeframe::Daily));
        assert!(caps.supports_timeframe(Timeframe::Weekly));
        assert!(!caps.supports_timeframe(Timeframe::M5)); // No minute data
        assert!(caps.index_data);
        assert_eq!(caps.rate_limit_rpm, Some(100));
    }

    #[test]
    fn test_provider_info() {
        let adapter = LixinAdapter::with_priority("test-token", 5);
        assert_eq!(adapter.name(), "lixin");
        assert_eq!(adapter.priority(), 5);
    }

    // Integration tests require network access and valid token
    // Run with: cargo test --features integration -- --ignored

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_health_check() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);
        let result = adapter.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_fetch_daily_candles() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let candles = adapter
            .get_daily_candles("000001.SZ", None, None, Some(10))
            .await
            .unwrap();

        assert!(!candles.is_empty());
        assert!(candles.len() <= 10);
        assert_eq!(candles[0].symbol, "000001.SZ");
    }
}
