//! Tushare Pro API adapter for A-share market data.
//!
//! # API Documentation
//! https://tushare.pro/document/2
//!
//! # Rate Limits
//! - Basic: 200 requests/minute
//! - Pro (积分2000+): 500 requests/minute
//!
//! # Data Types Supported
//! - Daily K-line: `daily`
//! - Minute K-line: `stk_mins` (1/5/15/30/60min)
//! - Real-time quotes: `quotes` (not REST, need websocket or polling)
//! - Auction data: `stk_auction`

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::{AuctionData, Candle, SymbolInfo, Timeframe};

/// Tushare API adapter
pub struct TushareAdapter {
    /// API token
    token: String,
    /// HTTP client
    client: reqwest::Client,
    /// API base URL
    base_url: String,
}

impl TushareAdapter {
    /// Create a new Tushare adapter
    pub fn new(token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            token: token.into(),
            client,
            base_url: "http://api.tushare.pro".to_string(),
        }
    }

    /// Create from config
    pub fn from_config(config: &zero_common::config::Config) -> Option<Self> {
        config
            .trading
            .as_ref()
            .and_then(|t| t.tushare_token.clone())
            .map(|token| Self::new(token))
    }

    /// Call the Tushare API
    async fn call_api<T: for<'de> Deserialize<'de>>(
        &self,
        api_name: &str,
        params: &HashMap<&str, String>,
        fields: Option<&[&str]>,
    ) -> Result<TushareResponse<T>> {
        let request = TushareRequest {
            api_name: api_name.to_string(),
            token: self.token.clone(),
            params: params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            fields: fields.map(|f| f.iter().map(|s| s.to_string()).collect()),
        };

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Tushare")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Tushare API error: {} - {}", status, body);
        }

        let result: TushareResponse<T> = response
            .json()
            .await
            .context("Failed to parse Tushare response")?;

        if result.code != 0 {
            anyhow::bail!(
                "Tushare API returned error: {} - {}",
                result.code,
                result.msg.unwrap_or_default()
            );
        }

        Ok(result)
    }

    /// Fetch daily K-line data
    pub async fn get_daily_candles(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>> {
        let mut params = HashMap::new();
        params.insert("ts_code", symbol.to_string());

        if let Some(start) = start_date {
            params.insert("start_date", start.format("%Y%m%d").to_string());
        }
        if let Some(end) = end_date {
            params.insert("end_date", end.format("%Y%m%d").to_string());
        }

        let fields = ["ts_code", "trade_date", "open", "high", "low", "close", "vol", "amount"];
        let response: TushareResponse<DailyKlineData> = self
            .call_api("daily", &params, Some(&fields))
            .await?;

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data returned"))?;
        let mut candles = self.parse_daily_candles(symbol, &data)?;

        // Apply limit
        if let Some(max) = limit {
            candles.truncate(max);
        }

        Ok(candles)
    }

    /// Fetch minute K-line data (requires Tushare Pro with high points)
    pub async fn get_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>> {
        let freq = match timeframe {
            Timeframe::M1 => "1min",
            Timeframe::M5 => "5min",
            Timeframe::M15 => "15min",
            Timeframe::M30 => "30min",
            Timeframe::H1 => "60min",
            _ => return Err(anyhow::anyhow!("Unsupported timeframe for minute data: {:?}", timeframe)),
        };

        let mut params = HashMap::new();
        params.insert("ts_code", symbol.to_string());
        params.insert("freq", freq.to_string());

        if let Some(start) = start_time {
            params.insert("start_date", start.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Some(end) = end_time {
            params.insert("end_date", end.format("%Y-%m-%d %H:%M:%S").to_string());
        }

        let fields = ["ts_code", "trade_time", "open", "high", "low", "close", "vol", "amount"];
        let response: TushareResponse<MinuteKlineData> = self
            .call_api("stk_mins", &params, Some(&fields))
            .await?;

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data returned"))?;
        self.parse_minute_candles(symbol, timeframe, &data)
    }

    /// Fetch auction data for pre-market analysis
    pub async fn get_auction_data(
        &self,
        symbol: &str,
        date: NaiveDate,
    ) -> Result<AuctionData> {
        let mut params = HashMap::new();
        params.insert("ts_code", symbol.to_string());
        params.insert("trade_date", date.format("%Y%m%d").to_string());

        let fields = ["ts_code", "trade_date", "open", "vol", "pct_change"];
        let response: TushareResponse<AuctionKlineData> = self
            .call_api("stk_auction", &params, Some(&fields))
            .await?;

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No auction data returned"))?;

        if data.items.is_empty() {
            anyhow::bail!("No auction data for {} on {}", symbol, date);
        }

        let item = &data.items[0];
        Ok(AuctionData {
            symbol: symbol.to_string(),
            date,
            expected_price: item.open,
            volume: item.vol,
            change_percent: item.pct_change.unwrap_or(0.0),
            timestamp: Utc::now(),
        })
    }

    /// Fetch stock basic info
    pub async fn get_stock_info(&self, symbol: &str) -> Result<SymbolInfo> {
        let mut params = HashMap::new();
        params.insert("ts_code", symbol.to_string());

        let fields = ["ts_code", "name", "exchange", "list_date", "industry"];
        let response: TushareResponse<StockBasicData> = self
            .call_api("stock_basic", &params, Some(&fields))
            .await?;

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No stock info returned"))?;

        if data.items.is_empty() {
            anyhow::bail!("Stock not found: {}", symbol);
        }

        let item = &data.items[0];
        Ok(SymbolInfo {
            symbol: item.ts_code.clone(),
            name: item.name.clone(),
            exchange: item.exchange.clone().unwrap_or_default(),
            list_date: item.list_date.clone().and_then(|s| NaiveDate::parse_from_str(&s, "%Y%m%d").ok()),
            industry: item.industry.clone(),
            is_suspended: false,
        })
    }

    /// Fetch index daily data (for CSI 300, CSI 500, etc.)
    pub async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>> {
        let mut params = HashMap::new();
        params.insert("ts_code", symbol.to_string());

        if let Some(start) = start_date {
            params.insert("start_date", start.format("%Y%m%d").to_string());
        }
        if let Some(end) = end_date {
            params.insert("end_date", end.format("%Y%m%d").to_string());
        }

        let fields = ["ts_code", "trade_date", "open", "high", "low", "close", "vol", "amount"];
        let response: TushareResponse<DailyKlineData> = self
            .call_api("index_daily", &params, Some(&fields))
            .await?;

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data returned"))?;
        self.parse_daily_candles(symbol, &data)
    }

    // ========================================================================
    // Parsing Helpers
    // ========================================================================

    fn parse_daily_candles(&self, symbol: &str, data: &DailyKlineData) -> Result<Vec<Candle>> {
        let mut candles = Vec::with_capacity(data.items.len());

        for item in &data.items {
            let date = NaiveDate::parse_from_str(&item.trade_date, "%Y%m%d")
                .context("Failed to parse trade date")?;

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp: date.and_hms_opt(15, 0, 0).unwrap().and_utc(),
                open: item.open,
                high: item.high,
                low: item.low,
                close: item.close,
                volume: item.vol.unwrap_or(0.0),
                amount: item.amount.unwrap_or(0.0),
            });
        }

        // Sort by timestamp ascending
        candles.sort_by_key(|c| c.timestamp);

        Ok(candles)
    }

    fn parse_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        data: &MinuteKlineData,
    ) -> Result<Vec<Candle>> {
        let mut candles = Vec::with_capacity(data.items.len());

        for item in &data.items {
            let timestamp = DateTime::parse_from_str(&item.trade_time, "%Y-%m-%d %H:%M:%S")
                .map(|dt| dt.with_timezone(&Utc))
                .context("Failed to parse trade time")?;

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe,
                timestamp,
                open: item.open,
                high: item.high,
                low: item.low,
                close: item.close,
                volume: item.vol.unwrap_or(0.0),
                amount: item.amount.unwrap_or(0.0),
            });
        }

        // Sort by timestamp ascending
        candles.sort_by_key(|c| c.timestamp);

        Ok(candles)
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct TushareRequest {
    api_name: String,
    token: String,
    params: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TushareResponse<T> {
    code: i32,
    msg: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct DailyKlineData {
    #[serde(default)]
    items: Vec<DailyKlineItem>,
}

#[derive(Debug, Deserialize)]
struct DailyKlineItem {
    #[serde(default)]
    ts_code: String,
    trade_date: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[serde(default)]
    vol: Option<f64>,
    #[serde(default)]
    amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct MinuteKlineData {
    #[serde(default)]
    items: Vec<MinuteKlineItem>,
}

#[derive(Debug, Deserialize)]
struct MinuteKlineItem {
    #[serde(default)]
    ts_code: String,
    trade_time: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[serde(default)]
    vol: Option<f64>,
    #[serde(default)]
    amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AuctionKlineData {
    #[serde(default)]
    items: Vec<AuctionKlineItem>,
}

#[derive(Debug, Deserialize)]
struct AuctionKlineItem {
    #[serde(default)]
    ts_code: String,
    #[serde(default)]
    trade_date: String,
    open: f64,
    vol: f64,
    #[serde(default)]
    pct_change: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct StockBasicData {
    #[serde(default)]
    items: Vec<StockBasicItem>,
}

#[derive(Debug, Deserialize)]
struct StockBasicItem {
    ts_code: String,
    name: String,
    #[serde(default)]
    exchange: Option<String>,
    #[serde(default)]
    list_date: Option<String>,
    #[serde(default)]
    industry: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tushare_adapter_creation() {
        let adapter = TushareAdapter::new("test_token");
        assert_eq!(adapter.token, "test_token");
        assert_eq!(adapter.base_url, "http://api.tushare.pro");
    }

    // Integration tests require a valid Tushare token
    // Run with: TUSHARE_TOKEN=xxx cargo test --features integration
}
