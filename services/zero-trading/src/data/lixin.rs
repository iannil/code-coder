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

/// Stock (company) candlestick endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/candlestick
const STOCK_DAILY_ENDPOINT: &str = "/cn/company/candlestick";

/// Index candlestick endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/index/candlestick
const INDEX_DAILY_ENDPOINT: &str = "/cn/index/candlestick";

/// Stock list endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company
const STOCK_LIST_ENDPOINT: &str = "/cn/company";

/// Financial statement endpoint for non-financial companies
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fs/non_financial
/// This is the unified endpoint for balance sheet, income statement, and cash flow data.
/// Use metricsList to specify which metrics: q.bs.* (balance sheet), q.ps.* (income), q.cfs.* (cash flow)
const FS_NON_FINANCIAL_ENDPOINT: &str = "/cn/company/fs/non_financial";

/// Balance sheet endpoint (DEPRECATED - use FS_NON_FINANCIAL_ENDPOINT instead)
#[allow(dead_code)]
const BALANCE_SHEET_ENDPOINT: &str = "/cn/company/fs/balance_sheet";

/// Income statement endpoint (DEPRECATED - use FS_NON_FINANCIAL_ENDPOINT instead)
#[allow(dead_code)]
const INCOME_STATEMENT_ENDPOINT: &str = "/cn/company/fs/income";

/// Cash flow statement endpoint (DEPRECATED - use FS_NON_FINANCIAL_ENDPOINT instead)
#[allow(dead_code)]
const CASH_FLOW_ENDPOINT: &str = "/cn/company/fs/cash_flow";

/// Default rate limit: 100 requests per minute (conservative)
const DEFAULT_RATE_LIMIT_RPM: u32 = 100;

/// Retry delay after rate limit error (seconds)
const RATE_LIMIT_RETRY_SECS: u64 = 10;

/// Maximum stocks per batch request
const MAX_BATCH_SIZE: usize = 100;

/// Non-financial fundamental data endpoint (valuation metrics like PE, PB, etc.)
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fundamental/non_financial
const NON_FINANCIAL_ENDPOINT: &str = "/cn/company/fundamental/non_financial";

/// Company profile endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/profile
const COMPANY_PROFILE_ENDPOINT: &str = "/cn/company/profile";

/// Fund list endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/fund
const FUND_LIST_ENDPOINT: &str = "/cn/fund";

/// Fund NAV endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/fund/nav
const FUND_NAV_ENDPOINT: &str = "/cn/fund/nav";

// ============================================================================
// Financial Sector Endpoints (银行/证券/保险)
// ============================================================================

/// Bank fundamental data endpoint (valuation metrics for banks)
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fundamental/bank
const BANK_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/bank";

/// Security (brokerage) fundamental data endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fundamental/security
const SECURITY_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/security";

/// Insurance fundamental data endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fundamental/insurance
const INSURANCE_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/insurance";

/// Bank financial statement endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/fs/bank
const BANK_FS_ENDPOINT: &str = "/cn/company/fs/bank";

// ============================================================================
// Company Information Endpoints
// ============================================================================

/// Company industry classification endpoint
/// Returns industry codes from SW (申万), CNI (国证), etc.
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/industries
const COMPANY_INDUSTRIES_ENDPOINT: &str = "/cn/company/industries";

/// Company index constituents endpoint
/// Returns indices that a stock belongs to
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/indices
const COMPANY_INDICES_ENDPOINT: &str = "/cn/company/indices";

/// Company major customers endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/customers
const COMPANY_CUSTOMERS_ENDPOINT: &str = "/cn/company/customers";

// ============================================================================
// Market Data Endpoints
// ============================================================================

/// Company announcement endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/announcement
const ANNOUNCEMENT_ENDPOINT: &str = "/cn/company/announcement";

/// Company block deal (大宗交易) endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/block-deal
const BLOCK_DEAL_ENDPOINT: &str = "/cn/company/block-deal";

/// Company pledge (股权质押) endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/pledge
const PLEDGE_ENDPOINT: &str = "/cn/company/pledge";

/// Company operating data endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/company/operating-data
const OPERATING_DATA_ENDPOINT: &str = "/cn/company/operating-data";

// ============================================================================
// Index Data Endpoints
// ============================================================================

/// Index fundamental data endpoint
/// API doc: https://www.lixinger.com/open/api/doc?api-key=cn/index/fundamental
const INDEX_FUNDAMENTAL_ENDPOINT: &str = "/cn/index/fundamental";

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
            .gzip(true) // Required by Lixin API
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

        // Check for error object first (new API format with code: 0)
        if let Some(error) = &result.error {
            let error_name = error.name.as_deref().unwrap_or("Unknown");
            let error_msg = error.message.as_deref().unwrap_or("");

            // Build detailed error message
            let msg = if let Some(validation_errors) = &error.messages {
                let details: Vec<String> = validation_errors
                    .iter()
                    .filter_map(|e| e.message.clone())
                    .collect();
                format!("{}: {} ({})", error_name, error_msg, details.join(", "))
            } else {
                format!("{}: {}", error_name, error_msg)
            };

            if error_name.contains("Auth") || msg.contains("token") || msg.contains("认证") {
                return Err(ProviderError::Auth(msg));
            }
            if error_name.contains("Rate") || msg.contains("频率") || msg.contains("限制") {
                return Err(ProviderError::RateLimited {
                    retry_after_secs: Some(RATE_LIMIT_RETRY_SECS),
                });
            }

            return Err(ProviderError::Internal(msg));
        }

        // Check for error code (code: 0 means error, code: 1 means success)
        // This is the opposite of the usual convention!
        if result.code == 0 {
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

        let today = Utc::now().date_naive();

        // Default to 1 year ago if no start date specified
        let start = start_date.unwrap_or_else(|| today - chrono::Duration::days(365));
        // Default to today if no end date specified (endDate is REQUIRED by API)
        let end = end_date.unwrap_or(today);

        let request = LixinCandlestickRequest {
            token: self.token.clone(),
            candlestick_type: "lxr_fc_rights".to_string(), // 理杏仁前复权 (recommended)
            stock_code,
            start_date: start.format("%Y-%m-%d").to_string(),
            end_date: end.format("%Y-%m-%d").to_string(),
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

        let today = Utc::now().date_naive();

        // Default to 1 year ago if no start date specified
        let start = start_date.unwrap_or_else(|| today - chrono::Duration::days(365));
        // Default to today if no end date specified (endDate is REQUIRED by API)
        let end = end_date.unwrap_or(today);

        let request = LixinCandlestickRequest {
            token: self.token.clone(),
            candlestick_type: "normal".to_string(), // 普通指数
            stock_code: index_code,
            start_date: start.format("%Y-%m-%d").to_string(),
            end_date: end.format("%Y-%m-%d").to_string(),
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
            // Try parsing ISO 8601 format first (new API: "2018-01-19T00:00:00+08:00")
            // Then fall back to simple date format (legacy: "2018-01-19")
            let timestamp = if item.date.contains('T') {
                DateTime::parse_from_rfc3339(&item.date)
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|_| {
                        // Try alternative ISO format without colon in timezone
                        DateTime::parse_from_str(&item.date, "%Y-%m-%dT%H:%M:%S%z")
                            .map(|dt| dt.with_timezone(&Utc))
                    })
                    .map_err(|e| ProviderError::Internal(format!("Failed to parse datetime: {}", e)))?
            } else {
                let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                    .map_err(|e| ProviderError::Internal(format!("Failed to parse date: {}", e)))?;
                // Use 15:00 (market close) as timestamp for Beijing time (UTC+8)
                // 15:00 Beijing = 07:00 UTC
                date.and_hms_opt(7, 0, 0)
                    .expect("07:00:00 is a valid time")
                    .and_utc()
            };

            candles.push(Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp,
                open: item.open,
                high: item.high,
                low: item.low,
                close: item.close,
                volume: item.volume.unwrap_or(0) as f64,
                amount: item.amount.unwrap_or(0) as f64,
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
            fs_type: None, // Get all types
            include_delisted: Some(false),
        };

        let response: LixinResponse<Vec<LixinStockItem>> = self
            .call_api(STOCK_LIST_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        let mut stocks = Vec::with_capacity(data.len());

        for item in data {
            // Use exchange field from API, fallback to code prefix detection
            let exchange = item.exchange
                .as_ref()
                .map(|e| e.to_uppercase())
                .unwrap_or_else(|| {
                    if item.stock_code.starts_with("6") {
                        "SH".to_string()
                    } else if item.stock_code.starts_with("0") || item.stock_code.starts_with("3") {
                        "SZ".to_string()
                    } else if item.stock_code.starts_with("8") || item.stock_code.starts_with("4") {
                        "BJ".to_string()
                    } else {
                        "UNKNOWN".to_string()
                    }
                });

            // Parse IPO date from ISO 8601 format
            let list_date = item.ipo_date
                .as_ref()
                .and_then(|d| {
                    // Try ISO 8601 format first: "2001-08-26T16:00:00.000Z"
                    if d.contains('T') {
                        DateTime::parse_from_rfc3339(d)
                            .ok()
                            .map(|dt| dt.date_naive())
                    } else {
                        // Fallback to simple date: "2001-08-26"
                        NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
                    }
                });

            let name = item.name.clone().unwrap_or_default();
            let is_st = name.contains("ST") || name.contains("*ST");
            // Use listing_status field: "normally_listed", "delisted", etc.
            let is_delisted = item.listing_status.as_deref() == Some("delisted");

            stocks.push(StockInfo {
                code: item.stock_code,
                name,
                exchange,
                industry: None, // Industry info not available in /cn/company endpoint
                list_date,
                is_suspended: is_delisted,
                is_st,
                market_cap: None, // Market cap not available in this endpoint
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
    /// Uses the unified /cn/company/fs/non_financial endpoint
    /// Note: This endpoint only supports single stock queries, so we process one at a time
    async fn batch_fetch_financial_data(
        &self,
        symbols: &[String],
        period_end: Option<NaiveDate>,
    ) -> Result<Vec<FinancialStatementData>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());
        let today = Utc::now().date_naive();

        // Process one stock at a time (API limitation)
        for symbol in symbols {
            let stock_code = match to_lixin_code(symbol) {
                Some(c) => c,
                None => continue,
            };

            // Build request with valid financial metrics only (single stock)
            // Note: invalid metrics removed to avoid 400 errors from Lixinger API
            let request = LixinFsNonFinancialRequest {
                token: self.token.clone(),
                stock_codes: vec![stock_code.clone()],
                metrics_list: FsMetricCodes::valid_metrics()
                    .into_iter()
                    .map(String::from)
                    .collect(),
                date: period_end.map(|d| d.format("%Y-%m-%d").to_string()),
                start_date: if period_end.is_none() {
                    Some((today - chrono::Duration::days(400)).format("%Y-%m-%d").to_string())
                } else {
                    None
                },
                end_date: if period_end.is_none() {
                    Some(today.format("%Y-%m-%d").to_string())
                } else {
                    None
                },
            };

            // Call the unified endpoint
            let response: LixinResponse<Vec<LixinFsNonFinancialData>> = match self
                .call_api(FS_NON_FINANCIAL_ENDPOINT, &request)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    debug!(symbol = %symbol, error = %e, "Failed to fetch financial data for symbol");
                    continue;
                }
            };

            let data = response.data.unwrap_or_default();

            // Get the most recent record
            let item = match data.iter().max_by_key(|d| &d.date) {
                Some(i) => i,
                None => continue,
            };

            let period = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                .unwrap_or_else(|_| period_end.unwrap_or(today));

            // Helper to extract f64 from metrics
            let get_f64 = |key: &str| -> Option<f64> {
                item.metrics.get(key).and_then(|v| {
                    v.as_ref().and_then(|val| val.as_f64())
                })
            };

            // Get valid metrics from API response
            let total_assets_val = get_f64(FsMetricCodes::TOTAL_ASSETS);
            let total_liabilities_val = get_f64(FsMetricCodes::TOTAL_LIABILITIES);

            // Derive total_equity: total_assets - total_liabilities (accounting equation)
            let total_equity_derived = match (total_assets_val, total_liabilities_val) {
                (Some(a), Some(l)) => Some(a - l),
                _ => None,
            };

            // Derive debt_to_equity: total_liabilities / total_equity
            let debt_to_equity_derived = match (total_liabilities_val, total_equity_derived) {
                (Some(l), Some(e)) if e > 0.0 => Some(l / e),
                _ => None,
            };

            results.push(FinancialStatementData {
                symbol: symbol.clone(),
                period_end: period,
                report_type: "annual".to_string(),
                // Invalid metrics (not in Lixinger fs/non_financial API):
                revenue: None,        // q.ps.or.t not valid
                gross_profit: None,   // q.ps.gp.t not valid
                // Valid metrics from API:
                operating_income: get_f64(FsMetricCodes::OPERATING_INCOME),
                net_income: get_f64(FsMetricCodes::NET_INCOME),
                interest_expense: get_f64(FsMetricCodes::INTEREST_EXPENSE),
                total_assets: total_assets_val,
                // Derived: total_equity = total_assets - total_liabilities
                total_equity: total_equity_derived,
                total_liabilities: total_liabilities_val,
                // Invalid metrics:
                cash: None,           // q.bs.caea.t not valid
                total_debt: get_f64(FsMetricCodes::TOTAL_DEBT),
                shares_outstanding: get_f64(FsMetricCodes::SHARES_OUTSTANDING),
                operating_cash_flow: get_f64(FsMetricCodes::OPERATING_CASH_FLOW),
                investing_cash_flow: get_f64(FsMetricCodes::INVESTING_CASH_FLOW),
                financing_cash_flow: get_f64(FsMetricCodes::FINANCING_CASH_FLOW),
                capex: None,          // q.cfs.cffaacogola.t not valid
                roe: get_f64(FsMetricCodes::ROE),
                roa: get_f64(FsMetricCodes::ROA),
                gross_margin: None,   // q.m.gpm.t not valid
                net_margin: None,     // q.m.npm.t not valid
                // Derived: debt_to_equity = total_liabilities / total_equity
                debt_to_equity: debt_to_equity_derived,
                current_ratio: None,  // q.m.cr.t not valid
                pe_ttm: None,         // Not available in fs endpoint
                pb: None,             // Not available in fs endpoint
                dividend_yield: None, // Not available in fs endpoint
            });

            // Rate limiting: small delay between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        debug!(
            count = results.len(),
            requested = symbols.len(),
            "Batch fetched financial data from Lixin (unified endpoint, single-stock mode)"
        );
        Ok(results)
    }

    /// DEPRECATED: Use batch_fetch_financial_data instead
    /// Kept for backward compatibility but no longer called
    #[allow(dead_code)]
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

    /// DEPRECATED: Use batch_fetch_financial_data instead
    #[allow(dead_code)]
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

    /// DEPRECATED: Use batch_fetch_financial_data instead
    #[allow(dead_code)]
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

        let today = Utc::now().date_naive();

        // If specific date requested, use it; otherwise get most recent data
        // Use startDate with a range to ensure we get data even if today has no data yet
        let request = if let Some(d) = date {
            LixinNonFinancialRequest {
                token: self.token.clone(),
                stock_codes: vec![stock_code.clone()],
                metrics_list: Some(ValuationMetricName::all_metrics()
                    .into_iter()
                    .map(String::from)
                    .collect()),
                date: Some(d.format("%Y-%m-%d").to_string()),
                start_date: None,
                end_date: None,
                limit: Some(1),
            }
        } else {
            // Query last 7 days to get most recent available data
            let start = today - chrono::Duration::days(7);
            LixinNonFinancialRequest {
                token: self.token.clone(),
                stock_codes: vec![stock_code.clone()],
                metrics_list: Some(ValuationMetricName::all_metrics()
                    .into_iter()
                    .map(String::from)
                    .collect()),
                date: None,
                start_date: Some(start.format("%Y-%m-%d").to_string()),
                end_date: Some(today.format("%Y-%m-%d").to_string()),
                limit: Some(1),
            }
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
    /// Note: Lixin's /cn/company/fundamental/non_financial API only supports single stock queries,
    /// so we process stocks one at a time with rate limiting.
    async fn batch_fetch_valuation_metrics(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        let mut results = Vec::with_capacity(symbols.len());

        // Process stocks one at a time (API limitation: only supports single stock per request)
        for symbol in symbols {
            match self.fetch_valuation_metrics(symbol, date).await {
                Ok(metrics) => results.push(metrics),
                Err(ProviderError::DataNotAvailable(_)) => {
                    // Skip stocks without valuation data
                    continue;
                }
                Err(e) => {
                    debug!(symbol, error = %e, "Failed to fetch valuation metrics");
                    // Continue with other stocks instead of failing the entire batch
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
        // Parse date - API returns ISO 8601 format like "2026-02-26T00:00:00+08:00"
        let date = if data.date.contains('T') {
            DateTime::parse_from_rfc3339(&data.date)
                .map(|dt| dt.date_naive())
                .map_err(|e| ProviderError::Internal(format!("Failed to parse datetime: {}", e)))?
        } else {
            NaiveDate::parse_from_str(&data.date, "%Y-%m-%d")
                .map_err(|e| ProviderError::Internal(format!("Failed to parse date: {}", e)))?
        };

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
    ///
    /// This method fetches historical valuation data and calculates statistics locally:
    /// - Percentile: Current value's position in historical distribution (0-100)
    /// - Quartiles: Q25, Q50 (median), Q80
    /// - Min, Max, Avg: Basic statistical measures
    async fn fetch_metric_statistics(
        &self,
        symbol: &str,
        metric: ValuationMetricName,
        granularity: StatisticsGranularity,
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatistics, ProviderError> {
        // Get current value first
        let current_date = date.unwrap_or_else(|| Utc::now().date_naive());
        let metrics = self.fetch_valuation_metrics(symbol, Some(current_date)).await?;
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

        // Calculate start date based on granularity
        let start_date = self.granularity_to_start_date(granularity, current_date);

        // Fetch historical valuation data
        let historical_data = self.fetch_historical_valuation_data(symbol, metric, start_date, current_date).await?;

        // If insufficient historical data, return basic stats
        if historical_data.len() < 10 {
            return Ok(ValuationStatistics {
                metric,
                granularity,
                current_value,
                percentile: None,
                q25: None,
                q50: None,
                q80: None,
                min: historical_data.iter().cloned().reduce(f64::min),
                max: historical_data.iter().cloned().reduce(f64::max),
                avg: if historical_data.is_empty() { None } else {
                    Some(historical_data.iter().sum::<f64>() / historical_data.len() as f64)
                },
            });
        }

        // Calculate statistics
        let stats = self.calculate_statistics(&historical_data, current_value);

        Ok(ValuationStatistics {
            metric,
            granularity,
            current_value,
            percentile: Some(stats.percentile),
            q25: Some(stats.q25),
            q50: Some(stats.q50),
            q80: Some(stats.q80),
            min: Some(stats.min),
            max: Some(stats.max),
            avg: Some(stats.avg),
        })
    }

    /// Convert granularity to start date
    fn granularity_to_start_date(&self, granularity: StatisticsGranularity, from_date: NaiveDate) -> NaiveDate {
        let days_back = match granularity {
            StatisticsGranularity::SinceListing => 365 * 30, // ~30 years max
            StatisticsGranularity::Y20 => 365 * 20,
            StatisticsGranularity::Y10 => 365 * 10,
            StatisticsGranularity::Y5 => 365 * 5,
            StatisticsGranularity::Y3 => 365 * 3,
            StatisticsGranularity::Y1 => 365,
        };
        from_date - chrono::Duration::days(days_back)
    }

    /// Fetch historical valuation data for a specific metric
    async fn fetch_historical_valuation_data(
        &self,
        symbol: &str,
        metric: ValuationMetricName,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<f64>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        // Map metric to API field name
        let metric_name = metric.to_api_metric();

        let request = LixinNonFinancialRequest {
            token: self.token.clone(),
            stock_codes: vec![stock_code],
            metrics_list: Some(vec![metric_name.to_string()]),
            date: None,
            start_date: Some(start_date.format("%Y-%m-%d").to_string()),
            end_date: Some(end_date.format("%Y-%m-%d").to_string()),
            limit: None, // Get all data points
        };

        let response: LixinResponse<Vec<LixinNonFinancialData>> = self
            .call_api(NON_FINANCIAL_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();

        // Extract the metric values, filtering out zeros and negatives for PE/PB
        let values: Vec<f64> = data.iter()
            .filter_map(|d| {
                let value = match metric {
                    ValuationMetricName::PeTtm => d.pe_ttm,
                    ValuationMetricName::DPeTtm => d.d_pe_ttm,
                    ValuationMetricName::Pb => d.pb,
                    ValuationMetricName::Dyr => d.dividend_yield,
                    ValuationMetricName::PsTtm => d.ps_ttm,
                    ValuationMetricName::Mc => d.market_cap,
                    ValuationMetricName::Cmc => d.circulating_market_cap,
                    ValuationMetricName::Ecmc => d.free_float_market_cap,
                    ValuationMetricName::Fb => d.financing_balance,
                    ValuationMetricName::Sb => d.short_balance,
                    ValuationMetricName::HaSh => d.northbound_holdings_shares,
                    ValuationMetricName::HaShm => d.northbound_holdings_value,
                };
                // Filter out invalid values (0, negative PE/PB, etc.)
                value.filter(|&v| v > 0.0)
            })
            .collect();

        debug!(
            symbol = %symbol,
            metric = %metric_name,
            points = values.len(),
            "Fetched historical valuation data"
        );

        Ok(values)
    }

    /// Calculate statistical measures from historical data
    fn calculate_statistics(&self, data: &[f64], current_value: f64) -> StatisticsResult {
        let mut sorted: Vec<f64> = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        let min = sorted[0];
        let max = sorted[n - 1];
        let sum: f64 = sorted.iter().sum();
        let avg = sum / n as f64;

        // Calculate percentile of current value
        let count_below = sorted.iter().filter(|&&v| v < current_value).count();
        let percentile = (count_below as f64 / n as f64) * 100.0;

        // Calculate quartiles using linear interpolation
        let q25 = self.percentile_value(&sorted, 25.0);
        let q50 = self.percentile_value(&sorted, 50.0);
        let q80 = self.percentile_value(&sorted, 80.0);

        StatisticsResult {
            min,
            max,
            avg,
            percentile,
            q25,
            q50,
            q80,
        }
    }

    /// Calculate a specific percentile value using linear interpolation
    fn percentile_value(&self, sorted: &[f64], p: f64) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return sorted[0];
        }

        // Use linear interpolation
        let rank = (p / 100.0) * (n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        let weight = rank - lower as f64;

        if lower == upper {
            sorted[lower]
        } else {
            sorted[lower] * (1.0 - weight) + sorted[upper] * weight
        }
    }

    // ========================================================================
    // Company Profile Methods
    // ========================================================================

    /// Fetch company profile for one or more stocks
    ///
    /// Returns detailed company information including:
    /// - Company name and description
    /// - Main business description
    /// - Registered capital, location (province/city)
    /// - Key personnel (chairman, CEO, legal representative)
    /// - Contact information
    pub async fn get_company_profile(
        &self,
        symbols: &[String],
    ) -> Result<Vec<CompanyProfile>, ProviderError> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(symbols.len());

        // Process in batches
        for chunk in symbols.chunks(MAX_BATCH_SIZE) {
            let stock_codes: Vec<String> = chunk
                .iter()
                .filter_map(|s| to_lixin_code(s))
                .collect();

            if stock_codes.is_empty() {
                continue;
            }

            let request = LixinProfileRequest {
                token: self.token.clone(),
                stock_codes,
            };

            let response: LixinResponse<Vec<CompanyProfile>> = self
                .call_api(COMPANY_PROFILE_ENDPOINT, &request)
                .await?;

            if let Some(data) = response.data {
                results.extend(data);
            }
        }

        debug!(
            count = results.len(),
            requested = symbols.len(),
            "Fetched company profiles from Lixin"
        );
        Ok(results)
    }

    // ========================================================================
    // Fund Data Methods
    // ========================================================================

    /// Fetch list of all funds
    ///
    /// Returns fund basic information including:
    /// - Fund code and name
    /// - Fund type and company
    /// - Fund manager
    /// - Fee rates
    pub async fn get_fund_list(
        &self,
        fund_type: Option<&str>,
    ) -> Result<Vec<FundInfo>, ProviderError> {
        let request = LixinFundListRequest {
            token: self.token.clone(),
            fund_type: fund_type.map(String::from),
        };

        let response: LixinResponse<Vec<FundInfo>> = self
            .call_api(FUND_LIST_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(count = data.len(), "Fetched fund list from Lixin");
        Ok(data)
    }

    /// Fetch fund NAV (Net Asset Value) data
    ///
    /// Returns daily NAV data for the specified funds:
    /// - Unit NAV and accumulated NAV
    /// - Daily return percentage
    pub async fn get_fund_nav(
        &self,
        fund_codes: &[String],
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<FundNav>, ProviderError> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let today = Utc::now().date_naive();
        let start = start_date.unwrap_or_else(|| today - chrono::Duration::days(30));
        let end = end_date.unwrap_or(today);

        let mut results = Vec::new();

        // Process in batches
        for chunk in fund_codes.chunks(MAX_BATCH_SIZE) {
            let request = LixinFundNavRequest {
                token: self.token.clone(),
                fund_codes: chunk.to_vec(),
                start_date: Some(start.format("%Y-%m-%d").to_string()),
                end_date: Some(end.format("%Y-%m-%d").to_string()),
            };

            let response: LixinResponse<Vec<FundNav>> = self
                .call_api(FUND_NAV_ENDPOINT, &request)
                .await?;

            if let Some(data) = response.data {
                results.extend(data);
            }
        }

        debug!(
            count = results.len(),
            requested = fund_codes.len(),
            "Fetched fund NAV data from Lixin"
        );
        Ok(results)
    }

    // ========================================================================
    // Company Industry and Index Methods
    // ========================================================================

    /// Get industry classification for a stock
    ///
    /// Returns industry codes from multiple classification systems:
    /// - SW (申万行业分类)
    /// - SW_2021 (申万2021新版)
    /// - CNI (国证行业分类)
    pub async fn get_company_industries(
        &self,
        symbol: &str,
    ) -> Result<Vec<IndustryClassification>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinIndustryRequest {
            token: self.token.clone(),
            stock_code,
        };

        let response: LixinResponse<Vec<IndustryClassification>> = self
            .call_api(COMPANY_INDUSTRIES_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(
            symbol = %symbol,
            count = data.len(),
            "Fetched industry classifications from Lixin"
        );
        Ok(data)
    }

    /// Get indices that a stock belongs to
    ///
    /// Returns index codes from multiple sources:
    /// - CSI (中证指数)
    /// - CNI (国证指数)
    /// - LXRI (理杏仁自建指数)
    pub async fn get_company_indices(
        &self,
        symbol: &str,
    ) -> Result<Vec<IndexConstituent>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinIndustryRequest {
            token: self.token.clone(),
            stock_code,
        };

        let response: LixinResponse<Vec<IndexConstituent>> = self
            .call_api(COMPANY_INDICES_ENDPOINT, &request)
            .await?;

        // Filter out empty objects (API sometimes returns {} entries)
        let data: Vec<IndexConstituent> = response.data
            .unwrap_or_default()
            .into_iter()
            .filter(|i| i.is_valid())
            .collect();

        debug!(
            symbol = %symbol,
            count = data.len(),
            "Fetched index constituents from Lixin"
        );
        Ok(data)
    }

    // ========================================================================
    // Announcement Methods
    // ========================================================================

    /// Get company announcements within a date range
    ///
    /// Returns announcement links with types:
    /// - "sm" (stock matters)
    /// - "bm" (board meeting)
    /// - "other"
    pub async fn get_announcements(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Announcement>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let today = Utc::now().date_naive();
        let start = start_date.unwrap_or_else(|| today - chrono::Duration::days(30));
        let end = end_date.unwrap_or(today);

        let request = LixinAnnouncementRequest {
            token: self.token.clone(),
            stock_code,
            start_date: Some(start.format("%Y-%m-%d").to_string()),
            end_date: Some(end.format("%Y-%m-%d").to_string()),
        };

        let response: LixinResponse<Vec<Announcement>> = self
            .call_api(ANNOUNCEMENT_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(
            symbol = %symbol,
            count = data.len(),
            "Fetched announcements from Lixin"
        );
        Ok(data)
    }

    // ========================================================================
    // Block Deal Methods
    // ========================================================================

    /// Get block deal (大宗交易) records for a stock
    ///
    /// Block deals are large volume trades typically executed
    /// through institutional channels with discounts/premiums.
    pub async fn get_block_deals(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<BlockDeal>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let today = Utc::now().date_naive();
        let start = start_date.unwrap_or_else(|| today - chrono::Duration::days(365));
        let end = end_date.unwrap_or(today);

        let request = LixinBlockDealRequest {
            token: self.token.clone(),
            stock_code,
            start_date: Some(start.format("%Y-%m-%d").to_string()),
            end_date: Some(end.format("%Y-%m-%d").to_string()),
        };

        let response: LixinResponse<Vec<BlockDeal>> = self
            .call_api(BLOCK_DEAL_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(
            symbol = %symbol,
            count = data.len(),
            "Fetched block deals from Lixin"
        );
        Ok(data)
    }

    // ========================================================================
    // Pledge Methods
    // ========================================================================

    /// Get stock pledge (股权质押) information
    ///
    /// Stock pledging is when shareholders use their shares as
    /// collateral for loans. High pledge ratios can indicate risk.
    pub async fn get_pledge_info(
        &self,
        symbol: &str,
    ) -> Result<Vec<PledgeInfo>, ProviderError> {
        let stock_code = to_lixin_code(symbol)
            .ok_or_else(|| ProviderError::InvalidRequest("Invalid symbol format".into()))?;

        let request = LixinPledgeRequest {
            token: self.token.clone(),
            stock_code,
        };

        let response: LixinResponse<Vec<PledgeInfo>> = self
            .call_api(PLEDGE_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(
            symbol = %symbol,
            count = data.len(),
            "Fetched pledge info from Lixin"
        );
        Ok(data)
    }

    // ========================================================================
    // Index Fundamental Methods
    // ========================================================================

    /// Get fundamental data for indices (沪深300, 中证500, etc.)
    ///
    /// Returns PE/PB/DYR valuation metrics with 10-year percentiles by default.
    /// Use `get_index_fundamental_with_metrics` for custom metric selection.
    pub async fn get_index_fundamental(
        &self,
        index_codes: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<IndexFundamental>, ProviderError> {
        self.get_index_fundamental_with_metrics(
            index_codes,
            date,
            &IndexFundamentalMetrics::common_metrics(),
        )
        .await
    }

    /// Get fundamental data for indices with custom metrics.
    ///
    /// # Metric Formats
    /// - Basic: `mc`, `tv`, `cp` (market cap, volume, close price)
    /// - Valuation: `pe_ttm.mcw`, `pb.mcw`, `dyr.mcw` (market cap weighted)
    /// - Percentile: `pe_ttm.y10.mcw.cvpos` (10-year percentile)
    ///
    /// # Example
    /// ```ignore
    /// let metrics = &["mc", "pe_ttm.mcw", "pb.mcw", "pe_ttm.y10.mcw.cvpos"];
    /// let data = adapter.get_index_fundamental_with_metrics(&codes, None, metrics).await?;
    /// ```
    pub async fn get_index_fundamental_with_metrics(
        &self,
        index_codes: &[String],
        date: Option<NaiveDate>,
        metrics: &[&str],
    ) -> Result<Vec<IndexFundamental>, ProviderError> {
        if index_codes.is_empty() {
            return Ok(Vec::new());
        }

        // Convert to Lixin format
        let stock_codes: Vec<String> = index_codes
            .iter()
            .filter_map(|s| to_lixin_code(s))
            .collect();

        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let today = Utc::now().date_naive();
        let target_date = date.unwrap_or(today);

        let request = LixinIndexFundamentalRequest {
            token: self.token.clone(),
            stock_codes,
            metrics_list: metrics.iter().map(|s| s.to_string()).collect(),
            date: Some(target_date.format("%Y-%m-%d").to_string()),
            start_date: None,
            end_date: None,
        };

        let response: LixinResponse<Vec<IndexFundamental>> = self
            .call_api(INDEX_FUNDAMENTAL_ENDPOINT, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        debug!(
            count = data.len(),
            requested = index_codes.len(),
            metrics_count = metrics.len(),
            "Fetched index fundamental data from Lixin"
        );
        Ok(data)
    }

    // ========================================================================
    // Financial Sector Methods (Bank/Security/Insurance)
    // ========================================================================


    /// Get valuation metrics for bank stocks
    ///
    /// Uses the specialized bank fundamental endpoint which
    /// handles bank-specific valuation characteristics.
    pub async fn get_bank_valuation(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        self.get_financial_sector_valuation(symbols, date, BANK_FUNDAMENTAL_ENDPOINT).await
    }

    /// Get valuation metrics for securities/brokerage stocks
    pub async fn get_security_valuation(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        self.get_financial_sector_valuation(symbols, date, SECURITY_FUNDAMENTAL_ENDPOINT).await
    }

    /// Get valuation metrics for insurance stocks
    pub async fn get_insurance_valuation(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        self.get_financial_sector_valuation(symbols, date, INSURANCE_FUNDAMENTAL_ENDPOINT).await
    }

    /// Internal method for fetching financial sector valuation
    async fn get_financial_sector_valuation(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
        endpoint: &str,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let stock_codes: Vec<String> = symbols
            .iter()
            .filter_map(|s| to_lixin_code(s))
            .collect();

        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let today = Utc::now().date_naive();
        let target_date = date.unwrap_or(today);

        let request = LixinNonFinancialRequest {
            token: self.token.clone(),
            stock_codes: stock_codes.clone(),
            metrics_list: Some(ValuationMetricName::all_metrics()
                .into_iter()
                .map(String::from)
                .collect()),
            date: Some(target_date.format("%Y-%m-%d").to_string()),
            start_date: None,
            end_date: None,
            limit: None,
        };

        let response: LixinResponse<Vec<LixinNonFinancialData>> = self
            .call_api(endpoint, &request)
            .await?;

        let data = response.data.unwrap_or_default();
        let mut results = Vec::with_capacity(data.len());

        for item in data {
            // Find the original symbol
            let symbol = symbols
                .iter()
                .find(|s| to_lixin_code(s).as_ref() == Some(&item.stock_code))
                .cloned()
                .unwrap_or_else(|| format!("{}.UNKNOWN", item.stock_code));

            let date = NaiveDate::parse_from_str(&item.date.split('T').next().unwrap_or(&item.date), "%Y-%m-%d")
                .unwrap_or(target_date);

            results.push(ValuationMetrics {
                symbol,
                date,
                pe_ttm: item.pe_ttm,
                pe_ttm_ex_non_recurring: item.d_pe_ttm,
                pb: item.pb,
                ps_ttm: item.ps_ttm,
                dividend_yield: item.dividend_yield,
                market_cap: item.market_cap,
                circulating_market_cap: item.circulating_market_cap,
                free_float_market_cap: item.free_float_market_cap,
                financing_balance: item.financing_balance,
                short_balance: item.short_balance,
                northbound_holdings_shares: item.northbound_holdings_shares,
                northbound_holdings_value: item.northbound_holdings_value,
            });
        }

        debug!(
            endpoint = %endpoint,
            count = results.len(),
            requested = symbols.len(),
            "Fetched financial sector valuation from Lixin"
        );
        Ok(results)
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

        let request = LixinCandlestickRequest {
            token: self.token.clone(),
            candlestick_type: "lxr_fc_rights".to_string(),
            stock_code: "000001".to_string(), // 平安银行
            start_date: start.format("%Y-%m-%d").to_string(),
            end_date: today.format("%Y-%m-%d").to_string(),
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

/// Lixinger candlestick request (for cn/company/candlestick and cn/index/candlestick)
///
/// API docs:
/// - Company: https://www.lixinger.com/open/api/doc?api-key=cn/company/candlestick
/// - Index: https://www.lixinger.com/open/api/doc?api-key=cn/index/candlestick
#[derive(Debug, Serialize)]
struct LixinCandlestickRequest {
    /// API token
    token: String,
    /// Candlestick type:
    /// - For company: "ex_rights" (不复权), "lxr_fc_rights" (理杏仁前复权), "fc_rights" (前复权), "bc_rights" (后复权)
    /// - For index: "normal" (普通), "total_return" (全收益)
    #[serde(rename = "type")]
    candlestick_type: String,
    /// Stock code (single code, e.g., "000001" or "000300")
    #[serde(rename = "stockCode")]
    stock_code: String,
    /// Start date (YYYY-MM-DD) - REQUIRED
    #[serde(rename = "startDate")]
    start_date: String,
    /// End date (YYYY-MM-DD) - REQUIRED by API
    #[serde(rename = "endDate")]
    end_date: String,
    /// Limit number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i32>,
}

/// Legacy Lixinger API request (for other endpoints like valuation metrics)
#[derive(Debug, Serialize)]
#[allow(dead_code)] // Reserved for future use with other API endpoints
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
///
/// The Lixin API has an unusual response format:
/// - Success: `{"code": 1, "message": "success", "data": [...]}`
/// - Error: `{"code": 0, "error": {"name": "...", "message": "..."}}`
///
/// Note: `code: 1` means success, `code: 0` means error (opposite of convention!)
#[derive(Debug, Deserialize)]
struct LixinResponse<T> {
    /// Response code (1 = success, 0 = error - opposite of convention!)
    code: i32,
    /// Success message (e.g., "success")
    #[serde(alias = "msg")]
    message: Option<String>,
    /// Response data (present on success)
    data: Option<T>,
    /// Error object (present on error)
    error: Option<LixinError>,
}

/// Lixinger API error object
#[derive(Debug, Deserialize)]
struct LixinError {
    /// Error name/type
    name: Option<String>,
    /// Error message
    message: Option<String>,
    /// Validation error messages
    messages: Option<Vec<LixinValidationError>>,
}

/// Lixinger validation error detail
#[derive(Debug, Deserialize)]
struct LixinValidationError {
    /// Field path
    path: Option<Vec<String>>,
    /// Error message
    message: Option<String>,
}

/// Candlestick data from Lixinger
/// Response from /cn/company/candlestick and /cn/index/candlestick
#[derive(Debug, Deserialize)]
struct LixinCandlestick {
    /// Date (ISO 8601 format: "2018-01-19T00:00:00+08:00")
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
    volume: Option<i64>,
    /// Trading amount (turnover)
    #[serde(default)]
    amount: Option<i64>,
    /// Change percentage (optional, reserved for future use)
    #[serde(default)]
    #[allow(dead_code)]
    change: Option<f64>,
}

// ============================================================================
// Stock List API Types
// ============================================================================

/// Stock list request for /cn/company endpoint
#[derive(Debug, Serialize)]
struct LixinStockListRequest {
    token: String,
    /// Financial statement type filter
    #[serde(rename = "fsType", skip_serializing_if = "Option::is_none")]
    fs_type: Option<String>,
    /// Include delisted stocks
    #[serde(rename = "includeDelisted", skip_serializing_if = "Option::is_none")]
    include_delisted: Option<bool>,
}

/// Stock item from /cn/company response
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields reserved for future use
struct LixinStockItem {
    /// Stock code (e.g., "000001")
    #[serde(rename = "stockCode")]
    stock_code: String,
    /// Stock name (e.g., "平安银行")
    #[serde(default)]
    name: Option<String>,
    /// Area code (e.g., "cn")
    #[serde(rename = "areaCode", default)]
    area_code: Option<String>,
    /// Market (e.g., "a")
    #[serde(default)]
    market: Option<String>,
    /// Exchange (e.g., "sh", "sz", "bj")
    #[serde(default)]
    exchange: Option<String>,
    /// Financial statement table type (e.g., "non_financial", "bank", etc.)
    #[serde(rename = "fsTableType", default)]
    fs_table_type: Option<String>,
    /// Listing status (e.g., "normally_listed", "delisted")
    #[serde(rename = "listingStatus", default)]
    listing_status: Option<String>,
    /// IPO date (ISO 8601 format)
    #[serde(rename = "ipoDate", default)]
    ipo_date: Option<String>,
    /// Mutual market flag (for HK-A connect)
    #[serde(rename = "mutualMarketFlag", default)]
    mutual_market_flag: Option<bool>,
}

// ============================================================================
// Company Profile API Types
// ============================================================================

/// Company profile request for /cn/company/profile endpoint
#[derive(Debug, Serialize)]
struct LixinProfileRequest {
    token: String,
    #[serde(rename = "stockCodes")]
    stock_codes: Vec<String>,
}

/// Company profile data from /cn/company/profile response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyProfile {
    /// Stock code (e.g., "000001")
    #[serde(rename = "stockCode")]
    pub stock_code: String,
    /// Company name
    #[serde(rename = "companyName", default)]
    pub company_name: Option<String>,
    /// Company profile/description
    #[serde(default)]
    pub profile: Option<String>,
    /// Main business description
    #[serde(rename = "mainBusiness", default)]
    pub main_business: Option<String>,
    /// Registered capital (in yuan)
    #[serde(rename = "registeredCapital", default)]
    pub registered_capital: Option<f64>,
    /// Province
    #[serde(default)]
    pub province: Option<String>,
    /// City
    #[serde(default)]
    pub city: Option<String>,
    /// Establishment date (ISO 8601)
    #[serde(rename = "establishDate", default)]
    pub establish_date: Option<String>,
    /// Company website
    #[serde(default)]
    pub website: Option<String>,
    /// Chairman name
    #[serde(default)]
    pub chairman: Option<String>,
    /// General manager name
    #[serde(rename = "generalManager", default)]
    pub general_manager: Option<String>,
    /// Legal representative name
    #[serde(rename = "legalRepresentative", default)]
    pub legal_representative: Option<String>,
    /// Number of employees
    #[serde(rename = "employeeNum", default)]
    pub employee_num: Option<i64>,
    /// Office address
    #[serde(rename = "officeAddress", default)]
    pub office_address: Option<String>,
    /// Board secretary name
    #[serde(rename = "boardSecretary", default)]
    pub board_secretary: Option<String>,
    /// Contact phone
    #[serde(default)]
    pub phone: Option<String>,
    /// Contact fax
    #[serde(default)]
    pub fax: Option<String>,
    /// Contact email
    #[serde(default)]
    pub email: Option<String>,
}

// ============================================================================
// Fund API Types
// ============================================================================

/// Fund list request for /cn/fund endpoint
#[derive(Debug, Serialize)]
struct LixinFundListRequest {
    token: String,
    /// Fund type filter (e.g., "stock", "hybrid", "bond", "money", "qdii")
    #[serde(rename = "fundType", skip_serializing_if = "Option::is_none")]
    fund_type: Option<String>,
}

/// Fund NAV request for /cn/fund/nav endpoint
#[derive(Debug, Serialize)]
struct LixinFundNavRequest {
    token: String,
    #[serde(rename = "fundCodes")]
    fund_codes: Vec<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

/// Fund info from /cn/fund response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundInfo {
    /// Fund code (e.g., "000001")
    #[serde(rename = "fundCode")]
    pub fund_code: String,
    /// Fund name
    #[serde(default)]
    pub name: Option<String>,
    /// Full name
    #[serde(rename = "fullName", default)]
    pub full_name: Option<String>,
    /// Fund type (e.g., "stock", "hybrid", "bond", "money", "qdii")
    #[serde(rename = "fundType", default)]
    pub fund_type: Option<String>,
    /// Fund company
    #[serde(rename = "fundCompany", default)]
    pub fund_company: Option<String>,
    /// Fund manager
    #[serde(rename = "fundManager", default)]
    pub fund_manager: Option<String>,
    /// Establishment date (ISO 8601)
    #[serde(rename = "establishDate", default)]
    pub establish_date: Option<String>,
    /// Management fee rate (annual, as percentage)
    #[serde(rename = "managementFeeRate", default)]
    pub management_fee_rate: Option<f64>,
    /// Custody fee rate (annual, as percentage)
    #[serde(rename = "custodyFeeRate", default)]
    pub custody_fee_rate: Option<f64>,
    /// Total asset scale (in 100 million yuan)
    #[serde(rename = "assetScale", default)]
    pub asset_scale: Option<f64>,
}

/// Fund NAV data from /cn/fund/nav response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundNav {
    /// Fund code
    #[serde(rename = "fundCode")]
    pub fund_code: String,
    /// Date (ISO 8601)
    pub date: String,
    /// Unit net asset value
    #[serde(rename = "unitNav")]
    pub unit_nav: f64,
    /// Accumulated unit net asset value
    #[serde(rename = "accumulatedUnitNav", default)]
    pub accumulated_unit_nav: Option<f64>,
    /// Daily return (percentage)
    #[serde(rename = "dailyReturn", default)]
    pub daily_return: Option<f64>,
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
    #[allow(dead_code)] // Used for API deserialization
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
    #[allow(dead_code)] // Used for API deserialization
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
// Unified Financial Statement API Types (fs/non_financial endpoint)
// ============================================================================

/// Request for unified financial statement data
/// Used with /cn/company/fs/non_financial endpoint
#[derive(Debug, Serialize)]
struct LixinFsNonFinancialRequest {
    token: String,
    #[serde(rename = "stockCodes")]
    stock_codes: Vec<String>,
    #[serde(rename = "metricsList")]
    metrics_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

/// Response from unified financial statement endpoint
/// Uses HashMap to capture dynamic metric field names
#[derive(Debug, Deserialize)]
struct LixinFsNonFinancialData {
    #[serde(rename = "stockCode")]
    stock_code: String,
    date: String,
    /// Dynamic fields for metrics (q.bs.ta.t, q.ps.or.t, etc.)
    #[serde(flatten)]
    metrics: std::collections::HashMap<String, Option<serde_json::Value>>,
}

/// Financial statement metric codes for the fs/non_financial API
struct FsMetricCodes;

impl FsMetricCodes {
    /// Balance sheet metrics
    const TOTAL_ASSETS: &'static str = "q.bs.ta.t";
    const TOTAL_EQUITY: &'static str = "q.bs.tse.t";
    const TOTAL_LIABILITIES: &'static str = "q.bs.tl.t";
    const CASH: &'static str = "q.bs.caea.t";
    const TOTAL_DEBT: &'static str = "q.bs.stl.t"; // short-term loans as proxy
    const SHARES_OUTSTANDING: &'static str = "q.bs.tsc.t";

    /// Income statement metrics
    const REVENUE: &'static str = "q.ps.or.t";
    const GROSS_PROFIT: &'static str = "q.ps.gp.t";
    const OPERATING_INCOME: &'static str = "q.ps.op.t";
    const NET_INCOME: &'static str = "q.ps.np.t";
    const INTEREST_EXPENSE: &'static str = "q.ps.ie.t";

    /// Cash flow metrics
    const OPERATING_CASH_FLOW: &'static str = "q.cfs.ncffoa.t";
    const INVESTING_CASH_FLOW: &'static str = "q.cfs.ncffia.t";
    const FINANCING_CASH_FLOW: &'static str = "q.cfs.ncfffa.t";
    const CAPEX: &'static str = "q.cfs.cffaacogola.t"; // cash for fixed assets

    /// Ratio metrics
    const ROE: &'static str = "q.m.roe.t";
    const ROA: &'static str = "q.m.roa.t";
    const GROSS_MARGIN: &'static str = "q.m.gpm.t";
    const NET_MARGIN: &'static str = "q.m.npm.t";
    const DEBT_TO_EQUITY: &'static str = "q.m.ld_e.t";
    const CURRENT_RATIO: &'static str = "q.m.cr.t";

    /// Get all metrics needed for financial statement data (includes invalid metrics)
    /// DEPRECATED: Use valid_metrics() instead
    #[allow(dead_code)]
    fn all_metrics() -> Vec<&'static str> {
        vec![
            Self::TOTAL_ASSETS, Self::TOTAL_EQUITY, Self::TOTAL_LIABILITIES,
            Self::CASH, Self::TOTAL_DEBT, Self::SHARES_OUTSTANDING,
            Self::REVENUE, Self::GROSS_PROFIT, Self::OPERATING_INCOME,
            Self::NET_INCOME, Self::INTEREST_EXPENSE,
            Self::OPERATING_CASH_FLOW, Self::INVESTING_CASH_FLOW,
            Self::FINANCING_CASH_FLOW, Self::CAPEX,
            Self::ROE, Self::ROA, Self::GROSS_MARGIN, Self::NET_MARGIN,
            Self::DEBT_TO_EQUITY, Self::CURRENT_RATIO,
        ]
    }

    /// Get only the metrics that are valid for the fs/non_financial API
    ///
    /// The following 9 metrics are NOT supported by the API and will cause 400 errors:
    /// - q.bs.tse.t (TOTAL_EQUITY) - derive from: total_assets - total_liabilities
    /// - q.bs.caea.t (CASH) - not available
    /// - q.ps.or.t (REVENUE) - not available
    /// - q.ps.gp.t (GROSS_PROFIT) - not available
    /// - q.cfs.cffaacogola.t (CAPEX) - not available
    /// - q.m.gpm.t (GROSS_MARGIN) - not available
    /// - q.m.npm.t (NET_MARGIN) - not available
    /// - q.m.ld_e.t (DEBT_TO_EQUITY) - derive from: total_liabilities / total_equity
    /// - q.m.cr.t (CURRENT_RATIO) - not available
    fn valid_metrics() -> Vec<&'static str> {
        vec![
            // Balance sheet (valid)
            Self::TOTAL_ASSETS, Self::TOTAL_LIABILITIES, Self::TOTAL_DEBT, Self::SHARES_OUTSTANDING,
            // Income statement (valid)
            Self::OPERATING_INCOME, Self::NET_INCOME, Self::INTEREST_EXPENSE,
            // Cash flow (valid)
            Self::OPERATING_CASH_FLOW, Self::INVESTING_CASH_FLOW, Self::FINANCING_CASH_FLOW,
            // Ratios (valid)
            Self::ROE, Self::ROA,
        ]
    }
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
    /// Convert to the API metric name string (snake_case format)
    pub fn to_api_metric(self) -> &'static str {
        match self {
            Self::PeTtm => "pe_ttm",
            Self::DPeTtm => "d_pe_ttm",
            Self::Pb => "pb",
            Self::PsTtm => "ps_ttm",
            Self::Dyr => "dyr",
            Self::Mc => "mc",
            Self::Cmc => "cmc",
            Self::Ecmc => "ecmc",
            Self::Fb => "fb",
            Self::Sb => "sb",
            Self::HaSh => "ha_sh",
            Self::HaShm => "ha_shm",
        }
    }

    /// Get all metrics as API strings (snake_case format)
    pub fn all_metrics() -> Vec<&'static str> {
        vec![
            "pe_ttm", "d_pe_ttm", "pb", "ps_ttm", "dyr",
            "mc", "cmc", "ecmc", "fb", "sb", "ha_sh", "ha_shm",
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
/// Note: API returns snake_case field names
#[derive(Debug, Deserialize)]
struct LixinNonFinancialData {
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "date")]
    date: String,

    // Core valuation metrics (snake_case from API)
    #[serde(rename = "pe_ttm", default)]
    pe_ttm: Option<f64>,
    #[serde(rename = "d_pe_ttm", default)]
    d_pe_ttm: Option<f64>,
    #[serde(rename = "pb", default)]
    pb: Option<f64>,
    #[serde(rename = "ps_ttm", default)]
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

    // Northbound (Hong Kong) holdings (snake_case from API)
    #[serde(rename = "ha_sh", default)]
    northbound_holdings_shares: Option<f64>,
    #[serde(rename = "ha_shm", default)]
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

/// Internal struct for statistics calculation result
struct StatisticsResult {
    min: f64,
    max: f64,
    avg: f64,
    percentile: f64,
    q25: f64,
    q50: f64,
    q80: f64,
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
// Company Industry Classification API Types
// ============================================================================

/// Request for company industry classification
#[derive(Debug, Serialize)]
struct LixinIndustryRequest {
    token: String,
    #[serde(rename = "stockCode")]
    stock_code: String,
}

/// Industry classification data from /cn/company/industries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryClassification {
    /// Area code (e.g., "cn")
    #[serde(rename = "areaCode", default)]
    pub area_code: Option<String>,
    /// Industry code (e.g., "480000" for SW 银行)
    #[serde(rename = "stockCode")]
    pub industry_code: String,
    /// Classification source (e.g., "sw", "sw_2021", "cni")
    pub source: String,
    /// Industry name (e.g., "银行")
    pub name: String,
}

// ============================================================================
// Company Index Constituent API Types
// ============================================================================

/// Index constituent data from /cn/company/indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConstituent {
    /// Area code (e.g., "cn")
    #[serde(rename = "areaCode", default)]
    pub area_code: Option<String>,
    /// Index code (e.g., "000300")
    /// Note: API may return empty objects, so this is optional
    #[serde(rename = "stockCode", default)]
    pub index_code: Option<String>,
    /// Index source (e.g., "csi", "cni", "lxri")
    #[serde(default)]
    pub source: Option<String>,
    /// Index name (e.g., "沪深300")
    #[serde(default)]
    pub name: Option<String>,
}

impl IndexConstituent {
    /// Check if this is a valid (non-empty) entry
    pub fn is_valid(&self) -> bool {
        self.index_code.is_some() && self.source.is_some() && self.name.is_some()
    }

    /// Get the index code, panics if empty
    pub fn code(&self) -> &str {
        self.index_code.as_deref().unwrap_or("")
    }
}

// ============================================================================
// Announcement API Types
// ============================================================================

/// Request for company announcements
#[derive(Debug, Serialize)]
struct LixinAnnouncementRequest {
    token: String,
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

/// Announcement data from /cn/company/announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    /// Announcement date (ISO 8601 format)
    pub date: String,
    /// PDF/link URL
    #[serde(rename = "linkUrl")]
    pub link_url: String,
    /// Announcement title
    #[serde(rename = "linkText")]
    pub link_text: String,
    /// Link type (e.g., "PDF")
    #[serde(rename = "linkType", default)]
    pub link_type: Option<String>,
    /// Announcement types (e.g., ["sm"], ["bm"], ["other"])
    #[serde(default)]
    pub types: Vec<String>,
}

// ============================================================================
// Block Deal API Types
// ============================================================================

/// Request for block deals
#[derive(Debug, Serialize)]
struct LixinBlockDealRequest {
    token: String,
    #[serde(rename = "stockCode")]
    stock_code: String,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

/// Block deal (大宗交易) data from /cn/company/block-deal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDeal {
    /// Stock code
    #[serde(rename = "stockCode")]
    pub stock_code: String,
    /// Trade date (ISO 8601 format)
    pub date: String,
    /// Trading price
    #[serde(rename = "tradingPrice")]
    pub trading_price: f64,
    /// Trading volume (shares)
    #[serde(rename = "tradingVolume")]
    pub trading_volume: i64,
    /// Trading amount (yuan)
    #[serde(rename = "tradingAmount")]
    pub trading_amount: f64,
    /// Buyer branch name
    #[serde(rename = "buyBranch", default)]
    pub buy_branch: Option<String>,
    /// Seller branch name
    #[serde(rename = "sellBranch", default)]
    pub sell_branch: Option<String>,
    /// Discount rate (positive = discount, negative = premium)
    #[serde(rename = "discountRate", default)]
    pub discount_rate: Option<f64>,
}

impl BlockDeal {
    /// Returns true if the trade was at a discount (below market price)
    pub fn is_discount(&self) -> bool {
        self.discount_rate.map_or(false, |r| r > 0.0)
    }

    /// Returns true if the trade was at a premium (above market price)
    pub fn is_premium(&self) -> bool {
        self.discount_rate.map_or(false, |r| r < 0.0)
    }

    /// Get discount/premium percentage as absolute value
    pub fn discount_premium_pct(&self) -> Option<f64> {
        self.discount_rate.map(|r| (r * 100.0).abs())
    }
}

// ============================================================================
// Pledge API Types
// ============================================================================

/// Request for pledge info
#[derive(Debug, Serialize)]
struct LixinPledgeRequest {
    token: String,
    #[serde(rename = "stockCode")]
    stock_code: String,
}

/// Stock pledge (股权质押) data from /cn/company/pledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PledgeInfo {
    /// Stock code
    #[serde(rename = "stockCode", default)]
    pub stock_code: Option<String>,
    /// Pledge date
    pub date: Option<String>,
    /// Pledgor name
    #[serde(default)]
    pub pledgor: Option<String>,
    /// Pledgee name (bank/institution)
    #[serde(default)]
    pub pledgee: Option<String>,
    /// Pledged shares count
    #[serde(rename = "pledgedShares", default)]
    pub pledged_shares: Option<i64>,
    /// Pledge start date
    #[serde(rename = "startDate", default)]
    pub start_date: Option<String>,
    /// Pledge end date (maturity)
    #[serde(rename = "endDate", default)]
    pub end_date: Option<String>,
    /// Pledge status
    #[serde(default)]
    pub status: Option<String>,
}

// ============================================================================
// Index Fundamental API Types
// ============================================================================

/// Request for index fundamental data
#[derive(Debug, Serialize)]
struct LixinIndexFundamentalRequest {
    token: String,
    #[serde(rename = "stockCodes")]
    stock_codes: Vec<String>,
    #[serde(rename = "metricsList")]
    metrics_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    start_date: Option<String>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

/// Index fundamental data from /cn/index/fundamental
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFundamental {
    /// Index code (e.g., "000300")
    #[serde(rename = "stockCode")]
    pub index_code: String,
    /// Date (ISO 8601 format)
    pub date: String,
    /// Total market cap of index constituents
    #[serde(default)]
    pub mc: Option<f64>,
    /// Weighted average PE (TTM)
    #[serde(rename = "pe_ttm", default)]
    pub pe_ttm: Option<f64>,
    /// Weighted average PB
    #[serde(default)]
    pub pb: Option<f64>,
    /// Weighted average dividend yield
    #[serde(default)]
    pub dyr: Option<f64>,
    /// PE-TTM market cap weighted
    #[serde(rename = "pe_ttm.mcw", default)]
    pub pe_ttm_mcw: Option<f64>,
    /// PB market cap weighted
    #[serde(rename = "pb.mcw", default)]
    pub pb_mcw: Option<f64>,
    /// Dividend yield market cap weighted
    #[serde(rename = "dyr.mcw", default)]
    pub dyr_mcw: Option<f64>,
    /// PE-TTM 10-year percentile (market cap weighted)
    #[serde(rename = "pe_ttm.y10.mcw.cvpos", default)]
    pub pe_ttm_y10_percentile: Option<f64>,
    /// PB 10-year percentile (market cap weighted)
    #[serde(rename = "pb.y10.mcw.cvpos", default)]
    pub pb_y10_percentile: Option<f64>,
    /// Dividend yield 10-year percentile (market cap weighted)
    #[serde(rename = "dyr.y10.mcw.cvpos", default)]
    pub dyr_y10_percentile: Option<f64>,
    /// Additional metrics captured dynamically
    #[serde(flatten)]
    pub extra_metrics: std::collections::HashMap<String, Option<serde_json::Value>>,
}

/// Available metrics for index fundamental API
///
/// Index fundamental API supports three metric formats:
/// 1. `[metricsName]` - Basic metrics like mc, tv, cp
/// 2. `[metricsName].[metricsType]` - Valuation with weighting, e.g., pe_ttm.mcw
/// 3. `[metricsName].[granularity].[metricsType].[statisticsDataType]` - Full format with percentiles
///
/// metricsType options: mcw (市值加权), ew (等权), ewpvo (正数等权), avg, median
/// statisticsDataType options: cv (当前值), cvpos (分位点%), minv, maxv, q2v, q5v, q8v, avgv
pub struct IndexFundamentalMetrics;

impl IndexFundamentalMetrics {
    // ========================================================================
    // Basic Metrics (simple format)
    // ========================================================================

    /// Market capitalization (total)
    pub const MC: &'static str = "mc";
    /// Trading volume
    pub const TV: &'static str = "tv";
    /// Trading amount (turnover)
    pub const TA: &'static str = "ta";
    /// Close price/point
    pub const CP: &'static str = "cp";
    /// Change percentage
    pub const CPC: &'static str = "cpc";
    /// Turnover rate
    pub const TO_R: &'static str = "to_r";

    // ========================================================================
    // Valuation Metrics with Market Cap Weighting
    // ========================================================================

    /// PE-TTM (market cap weighted)
    pub const PE_TTM_MCW: &'static str = "pe_ttm.mcw";
    /// PB (market cap weighted)
    pub const PB_MCW: &'static str = "pb.mcw";
    /// PS-TTM (market cap weighted)
    pub const PS_TTM_MCW: &'static str = "ps_ttm.mcw";
    /// Dividend yield (market cap weighted)
    pub const DYR_MCW: &'static str = "dyr.mcw";

    // ========================================================================
    // Valuation Metrics with Equal Weighting
    // ========================================================================

    /// PE-TTM (equal weighted)
    pub const PE_TTM_EW: &'static str = "pe_ttm.ew";
    /// PB (equal weighted)
    pub const PB_EW: &'static str = "pb.ew";
    /// Dividend yield (equal weighted)
    pub const DYR_EW: &'static str = "dyr.ew";

    // ========================================================================
    // Percentile Metrics (10-year, market cap weighted)
    // ========================================================================

    /// PE-TTM 10-year percentile (market cap weighted)
    pub const PE_TTM_Y10_MCW_CVPOS: &'static str = "pe_ttm.y10.mcw.cvpos";
    /// PB 10-year percentile (market cap weighted)
    pub const PB_Y10_MCW_CVPOS: &'static str = "pb.y10.mcw.cvpos";
    /// Dividend yield 10-year percentile (market cap weighted)
    pub const DYR_Y10_MCW_CVPOS: &'static str = "dyr.y10.mcw.cvpos";

    /// PE-TTM 5-year percentile (market cap weighted)
    pub const PE_TTM_Y5_MCW_CVPOS: &'static str = "pe_ttm.y5.mcw.cvpos";
    /// PB 5-year percentile (market cap weighted)
    pub const PB_Y5_MCW_CVPOS: &'static str = "pb.y5.mcw.cvpos";

    // ========================================================================
    // Metric Collections
    // ========================================================================

    /// Get basic market cap metric only
    pub fn basic_metrics() -> Vec<&'static str> {
        vec![Self::MC]
    }

    /// Get standard valuation metrics (market cap weighted)
    pub fn valuation_metrics() -> Vec<&'static str> {
        vec![
            Self::MC,
            Self::PE_TTM_MCW,
            Self::PB_MCW,
            Self::DYR_MCW,
        ]
    }

    /// Get valuation metrics with 10-year percentiles
    pub fn valuation_with_percentiles() -> Vec<&'static str> {
        vec![
            Self::MC,
            Self::PE_TTM_MCW,
            Self::PB_MCW,
            Self::DYR_MCW,
            Self::PE_TTM_Y10_MCW_CVPOS,
            Self::PB_Y10_MCW_CVPOS,
            Self::DYR_Y10_MCW_CVPOS,
        ]
    }

    /// Get all commonly used metrics (default: valuation with percentiles)
    pub fn common_metrics() -> Vec<&'static str> {
        Self::valuation_with_percentiles()
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

    // ========================================================================
    // Statistics Calculation Unit Tests
    // ========================================================================

    #[test]
    fn test_statistics_calculation() {
        let adapter = LixinAdapter::new("test-token");

        // Test with a known dataset
        let data = vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0];
        let current_value = 27.5;

        let stats = adapter.calculate_statistics(&data, current_value);

        // Min should be 5
        assert!((stats.min - 5.0).abs() < 0.001);
        // Max should be 50
        assert!((stats.max - 50.0).abs() < 0.001);
        // Average should be 27.5
        assert!((stats.avg - 27.5).abs() < 0.001);
        // Percentile of 27.5 should be 50% (5 values below it)
        assert!((stats.percentile - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_percentile_calculation() {
        let adapter = LixinAdapter::new("test-token");

        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Q25 (25th percentile) should be ~2.0
        let q25 = adapter.percentile_value(&sorted, 25.0);
        assert!((q25 - 2.0).abs() < 0.001);

        // Q50 (median) should be 3.0
        let q50 = adapter.percentile_value(&sorted, 50.0);
        assert!((q50 - 3.0).abs() < 0.001);

        // Q75 should be 4.0
        let q75 = adapter.percentile_value(&sorted, 75.0);
        assert!((q75 - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_granularity_to_start_date() {
        let adapter = LixinAdapter::new("test-token");
        let reference_date = NaiveDate::from_ymd_opt(2026, 2, 27).unwrap();

        // Y1 should go back 365 days
        let y1_start = adapter.granularity_to_start_date(StatisticsGranularity::Y1, reference_date);
        assert_eq!((reference_date - y1_start).num_days(), 365);

        // Y5 should go back 5*365 days
        let y5_start = adapter.granularity_to_start_date(StatisticsGranularity::Y5, reference_date);
        assert_eq!((reference_date - y5_start).num_days(), 365 * 5);
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_valuation_statistics_with_local_calculation() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n=== Testing Valuation Statistics with Local Calculation ===");

        let stats = adapter
            .get_valuation_statistics(
                "000001.SZ",
                &[ValuationMetricName::PeTtm],
                &[StatisticsGranularity::Y3],
                None,
            )
            .await
            .unwrap();

        println!("Symbol: {}", stats.symbol);
        println!("Date: {}", stats.date);

        for stat in &stats.pe_ttm_stats {
            println!("\nPE TTM Statistics ({:?}):", stat.granularity);
            println!("  Current: {:.2}", stat.current_value);
            if let Some(p) = stat.percentile {
                println!("  Percentile: {:.1}% ({})", p, stat.valuation_zone());
            }
            if let Some(v) = stat.q25 { println!("  Q25: {:.2}", v); }
            if let Some(v) = stat.q50 { println!("  Q50: {:.2}", v); }
            if let Some(v) = stat.q80 { println!("  Q80: {:.2}", v); }
            if let Some(v) = stat.min { println!("  Min: {:.2}", v); }
            if let Some(v) = stat.max { println!("  Max: {:.2}", v); }
            if let Some(v) = stat.avg { println!("  Avg: {:.2}", v); }
        }

        // Verify we got actual statistics
        assert!(!stats.pe_ttm_stats.is_empty());
        let pe_stats = &stats.pe_ttm_stats[0];
        assert!(pe_stats.current_value > 0.0);
        // With local calculation, we should have percentile data
        // (may be None if insufficient historical data)
    }

    // ========================================================================
    // Comprehensive API Verification Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_verify_stock_candlestick_endpoint() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n=== Testing Stock Candlestick Endpoint: {} ===", STOCK_DAILY_ENDPOINT);

        let result = adapter
            .get_daily_candles("000001.SZ", None, None, Some(5))
            .await;

        match result {
            Ok(candles) => {
                println!("✅ Stock candlestick: SUCCESS ({} candles)", candles.len());
                if let Some(first) = candles.first() {
                    println!("   Sample: {} @ {} O:{:.2} H:{:.2} L:{:.2} C:{:.2}",
                        first.symbol, first.timestamp.format("%Y-%m-%d"),
                        first.open, first.high, first.low, first.close);
                }
                assert!(!candles.is_empty());
            }
            Err(e) => {
                println!("❌ Stock candlestick: FAILED - {:?}", e);
                panic!("Stock candlestick endpoint failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_verify_index_candlestick_endpoint() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n=== Testing Index Candlestick Endpoint: {} ===", INDEX_DAILY_ENDPOINT);

        let result = adapter
            .get_index_daily("000300.SH", None, None)  // 沪深300
            .await;

        match result {
            Ok(candles) => {
                println!("✅ Index candlestick: SUCCESS ({} candles)", candles.len());
                if let Some(first) = candles.first() {
                    println!("   Sample: {} @ {} O:{:.2} H:{:.2} L:{:.2} C:{:.2}",
                        first.symbol, first.timestamp.format("%Y-%m-%d"),
                        first.open, first.high, first.low, first.close);
                }
                assert!(!candles.is_empty());
            }
            Err(e) => {
                println!("❌ Index candlestick: FAILED - {:?}", e);
                panic!("Index candlestick endpoint failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_verify_stock_list_endpoint() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n=== Testing Stock List Endpoint: {} ===", STOCK_LIST_ENDPOINT);

        let result = adapter.list_all_stocks().await;

        match result {
            Ok(stocks) => {
                println!("✅ Stock list: SUCCESS ({} stocks)", stocks.len());
                if let Some(first) = stocks.first() {
                    println!("   Sample: {} ({}) - Exchange: {}",
                        first.code, first.name, first.exchange);
                }
                assert!(!stocks.is_empty());
            }
            Err(e) => {
                println!("❌ Stock list: FAILED - {:?}", e);
                panic!("Stock list endpoint failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_verify_fundamental_endpoint() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n=== Testing Fundamental Endpoint: {} ===", NON_FINANCIAL_ENDPOINT);

        let result = adapter
            .get_valuation_metrics("600519.SH", None)  // 贵州茅台
            .await;

        match result {
            Ok(metrics) => {
                println!("✅ Fundamental data: SUCCESS");
                println!("   Symbol: {}", metrics.symbol);
                println!("   PE TTM: {:?}", metrics.pe_ttm);
                println!("   PB: {:?}", metrics.pb);
                println!("   Market Cap: {:?}", metrics.market_cap);
                assert!(metrics.pe_ttm.is_some() || metrics.pb.is_some());
            }
            Err(e) => {
                println!("❌ Fundamental data: FAILED - {:?}", e);
                panic!("Fundamental endpoint failed: {:?}", e);
            }
        }
    }

    /// Run all endpoint verifications
    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_verify_all_endpoints() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n");
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║           Lixin API Endpoint Verification Report               ║");
        println!("╚════════════════════════════════════════════════════════════════╝");

        let mut results: Vec<(&str, bool, String)> = Vec::new();

        // 1. Health Check
        print!("\n[1/5] Health Check...");
        match adapter.health_check().await {
            Ok(_) => {
                println!(" ✅ PASS");
                results.push(("Health Check", true, "OK".to_string()));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Health Check", false, format!("{:?}", e)));
            }
        }

        // 2. Stock Candlestick
        print!("[2/5] Stock Candlestick ({})...", STOCK_DAILY_ENDPOINT);
        match adapter.get_daily_candles("000001.SZ", None, None, Some(5)).await {
            Ok(candles) => {
                println!(" ✅ PASS ({} candles)", candles.len());
                results.push(("Stock Candlestick", true, format!("{} candles", candles.len())));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Stock Candlestick", false, format!("{:?}", e)));
            }
        }

        // Add delay to avoid rate limiting
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 3. Index Candlestick
        print!("[3/5] Index Candlestick ({})...", INDEX_DAILY_ENDPOINT);
        match adapter.get_index_daily("000300.SH", None, None).await {
            Ok(candles) => {
                println!(" ✅ PASS ({} candles)", candles.len());
                results.push(("Index Candlestick", true, format!("{} candles", candles.len())));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Index Candlestick", false, format!("{:?}", e)));
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 4. Stock List
        print!("[4/5] Stock List ({})...", STOCK_LIST_ENDPOINT);
        match adapter.list_all_stocks().await {
            Ok(stocks) => {
                println!(" ✅ PASS ({} stocks)", stocks.len());
                results.push(("Stock List", true, format!("{} stocks", stocks.len())));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Stock List", false, format!("{:?}", e)));
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 5. Fundamental Data
        print!("[5/5] Fundamental Data ({})...", NON_FINANCIAL_ENDPOINT);
        match adapter.get_valuation_metrics("600519.SH", None).await {
            Ok(metrics) => {
                let info = format!("PE:{:?} PB:{:?}", metrics.pe_ttm, metrics.pb);
                println!(" ✅ PASS ({})", info);
                results.push(("Fundamental Data", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Fundamental Data", false, format!("{:?}", e)));
            }
        }

        // Summary
        println!("\n════════════════════════════════════════════════════════════════");
        println!("Summary:");
        let passed = results.iter().filter(|(_, ok, _)| *ok).count();
        let total = results.len();
        println!("  Passed: {}/{}", passed, total);

        for (name, ok, detail) in &results {
            let status = if *ok { "✅" } else { "❌" };
            println!("  {} {}: {}", status, name, detail);
        }
        println!("════════════════════════════════════════════════════════════════\n");

        // Assert all passed
        assert_eq!(passed, total, "Not all endpoints passed verification");
    }

    // ========================================================================
    // Company Profile Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_company_profile() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let symbols = vec!["000001.SZ".to_string()];
        let profiles = adapter.get_company_profile(&symbols).await.unwrap();

        assert!(!profiles.is_empty(), "Should return at least one profile");
        let profile = &profiles[0];
        assert_eq!(profile.stock_code, "000001");
        println!("Company profile for 000001.SZ:");
        println!("  Name: {:?}", profile.company_name);
        println!("  Province: {:?}", profile.province);
        println!("  City: {:?}", profile.city);
        println!("  Main business: {:?}", profile.main_business.as_ref().map(|s| &s[..s.len().min(100)]));
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_company_profile_batch() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let symbols = vec![
            "000001.SZ".to_string(),
            "000002.SZ".to_string(),
            "600000.SH".to_string(),
        ];
        let profiles = adapter.get_company_profile(&symbols).await.unwrap();

        assert!(profiles.len() >= 1, "Should return at least one profile");
        println!("Fetched {} company profiles", profiles.len());
        for profile in &profiles {
            println!("  {} - {:?}", profile.stock_code, profile.company_name);
        }
    }

    // ========================================================================
    // Fund Data Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_fund_list() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        let funds = adapter.get_fund_list(None).await.unwrap();

        println!("Total funds: {}", funds.len());
        if funds.len() > 0 {
            println!("Sample funds:");
            for fund in funds.iter().take(5) {
                println!("  {} - {:?} ({:?})",
                    fund.fund_code,
                    fund.name,
                    fund.fund_type
                );
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_fund_list_by_type() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with stock fund type
        let stock_funds = adapter.get_fund_list(Some("stock")).await.unwrap();
        println!("Stock funds: {}", stock_funds.len());

        // Test with bond fund type
        let bond_funds = adapter.get_fund_list(Some("bond")).await.unwrap();
        println!("Bond funds: {}", bond_funds.len());
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_fund_nav() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // First get some fund codes from the list
        let funds = adapter.get_fund_list(None).await.unwrap();
        if funds.is_empty() {
            println!("No funds available to test NAV");
            return;
        }

        let fund_codes: Vec<String> = funds.iter()
            .take(3)
            .map(|f| f.fund_code.clone())
            .collect();

        let navs = adapter.get_fund_nav(&fund_codes, None, None).await.unwrap();
        println!("Fund NAV data points: {}", navs.len());
        for nav in navs.iter().take(5) {
            println!("  {} @ {} NAV: {:.4}",
                nav.fund_code,
                nav.date,
                nav.unit_nav
            );
        }
    }

    // ========================================================================
    // Extended Endpoint Verification
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token - comprehensive endpoint verification"]
    async fn test_endpoint_verification_report_extended() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n════════════════════════════════════════════════════════════════");
        println!("          Lixin API Extended Endpoint Verification Report");
        println!("════════════════════════════════════════════════════════════════\n");

        let mut results: Vec<(&str, bool, String)> = Vec::new();

        // 1. Company Profile
        print!("[1/2] Company Profile ({})...", COMPANY_PROFILE_ENDPOINT);
        match adapter.get_company_profile(&["000001.SZ".to_string()]).await {
            Ok(profiles) => {
                let info = if profiles.is_empty() {
                    "0 profiles".to_string()
                } else {
                    format!("{} profiles, name: {:?}", profiles.len(), profiles[0].company_name)
                };
                println!(" ✅ PASS ({})", info);
                results.push(("Company Profile", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Company Profile", false, format!("{:?}", e)));
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // 2. Fund List
        print!("[2/2] Fund List ({})...", FUND_LIST_ENDPOINT);
        match adapter.get_fund_list(None).await {
            Ok(funds) => {
                println!(" ✅ PASS ({} funds)", funds.len());
                results.push(("Fund List", true, format!("{} funds", funds.len())));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Fund List", false, format!("{:?}", e)));
            }
        }

        // Summary
        println!("\n════════════════════════════════════════════════════════════════");
        println!("Summary:");
        let passed = results.iter().filter(|(_, ok, _)| *ok).count();
        let total = results.len();
        println!("  Passed: {}/{}", passed, total);

        for (name, ok, detail) in &results {
            let status = if *ok { "✅" } else { "❌" };
            println!("  {} {}: {}", status, name, detail);
        }
        println!("════════════════════════════════════════════════════════════════\n");
    }

    // ========================================================================
    // New API Integration Tests
    // ========================================================================

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_company_industries() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 平安银行 (000001.SZ)
        let industries = adapter.get_company_industries("000001.SZ").await.unwrap();
        println!("Industries for 000001.SZ:");
        for ind in &industries {
            println!("  {} ({}) - {}", ind.name, ind.source, ind.industry_code);
        }
        assert!(!industries.is_empty(), "Should have at least one industry classification");

        // Check for expected sources
        let has_sw = industries.iter().any(|i| i.source.starts_with("sw"));
        println!("Has SW classification: {}", has_sw);
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_company_indices() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 平安银行 (000001.SZ)
        let indices = adapter.get_company_indices("000001.SZ").await.unwrap();
        println!("Index memberships for 000001.SZ: {} indices", indices.len());
        for idx in indices.iter().take(10) {
            println!("  {:?} ({:?}) - {:?}", idx.name, idx.source, idx.index_code);
        }
        assert!(!indices.is_empty(), "Should belong to at least one index");

        // Check for expected indices (沪深300)
        let in_hs300 = indices.iter().any(|i| i.index_code.as_deref() == Some("000300"));
        println!("In 沪深300: {}", in_hs300);
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_announcements() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 平安银行 (000001.SZ), last 30 days
        let announcements = adapter.get_announcements("000001.SZ", None, None).await.unwrap();
        println!("Recent announcements for 000001.SZ: {}", announcements.len());
        for ann in announcements.iter().take(5) {
            println!("  {} - {} ({:?})", ann.date, ann.link_text, ann.types);
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_block_deals() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 平安银行 (000001.SZ), last year
        let start = Utc::now().date_naive() - chrono::Duration::days(365);
        let deals = adapter.get_block_deals("000001.SZ", Some(start), None).await.unwrap();
        println!("Block deals for 000001.SZ: {} deals", deals.len());
        for deal in deals.iter().take(5) {
            let discount_str = deal.discount_rate
                .map(|r| format!("{:.2}%", r * 100.0))
                .unwrap_or_else(|| "N/A".to_string());
            println!(
                "  {} @ {:.2} vol={} disc={}",
                deal.date, deal.trading_price, deal.trading_volume, discount_str
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_index_fundamental() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 沪深300 (000300.SH)
        let indices = vec!["000300.SH".to_string()];
        let data = adapter.get_index_fundamental(&indices, None).await.unwrap();
        println!("Index fundamental data:");
        for idx in &data {
            println!(
                "  {} @ {} MC: {:?}",
                idx.index_code, idx.date, idx.mc
            );
        }
        assert!(!data.is_empty(), "Should return data for 沪深300");
        assert!(data[0].mc.is_some(), "Should have market cap");
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_bank_valuation() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 工商银行 (601398.SH)
        let symbols = vec!["601398.SH".to_string()];
        let data = adapter.get_bank_valuation(&symbols, None).await.unwrap();
        println!("Bank valuation data:");
        for v in &data {
            println!(
                "  {} @ {} PE: {:?} PB: {:?} DY: {:?}%",
                v.symbol, v.date,
                v.pe_ttm, v.pb,
                v.dividend_yield.map(|d| d * 100.0)
            );
        }
        assert!(!data.is_empty(), "Should return data for ICBC");
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_security_valuation() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 华泰证券 (601688.SH)
        let symbols = vec!["601688.SH".to_string()];
        let data = adapter.get_security_valuation(&symbols, None).await.unwrap();
        println!("Security valuation data:");
        for v in &data {
            println!(
                "  {} @ {} PE: {:?} PB: {:?}",
                v.symbol, v.date, v.pe_ttm, v.pb
            );
        }
        assert!(!data.is_empty(), "Should return data for Huatai");
    }

    #[tokio::test]
    #[ignore = "requires valid API token"]
    async fn test_get_insurance_valuation() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        // Test with 中国平安 (601318.SH)
        let symbols = vec!["601318.SH".to_string()];
        let data = adapter.get_insurance_valuation(&symbols, None).await.unwrap();
        println!("Insurance valuation data:");
        for v in &data {
            println!(
                "  {} @ {} PE: {:?} PB: {:?}",
                v.symbol, v.date, v.pe_ttm, v.pb
            );
        }
        assert!(!data.is_empty(), "Should return data for Ping An");
    }

    #[tokio::test]
    #[ignore = "requires valid API token - comprehensive new API verification"]
    async fn test_new_api_verification_report() {
        let token = std::env::var("LIXIN_TOKEN").expect("LIXIN_TOKEN not set");
        let adapter = LixinAdapter::new(token);

        println!("\n════════════════════════════════════════════════════════════════");
        println!("          Lixin API New Endpoints Verification Report");
        println!("════════════════════════════════════════════════════════════════\n");

        let mut results: Vec<(&str, bool, String)> = Vec::new();
        let delay = std::time::Duration::from_millis(500);

        // 1. Company Industries
        print!("[1/8] Company Industries ({})...", COMPANY_INDUSTRIES_ENDPOINT);
        match adapter.get_company_industries("000001.SZ").await {
            Ok(data) => {
                let info = format!("{} classifications", data.len());
                println!(" ✅ PASS ({})", info);
                results.push(("Company Industries", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Company Industries", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 2. Company Indices
        print!("[2/8] Company Indices ({})...", COMPANY_INDICES_ENDPOINT);
        match adapter.get_company_indices("000001.SZ").await {
            Ok(data) => {
                let info = format!("{} indices", data.len());
                println!(" ✅ PASS ({})", info);
                results.push(("Company Indices", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Company Indices", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 3. Announcements
        print!("[3/8] Announcements ({})...", ANNOUNCEMENT_ENDPOINT);
        match adapter.get_announcements("000001.SZ", None, None).await {
            Ok(data) => {
                let info = format!("{} announcements", data.len());
                println!(" ✅ PASS ({})", info);
                results.push(("Announcements", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Announcements", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 4. Block Deals
        print!("[4/8] Block Deals ({})...", BLOCK_DEAL_ENDPOINT);
        let start = Utc::now().date_naive() - chrono::Duration::days(365);
        match adapter.get_block_deals("000001.SZ", Some(start), None).await {
            Ok(data) => {
                let info = format!("{} deals", data.len());
                println!(" ✅ PASS ({})", info);
                results.push(("Block Deals", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Block Deals", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 5. Index Fundamental
        print!("[5/8] Index Fundamental ({})...", INDEX_FUNDAMENTAL_ENDPOINT);
        match adapter.get_index_fundamental(&["000300.SH".to_string()], None).await {
            Ok(data) => {
                let info = format!("{} indices", data.len());
                println!(" ✅ PASS ({})", info);
                results.push(("Index Fundamental", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Index Fundamental", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 6. Bank Valuation
        print!("[6/8] Bank Valuation ({})...", BANK_FUNDAMENTAL_ENDPOINT);
        match adapter.get_bank_valuation(&["601398.SH".to_string()], None).await {
            Ok(data) => {
                let info = if data.is_empty() {
                    "0 records".to_string()
                } else {
                    format!("PE: {:?}, PB: {:?}", data[0].pe_ttm, data[0].pb)
                };
                println!(" ✅ PASS ({})", info);
                results.push(("Bank Valuation", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Bank Valuation", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 7. Security Valuation
        print!("[7/8] Security Valuation ({})...", SECURITY_FUNDAMENTAL_ENDPOINT);
        match adapter.get_security_valuation(&["601688.SH".to_string()], None).await {
            Ok(data) => {
                let info = if data.is_empty() {
                    "0 records".to_string()
                } else {
                    format!("PE: {:?}, PB: {:?}", data[0].pe_ttm, data[0].pb)
                };
                println!(" ✅ PASS ({})", info);
                results.push(("Security Valuation", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Security Valuation", false, format!("{:?}", e)));
            }
        }
        tokio::time::sleep(delay).await;

        // 8. Insurance Valuation
        print!("[8/8] Insurance Valuation ({})...", INSURANCE_FUNDAMENTAL_ENDPOINT);
        match adapter.get_insurance_valuation(&["601318.SH".to_string()], None).await {
            Ok(data) => {
                let info = if data.is_empty() {
                    "0 records".to_string()
                } else {
                    format!("PE: {:?}, PB: {:?}", data[0].pe_ttm, data[0].pb)
                };
                println!(" ✅ PASS ({})", info);
                results.push(("Insurance Valuation", true, info));
            }
            Err(e) => {
                println!(" ❌ FAIL: {:?}", e);
                results.push(("Insurance Valuation", false, format!("{:?}", e)));
            }
        }

        // Summary
        println!("\n════════════════════════════════════════════════════════════════");
        println!("Summary:");
        let passed = results.iter().filter(|(_, ok, _)| *ok).count();
        let total = results.len();
        println!("  Passed: {}/{}", passed, total);

        for (name, ok, detail) in &results {
            let status = if *ok { "✅" } else { "❌" };
            println!("  {} {}: {}", status, name, detail);
        }
        println!("════════════════════════════════════════════════════════════════\n");

        assert!(passed >= 6, "At least 6 out of 8 APIs should pass");
    }
}
