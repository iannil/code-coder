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
//! - Default: 100 requests/minute (conservative)
//! - Proactive rate limiting enabled to avoid errors
//! - Recommended as backup data source

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use super::provider::{DataCapabilities, DataProvider, ProviderError, StockInfo, FinancialStatementData};
use super::rate_limiter::{RateLimiter, SharedRateLimiter};
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

/// Stock list endpoint
const STOCK_LIST_ENDPOINT: &str = "/a/stock";

/// Balance sheet endpoint
const BALANCE_SHEET_ENDPOINT: &str = "/a/stock/fs/balance-sheet";

/// Income statement endpoint
const INCOME_STATEMENT_ENDPOINT: &str = "/a/stock/fs/income-statement";

/// Cash flow statement endpoint
const CASH_FLOW_ENDPOINT: &str = "/a/stock/fs/cash-flow-statement";

/// Default rate limit: 100 requests per minute (conservative)
const DEFAULT_RATE_LIMIT_RPM: u32 = 100;

/// Retry delay after rate limit error (seconds)
const RATE_LIMIT_RETRY_SECS: u64 = 10;

/// Maximum stocks per batch request
const MAX_BATCH_SIZE: usize = 100;

/// Non-financial fundamental data endpoint (valuation metrics)
const NON_FINANCIAL_ENDPOINT: &str = "/api/cn/company/fundamental/non_financial";

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
///
/// Rate limiting is applied proactively to avoid hitting API limits.
pub struct LixinAdapter {
    /// API token
    token: String,
    /// HTTP client
    client: reqwest::Client,
    /// Priority level
    priority: u8,
    /// Rate limiter for proactive throttling
    rate_limiter: SharedRateLimiter,
}

impl LixinAdapter {
    /// Create a new Lixin adapter with token
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_priority(token, 2) // Default priority 2 (after iTick)
    }

    /// Create with custom priority
    pub fn with_priority(token: impl Into<String>, priority: u8) -> Self {
        Self::with_rate_limit(token, priority, DEFAULT_RATE_LIMIT_RPM)
    }

    /// Create with custom priority and rate limit
    pub fn with_rate_limit(token: impl Into<String>, priority: u8, rate_limit_rpm: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let rate_limiter = Arc::new(RateLimiter::new("lixin", rate_limit_rpm));

        Self {
            token: token.into(),
            client,
            priority,
            rate_limiter,
        }
    }

    /// Create from config
    pub fn from_config(config: &zero_common::config::Config) -> Option<Self> {
        let token = config.lixin_token()?;

        // Check if lixin is enabled in data_sources config
        let (priority, rate_limit_rpm) = config
            .trading
            .as_ref()
            .and_then(|t| t.data_sources.as_ref())
            .and_then(|ds| {
                ds.sources
                    .iter()
                    .find(|s| s.provider == "lixin" && s.enabled)
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
            .unwrap_or((2, DEFAULT_RATE_LIMIT_RPM));

        Some(Self::with_rate_limit(token, priority, rate_limit_rpm))
    }

    /// Call the Lixinger API with a generic request type
    async fn call_api<R, T>(&self, endpoint: &str, request: &R) -> Result<LixinResponse<T>, ProviderError>
    where
        R: Serialize,
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", LIXIN_API_BASE, endpoint);

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await;

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
            // Try to parse Retry-After header
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            return Err(ProviderError::RateLimited {
                retry_after_secs: retry_after.or(Some(RATE_LIMIT_RETRY_SECS)),
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
                    retry_after_secs: Some(RATE_LIMIT_RETRY_SECS),
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
            // SAFETY: 15:00:00 is always a valid time, so and_hms_opt cannot return None
            let timestamp = date
                .and_hms_opt(15, 0, 0)
                .expect("15:00:00 is a valid time")
                .and_utc();

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

    // ========================================================================
    // Stock Screener Methods
    // ========================================================================

    /// Fetch all stocks from Lixin API
    async fn fetch_stock_list(&self) -> Result<Vec<StockInfo>, ProviderError> {
        let request = LixinStockListRequest {
            token: self.token.clone(),
        };

        let response: LixinResponse<Vec<LixinStockItem>> = self
            .call_api(STOCK_LIST_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        let mut stocks = Vec::with_capacity(data.len());

        for item in data {
            let exchange = if item.stock_code.starts_with("6") {
                "SH"
            } else if item.stock_code.starts_with("0") || item.stock_code.starts_with("3") {
                "SZ"
            } else if item.stock_code.starts_with("8") || item.stock_code.starts_with("4") {
                "BJ"
            } else {
                "UNKNOWN"
            };

            let list_date = item.list_date
                .as_ref()
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

            let is_st = item.name.contains("ST") || item.name.contains("*ST");

            stocks.push(StockInfo {
                code: item.stock_code,
                name: item.name,
                exchange: exchange.to_string(),
                industry: item.industry,
                list_date,
                is_suspended: item.is_suspended.unwrap_or(false),
                is_st,
                market_cap: item.market_cap,
            });
        }

        debug!(count = stocks.len(), "Fetched stock list from Lixin");
        Ok(stocks)
    }

    /// Fetch financial data for a single stock
    async fn fetch_financial_data(
        &self,
        symbol: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<FinancialStatementData, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        // Fetch all three financial statements
        let balance_sheet = self.fetch_balance_sheet(&stock_code, period_end).await?;
        let income_statement = self.fetch_income_statement(&stock_code, period_end).await?;
        let cash_flow = self.fetch_cash_flow(&stock_code, period_end).await?;

        // Combine into FinancialStatementData
        let period = period_end.unwrap_or_else(|| {
            balance_sheet.first()
                .and_then(|b| NaiveDate::parse_from_str(&b.report_date, "%Y-%m-%d").ok())
                .unwrap_or_else(|| chrono::Local::now().date_naive())
        });

        let bs = balance_sheet.first().ok_or_else(|| {
            ProviderError::DataNotAvailable(format!("No balance sheet data for {}", symbol))
        })?;
        let is = income_statement.first().ok_or_else(|| {
            ProviderError::DataNotAvailable(format!("No income statement data for {}", symbol))
        })?;
        let cf = cash_flow.first().ok_or_else(|| {
            ProviderError::DataNotAvailable(format!("No cash flow data for {}", symbol))
        })?;

        Ok(FinancialStatementData {
            symbol: symbol.to_string(),
            period_end: period,
            report_type: "annual".to_string(),

            // Income Statement
            revenue: is.revenue,
            gross_profit: is.gross_profit,
            operating_income: is.operating_income,
            net_income: is.net_income,
            interest_expense: is.interest_expense,

            // Balance Sheet
            total_assets: bs.total_assets,
            total_equity: bs.total_equity,
            total_liabilities: bs.total_liabilities,
            cash: bs.cash_and_equivalents,
            total_debt: bs.total_debt,
            shares_outstanding: bs.shares_outstanding,

            // Cash Flow
            operating_cash_flow: cf.operating_cash_flow,
            investing_cash_flow: cf.investing_cash_flow,
            financing_cash_flow: cf.financing_cash_flow,
            capex: cf.capex,

            // Derived metrics
            roe: bs.roe,
            roa: bs.roa,
            gross_margin: is.gross_margin,
            net_margin: is.net_margin,
            debt_to_equity: bs.debt_to_equity,
            current_ratio: bs.current_ratio,
            pe_ttm: bs.pe_ttm,
            pb: bs.pb,
            dividend_yield: bs.dividend_yield,
        })
    }

    /// Fetch balance sheet data
    async fn fetch_balance_sheet(
        &self,
        stock_code: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinBalanceSheet>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: vec![stock_code.to_string()],
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: Some(vec![
                "total_assets".to_string(),
                "total_equity".to_string(),
                "total_liabilities".to_string(),
                "cash_and_equivalents".to_string(),
                "total_debt".to_string(),
                "shares_outstanding".to_string(),
                "roe".to_string(),
                "roa".to_string(),
                "debt_to_equity".to_string(),
                "current_ratio".to_string(),
                "pe_ttm".to_string(),
                "pb".to_string(),
                "dividend_yield".to_string(),
            ]),
        };

        let response: LixinResponse<Vec<LixinBalanceSheet>> = self
            .call_api(BALANCE_SHEET_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    /// Fetch income statement data
    async fn fetch_income_statement(
        &self,
        stock_code: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinIncomeStatement>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: vec![stock_code.to_string()],
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: Some(vec![
                "revenue".to_string(),
                "gross_profit".to_string(),
                "operating_income".to_string(),
                "net_income".to_string(),
                "interest_expense".to_string(),
                "gross_margin".to_string(),
                "net_margin".to_string(),
            ]),
        };

        let response: LixinResponse<Vec<LixinIncomeStatement>> = self
            .call_api(INCOME_STATEMENT_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    /// Fetch cash flow statement data
    async fn fetch_cash_flow(
        &self,
        stock_code: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinCashFlow>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: vec![stock_code.to_string()],
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: Some(vec![
                "operating_cash_flow".to_string(),
                "investing_cash_flow".to_string(),
                "financing_cash_flow".to_string(),
                "capex".to_string(),
            ]),
        };

        let response: LixinResponse<Vec<LixinCashFlow>> = self
            .call_api(CASH_FLOW_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    /// Batch fetch financial data for multiple stocks
    async fn batch_fetch_financial_data(
        &self,
        symbols: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<FinancialStatementData>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());

        // Process in batches to respect API limits
        for chunk in symbols.chunks(MAX_BATCH_SIZE) {
            let stock_codes: Vec<String> = chunk
                .iter()
                .filter_map(|s| to_lixin_code(s))
                .collect();

            if stock_codes.is_empty() {
                continue;
            }

            // Fetch all financial data for this batch
            let balance_sheets = self.batch_fetch_balance_sheets(&stock_codes, period_end).await?;
            let income_statements = self.batch_fetch_income_statements(&stock_codes, period_end).await?;
            let cash_flows = self.batch_fetch_cash_flows(&stock_codes, period_end).await?;

            // Combine data by stock code
            for symbol in chunk.iter() {
                let code = match to_lixin_code(symbol) {
                    Some(c) => c,
                    None => continue,
                };

                let bs = balance_sheets.iter().find(|b| b.stock_code == code);
                let is = income_statements.iter().find(|i| i.stock_code == code);
                let cf = cash_flows.iter().find(|c| c.stock_code == code);

                if let (Some(bs), Some(is), Some(cf)) = (bs, is, cf) {
                    let period = period_end.unwrap_or_else(|| {
                        NaiveDate::parse_from_str(&bs.report_date, "%Y-%m-%d")
                            .unwrap_or_else(|_| chrono::Local::now().date_naive())
                    });

                    results.push(FinancialStatementData {
                        symbol: symbol.clone(),
                        period_end: period,
                        report_type: "annual".to_string(),
                        revenue: is.revenue,
                        gross_profit: is.gross_profit,
                        operating_income: is.operating_income,
                        net_income: is.net_income,
                        interest_expense: is.interest_expense,
                        total_assets: bs.total_assets,
                        total_equity: bs.total_equity,
                        total_liabilities: bs.total_liabilities,
                        cash: bs.cash_and_equivalents,
                        total_debt: bs.total_debt,
                        shares_outstanding: bs.shares_outstanding,
                        operating_cash_flow: cf.operating_cash_flow,
                        investing_cash_flow: cf.investing_cash_flow,
                        financing_cash_flow: cf.financing_cash_flow,
                        capex: cf.capex,
                        roe: bs.roe,
                        roa: bs.roa,
                        gross_margin: is.gross_margin,
                        net_margin: is.net_margin,
                        debt_to_equity: bs.debt_to_equity,
                        current_ratio: bs.current_ratio,
                        pe_ttm: bs.pe_ttm,
                        pb: bs.pb,
                        dividend_yield: bs.dividend_yield,
                    });
                }
            }
        }

        debug!(
            count = results.len(),
            requested = symbols.len(),
            "Batch fetched financial data from Lixin"
        );
        Ok(results)
    }

    async fn batch_fetch_balance_sheets(
        &self,
        stock_codes: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinBalanceSheet>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: stock_codes.to_vec(),
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: None,
        };

        let response: LixinResponse<Vec<LixinBalanceSheet>> = self
            .call_api(BALANCE_SHEET_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    async fn batch_fetch_income_statements(
        &self,
        stock_codes: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinIncomeStatement>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: stock_codes.to_vec(),
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: None,
        };

        let response: LixinResponse<Vec<LixinIncomeStatement>> = self
            .call_api(INCOME_STATEMENT_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    async fn batch_fetch_cash_flows(
        &self,
        stock_codes: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<LixinCashFlow>, ProviderError> {
        let request = LixinFinancialRequest {
            token: self.token.clone(),
            stock_codes: stock_codes.to_vec(),
            date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
            metrics: None,
        };

        let response: LixinResponse<Vec<LixinCashFlow>> = self
            .call_api(CASH_FLOW_ENDPOINT, &request)
            .await?;

        Ok(response.data.unwrap_or_default())
    }

    // ========================================================================
    // Valuation Metrics Methods
    // ========================================================================

    /// Fetch valuation metrics for a single stock
    async fn fetch_valuation_metrics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<ValuationMetrics, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinNonFinancialRequest {
            token: self.token.clone(),
            stock_codes: vec![stock_code.clone()],
            metrics_list: Some(ValuationMetricName::all_metrics()
                .into_iter()
                .map(String::from)
                .collect()),
            date: date.map(|d| d.format("%Y-%m-%d").to_string()),
            start_date: None,
            end_date: None,
            limit: Some(1),
        };

        let response: LixinResponse<Vec<LixinNonFinancialData>> = self
            .call_api(NON_FINANCIAL_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        let item = data.first()
            .ok_or_else(|| ProviderError::DataNotAvailable(format!("No valuation data for {}", symbol)))?;

        self.convert_to_valuation_metrics(symbol, item)
    }

    /// Batch fetch valuation metrics for multiple stocks
    async fn batch_fetch_valuation_metrics(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());

        // Process in batches to respect API limits
        for chunk in symbols.chunks(MAX_BATCH_SIZE) {
            let stock_codes: Vec<String> = chunk
                .iter()
                .filter_map(|s| to_lixin_code(s))
                .collect();

            if stock_codes.is_empty() {
                continue;
            }

            let request = LixinNonFinancialRequest {
                token: self.token.clone(),
                stock_codes: stock_codes.clone(),
                metrics_list: Some(ValuationMetricName::all_metrics()
                    .into_iter()
                    .map(String::from)
                    .collect()),
                date: date.map(|d| d.format("%Y-%m-%d").to_string()),
                start_date: None,
                end_date: None,
                limit: None,
            };

            let response: LixinResponse<Vec<LixinNonFinancialData>> = self
                .call_api(NON_FINANCIAL_ENDPOINT, &request)
                .await?;

            let data = response.data.unwrap_or_default();

            // Map results back to original symbols
            for item in data {
                // Find the original symbol that matches this stock code
                let original_symbol = chunk.iter()
                    .find(|s| to_lixin_code(s).as_deref() == Some(item.stock_code.as_str()))
                    .map(|s| s.as_str());

                if let Some(symbol) = original_symbol {
                    match self.convert_to_valuation_metrics(symbol, &item) {
                        Ok(metrics) => results.push(metrics),
                        Err(e) => {
                            debug!(symbol = %item.stock_code, error = %e, "Failed to convert valuation metrics");
                        }
                    }
                }
            }
        }

        debug!(
            count = results.len(),
            requested = symbols.len(),
            "Batch fetched valuation metrics from Lixin"
        );
        Ok(results)
    }

    /// Convert Lixinger API data to unified ValuationMetrics
    fn convert_to_valuation_metrics(
        &self,
        symbol: &str,
        data: &LixinNonFinancialData,
    ) -> Result<ValuationMetrics, ProviderError> {
        let date = NaiveDate::parse_from_str(&data.date, "%Y-%m-%d")
            .map_err(|e| ProviderError::Internal(format!("Failed to parse date: {}", e)))?;

        Ok(ValuationMetrics {
            symbol: symbol.to_string(),
            date,
            pe_ttm: data.pe_ttm,
            pe_ttm_ex_non_recurring: data.d_pe_ttm,
            pb: data.pb,
            ps_ttm: data.ps_ttm,
            dividend_yield: data.dividend_yield,
            market_cap: data.market_cap,
            circulating_market_cap: data.circulating_market_cap,
            free_float_market_cap: data.free_float_market_cap,
            financing_balance: data.financing_balance,
            short_balance: data.short_balance,
            northbound_holdings_shares: data.northbound_holdings_shares,
            northbound_holdings_value: data.northbound_holdings_value,
        })
    }

    /// Fetch valuation statistics with historical percentile data
    ///
    /// This method requires the statistics API which provides percentile
    /// rankings for metrics over different time periods.
    async fn fetch_valuation_statistics(
        &self,
        symbol: &str,
        metrics: &[ValuationMetricName],
        granularities: &[StatisticsGranularity],
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatisticsSet, ProviderError> {
        // For now, we construct a basic statistics set
        // Full implementation would call the statistics-specific endpoint
        let current_metrics = self.fetch_valuation_metrics(symbol, date).await?;

        let mut pe_ttm_stats = Vec::new();
        let mut pb_stats = Vec::new();
        let mut dividend_yield_stats = Vec::new();

        // Build statistics structures for each requested metric and granularity
        for &metric in metrics {
            for &granularity in granularities {
                let stats = self.fetch_metric_statistics(symbol, metric, granularity, date).await?;

                match metric {
                    ValuationMetricName::PeTtm | ValuationMetricName::DPeTtm => {
                        pe_ttm_stats.push(stats);
                    }
                    ValuationMetricName::Pb => {
                        pb_stats.push(stats);
                    }
                    ValuationMetricName::Dyr => {
                        dividend_yield_stats.push(stats);
                    }
                    _ => {}
                }
            }
        }

        Ok(ValuationStatisticsSet {
            symbol: symbol.to_string(),
            date: current_metrics.date,
            pe_ttm_stats,
            pb_stats,
            dividend_yield_stats,
        })
    }

    /// Fetch statistics for a single metric
    async fn fetch_metric_statistics(
        &self,
        symbol: &str,
        metric: ValuationMetricName,
        granularity: StatisticsGranularity,
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatistics, ProviderError> {
        // Get current value first
        let metrics = self.fetch_valuation_metrics(symbol, date).await?;
        let current_value = match metric {
            ValuationMetricName::PeTtm => metrics.pe_ttm.unwrap_or(0.0),
            ValuationMetricName::DPeTtm => metrics.pe_ttm_ex_non_recurring.unwrap_or(0.0),
            ValuationMetricName::Pb => metrics.pb.unwrap_or(0.0),
            ValuationMetricName::Dyr => metrics.dividend_yield.unwrap_or(0.0),
            _ => 0.0,
        };

        if current_value == 0.0 {
            return Ok(ValuationStatistics {
                metric,
                granularity,
                current_value,
                percentile: None,
                q25: None,
                q50: None,
                q80: None,
                min: None,
                max: None,
                avg: None,
            });
        }

        // The full Lixinger statistics API would be called here
        // For now, return placeholder structure
        // TODO: Implement full statistics API call when available
        Ok(ValuationStatistics {
            metric,
            granularity,
            current_value,
            percentile: None,  // Would come from API's cvpos field
            q25: None,         // Would come from API's q2v field
            q50: None,         // Would come from API's q5v field
            q80: None,         // Would come from API's q8v field
            min: None,         // Would come from API's minv field
            max: None,         // Would come from API's maxv field
            avg: None,         // Would come from API's avgv field
        })
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

        match self.call_api::<_, Vec<LixinCandlestick>>(STOCK_DAILY_ENDPOINT, &request).await {
            Ok(_) => Ok(()),
            // Rate limited means the API is working, just temporarily throttled
            // This should not be considered unhealthy
            Err(ProviderError::RateLimited { .. }) => {
                debug!("Lixin health check: rate limited but API is responsive");
                Ok(())
            }
            Err(e) => Err(e),
        }
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

    // ========================================================================
    // Stock Screener Methods
    // ========================================================================

    async fn list_all_stocks(&self) -> Result<Vec<StockInfo>, ProviderError> {
        self.fetch_stock_list().await
    }

    async fn get_financial_data(
        &self,
        symbol: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<FinancialStatementData, ProviderError> {
        self.fetch_financial_data(symbol, period_end).await
    }

    async fn batch_get_financial_data(
        &self,
        symbols: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<FinancialStatementData>, ProviderError> {
        self.batch_fetch_financial_data(symbols, period_end).await
    }

    // ========================================================================
    // Valuation Metrics Methods
    // ========================================================================

    async fn get_valuation_metrics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<ValuationMetrics, ProviderError> {
        self.fetch_valuation_metrics(symbol, date).await
    }

    async fn batch_get_valuation_metrics(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        self.batch_fetch_valuation_metrics(symbols, date).await
    }

    async fn get_valuation_statistics(
        &self,
        symbol: &str,
        metrics: &[ValuationMetricName],
        granularities: &[StatisticsGranularity],
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatisticsSet, ProviderError> {
        self.fetch_valuation_statistics(symbol, metrics, granularities, date).await
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
// Stock List API Types
// ============================================================================

/// Stock list request
#[derive(Debug, Serialize)]
struct LixinStockListRequest {
    token: String,
}

/// Stock item from list response
#[derive(Debug, Deserialize)]
struct LixinStockItem {
    /// Stock code (e.g., "000001")
    #[serde(rename = "stockCode")]
    stock_code: String,
    /// Stock name (e.g., "平安银行")
    #[serde(rename = "name", default)]
    name: String,
    /// Industry classification
    #[serde(rename = "industry", default)]
    industry: Option<String>,
    /// Listing date (YYYY-MM-DD)
    #[serde(rename = "listDate", default)]
    list_date: Option<String>,
    /// Whether trading is suspended
    #[serde(rename = "isSuspended", default)]
    is_suspended: Option<bool>,
    /// Market cap in billion yuan
    #[serde(rename = "marketCap", default)]
    market_cap: Option<f64>,
}

// ============================================================================
// Financial Statement API Types
// ============================================================================

/// Financial data request
#[derive(Debug, Serialize)]
struct LixinFinancialRequest {
    token: String,
    #[serde(rename = "stockCodes")]
    stock_codes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<Vec<String>>,
}

/// Balance sheet data
#[derive(Debug, Deserialize)]
struct LixinBalanceSheet {
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "date", default)]
    report_date: String,
    #[serde(rename = "totalAssets", default)]
    total_assets: Option<f64>,
    #[serde(rename = "totalEquity", default)]
    total_equity: Option<f64>,
    #[serde(rename = "totalLiabilities", default)]
    total_liabilities: Option<f64>,
    #[serde(rename = "cashAndEquivalents", default)]
    cash_and_equivalents: Option<f64>,
    #[serde(rename = "totalDebt", default)]
    total_debt: Option<f64>,
    #[serde(rename = "sharesOutstanding", default)]
    shares_outstanding: Option<f64>,
    #[serde(rename = "roe", default)]
    roe: Option<f64>,
    #[serde(rename = "roa", default)]
    roa: Option<f64>,
    #[serde(rename = "debtToEquity", default)]
    debt_to_equity: Option<f64>,
    #[serde(rename = "currentRatio", default)]
    current_ratio: Option<f64>,
    #[serde(rename = "peTtm", default)]
    pe_ttm: Option<f64>,
    #[serde(rename = "pb", default)]
    pb: Option<f64>,
    #[serde(rename = "dividendYield", default)]
    dividend_yield: Option<f64>,
}

/// Income statement data
#[derive(Debug, Deserialize)]
struct LixinIncomeStatement {
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "date", default)]
    report_date: String,
    #[serde(rename = "revenue", default)]
    revenue: Option<f64>,
    #[serde(rename = "grossProfit", default)]
    gross_profit: Option<f64>,
    #[serde(rename = "operatingIncome", default)]
    operating_income: Option<f64>,
    #[serde(rename = "netIncome", default)]
    net_income: Option<f64>,
    #[serde(rename = "interestExpense", default)]
    interest_expense: Option<f64>,
    #[serde(rename = "grossMargin", default)]
    gross_margin: Option<f64>,
    #[serde(rename = "netMargin", default)]
    net_margin: Option<f64>,
}

/// Cash flow statement data
#[derive(Debug, Deserialize)]
struct LixinCashFlow {
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "date", default)]
    report_date: String,
    #[serde(rename = "operatingCashFlow", default)]
    operating_cash_flow: Option<f64>,
    #[serde(rename = "investingCashFlow", default)]
    investing_cash_flow: Option<f64>,
    #[serde(rename = "financingCashFlow", default)]
    financing_cash_flow: Option<f64>,
    #[serde(rename = "capex", default)]
    capex: Option<f64>,
}

// ============================================================================
// Valuation Metrics API Types
// ============================================================================

/// Names of valuation metrics supported by the API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationMetricName {
    /// PE ratio (TTM)
    PeTtm,
    /// PE ratio (TTM) excluding non-recurring items
    DPeTtm,
    /// PB ratio
    Pb,
    /// PS ratio (TTM)
    PsTtm,
    /// Dividend yield
    Dyr,
    /// Market cap (total)
    Mc,
    /// Circulating market cap
    Cmc,
    /// Free float market cap
    Ecmc,
    /// Financing balance
    Fb,
    /// Short balance
    Sb,
    /// Northbound holdings (shares)
    HaSh,
    /// Northbound holdings (market value)
    HaShm,
}

impl ValuationMetricName {
    /// Convert to the API metric name string
    pub fn to_api_metric(self) -> &'static str {
        match self {
            Self::PeTtm => "peTtm",
            Self::DPeTtm => "dPeTtm",
            Self::Pb => "pb",
            Self::PsTtm => "psTtm",
            Self::Dyr => "dyr",
            Self::Mc => "mc",
            Self::Cmc => "cmc",
            Self::Ecmc => "ecmc",
            Self::Fb => "fb",
            Self::Sb => "sb",
            Self::HaSh => "haSh",
            Self::HaShm => "haShm",
        }
    }

    /// Get all metrics as API strings
    pub fn all_metrics() -> Vec<&'static str> {
        vec![
            "peTtm", "dPeTtm", "pb", "psTtm", "dyr",
            "mc", "cmc", "ecmc", "fb", "sb", "haSh", "haShm",
        ]
    }
}

/// Non-financial fundamental data request for valuation metrics
#[derive(Debug, Serialize)]
struct LixinNonFinancialRequest {
    token: String,
    #[serde(rename = "stockCodes")]
    stock_codes: Vec<String>,
    #[serde(rename = "metricsList", skip_serializing_if = "Option::is_none")]
    metrics_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i32>,
}

/// Non-financial valuation data from Lixinger API
#[derive(Debug, Deserialize)]
struct LixinNonFinancialData {
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "date")]
    date: String,

    // Core valuation metrics
    #[serde(rename = "peTtm", default)]
    pe_ttm: Option<f64>,
    #[serde(rename = "dPeTtm", default)]
    d_pe_ttm: Option<f64>,
    #[serde(rename = "pb", default)]
    pb: Option<f64>,
    #[serde(rename = "psTtm", default)]
    ps_ttm: Option<f64>,
    #[serde(rename = "dyr", default)]
    dividend_yield: Option<f64>,

    // Market cap related
    #[serde(rename = "mc", default)]
    market_cap: Option<f64>,
    #[serde(rename = "cmc", default)]
    circulating_market_cap: Option<f64>,
    #[serde(rename = "ecmc", default)]
    free_float_market_cap: Option<f64>,

    // Margin trading
    #[serde(rename = "fb", default)]
    financing_balance: Option<f64>,
    #[serde(rename = "sb", default)]
    short_balance: Option<f64>,

    // Northbound (Hong Kong) holdings
    #[serde(rename = "haSh", default)]
    northbound_holdings_shares: Option<f64>,
    #[serde(rename = "haShm", default)]
    northbound_holdings_value: Option<f64>,
}

/// Unified valuation metrics structure for external use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationMetrics {
    pub symbol: String,
    pub date: NaiveDate,

    // Core valuation
    pub pe_ttm: Option<f64>,
    pub pe_ttm_ex_non_recurring: Option<f64>,
    pub pb: Option<f64>,
    pub ps_ttm: Option<f64>,
    pub dividend_yield: Option<f64>,

    // Market cap (in billion yuan typically)
    pub market_cap: Option<f64>,
    pub circulating_market_cap: Option<f64>,
    pub free_float_market_cap: Option<f64>,

    // Margin trading (in million yuan typically)
    pub financing_balance: Option<f64>,
    pub short_balance: Option<f64>,

    // Northbound holdings
    pub northbound_holdings_shares: Option<f64>,
    pub northbound_holdings_value: Option<f64>,
}

impl ValuationMetrics {
    /// Calculate the ratio of circulating to total market cap
    pub fn circulating_ratio(&self) -> Option<f64> {
        match (self.circulating_market_cap, self.market_cap) {
            (Some(cmc), Some(mc)) if mc > 0.0 => Some(cmc / mc),
            _ => None,
        }
    }

    /// Calculate short interest ratio (short balance / financing balance)
    pub fn short_interest_ratio(&self) -> Option<f64> {
        match (self.short_balance, self.financing_balance) {
            (Some(sb), Some(fb)) if fb > 0.0 => Some(sb / fb),
            _ => None,
        }
    }

    /// Check if valuation is attractive based on common thresholds
    pub fn is_value_attractive(&self) -> bool {
        // Simple value screen: PE < 15, PB < 2, dividend yield > 3%
        let pe_ok = self.pe_ttm.map_or(false, |pe| pe > 0.0 && pe < 15.0);
        let pb_ok = self.pb.map_or(false, |pb| pb > 0.0 && pb < 2.0);
        let div_ok = self.dividend_yield.map_or(false, |dy| dy > 3.0);

        pe_ok && pb_ok && div_ok
    }
}

/// Statistics time granularity for historical percentile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatisticsGranularity {
    /// Since listing (full history)
    SinceListing,
    /// 20 years
    Y20,
    /// 10 years
    Y10,
    /// 5 years
    Y5,
    /// 3 years
    Y3,
    /// 1 year
    Y1,
}

impl StatisticsGranularity {
    /// Convert to the API granularity string
    pub fn to_api_granularity(&self) -> &'static str {
        match self {
            Self::SinceListing => "fs",    // from start (since listing)
            Self::Y20 => "20y",
            Self::Y10 => "10y",
            Self::Y5 => "5y",
            Self::Y3 => "3y",
            Self::Y1 => "1y",
        }
    }

    /// Parse from API string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fs" => Some(Self::SinceListing),
            "20y" => Some(Self::Y20),
            "10y" => Some(Self::Y10),
            "5y" => Some(Self::Y5),
            "3y" => Some(Self::Y3),
            "1y" => Some(Self::Y1),
            _ => None,
        }
    }
}

/// Valuation statistics for a single metric and granularity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationStatistics {
    pub metric: ValuationMetricName,
    pub granularity: StatisticsGranularity,
    pub current_value: f64,
    pub percentile: Option<f64>,
    pub q25: Option<f64>,
    pub q50: Option<f64>,
    pub q80: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub avg: Option<f64>,
}

impl ValuationStatistics {
    /// Check if current value is in low valuation zone (bottom 20%)
    pub fn is_low_valuation(&self) -> bool {
        self.percentile.map_or(false, |p| p < 20.0)
    }

    /// Check if current value is in high valuation zone (top 20%)
    pub fn is_high_valuation(&self) -> bool {
        self.percentile.map_or(false, |p| p > 80.0)
    }

    /// Get valuation zone description
    pub fn valuation_zone(&self) -> &'static str {
        match self.percentile {
            Some(p) if p < 10.0 => "极度低估",
            Some(p) if p < 25.0 => "低估",
            Some(p) if p < 45.0 => "偏低",
            Some(p) if p < 55.0 => "合理",
            Some(p) if p < 75.0 => "偏高",
            Some(p) if p < 90.0 => "高估",
            Some(_) => "极度高估",
            None => "未知",
        }
    }
}

/// Complete set of valuation statistics for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationStatisticsSet {
    pub symbol: String,
    pub date: NaiveDate,
    pub pe_ttm_stats: Vec<ValuationStatistics>,
    pub pb_stats: Vec<ValuationStatistics>,
    pub dividend_yield_stats: Vec<ValuationStatistics>,
}

impl ValuationStatisticsSet {
    /// Get statistics for a specific metric
    pub fn get_metric_stats(&self, metric: ValuationMetricName) -> Option<&[ValuationStatistics]> {
        match metric {
            ValuationMetricName::PeTtm | ValuationMetricName::DPeTtm => {
                if self.pe_ttm_stats.is_empty() { None } else { Some(&self.pe_ttm_stats) }
            }
            ValuationMetricName::Pb => {
                if self.pb_stats.is_empty() { None } else { Some(&self.pb_stats) }
            }
            ValuationMetricName::Dyr => {
                if self.dividend_yield_stats.is_empty() { None } else { Some(&self.dividend_yield_stats) }
            }
            _ => None,
        }
    }

    /// Calculate a composite value score (0-100, higher = more expensive)
    pub fn composite_value_score(&self) -> Option<f64> {
        // Average of PE and PB percentiles for 5-year lookback
        let pe_pct = self.pe_ttm_stats
            .iter()
            .find(|s| s.granularity == StatisticsGranularity::Y5)
            .and_then(|s| s.percentile)?;

        let pb_pct = self.pb_stats
            .iter()
            .find(|s| s.granularity == StatisticsGranularity::Y5)
            .and_then(|s| s.percentile)?;

        // Higher percentile = higher valuation = lower value score
        // Invert so 100 = best value, 0 = worst value
        Some(200.0 - (pe_pct + pb_pct) / 2.0)
    }
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

    // ========================================================================
    // Valuation Metrics Integration Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_valuation_metrics() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let metrics = adapter
            .get_valuation_metrics("000001.SZ", None)
            .await
            .unwrap();

        assert_eq!(metrics.symbol, "000001.SZ");
        // At least one valuation metric should be present
        assert!(
            metrics.pe_ttm.is_some() || metrics.pb.is_some() || metrics.market_cap.is_some(),
            "At least one valuation metric should be present"
        );
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_batch_get_valuation_metrics() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let symbols = vec![
            "000001.SZ".to_string(),
            "000002.SZ".to_string(),
            "600000.SH".to_string(),
        ];

        let metrics_list = adapter
            .batch_get_valuation_metrics(&symbols, None)
            .await
            .unwrap();

        assert!(!metrics_list.is_empty());
        // Each result should have a symbol
        for metrics in &metrics_list {
            assert!(!metrics.symbol.is_empty());
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_valuation_statistics() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let stats = adapter
            .get_valuation_statistics(
                "000001.SZ",
                &[ValuationMetricName::PeTtm, ValuationMetricName::Pb],
                &[StatisticsGranularity::Y3, StatisticsGranularity::Y5],
                None,
            )
            .await
            .unwrap();

        assert_eq!(stats.symbol, "000001.SZ");
        // Should have statistics for requested metrics
        assert!(!stats.pe_ttm_stats.is_empty() || !stats.pb_stats.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_valuation_metrics_helpers() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let metrics = adapter
            .get_valuation_metrics("600519.SH", None)  // 贵州茅台 - typically has high valuation
            .await
            .unwrap();

        // Test circulating_ratio calculation
        if let (Some(cmc), Some(mc)) = (metrics.circulating_market_cap, metrics.market_cap) {
            if mc > 0.0 {
                let ratio = cmc / mc;
                assert!(ratio > 0.0 && ratio <= 1.0, "Circulating ratio should be between 0 and 1");
            }
        }

        // Test is_value_attractive (Kweichow Moutai is typically NOT a value stock)
        // This test mainly verifies the method works
        let is_value = metrics.is_value_attractive();
        assert!(!is_value || metrics.pe_ttm.map_or(false, |pe| pe < 15.0));
    }
}
