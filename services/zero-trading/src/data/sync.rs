//! Data synchronization module.
//!
//! Provides scheduled sync tasks to keep local storage up-to-date
//! with remote data sources, using incremental updates where possible.

use anyhow::Result;
use chrono::{Duration, Local, NaiveDate, Timelike, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, info, warn};

use super::local_storage::{LocalStorage, SyncStatus};
use super::{DataProviderRouter, Timeframe};

// ============================================================================
// Sync Configuration
// ============================================================================

/// Configuration for data synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Interval between sync runs (in minutes)
    pub sync_interval_minutes: u64,
    /// Whether to sync candles
    pub sync_candles: bool,
    /// Whether to sync macro indicators
    pub sync_macro: bool,
    /// Symbols to sync candles for
    pub candle_symbols: Vec<String>,
    /// Number of days of history to sync for new symbols
    pub initial_history_days: u32,
    /// Workflow API endpoint for macro data
    pub workflow_endpoint: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_minutes: 60,
            sync_candles: true,
            sync_macro: true,
            candle_symbols: vec![
                "000300.SH".to_string(), // CSI 300
                "000905.SH".to_string(), // CSI 500
                "000001.SH".to_string(), // SSE Composite
            ],
            initial_history_days: 365,
            workflow_endpoint: "http://127.0.0.1:4432".to_string(),
        }
    }
}

// ============================================================================
// Data Synchronizer
// ============================================================================

/// Data synchronizer that keeps local storage in sync with remote sources
pub struct DataSynchronizer {
    /// Local storage instance
    storage: Arc<LocalStorage>,
    /// Data provider router for fetching remote data
    router: Arc<DataProviderRouter>,
    /// Sync configuration
    config: SyncConfig,
    /// HTTP client for API calls
    http_client: reqwest::Client,
}

/// Response from workflow API for macro data
#[derive(Debug, Deserialize)]
struct MacroApiResponse {
    success: bool,
    data: Option<MacroData>,
    error: Option<String>,
}

/// Macro economic data from workflow API
#[derive(Debug, Deserialize)]
struct MacroData {
    pmi: Option<f64>,
    m2_yoy: Option<f64>,
    social_financing: Option<f64>,
    cpi_yoy: Option<f64>,
    ppi_yoy: Option<f64>,
    gdp_yoy: Option<f64>,
    industrial_value_added: Option<f64>,
    fixed_asset_investment: Option<f64>,
    retail_sales: Option<f64>,
    export_yoy: Option<f64>,
    import_yoy: Option<f64>,
    lpr_1y: Option<f64>,
    mlf_rate: Option<f64>,
}

impl DataSynchronizer {
    /// Create a new data synchronizer
    pub fn new(
        storage: Arc<LocalStorage>,
        router: Arc<DataProviderRouter>,
        config: SyncConfig,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            storage,
            router,
            config,
            http_client,
        }
    }

    /// Start the synchronization background task
    pub async fn start(&self) -> Result<()> {
        info!(
            interval_minutes = self.config.sync_interval_minutes,
            "Starting data synchronizer"
        );

        // Run initial sync
        self.sync_all().await;

        // Set up periodic sync
        let mut sync_interval =
            interval(TokioDuration::from_secs(self.config.sync_interval_minutes * 60));

        loop {
            sync_interval.tick().await;

            // Only sync during reasonable hours (don't sync at 3am)
            let hour = Local::now().hour();
            if (6..=23).contains(&hour) {
                self.sync_all().await;
            } else {
                debug!(hour, "Skipping sync outside active hours");
            }
        }
    }

    /// Perform a full synchronization
    pub async fn sync_all(&self) {
        info!("Starting full data synchronization");

        if self.config.sync_candles {
            self.sync_candles().await;
        }

        if self.config.sync_macro {
            self.sync_macro_indicators().await;
        }

        // Cleanup expired data
        if let Err(e) = self.cleanup().await {
            warn!(error = %e, "Cleanup failed");
        }

        info!("Data synchronization completed");
    }

    /// Sync candle data for all configured symbols
    async fn sync_candles(&self) {
        for symbol in &self.config.candle_symbols {
            if let Err(e) = self.sync_symbol_candles(symbol).await {
                warn!(
                    symbol,
                    error = %e,
                    "Failed to sync candles"
                );
                // Update sync status
                let _ = self
                    .storage
                    .update_sync_metadata(
                        "candles",
                        Some(symbol),
                        SyncStatus::Failed,
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
            }
        }
    }

    /// Sync daily candles for a single symbol
    async fn sync_symbol_candles(&self, symbol: &str) -> Result<()> {
        let timeframe = Timeframe::Daily;

        // Check when we last synced
        let last_sync = self.storage.get_latest_candle_timestamp(symbol, timeframe).await?;

        // Determine date range
        let (start_date, is_incremental) = match last_sync {
            Some(last_ts) => {
                // Incremental sync: start from last timestamp
                let start = (last_ts + Duration::hours(1)).date_naive();
                (Some(start), true)
            }
            None => {
                // Full sync: fetch history
                let start = Local::now().date_naive() - Duration::days(self.config.initial_history_days as i64);
                (Some(start), false)
            }
        };

        let end_date = Some(Local::now().date_naive());

        // Skip if no new data needed
        if let (Some(start), Some(end)) = (start_date, end_date) {
            if start >= end {
                debug!(symbol, "Candles already up to date");
                return Ok(());
            }
        }

        debug!(
            symbol,
            start = ?start_date,
            end = ?end_date,
            incremental = is_incremental,
            "Syncing daily candles"
        );

        // Fetch from remote using get_daily_candles
        let candles = self
            .router
            .get_daily_candles(symbol, start_date, end_date, None)
            .await?;

        if candles.is_empty() {
            debug!(symbol, "No new candles to sync");
            return Ok(());
        }

        // Save to local storage
        let count = self
            .storage
            .save_candles(&candles, "sync")
            .await?;

        info!(
            symbol,
            count,
            "Synced candles to local storage"
        );

        // Update sync metadata
        let next_sync = Utc::now() + Duration::minutes(self.config.sync_interval_minutes as i64);
        self.storage
            .update_sync_metadata("candles", Some(symbol), SyncStatus::Success, Some(next_sync), None)
            .await?;

        Ok(())
    }

    /// Sync macro economic indicators from workflow API
    async fn sync_macro_indicators(&self) {
        debug!("Syncing macro indicators from workflow API");

        // Update sync status to in-progress
        let _ = self
            .storage
            .update_sync_metadata("macro", None, SyncStatus::InProgress, None, None)
            .await;

        // Fetch from workflow API
        let url = format!("{}/api/v1/economic/china", self.config.workflow_endpoint);
        match self.fetch_and_save_macro_data(&url).await {
            Ok(count) => {
                info!(count, "Synced macro indicators from workflow API");
                let next_sync = Utc::now() + Duration::minutes(self.config.sync_interval_minutes as i64);
                let _ = self
                    .storage
                    .update_sync_metadata("macro", None, SyncStatus::Success, Some(next_sync), None)
                    .await;
            }
            Err(e) => {
                warn!(error = %e, "Failed to sync macro indicators");
                let _ = self
                    .storage
                    .update_sync_metadata("macro", None, SyncStatus::Failed, None, Some(&e.to_string()))
                    .await;
            }
        }

        debug!("Macro indicators sync complete");
    }

    /// Fetch macro data from workflow API and save to local storage
    async fn fetch_and_save_macro_data(&self, url: &str) -> Result<usize> {
        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Workflow API returned status: {}", response.status());
        }

        let api_response: MacroApiResponse = response.json().await?;

        if !api_response.success {
            anyhow::bail!(
                "Workflow API error: {}",
                api_response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let Some(data) = api_response.data else {
            anyhow::bail!("Workflow API returned no data");
        };

        // Save each indicator to local storage
        let today = Local::now().date_naive();
        let source = "workflow_api";
        let mut count = 0;

        if let Some(pmi) = data.pmi {
            self.save_indicator("PMI", today, pmi, source).await?;
            count += 1;
        }
        if let Some(m2) = data.m2_yoy {
            self.save_indicator("M2_YOY", today, m2, source).await?;
            count += 1;
        }
        if let Some(sf) = data.social_financing {
            self.save_indicator("SOCIAL_FINANCING", today, sf, source).await?;
            count += 1;
        }
        if let Some(cpi) = data.cpi_yoy {
            self.save_indicator("CPI_YOY", today, cpi, source).await?;
            count += 1;
        }
        if let Some(ppi) = data.ppi_yoy {
            self.save_indicator("PPI_YOY", today, ppi, source).await?;
            count += 1;
        }
        if let Some(gdp) = data.gdp_yoy {
            self.save_indicator("GDP_YOY", today, gdp, source).await?;
            count += 1;
        }
        if let Some(iva) = data.industrial_value_added {
            self.save_indicator("INDUSTRIAL_VA", today, iva, source).await?;
            count += 1;
        }
        if let Some(fai) = data.fixed_asset_investment {
            self.save_indicator("FAI_YOY", today, fai, source).await?;
            count += 1;
        }
        if let Some(retail) = data.retail_sales {
            self.save_indicator("RETAIL_SALES", today, retail, source).await?;
            count += 1;
        }
        if let Some(exp) = data.export_yoy {
            self.save_indicator("EXPORT_YOY", today, exp, source).await?;
            count += 1;
        }
        if let Some(imp) = data.import_yoy {
            self.save_indicator("IMPORT_YOY", today, imp, source).await?;
            count += 1;
        }
        if let Some(lpr) = data.lpr_1y {
            self.save_indicator("LPR_1Y", today, lpr, source).await?;
            count += 1;
        }
        if let Some(mlf) = data.mlf_rate {
            self.save_indicator("MLF_RATE", today, mlf, source).await?;
            count += 1;
        }

        Ok(count)
    }

    /// Save a single macro indicator to local storage
    async fn save_indicator(&self, code: &str, date: NaiveDate, value: f64, source: &str) -> Result<()> {
        self.storage
            .save_macro_indicator(code, date, value, None, None, source)
            .await
    }

    /// Cleanup old and expired data
    async fn cleanup(&self) -> Result<()> {
        let (candles_deleted, financials_deleted, cache_deleted) =
            self.storage.cleanup_old_data().await?;

        if candles_deleted > 0 || financials_deleted > 0 || cache_deleted > 0 {
            info!(
                candles = candles_deleted,
                financials = financials_deleted,
                cache = cache_deleted,
                "Cleaned up old data"
            );
        }

        Ok(())
    }

    /// Get sync statistics
    pub async fn get_sync_status(&self) -> SyncReport {
        let storage_stats = self.storage.get_stats().await.ok();

        let candle_sync = self
            .storage
            .get_sync_metadata("candles", None)
            .await
            .ok()
            .flatten();

        let macro_sync = self
            .storage
            .get_sync_metadata("macro", None)
            .await
            .ok()
            .flatten();

        SyncReport {
            candle_count: storage_stats.as_ref().map(|s| s.candle_count),
            financial_count: storage_stats.as_ref().map(|s| s.financial_count),
            macro_count: storage_stats.as_ref().map(|s| s.macro_count),
            cache_count: storage_stats.as_ref().map(|s| s.cache_count),
            db_size_bytes: storage_stats.as_ref().map(|s| s.db_size_bytes),
            last_candle_sync: candle_sync.as_ref().map(|s| s.last_sync_at),
            last_macro_sync: macro_sync.as_ref().map(|s| s.last_sync_at),
            candle_sync_status: candle_sync.map(|s| format!("{}", s.sync_status)),
            macro_sync_status: macro_sync.map(|s| format!("{}", s.sync_status)),
        }
    }

    /// Force sync a specific symbol
    pub async fn force_sync_symbol(&self, symbol: &str) -> Result<()> {
        info!(symbol, "Force syncing symbol");
        self.sync_symbol_candles(symbol).await
    }

    /// Add a symbol to the sync list
    pub fn add_symbol(&mut self, symbol: String) {
        if !self.config.candle_symbols.contains(&symbol) {
            self.config.candle_symbols.push(symbol);
        }
    }

    /// Remove a symbol from the sync list
    pub fn remove_symbol(&mut self, symbol: &str) {
        self.config.candle_symbols.retain(|s| s != symbol);
    }
}

// ============================================================================
// Sync Report
// ============================================================================

/// Report on synchronization status
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub candle_count: Option<u64>,
    pub financial_count: Option<u64>,
    pub macro_count: Option<u64>,
    pub cache_count: Option<u64>,
    pub db_size_bytes: Option<u64>,
    pub last_candle_sync: Option<chrono::DateTime<Utc>>,
    pub last_macro_sync: Option<chrono::DateTime<Utc>>,
    pub candle_sync_status: Option<String>,
    pub macro_sync_status: Option<String>,
}

impl std::fmt::Display for SyncReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Sync Status Report ===")?;
        if let Some(count) = self.candle_count {
            writeln!(f, "Candles: {} records", count)?;
        }
        if let Some(count) = self.financial_count {
            writeln!(f, "Financials: {} records", count)?;
        }
        if let Some(count) = self.macro_count {
            writeln!(f, "Macro indicators: {} records", count)?;
        }
        if let Some(count) = self.cache_count {
            writeln!(f, "Cached analyses: {} records", count)?;
        }
        if let Some(size) = self.db_size_bytes {
            writeln!(f, "Database size: {:.2} MB", size as f64 / 1_048_576.0)?;
        }
        if let Some(ref status) = self.candle_sync_status {
            writeln!(f, "Candle sync status: {}", status)?;
        }
        if let Some(ref status) = self.macro_sync_status {
            writeln!(f, "Macro sync status: {}", status)?;
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.sync_interval_minutes, 60);
        assert!(config.sync_candles);
        assert!(config.sync_macro);
        assert!(!config.candle_symbols.is_empty());
    }

    #[test]
    fn test_sync_report_display() {
        let report = SyncReport {
            candle_count: Some(1000),
            financial_count: Some(50),
            macro_count: Some(200),
            cache_count: Some(10),
            db_size_bytes: Some(5_242_880),
            last_candle_sync: Some(Utc::now()),
            last_macro_sync: Some(Utc::now()),
            candle_sync_status: Some("success".to_string()),
            macro_sync_status: Some("success".to_string()),
        };

        let display = format!("{}", report);
        assert!(display.contains("Candles: 1000"));
        assert!(display.contains("5.00 MB"));
    }
}
