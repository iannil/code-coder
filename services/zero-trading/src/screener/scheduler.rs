//! Screener scheduler module.
//!
//! Provides scheduling functionality for:
//! - Daily market scans
//! - Weekly data synchronization
//! - Manual trigger support

use std::sync::Arc;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use zero_common::config::Config;

use crate::data::{DataProvider, LocalStorage};
use super::config::ScreenerConfig;
use super::engine::{ScreenerEngine, ScreenerResult};
use super::report::{ScreenerReport, ReportFormat};

// ============================================================================
// Scheduler State
// ============================================================================

/// State of the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerState {
    /// Scheduler is idle
    Idle,
    /// Running a scan
    Running,
    /// Syncing data
    Syncing,
    /// Stopped
    Stopped,
}

impl std::fmt::Display for SchedulerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Running => write!(f, "running"),
            Self::Syncing => write!(f, "syncing"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

/// Scheduler status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub state: SchedulerState,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub next_scan_at: Option<DateTime<Utc>>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub last_scan_id: Option<String>,
    pub last_scan_stock_count: Option<usize>,
    pub error_message: Option<String>,
}

// ============================================================================
// Scan History
// ============================================================================

/// Record of a completed scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryEntry {
    pub id: String,
    pub completed_at: DateTime<Utc>,
    pub duration_secs: f64,
    pub total_scanned: usize,
    pub passed_count: usize,
    pub config_summary: String,
}

impl From<&ScreenerResult> for ScanHistoryEntry {
    fn from(result: &ScreenerResult) -> Self {
        Self {
            id: result.id.clone(),
            completed_at: result.completed_at,
            duration_secs: result.duration_secs,
            total_scanned: result.total_scanned,
            passed_count: result.stocks.len(),
            config_summary: result.config_summary.clone(),
        }
    }
}

// ============================================================================
// Screener Scheduler
// ============================================================================

/// Scheduler for automated screener operations.
pub struct ScreenerScheduler<P: DataProvider + 'static> {
    config: ScreenerConfig,
    global_config: Arc<Config>,
    provider: Arc<P>,
    storage: Arc<LocalStorage>,
    state: Arc<RwLock<SchedulerState>>,
    status: Arc<RwLock<SchedulerStatus>>,
    history: Arc<RwLock<Vec<ScanHistoryEntry>>>,
    latest_result: Arc<RwLock<Option<ScreenerResult>>>,
}

impl<P: DataProvider + 'static> ScreenerScheduler<P> {
    /// Create a new scheduler.
    pub fn new(
        config: ScreenerConfig,
        global_config: Arc<Config>,
        provider: Arc<P>,
        storage: Arc<LocalStorage>,
    ) -> Self {
        let status = SchedulerStatus {
            state: SchedulerState::Idle,
            last_scan_at: None,
            last_sync_at: None,
            next_scan_at: None,
            next_sync_at: None,
            last_scan_id: None,
            last_scan_stock_count: None,
            error_message: None,
        };

        Self {
            config,
            global_config,
            provider,
            storage,
            state: Arc::new(RwLock::new(SchedulerState::Idle)),
            status: Arc::new(RwLock::new(status)),
            history: Arc::new(RwLock::new(Vec::new())),
            latest_result: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current scheduler status.
    pub async fn status(&self) -> SchedulerStatus {
        self.status.read().await.clone()
    }

    /// Get current state.
    pub async fn state(&self) -> SchedulerState {
        *self.state.read().await
    }

    /// Get scan history.
    pub async fn history(&self, limit: usize) -> Vec<ScanHistoryEntry> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get the latest scan result.
    pub async fn latest_result(&self) -> Option<ScreenerResult> {
        self.latest_result.read().await.clone()
    }

    /// Manually trigger a full scan.
    pub async fn trigger_full_scan(&self) -> Result<ScreenerResult> {
        // Check if already running
        {
            let state = self.state.read().await;
            if *state == SchedulerState::Running {
                return Err(anyhow::anyhow!("A scan is already in progress"));
            }
        }

        // Set state to running
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Running;
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.state = SchedulerState::Running;
            status.error_message = None;
        }

        info!("Triggered manual full scan");

        // Run the scan
        let result = self.run_scan(false).await;

        // Reset state
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Idle;
        }

        match result {
            Ok(scan_result) => {
                // Update status
                {
                    let mut status = self.status.write().await;
                    status.state = SchedulerState::Idle;
                    status.last_scan_at = Some(scan_result.completed_at);
                    status.last_scan_id = Some(scan_result.id.clone());
                    status.last_scan_stock_count = Some(scan_result.stocks.len());
                }

                // Add to history
                {
                    let mut history = self.history.write().await;
                    history.push(ScanHistoryEntry::from(&scan_result));
                    // Keep only last 100 entries
                    if history.len() > 100 {
                        history.remove(0);
                    }
                }

                // Store latest result
                {
                    let mut latest = self.latest_result.write().await;
                    *latest = Some(scan_result.clone());
                }

                Ok(scan_result)
            }
            Err(e) => {
                // Update status with error
                {
                    let mut status = self.status.write().await;
                    status.state = SchedulerState::Idle;
                    status.error_message = Some(e.to_string());
                }
                Err(e)
            }
        }
    }

    /// Manually trigger a quick scan.
    pub async fn trigger_quick_scan(&self) -> Result<ScreenerResult> {
        // Check if already running
        {
            let state = self.state.read().await;
            if *state == SchedulerState::Running {
                return Err(anyhow::anyhow!("A scan is already in progress"));
            }
        }

        // Set state to running
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Running;
        }

        info!("Triggered manual quick scan");

        let result = self.run_scan(true).await;

        // Reset state
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Idle;
        }

        match result {
            Ok(scan_result) => {
                {
                    let mut status = self.status.write().await;
                    status.state = SchedulerState::Idle;
                    status.last_scan_at = Some(scan_result.completed_at);
                    status.last_scan_id = Some(scan_result.id.clone());
                    status.last_scan_stock_count = Some(scan_result.stocks.len());
                }

                {
                    let mut history = self.history.write().await;
                    history.push(ScanHistoryEntry::from(&scan_result));
                }

                {
                    let mut latest = self.latest_result.write().await;
                    *latest = Some(scan_result.clone());
                }

                Ok(scan_result)
            }
            Err(e) => {
                {
                    let mut status = self.status.write().await;
                    status.state = SchedulerState::Idle;
                    status.error_message = Some(e.to_string());
                }
                Err(e)
            }
        }
    }

    /// Manually trigger data sync.
    pub async fn trigger_sync(&self) -> Result<()> {
        // Check if already syncing
        {
            let state = self.state.read().await;
            if *state == SchedulerState::Syncing {
                return Err(anyhow::anyhow!("Sync is already in progress"));
            }
        }

        // Set state to syncing
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Syncing;
        }

        {
            let mut status = self.status.write().await;
            status.state = SchedulerState::Syncing;
        }

        info!("Triggered manual data sync");

        let result = self.run_sync().await;

        // Reset state
        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Idle;
        }

        match result {
            Ok(()) => {
                let mut status = self.status.write().await;
                status.state = SchedulerState::Idle;
                status.last_sync_at = Some(Utc::now());
                Ok(())
            }
            Err(e) => {
                let mut status = self.status.write().await;
                status.state = SchedulerState::Idle;
                status.error_message = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Stop the scheduler.
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = SchedulerState::Stopped;

        let mut status = self.status.write().await;
        status.state = SchedulerState::Stopped;

        info!("Screener scheduler stopped");
    }

    /// Check if the scheduler is running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.read().await;
        *state == SchedulerState::Running
    }

    // ========================================================================
    // Internal Methods
    // ========================================================================

    async fn run_scan(&self, quick: bool) -> Result<ScreenerResult> {
        let engine = ScreenerEngine::new(
            self.config.clone(),
            Arc::clone(&self.provider),
            Arc::clone(&self.storage),
            &self.global_config,
        );

        let result = if quick {
            engine.run_quick_scan().await?
        } else {
            engine.run_full_scan().await?
        };

        // Generate and save reports if configured
        if self.config.output.local_report_enabled {
            self.save_reports(&result).await?;
        }

        Ok(result)
    }

    async fn run_sync(&self) -> Result<()> {
        info!("Starting data sync from provider");

        // Fetch stock list
        let stocks = self.provider.list_all_stocks().await
            .context("Failed to fetch stock list")?;

        info!(count = stocks.len(), "Fetched stock list");

        // Save to local storage
        self.storage.save_stock_list(&stocks, self.provider.name()).await
            .context("Failed to save stock list")?;

        // Fetch financial data in batches
        let symbols: Vec<String> = stocks.iter().map(|s| s.symbol()).collect();
        let chunk_size = 100;
        let mut total_saved = 0;

        for chunk in symbols.chunks(chunk_size) {
            match self.provider.batch_get_financial_data(chunk, None).await {
                Ok(financials) => {
                    let saved = self.storage
                        .save_financial_statements(&financials, self.provider.name())
                        .await
                        .unwrap_or(0);
                    total_saved += saved;
                    debug!(chunk_size = chunk.len(), saved, "Saved financial data batch");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to fetch financial data batch");
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!(total = total_saved, "Data sync complete");
        Ok(())
    }

    async fn save_reports(&self, result: &ScreenerResult) -> Result<()> {
        let report = ScreenerReport::new(result.clone());

        // Expand ~ in path manually
        let report_dir = &self.config.output.report_dir;
        let base_dir = if report_dir.starts_with("~/") {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            home.join(&report_dir[2..])
        } else {
            std::path::PathBuf::from(report_dir)
        };
        let base_path = base_dir;

        for format_str in &self.config.output.report_format {
            let format: ReportFormat = format_str.parse().unwrap_or(ReportFormat::Markdown);
            let extension = match format {
                ReportFormat::Markdown => "md",
                ReportFormat::Json => "json",
                ReportFormat::Telegram => "txt",
            };

            let file_path = base_path.join(format!("{}_{}.{}", result.id, format, extension));

            match report.save_to_file(&file_path, format) {
                Ok(path) => info!(path = %path.display(), format = %format, "Saved report"),
                Err(e) => warn!(error = %e, format = %format, "Failed to save report"),
            }
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
    fn test_scheduler_state_display() {
        assert_eq!(SchedulerState::Idle.to_string(), "idle");
        assert_eq!(SchedulerState::Running.to_string(), "running");
        assert_eq!(SchedulerState::Syncing.to_string(), "syncing");
    }

    #[test]
    fn test_scan_history_entry() {
        let now = Utc::now();
        let entry = ScanHistoryEntry {
            id: "test".to_string(),
            completed_at: now,
            duration_secs: 5.5,
            total_scanned: 4000,
            passed_count: 100,
            config_summary: "test config".to_string(),
        };

        assert_eq!(entry.id, "test");
        assert_eq!(entry.passed_count, 100);
    }
}
