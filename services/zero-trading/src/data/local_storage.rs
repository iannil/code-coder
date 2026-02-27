//! Local financial data storage using SQLite.
//!
//! Provides persistent storage for:
//! - K-line/candle data
//! - Financial statement data
//! - Valuation input data
//! - Macro economic indicators
//! - Analysis result caching
//!
//! This module enables offline access and reduces API calls by caching
//! data locally with intelligent refresh strategies.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use super::{Candle, Timeframe, StockInfo, FinancialStatementData};
use super::lixin::{ValuationMetrics, ValuationMetricName, StatisticsGranularity, ValuationStatistics, ValuationStatisticsSet};
use crate::valuation::types::ValuationInput;
use crate::value::types::FinancialData;

// ============================================================================
// Database Schema
// ============================================================================

const CREATE_TABLES_SQL: &str = r#"
-- K-line data table
CREATE TABLE IF NOT EXISTS candles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    amount REAL DEFAULT 0,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, timeframe, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_candles_symbol_tf_ts
ON candles(symbol, timeframe, timestamp DESC);

-- Financial data table
CREATE TABLE IF NOT EXISTS financials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    period_end TEXT NOT NULL,
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    interest_expense REAL,
    total_assets REAL,
    total_equity REAL,
    total_liabilities REAL,
    cash REAL,
    total_debt REAL,
    operating_cash_flow REAL,
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    capex REAL,
    free_cash_flow REAL,
    avg_roe_5y REAL,
    avg_gross_margin_5y REAL,
    avg_net_margin_5y REAL,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, period_end)
);

CREATE INDEX IF NOT EXISTS idx_financials_symbol_period
ON financials(symbol, period_end DESC);

-- Valuation data table
CREATE TABLE IF NOT EXISTS valuations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    price REAL NOT NULL,
    eps_ttm REAL,
    eps_forward REAL,
    eps_growth_rate REAL,
    book_value_per_share REAL,
    roe REAL,
    dividend_per_share REAL,
    payout_ratio REAL,
    dividend_growth_5y REAL,
    consecutive_dividend_years INTEGER,
    risk_free_rate REAL,
    historical_pe_json TEXT,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, date)
);

CREATE INDEX IF NOT EXISTS idx_valuations_symbol_date
ON valuations(symbol, date DESC);

-- Macro economic indicators table
CREATE TABLE IF NOT EXISTS macro_indicators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    indicator_code TEXT NOT NULL,
    date TEXT NOT NULL,
    value REAL NOT NULL,
    yoy_change REAL,
    mom_change REAL,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(indicator_code, date)
);

CREATE INDEX IF NOT EXISTS idx_macro_indicator_date
ON macro_indicators(indicator_code, date DESC);

-- Analysis result cache table
CREATE TABLE IF NOT EXISTS analysis_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    analysis_type TEXT NOT NULL,
    result_json TEXT NOT NULL,
    expires_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, analysis_type)
);

CREATE INDEX IF NOT EXISTS idx_analysis_cache_symbol_type
ON analysis_cache(symbol, analysis_type);

CREATE INDEX IF NOT EXISTS idx_analysis_cache_expires
ON analysis_cache(expires_at);

-- Stock info table for screener
CREATE TABLE IF NOT EXISTS stocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    exchange TEXT NOT NULL,
    industry TEXT,
    list_date TEXT,
    is_suspended INTEGER DEFAULT 0,
    is_st INTEGER DEFAULT 0,
    market_cap REAL,
    source TEXT NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(code, exchange)
);

CREATE INDEX IF NOT EXISTS idx_stocks_exchange
ON stocks(exchange);

CREATE INDEX IF NOT EXISTS idx_stocks_industry
ON stocks(industry);

CREATE INDEX IF NOT EXISTS idx_stocks_is_st
ON stocks(is_st);

CREATE INDEX IF NOT EXISTS idx_stocks_market_cap
ON stocks(market_cap);

-- Financial statement data table for screener (from provider)
CREATE TABLE IF NOT EXISTS financial_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    period_end TEXT NOT NULL,
    report_type TEXT NOT NULL,
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    interest_expense REAL,
    total_assets REAL,
    total_equity REAL,
    total_liabilities REAL,
    cash REAL,
    total_debt REAL,
    shares_outstanding REAL,
    operating_cash_flow REAL,
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    capex REAL,
    roe REAL,
    roa REAL,
    gross_margin REAL,
    net_margin REAL,
    debt_to_equity REAL,
    current_ratio REAL,
    pe_ttm REAL,
    pb REAL,
    dividend_yield REAL,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, period_end)
);

CREATE INDEX IF NOT EXISTS idx_fs_symbol_period
ON financial_statements(symbol, period_end DESC);

CREATE INDEX IF NOT EXISTS idx_fs_roe
ON financial_statements(roe);

CREATE INDEX IF NOT EXISTS idx_fs_gross_margin
ON financial_statements(gross_margin);

-- Valuation metrics table
CREATE TABLE IF NOT EXISTS valuation_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL,
    pe_ttm REAL,
    pe_ttm_ex_non_recurring REAL,
    pb REAL,
    ps_ttm REAL,
    dividend_yield REAL,
    market_cap REAL,
    circulating_market_cap REAL,
    free_float_market_cap REAL,
    financing_balance REAL,
    short_balance REAL,
    northbound_holdings_shares REAL,
    northbound_holdings_value REAL,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, date)
);

CREATE INDEX IF NOT EXISTS idx_valuation_metrics_symbol_date
ON valuation_metrics(symbol, date DESC);

CREATE INDEX IF NOT EXISTS idx_valuation_metrics_pe_ttm
ON valuation_metrics(pe_ttm);

CREATE INDEX IF NOT EXISTS idx_valuation_metrics_pb
ON valuation_metrics(pb);

CREATE INDEX IF NOT EXISTS idx_valuation_metrics_market_cap
ON valuation_metrics(market_cap);

CREATE INDEX IF NOT EXISTS idx_valuation_metrics_dividend_yield
ON valuation_metrics(dividend_yield);

-- Valuation statistics table
CREATE TABLE IF NOT EXISTS valuation_statistics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    metric TEXT NOT NULL,
    granularity TEXT NOT NULL,
    date TEXT NOT NULL,
    current_value REAL NOT NULL,
    percentile REAL,
    q25 REAL,
    q50 REAL,
    q80 REAL,
    min_val REAL,
    max_val REAL,
    avg_val REAL,
    source TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, metric, granularity, date)
);

CREATE INDEX IF NOT EXISTS idx_valuation_stats_symbol_date
ON valuation_statistics(symbol, date DESC);

CREATE INDEX IF NOT EXISTS idx_valuation_stats_metric
ON valuation_statistics(metric, granularity);

-- Sync metadata table
CREATE TABLE IF NOT EXISTS sync_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_type TEXT NOT NULL,
    symbol TEXT DEFAULT '__global__',
    last_sync_at TEXT NOT NULL,
    next_sync_at TEXT,
    sync_status TEXT NOT NULL,
    error_message TEXT,
    UNIQUE(data_type, symbol)
);

CREATE INDEX IF NOT EXISTS idx_sync_metadata_type
ON sync_metadata(data_type, symbol);
"#;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for local storage
#[derive(Debug, Clone)]
pub struct LocalStorageConfig {
    /// Path to SQLite database
    pub db_path: PathBuf,
    /// Whether local storage is enabled
    pub enabled: bool,
    /// Candle data retention in days
    pub candle_retention_days: u32,
    /// Financial data retention in years
    pub financial_retention_years: u32,
    /// Auto sync on startup
    pub auto_sync_on_startup: bool,
}

impl Default for LocalStorageConfig {
    fn default() -> Self {
        Self {
            db_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codecoder")
                .join("financial.db"),
            enabled: true,
            candle_retention_days: 365,
            financial_retention_years: 5,
            auto_sync_on_startup: true,
        }
    }
}

// ============================================================================
// Sync Status
// ============================================================================

/// Sync status for data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Success,
    Failed,
    Pending,
    InProgress,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
        }
    }
}

impl SyncStatus {
    fn from_str(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "failed" => Self::Failed,
            "pending" => Self::Pending,
            "in_progress" => Self::InProgress,
            _ => Self::Pending,
        }
    }
}

/// Sync metadata record
#[derive(Debug, Clone)]
pub struct SyncMetadata {
    pub data_type: String,
    pub symbol: Option<String>,
    pub last_sync_at: DateTime<Utc>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
    pub error_message: Option<String>,
}

// ============================================================================
// Local Storage
// ============================================================================

/// Local SQLite storage for financial data
pub struct LocalStorage {
    /// SQLite connection wrapped in Mutex for thread safety
    /// Note: We use Mutex instead of RwLock because rusqlite::Connection
    /// is Send but not Sync, and Mutex<T> is Sync when T: Send
    db: Arc<Mutex<Connection>>,
    config: LocalStorageConfig,
}

impl LocalStorage {
    /// Create a new LocalStorage instance
    pub fn new(config: LocalStorageConfig) -> Result<Self> {
        if !config.enabled {
            return Err(anyhow::anyhow!("Local storage is disabled"));
        }

        // Ensure directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open database connection
        let conn = Connection::open(&config.db_path)
            .context("Failed to open local storage database")?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .context("Failed to set database pragmas")?;

        // Create tables
        conn.execute_batch(CREATE_TABLES_SQL)
            .context("Failed to create database tables")?;

        info!(db_path = %config.db_path.display(), "Initialized local storage");

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            config,
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(LocalStorageConfig::default())
    }

    /// Check if local storage is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.config.db_path
    }

    // ========================================================================
    // Candle Data Operations
    // ========================================================================

    /// Get candles from local storage
    pub async fn get_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>> {
        let db = self.db.lock().await;
        let tf_str = timeframe.to_string();

        let mut sql = String::from(
            "SELECT symbol, timeframe, timestamp, open, high, low, close, volume, amount
             FROM candles WHERE symbol = ?1 AND timeframe = ?2",
        );

        if start_date.is_some() {
            sql.push_str(" AND timestamp >= ?3");
        }
        if end_date.is_some() {
            sql.push_str(" AND timestamp <= ?4");
        }
        sql.push_str(" ORDER BY timestamp DESC");
        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }

        let mut stmt = db.prepare(&sql)?;

        let rows = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                stmt.query_map(params![symbol, tf_str, start.to_string(), end.to_string()], Self::row_to_candle)?
            }
            (Some(start), None) => {
                stmt.query_map(params![symbol, tf_str, start.to_string()], Self::row_to_candle)?
            }
            (None, Some(end)) => {
                // Need different SQL for this case
                let sql2 = format!(
                    "SELECT symbol, timeframe, timestamp, open, high, low, close, volume, amount
                     FROM candles WHERE symbol = ?1 AND timeframe = ?2 AND timestamp <= ?3
                     ORDER BY timestamp DESC{}",
                    limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default()
                );
                let mut stmt2 = db.prepare(&sql2)?;
                let results: Vec<Candle> = stmt2
                    .query_map(params![symbol, tf_str, end.to_string()], Self::row_to_candle)?
                    .filter_map(|r| r.ok())
                    .collect();
                // Return early for this branch (results are already in DESC order, reverse for chronological)
                return Ok(results.into_iter().rev().collect());
            }
            (None, None) => {
                stmt.query_map(params![symbol, tf_str], Self::row_to_candle)?
            }
        };

        let mut candles: Vec<Candle> = rows.filter_map(|r| r.ok()).collect();
        candles.reverse(); // Return in chronological order
        Ok(candles)
    }

    fn row_to_candle(row: &rusqlite::Row) -> rusqlite::Result<Candle> {
        let symbol: String = row.get(0)?;
        let tf_str: String = row.get(1)?;
        let timestamp_str: String = row.get(2)?;

        let timeframe = Timeframe::from_str(&tf_str).unwrap_or(Timeframe::Daily);
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Candle {
            symbol,
            timeframe,
            timestamp,
            open: row.get(3)?,
            high: row.get(4)?,
            low: row.get(5)?,
            close: row.get(6)?,
            volume: row.get(7)?,
            amount: row.get(8)?,
        })
    }

    /// Save candles to local storage
    pub async fn save_candles(&self, candles: &[Candle], source: &str) -> Result<usize> {
        if candles.is_empty() {
            return Ok(0);
        }

        let db = self.db.lock().await;

        let mut count = 0;
        for candle in candles {
            let result = db.execute(
                r#"
                INSERT OR REPLACE INTO candles
                (symbol, timeframe, timestamp, open, high, low, close, volume, amount, source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    candle.symbol,
                    candle.timeframe.to_string(),
                    candle.timestamp.to_rfc3339(),
                    candle.open,
                    candle.high,
                    candle.low,
                    candle.close,
                    candle.volume,
                    candle.amount,
                    source,
                ],
            );

            if result.is_ok() {
                count += 1;
            }
        }

        debug!(symbol = %candles[0].symbol, count, "Saved candles to local storage");
        Ok(count)
    }

    /// Check if candles exist for a symbol and timeframe
    pub async fn has_candles(&self, symbol: &str, timeframe: Timeframe) -> Result<bool> {
        let db = self.db.lock().await;
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM candles WHERE symbol = ?1 AND timeframe = ?2",
            params![symbol, timeframe.to_string()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get the most recent candle timestamp for a symbol
    pub async fn get_latest_candle_timestamp(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<Option<DateTime<Utc>>> {
        let db = self.db.lock().await;
        let result: rusqlite::Result<String> = db.query_row(
            "SELECT MAX(timestamp) FROM candles WHERE symbol = ?1 AND timeframe = ?2",
            params![symbol, timeframe.to_string()],
            |row| row.get(0),
        );

        match result {
            Ok(ts_str) => {
                let ts = DateTime::parse_from_rfc3339(&ts_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok();
                Ok(ts)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ========================================================================
    // Financial Data Operations
    // ========================================================================

    /// Get financial data for a symbol
    pub async fn get_financial(
        &self,
        symbol: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<Option<FinancialData>> {
        let db = self.db.lock().await;

        let sql = if period_end.is_some() {
            "SELECT * FROM financials WHERE symbol = ?1 AND period_end = ?2"
        } else {
            "SELECT * FROM financials WHERE symbol = ?1 ORDER BY period_end DESC LIMIT 1"
        };

        let result = if let Some(period) = period_end {
            db.query_row(sql, params![symbol, period.to_string()], Self::row_to_financial)
        } else {
            db.query_row(sql, params![symbol], Self::row_to_financial)
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_financial(row: &rusqlite::Row) -> rusqlite::Result<FinancialData> {
        let symbol: String = row.get(1)?;
        let period_str: String = row.get(2)?;
        let period_end = NaiveDate::parse_from_str(&period_str, "%Y-%m-%d")
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .unwrap_or_else(|_| Utc::now());

        Ok(FinancialData {
            symbol,
            period_end,
            revenue: row.get(3)?,
            gross_profit: row.get(4)?,
            operating_income: row.get(5)?,
            net_income: row.get(6)?,
            interest_expense: row.get(7)?,
            total_assets: row.get(8)?,
            total_equity: row.get(9)?,
            total_liabilities: row.get(10)?,
            cash: row.get(11)?,
            total_debt: row.get(12)?,
            operating_cash_flow: row.get(13)?,
            investing_cash_flow: row.get(14)?,
            financing_cash_flow: row.get(15)?,
            capex: row.get(16)?,
            free_cash_flow: row.get(17)?,
            avg_roe_5y: row.get(18)?,
            avg_gross_margin_5y: row.get(19)?,
            avg_net_margin_5y: row.get(20)?,
        })
    }

    /// Save financial data
    pub async fn save_financial(&self, data: &FinancialData, source: &str) -> Result<()> {
        let db = self.db.lock().await;

        db.execute(
            r#"
            INSERT OR REPLACE INTO financials
            (symbol, period_end, revenue, gross_profit, operating_income, net_income,
             interest_expense, total_assets, total_equity, total_liabilities, cash, total_debt,
             operating_cash_flow, investing_cash_flow, financing_cash_flow, capex, free_cash_flow,
             avg_roe_5y, avg_gross_margin_5y, avg_net_margin_5y, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            "#,
            params![
                data.symbol,
                data.period_end.format("%Y-%m-%d").to_string(),
                data.revenue,
                data.gross_profit,
                data.operating_income,
                data.net_income,
                data.interest_expense,
                data.total_assets,
                data.total_equity,
                data.total_liabilities,
                data.cash,
                data.total_debt,
                data.operating_cash_flow,
                data.investing_cash_flow,
                data.financing_cash_flow,
                data.capex,
                data.free_cash_flow,
                data.avg_roe_5y,
                data.avg_gross_margin_5y,
                data.avg_net_margin_5y,
                source,
            ],
        )?;

        debug!(symbol = %data.symbol, "Saved financial data to local storage");
        Ok(())
    }

    /// Get financial history for a symbol
    pub async fn get_financial_history(
        &self,
        symbol: &str,
        years: u32,
    ) -> Result<Vec<FinancialData>> {
        let db = self.db.lock().await;

        let mut stmt = db.prepare(
            "SELECT * FROM financials WHERE symbol = ?1 ORDER BY period_end DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![symbol, years * 4], Self::row_to_financial)?;
        let mut results = Vec::new();
        for row in rows {
            if let Ok(data) = row {
                results.push(data);
            }
        }
        Ok(results)
    }

    // ========================================================================
    // Valuation Data Operations
    // ========================================================================

    /// Get valuation data for a symbol
    pub async fn get_valuation(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<Option<ValuationInput>> {
        let db = self.db.lock().await;

        let sql = if date.is_some() {
            "SELECT * FROM valuations WHERE symbol = ?1 AND date = ?2"
        } else {
            "SELECT * FROM valuations WHERE symbol = ?1 ORDER BY date DESC LIMIT 1"
        };

        let result = if let Some(d) = date {
            db.query_row(sql, params![symbol, d.to_string()], Self::row_to_valuation)
        } else {
            db.query_row(sql, params![symbol], Self::row_to_valuation)
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_valuation(row: &rusqlite::Row) -> rusqlite::Result<ValuationInput> {
        let symbol: String = row.get(1)?;
        let historical_pe_json: Option<String> = row.get(14)?;
        let historical_pe = historical_pe_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        Ok(ValuationInput {
            symbol,
            price: row.get(3)?,
            eps_ttm: row.get(4)?,
            eps_forward: row.get(5)?,
            eps_growth_rate: row.get(6)?,
            book_value_per_share: row.get(7)?,
            roe: row.get(8)?,
            dividend_per_share: row.get(9)?,
            payout_ratio: row.get(10)?,
            dividend_growth_5y: row.get(11)?,
            consecutive_dividend_years: row.get(12)?,
            risk_free_rate: row.get(13)?,
            historical_pe,
        })
    }

    /// Save valuation data
    pub async fn save_valuation(
        &self,
        data: &ValuationInput,
        date: NaiveDate,
        source: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        let historical_pe_json = serde_json::to_string(&data.historical_pe)?;

        db.execute(
            r#"
            INSERT OR REPLACE INTO valuations
            (symbol, date, price, eps_ttm, eps_forward, eps_growth_rate, book_value_per_share,
             roe, dividend_per_share, payout_ratio, dividend_growth_5y, consecutive_dividend_years,
             risk_free_rate, historical_pe_json, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                data.symbol,
                date.to_string(),
                data.price,
                data.eps_ttm,
                data.eps_forward,
                data.eps_growth_rate,
                data.book_value_per_share,
                data.roe,
                data.dividend_per_share,
                data.payout_ratio,
                data.dividend_growth_5y,
                data.consecutive_dividend_years,
                data.risk_free_rate,
                historical_pe_json,
                source,
            ],
        )?;

        debug!(symbol = %data.symbol, "Saved valuation data to local storage");
        Ok(())
    }

    // ========================================================================
    // Macro Indicator Operations
    // ========================================================================

    /// Get macro indicator value
    pub async fn get_macro_indicator(
        &self,
        indicator_code: &str,
        date: Option<NaiveDate>,
    ) -> Result<Option<(f64, Option<f64>, Option<f64>)>> {
        let db = self.db.lock().await;

        let sql = if date.is_some() {
            "SELECT value, yoy_change, mom_change FROM macro_indicators WHERE indicator_code = ?1 AND date = ?2"
        } else {
            "SELECT value, yoy_change, mom_change FROM macro_indicators WHERE indicator_code = ?1 ORDER BY date DESC LIMIT 1"
        };

        let result = if let Some(d) = date {
            db.query_row(sql, params![indicator_code, d.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
        } else {
            db.query_row(sql, params![indicator_code], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save macro indicator
    pub async fn save_macro_indicator(
        &self,
        indicator_code: &str,
        date: NaiveDate,
        value: f64,
        yoy_change: Option<f64>,
        mom_change: Option<f64>,
        source: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        db.execute(
            r#"
            INSERT OR REPLACE INTO macro_indicators
            (indicator_code, date, value, yoy_change, mom_change, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![indicator_code, date.to_string(), value, yoy_change, mom_change, source],
        )?;

        debug!(indicator = indicator_code, "Saved macro indicator to local storage");
        Ok(())
    }

    /// Get macro indicator history
    pub async fn get_macro_indicator_history(
        &self,
        indicator_code: &str,
        days: u32,
    ) -> Result<Vec<(NaiveDate, f64, Option<f64>, Option<f64>)>> {
        let db = self.db.lock().await;

        let mut stmt = db.prepare(
            "SELECT date, value, yoy_change, mom_change FROM macro_indicators
             WHERE indicator_code = ?1 ORDER BY date DESC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![indicator_code, days], |row| {
            let date_str: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::Local::now().date_naive());
            Ok((date, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(data) = row {
                results.push(data);
            }
        }
        Ok(results)
    }

    // ========================================================================
    // Analysis Cache Operations
    // ========================================================================

    /// Get cached analysis result
    pub async fn get_analysis_cache<T: DeserializeOwned>(
        &self,
        symbol: &str,
        analysis_type: &str,
    ) -> Result<Option<T>> {
        let db = self.db.lock().await;

        let result: rusqlite::Result<(String, Option<String>)> = db.query_row(
            "SELECT result_json, expires_at FROM analysis_cache WHERE symbol = ?1 AND analysis_type = ?2",
            params![symbol, analysis_type],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((json, expires_at)) => {
                // Check if expired
                if let Some(exp_str) = expires_at {
                    if let Ok(exp) = DateTime::parse_from_rfc3339(&exp_str) {
                        if exp.with_timezone(&Utc) < Utc::now() {
                            debug!(symbol, analysis_type, "Analysis cache expired");
                            return Ok(None);
                        }
                    }
                }

                let data: T = serde_json::from_str(&json)?;
                Ok(Some(data))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save analysis result to cache
    pub async fn save_analysis_cache<T: Serialize>(
        &self,
        symbol: &str,
        analysis_type: &str,
        result: &T,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let db = self.db.lock().await;

        let json = serde_json::to_string(result)?;
        let exp_str = expires_at.map(|e| e.to_rfc3339());

        db.execute(
            r#"
            INSERT OR REPLACE INTO analysis_cache
            (symbol, analysis_type, result_json, expires_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![symbol, analysis_type, json, exp_str],
        )?;

        debug!(symbol, analysis_type, "Saved analysis result to cache");
        Ok(())
    }

    /// Clear expired cache entries
    pub async fn clear_expired_cache(&self) -> Result<usize> {
        let db = self.db.lock().await;

        let now = Utc::now().to_rfc3339();
        let count = db.execute(
            "DELETE FROM analysis_cache WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )?;

        if count > 0 {
            info!(count, "Cleared expired cache entries");
        }
        Ok(count)
    }

    /// Invalidate cache for a symbol
    pub async fn invalidate_cache(&self, symbol: &str, analysis_type: Option<&str>) -> Result<()> {
        let db = self.db.lock().await;

        if let Some(at) = analysis_type {
            db.execute(
                "DELETE FROM analysis_cache WHERE symbol = ?1 AND analysis_type = ?2",
                params![symbol, at],
            )?;
        } else {
            db.execute(
                "DELETE FROM analysis_cache WHERE symbol = ?1",
                params![symbol],
            )?;
        }

        debug!(symbol, "Invalidated analysis cache");
        Ok(())
    }

    // ========================================================================
    // Sync Metadata Operations
    // ========================================================================

    /// Get sync metadata
    pub async fn get_sync_metadata(
        &self,
        data_type: &str,
        symbol: Option<&str>,
    ) -> Result<Option<SyncMetadata>> {
        let db = self.db.lock().await;
        let symbol_key = symbol.unwrap_or("__global__");

        let result = db.query_row(
            "SELECT data_type, symbol, last_sync_at, next_sync_at, sync_status, error_message
             FROM sync_metadata WHERE data_type = ?1 AND symbol = ?2",
            params![data_type, symbol_key],
            |row| {
                let last_sync_str: String = row.get(2)?;
                let next_sync_str: Option<String> = row.get(3)?;
                let status_str: String = row.get(4)?;
                let symbol_val: String = row.get(1)?;

                Ok(SyncMetadata {
                    data_type: row.get(0)?,
                    symbol: if symbol_val == "__global__" { None } else { Some(symbol_val) },
                    last_sync_at: DateTime::parse_from_rfc3339(&last_sync_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    next_sync_at: next_sync_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                    sync_status: SyncStatus::from_str(&status_str),
                    error_message: row.get(5)?,
                })
            },
        );

        match result {
            Ok(meta) => Ok(Some(meta)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update sync metadata
    pub async fn update_sync_metadata(
        &self,
        data_type: &str,
        symbol: Option<&str>,
        status: SyncStatus,
        next_sync_at: Option<DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let db = self.db.lock().await;
        let symbol_key = symbol.unwrap_or("__global__");

        db.execute(
            r#"
            INSERT OR REPLACE INTO sync_metadata
            (data_type, symbol, last_sync_at, next_sync_at, sync_status, error_message)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                data_type,
                symbol_key,
                Utc::now().to_rfc3339(),
                next_sync_at.map(|t| t.to_rfc3339()),
                status.to_string(),
                error_message,
            ],
        )?;

        Ok(())
    }

    // ========================================================================
    // Maintenance Operations
    // ========================================================================

    /// Clean up old data based on retention settings
    pub async fn cleanup_old_data(&self) -> Result<(usize, usize, usize)> {
        let db = self.db.lock().await;

        let candle_cutoff = Utc::now() - chrono::Duration::days(self.config.candle_retention_days as i64);
        let financial_cutoff = Utc::now() - chrono::Duration::days((self.config.financial_retention_years * 365) as i64);

        let candles_deleted = db.execute(
            "DELETE FROM candles WHERE created_at < ?1",
            params![candle_cutoff.to_rfc3339()],
        )?;

        let financials_deleted = db.execute(
            "DELETE FROM financials WHERE created_at < ?1",
            params![financial_cutoff.to_rfc3339()],
        )?;

        let cache_deleted = self.clear_expired_cache().await?;

        info!(
            candles = candles_deleted,
            financials = financials_deleted,
            cache = cache_deleted,
            "Cleaned up old data"
        );

        Ok((candles_deleted, financials_deleted, cache_deleted))
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<LocalStorageStats> {
        let db = self.db.lock().await;

        let candle_count: i64 = db.query_row("SELECT COUNT(*) FROM candles", [], |row| row.get(0))?;
        let financial_count: i64 = db.query_row("SELECT COUNT(*) FROM financials", [], |row| row.get(0))?;
        let valuation_count: i64 = db.query_row("SELECT COUNT(*) FROM valuations", [], |row| row.get(0))?;
        let macro_count: i64 = db.query_row("SELECT COUNT(*) FROM macro_indicators", [], |row| row.get(0))?;
        let cache_count: i64 = db.query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))?;

        let unique_symbols: i64 = db.query_row(
            "SELECT COUNT(DISTINCT symbol) FROM candles",
            [],
            |row| row.get(0),
        )?;

        // Get file size
        let file_size = std::fs::metadata(&self.config.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(LocalStorageStats {
            candle_count: candle_count as u64,
            financial_count: financial_count as u64,
            valuation_count: valuation_count as u64,
            macro_count: macro_count as u64,
            cache_count: cache_count as u64,
            unique_symbols: unique_symbols as u64,
            db_size_bytes: file_size,
        })
    }

    /// Vacuum the database to reclaim space
    pub async fn vacuum(&self) -> Result<()> {
        let db = self.db.lock().await;
        db.execute_batch("VACUUM")?;
        info!("Vacuumed local storage database");
        Ok(())
    }

    // ========================================================================
    // Stock Info Operations (for Screener)
    // ========================================================================

    /// Save stock list to local storage
    pub async fn save_stock_list(&self, stocks: &[StockInfo], source: &str) -> Result<usize> {
        if stocks.is_empty() {
            return Ok(0);
        }

        let db = self.db.lock().await;
        let mut count = 0;

        for stock in stocks {
            let result = db.execute(
                r#"
                INSERT OR REPLACE INTO stocks
                (code, name, exchange, industry, list_date, is_suspended, is_st, market_cap, source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    stock.code,
                    stock.name,
                    stock.exchange,
                    stock.industry,
                    stock.list_date.map(|d| d.to_string()),
                    if stock.is_suspended { 1 } else { 0 },
                    if stock.is_st { 1 } else { 0 },
                    stock.market_cap,
                    source,
                ],
            );

            if result.is_ok() {
                count += 1;
            }
        }

        debug!(count, "Saved stock list to local storage");
        Ok(count)
    }

    /// Get all stocks from local storage
    pub async fn get_all_stocks(&self) -> Result<Vec<StockInfo>> {
        let db = self.db.lock().await;

        let mut stmt = db.prepare(
            "SELECT code, name, exchange, industry, list_date, is_suspended, is_st, market_cap
             FROM stocks ORDER BY code",
        )?;

        let rows = stmt.query_map([], Self::row_to_stock_info)?;
        let mut stocks = Vec::new();
        for row in rows {
            if let Ok(stock) = row {
                stocks.push(stock);
            }
        }
        Ok(stocks)
    }

    /// Get stocks filtered by criteria (for screening)
    ///
    /// # Arguments
    /// * `exclude_st` - Exclude ST stocks
    /// * `exclude_bj` - Exclude Beijing Exchange stocks
    /// * `min_listing_days` - Minimum days since listing
    /// * `exchange_filter` - Only include these exchanges (e.g., ["SZ", "SH"])
    pub async fn get_stocks_by_filter(
        &self,
        exclude_st: bool,
        exclude_bj: bool,
        min_listing_days: Option<u32>,
    ) -> Result<Vec<StockInfo>> {
        let db = self.db.lock().await;

        let mut sql = String::from(
            "SELECT code, name, exchange, industry, list_date, is_suspended, is_st, market_cap
             FROM stocks WHERE is_suspended = 0",
        );

        if exclude_st {
            sql.push_str(" AND is_st = 0");
        }
        if exclude_bj {
            sql.push_str(" AND exchange != 'BJ'");
        }
        if let Some(min_days) = min_listing_days {
            let cutoff = chrono::Local::now().date_naive() - chrono::Duration::days(min_days as i64);
            sql.push_str(&format!(" AND list_date <= '{}'", cutoff));
        }

        sql.push_str(" ORDER BY code");

        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_stock_info)?;

        let mut stocks = Vec::new();
        for row in rows {
            if let Ok(stock) = row {
                stocks.push(stock);
            }
        }
        Ok(stocks)
    }

    fn row_to_stock_info(row: &rusqlite::Row) -> rusqlite::Result<StockInfo> {
        let list_date_str: Option<String> = row.get(4)?;
        let list_date = list_date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        Ok(StockInfo {
            code: row.get(0)?,
            name: row.get(1)?,
            exchange: row.get(2)?,
            industry: row.get(3)?,
            list_date,
            is_suspended: row.get::<_, i32>(5)? != 0,
            is_st: row.get::<_, i32>(6)? != 0,
            market_cap: row.get(7)?,
        })
    }

    /// Get the count of stocks in local storage
    pub async fn get_stock_count(&self) -> Result<u64> {
        let db = self.db.lock().await;
        let count: i64 = db.query_row("SELECT COUNT(*) FROM stocks", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    // ========================================================================
    // Financial Statement Operations (for Screener)
    // ========================================================================

    /// Save financial statement data to local storage
    pub async fn save_financial_statement(
        &self,
        data: &FinancialStatementData,
        source: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        db.execute(
            r#"
            INSERT OR REPLACE INTO financial_statements
            (symbol, period_end, report_type, revenue, gross_profit, operating_income, net_income,
             interest_expense, total_assets, total_equity, total_liabilities, cash, total_debt,
             shares_outstanding, operating_cash_flow, investing_cash_flow, financing_cash_flow,
             capex, roe, roa, gross_margin, net_margin, debt_to_equity, current_ratio,
             pe_ttm, pb, dividend_yield, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                    ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
            "#,
            params![
                data.symbol,
                data.period_end.to_string(),
                data.report_type,
                data.revenue,
                data.gross_profit,
                data.operating_income,
                data.net_income,
                data.interest_expense,
                data.total_assets,
                data.total_equity,
                data.total_liabilities,
                data.cash,
                data.total_debt,
                data.shares_outstanding,
                data.operating_cash_flow,
                data.investing_cash_flow,
                data.financing_cash_flow,
                data.capex,
                data.roe,
                data.roa,
                data.gross_margin,
                data.net_margin,
                data.debt_to_equity,
                data.current_ratio,
                data.pe_ttm,
                data.pb,
                data.dividend_yield,
                source,
            ],
        )?;

        debug!(symbol = %data.symbol, "Saved financial statement to local storage");
        Ok(())
    }

    /// Batch save financial statements
    pub async fn save_financial_statements(
        &self,
        data: &[FinancialStatementData],
        source: &str,
    ) -> Result<usize> {
        let mut count = 0;
        for item in data {
            if self.save_financial_statement(item, source).await.is_ok() {
                count += 1;
            }
        }
        debug!(count, "Saved financial statements to local storage");
        Ok(count)
    }

    /// Get financial statement data for a symbol
    pub async fn get_financial_statement(
        &self,
        symbol: &str,
        period_end: Option<NaiveDate>,
    ) -> Result<Option<FinancialStatementData>> {
        let db = self.db.lock().await;

        let sql = if period_end.is_some() {
            "SELECT * FROM financial_statements WHERE symbol = ?1 AND period_end = ?2"
        } else {
            "SELECT * FROM financial_statements WHERE symbol = ?1 ORDER BY period_end DESC LIMIT 1"
        };

        let result = if let Some(period) = period_end {
            db.query_row(sql, params![symbol, period.to_string()], Self::row_to_financial_statement)
        } else {
            db.query_row(sql, params![symbol], Self::row_to_financial_statement)
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_financial_statement(row: &rusqlite::Row) -> rusqlite::Result<FinancialStatementData> {
        let symbol: String = row.get(1)?;
        let period_str: String = row.get(2)?;
        let period_end = NaiveDate::parse_from_str(&period_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Local::now().date_naive());

        Ok(FinancialStatementData {
            symbol,
            period_end,
            report_type: row.get(3)?,
            revenue: row.get(4)?,
            gross_profit: row.get(5)?,
            operating_income: row.get(6)?,
            net_income: row.get(7)?,
            interest_expense: row.get(8)?,
            total_assets: row.get(9)?,
            total_equity: row.get(10)?,
            total_liabilities: row.get(11)?,
            cash: row.get(12)?,
            total_debt: row.get(13)?,
            shares_outstanding: row.get(14)?,
            operating_cash_flow: row.get(15)?,
            investing_cash_flow: row.get(16)?,
            financing_cash_flow: row.get(17)?,
            capex: row.get(18)?,
            roe: row.get(19)?,
            roa: row.get(20)?,
            gross_margin: row.get(21)?,
            net_margin: row.get(22)?,
            debt_to_equity: row.get(23)?,
            current_ratio: row.get(24)?,
            pe_ttm: row.get(25)?,
            pb: row.get(26)?,
            dividend_yield: row.get(27)?,
        })
    }

    /// Get financial statements by filter criteria (for screening)
    ///
    /// # Arguments
    /// * `min_roe` - Minimum ROE percentage
    /// * `min_gross_margin` - Minimum gross margin percentage
    /// * `max_debt_ratio` - Maximum debt to equity ratio
    /// * `healthy_cash_flow` - Only include stocks with positive OCF
    pub async fn get_financial_statements_by_filter(
        &self,
        min_roe: Option<f64>,
        min_gross_margin: Option<f64>,
        max_debt_ratio: Option<f64>,
        healthy_cash_flow: bool,
    ) -> Result<Vec<FinancialStatementData>> {
        let db = self.db.lock().await;

        // Get only the latest financial statement for each symbol
        let mut sql = String::from(
            "SELECT * FROM financial_statements fs
             WHERE fs.period_end = (
                 SELECT MAX(period_end) FROM financial_statements
                 WHERE symbol = fs.symbol
             )",
        );

        if let Some(min) = min_roe {
            sql.push_str(&format!(" AND roe >= {}", min));
        }
        if let Some(min) = min_gross_margin {
            sql.push_str(&format!(" AND gross_margin >= {}", min));
        }
        if let Some(max) = max_debt_ratio {
            sql.push_str(&format!(" AND debt_to_equity <= {}", max));
        }
        if healthy_cash_flow {
            sql.push_str(" AND operating_cash_flow > 0");
        }

        sql.push_str(" ORDER BY roe DESC");

        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_financial_statement)?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(data) = row {
                results.push(data);
            }
        }
        Ok(results)
    }

    /// Get stock symbols with financial data in local storage
    pub async fn get_symbols_with_financials(&self) -> Result<Vec<String>> {
        let db = self.db.lock().await;

        let mut stmt = db.prepare(
            "SELECT DISTINCT symbol FROM financial_statements ORDER BY symbol",
        )?;

        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut symbols: Vec<String> = Vec::new();
        for row in rows {
            if let Ok(symbol) = row {
                symbols.push(symbol);
            }
        }
        Ok(symbols)
    }

    /// Check if we have recent financial data for a symbol
    pub async fn has_recent_financial_data(&self, symbol: &str, max_age_days: i64) -> Result<bool> {
        let db = self.db.lock().await;

        let cutoff = chrono::Local::now().date_naive() - chrono::Duration::days(max_age_days);

        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM financial_statements WHERE symbol = ?1 AND created_at >= ?2",
            params![symbol, cutoff.to_string()],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    // ========================================================================
    // Valuation Metrics Operations
    // ========================================================================

    /// Save valuation metrics to local storage
    pub async fn save_valuation_metrics(
        &self,
        data: &ValuationMetrics,
        source: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        db.execute(
            r#"
            INSERT OR REPLACE INTO valuation_metrics
            (symbol, date, pe_ttm, pe_ttm_ex_non_recurring, pb, ps_ttm, dividend_yield,
             market_cap, circulating_market_cap, free_float_market_cap,
             financing_balance, short_balance, northbound_holdings_shares, northbound_holdings_value, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                data.symbol,
                data.date.to_string(),
                data.pe_ttm,
                data.pe_ttm_ex_non_recurring,
                data.pb,
                data.ps_ttm,
                data.dividend_yield,
                data.market_cap,
                data.circulating_market_cap,
                data.free_float_market_cap,
                data.financing_balance,
                data.short_balance,
                data.northbound_holdings_shares,
                data.northbound_holdings_value,
                source,
            ],
        )?;

        debug!(symbol = %data.symbol, "Saved valuation metrics to local storage");
        Ok(())
    }

    /// Batch save valuation metrics
    pub async fn save_valuation_metrics_batch(
        &self,
        data: &[ValuationMetrics],
        source: &str,
    ) -> Result<usize> {
        let mut count = 0;
        for item in data {
            if self.save_valuation_metrics(item, source).await.is_ok() {
                count += 1;
            }
        }
        debug!(count, "Saved valuation metrics batch to local storage");
        Ok(count)
    }

    /// Get valuation metrics for a symbol
    pub async fn get_valuation_metrics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<Option<ValuationMetrics>> {
        let db = self.db.lock().await;

        let sql = if date.is_some() {
            "SELECT * FROM valuation_metrics WHERE symbol = ?1 AND date = ?2"
        } else {
            "SELECT * FROM valuation_metrics WHERE symbol = ?1 ORDER BY date DESC LIMIT 1"
        };

        let result = if let Some(d) = date {
            db.query_row(sql, params![symbol, d.to_string()], Self::row_to_valuation_metrics)
        } else {
            db.query_row(sql, params![symbol], Self::row_to_valuation_metrics)
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn row_to_valuation_metrics(row: &rusqlite::Row) -> rusqlite::Result<ValuationMetrics> {
        let symbol: String = row.get(1)?;
        let date_str: String = row.get(2)?;
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Local::now().date_naive());

        Ok(ValuationMetrics {
            symbol,
            date,
            pe_ttm: row.get(3)?,
            pe_ttm_ex_non_recurring: row.get(4)?,
            pb: row.get(5)?,
            ps_ttm: row.get(6)?,
            dividend_yield: row.get(7)?,
            market_cap: row.get(8)?,
            circulating_market_cap: row.get(9)?,
            free_float_market_cap: row.get(10)?,
            financing_balance: row.get(11)?,
            short_balance: row.get(12)?,
            northbound_holdings_shares: row.get(13)?,
            northbound_holdings_value: row.get(14)?,
        })
    }

    /// Get valuation metrics by filter criteria (for screening)
    ///
    /// # Arguments
    /// * `min_pe` - Minimum PE ratio
    /// * `max_pe` - Maximum PE ratio
    /// * `min_pb` - Minimum PB ratio
    /// * `max_pb` - Maximum PB ratio
    /// * `min_market_cap` - Minimum market cap (in billion yuan)
    /// * `max_market_cap` - Maximum market cap (in billion yuan)
    /// * `min_dividend_yield` - Minimum dividend yield percentage
    pub async fn get_valuation_metrics_by_filter(
        &self,
        min_pe: Option<f64>,
        max_pe: Option<f64>,
        min_pb: Option<f64>,
        max_pb: Option<f64>,
        min_market_cap: Option<f64>,
        max_market_cap: Option<f64>,
        min_dividend_yield: Option<f64>,
    ) -> Result<Vec<ValuationMetrics>> {
        let db = self.db.lock().await;

        // Get only the latest valuation metrics for each symbol
        let mut sql = String::from(
            "SELECT * FROM valuation_metrics vm
             WHERE vm.date = (
                 SELECT MAX(date) FROM valuation_metrics
                 WHERE symbol = vm.symbol
             )",
        );

        if let Some(min) = min_pe {
            sql.push_str(&format!(" AND pe_ttm >= {}", min));
        }
        if let Some(max) = max_pe {
            sql.push_str(&format!(" AND pe_ttm <= {}", max));
        }
        if let Some(min) = min_pb {
            sql.push_str(&format!(" AND pb >= {}", min));
        }
        if let Some(max) = max_pb {
            sql.push_str(&format!(" AND pb <= {}", max));
        }
        if let Some(min) = min_market_cap {
            sql.push_str(&format!(" AND market_cap >= {}", min));
        }
        if let Some(max) = max_market_cap {
            sql.push_str(&format!(" AND market_cap <= {}", max));
        }
        if let Some(min) = min_dividend_yield {
            sql.push_str(&format!(" AND dividend_yield >= {}", min));
        }

        sql.push_str(" ORDER BY pe_ttm ASC");

        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_valuation_metrics)?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(data) = row {
                results.push(data);
            }
        }
        Ok(results)
    }

    /// Get stock symbols with valuation metrics in local storage
    pub async fn get_symbols_with_valuation_metrics(&self) -> Result<Vec<String>> {
        let db = self.db.lock().await;

        let mut stmt = db.prepare(
            "SELECT DISTINCT symbol FROM valuation_metrics ORDER BY symbol",
        )?;

        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut symbols: Vec<String> = Vec::new();
        for row in rows {
            if let Ok(symbol) = row {
                symbols.push(symbol);
            }
        }
        Ok(symbols)
    }

    // ========================================================================
    // Valuation Statistics Operations
    // ========================================================================

    /// Save valuation statistics to local storage
    pub async fn save_valuation_statistics(
        &self,
        data: &ValuationStatisticsSet,
        source: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        // Save all statistics
        for stats in &data.pe_ttm_stats {
            self.save_single_valuation_stat(
                &db,
                &data.symbol,
                &data.date,
                stats.metric,
                stats.granularity,
                stats.current_value,
                stats.percentile,
                stats.q25,
                stats.q50,
                stats.q80,
                stats.min,
                stats.max,
                stats.avg,
                source,
            )?;
        }

        for stats in &data.pb_stats {
            self.save_single_valuation_stat(
                &db,
                &data.symbol,
                &data.date,
                stats.metric,
                stats.granularity,
                stats.current_value,
                stats.percentile,
                stats.q25,
                stats.q50,
                stats.q80,
                stats.min,
                stats.max,
                stats.avg,
                source,
            )?;
        }

        for stats in &data.dividend_yield_stats {
            self.save_single_valuation_stat(
                &db,
                &data.symbol,
                &data.date,
                stats.metric,
                stats.granularity,
                stats.current_value,
                stats.percentile,
                stats.q25,
                stats.q50,
                stats.q80,
                stats.min,
                stats.max,
                stats.avg,
                source,
            )?;
        }

        debug!(symbol = %data.symbol, "Saved valuation statistics to local storage");
        Ok(())
    }

    fn save_single_valuation_stat(
        &self,
        db: &Connection,
        symbol: &str,
        date: &NaiveDate,
        metric: ValuationMetricName,
        granularity: StatisticsGranularity,
        current_value: f64,
        percentile: Option<f64>,
        q25: Option<f64>,
        q50: Option<f64>,
        q80: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
        avg: Option<f64>,
        source: &str,
    ) -> rusqlite::Result<()> {
        db.execute(
            r#"
            INSERT OR REPLACE INTO valuation_statistics
            (symbol, metric, granularity, date, current_value, percentile, q25, q50, q80, min_val, max_val, avg_val, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                symbol,
                format!("{:?}", metric).to_lowercase(),
                format!("{:?}", granularity).to_lowercase(),
                date.to_string(),
                current_value,
                percentile,
                q25,
                q50,
                q80,
                min,
                max,
                avg,
                source,
            ],
        )?;
        Ok(())
    }

    /// Get valuation statistics for a symbol
    pub async fn get_valuation_statistics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<Option<ValuationStatisticsSet>> {
        let db = self.db.lock().await;

        let date_filter = if let Some(d) = date {
            format!("AND date = '{}'", d.to_string())
        } else {
            String::new()
        };

        // Query PE stats
        let pe_ttm_stats = self.get_statistics_for_metric(
            &db,
            symbol,
            &date_filter,
            ValuationMetricName::PeTtm,
        )?;

        // Query PB stats
        let pb_stats = self.get_statistics_for_metric(
            &db,
            symbol,
            &date_filter,
            ValuationMetricName::Pb,
        )?;

        // Query dividend yield stats
        let dividend_yield_stats = self.get_statistics_for_metric(
            &db,
            symbol,
            &date_filter,
            ValuationMetricName::Dyr,
        )?;

        if pe_ttm_stats.is_empty() && pb_stats.is_empty() && dividend_yield_stats.is_empty() {
            return Ok(None);
        }

        let data_date = if let Some(d) = date {
            d
        } else {
            // Get the latest date from the data
            let latest_date: Option<String> = db.query_row(
                "SELECT MAX(date) FROM valuation_statistics WHERE symbol = ?1",
                params![symbol],
                |row| row.get(0),
            ).ok();
            latest_date
                .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .unwrap_or_else(|| chrono::Local::now().date_naive())
        };

        Ok(Some(ValuationStatisticsSet {
            symbol: symbol.to_string(),
            date: data_date,
            pe_ttm_stats,
            pb_stats,
            dividend_yield_stats,
        }))
    }

    fn get_statistics_for_metric(
        &self,
        db: &Connection,
        symbol: &str,
        date_filter: &str,
        metric: ValuationMetricName,
    ) -> Result<Vec<ValuationStatistics>> {
        let metric_str = format!("{:?}", metric).to_lowercase();

        let sql = format!(
            "SELECT * FROM valuation_statistics WHERE symbol = ?1 AND metric = '{}' {}",
            metric_str, date_filter
        );

        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map(params![symbol], Self::row_to_valuation_statistics)?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(stats) = row {
                results.push(stats);
            }
        }
        Ok(results)
    }

    fn row_to_valuation_statistics(row: &rusqlite::Row) -> rusqlite::Result<ValuationStatistics> {
        let metric_str: String = row.get(2)?;
        let granularity_str: String = row.get(3)?;

        let metric = match metric_str.to_lowercase().as_str() {
            "pe_ttm" => ValuationMetricName::PeTtm,
            "d_pe_ttm" => ValuationMetricName::DPeTtm,
            "pb" => ValuationMetricName::Pb,
            "ps_ttm" => ValuationMetricName::PsTtm,
            "dyr" => ValuationMetricName::Dyr,
            _ => ValuationMetricName::PeTtm,
        };

        let granularity = match granularity_str.to_lowercase().as_str() {
            "fs" => StatisticsGranularity::SinceListing,
            "y20" => StatisticsGranularity::Y20,
            "y10" => StatisticsGranularity::Y10,
            "y5" => StatisticsGranularity::Y5,
            "y3" => StatisticsGranularity::Y3,
            "y1" => StatisticsGranularity::Y1,
            _ => StatisticsGranularity::Y5,
        };

        Ok(ValuationStatistics {
            metric,
            granularity,
            current_value: row.get(5)?,
            percentile: row.get(6)?,
            q25: row.get(7)?,
            q50: row.get(8)?,
            q80: row.get(9)?,
            min: row.get(10)?,
            max: row.get(11)?,
            avg: row.get(12)?,
        })
    }
}

/// Statistics about local storage
#[derive(Debug, Clone)]
pub struct LocalStorageStats {
    pub candle_count: u64,
    pub financial_count: u64,
    pub valuation_count: u64,
    pub macro_count: u64,
    pub cache_count: u64,
    pub unique_symbols: u64,
    pub db_size_bytes: u64,
}

impl std::fmt::Display for LocalStorageStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LocalStorage: {} candles, {} financials, {} valuations, {} macro, {} cache ({} symbols, {:.2} MB)",
            self.candle_count,
            self.financial_count,
            self.valuation_count,
            self.macro_count,
            self.cache_count,
            self.unique_symbols,
            self.db_size_bytes as f64 / 1_048_576.0
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_storage() -> LocalStorage {
        let dir = tempdir().unwrap();
        let config = LocalStorageConfig {
            db_path: dir.path().join("test_financial.db"),
            enabled: true,
            ..Default::default()
        };
        LocalStorage::new(config).unwrap()
    }

    fn create_test_candle(symbol: &str, timestamp: DateTime<Utc>) -> Candle {
        Candle {
            symbol: symbol.to_string(),
            timeframe: Timeframe::Daily,
            timestamp,
            open: 10.0,
            high: 11.0,
            low: 9.5,
            close: 10.5,
            volume: 1000000.0,
            amount: 10500000.0,
        }
    }

    #[tokio::test]
    async fn test_storage_creation() {
        let storage = create_test_storage();
        assert!(storage.is_enabled());
    }

    #[tokio::test]
    async fn test_candle_save_and_retrieve() {
        let storage = create_test_storage();

        let now = Utc::now();
        let candles = vec![
            create_test_candle("000001.SZ", now - chrono::Duration::days(2)),
            create_test_candle("000001.SZ", now - chrono::Duration::days(1)),
            create_test_candle("000001.SZ", now),
        ];

        // Save candles
        let count = storage.save_candles(&candles, "test").await.unwrap();
        assert_eq!(count, 3);

        // Retrieve candles
        let retrieved = storage
            .get_candles("000001.SZ", Timeframe::Daily, None, None, None)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 3);

        // Check has_candles
        let has = storage.has_candles("000001.SZ", Timeframe::Daily).await.unwrap();
        assert!(has);

        // Check non-existent
        let has_other = storage.has_candles("999999.SZ", Timeframe::Daily).await.unwrap();
        assert!(!has_other);
    }

    #[tokio::test]
    async fn test_candle_limit() {
        let storage = create_test_storage();

        let now = Utc::now();
        let candles: Vec<_> = (0..10)
            .map(|i| create_test_candle("000001.SZ", now - chrono::Duration::days(i)))
            .collect();

        storage.save_candles(&candles, "test").await.unwrap();

        let retrieved = storage
            .get_candles("000001.SZ", Timeframe::Daily, None, None, Some(5))
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 5);
    }

    #[tokio::test]
    async fn test_analysis_cache() {
        let storage = create_test_storage();

        #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestResult {
            score: f64,
            passed: bool,
        }

        let result = TestResult {
            score: 85.0,
            passed: true,
        };

        // Save to cache
        storage
            .save_analysis_cache("000001.SZ", "test_analysis", &result, None)
            .await
            .unwrap();

        // Retrieve from cache
        let cached: Option<TestResult> = storage
            .get_analysis_cache("000001.SZ", "test_analysis")
            .await
            .unwrap();
        assert_eq!(cached.unwrap(), result);

        // Invalidate cache
        storage.invalidate_cache("000001.SZ", Some("test_analysis")).await.unwrap();

        // Should be gone
        let cached: Option<TestResult> = storage
            .get_analysis_cache("000001.SZ", "test_analysis")
            .await
            .unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_analysis_cache_expiration() {
        let storage = create_test_storage();

        #[derive(Serialize, serde::Deserialize)]
        struct TestResult {
            value: i32,
        }

        let result = TestResult { value: 42 };

        // Save with past expiration
        let expired = Utc::now() - chrono::Duration::hours(1);
        storage
            .save_analysis_cache("000001.SZ", "expired_test", &result, Some(expired))
            .await
            .unwrap();

        // Should return None due to expiration
        let cached: Option<TestResult> = storage
            .get_analysis_cache("000001.SZ", "expired_test")
            .await
            .unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_macro_indicator() {
        let storage = create_test_storage();

        let date = chrono::Local::now().date_naive();
        storage
            .save_macro_indicator("PMI", date, 51.5, Some(2.3), Some(0.5), "test")
            .await
            .unwrap();

        let result = storage.get_macro_indicator("PMI", Some(date)).await.unwrap();
        assert!(result.is_some());
        let (value, yoy, mom) = result.unwrap();
        assert!((value - 51.5).abs() < 0.001);
        assert!((yoy.unwrap() - 2.3).abs() < 0.001);
        assert!((mom.unwrap() - 0.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_sync_metadata() {
        let storage = create_test_storage();

        storage
            .update_sync_metadata("candles", Some("000001.SZ"), SyncStatus::Success, None, None)
            .await
            .unwrap();

        let meta = storage
            .get_sync_metadata("candles", Some("000001.SZ"))
            .await
            .unwrap();
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().sync_status, SyncStatus::Success);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let storage = create_test_storage();

        // Add some data
        let candles = vec![create_test_candle("000001.SZ", Utc::now())];
        storage.save_candles(&candles, "test").await.unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.candle_count, 1);
        assert_eq!(stats.unique_symbols, 1);
    }
}
