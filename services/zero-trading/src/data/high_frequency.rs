//! High-frequency economic data collection and storage.
//!
//! This module provides infrastructure for collecting, storing, and querying
//! high-frequency economic indicators that can be used to predict and validate
//! official macro data releases.
//!
//! # Data Sources
//! - Wind (发电耗煤、高炉开工率)
//! - Mysteel (钢铁数据)
//! - 中指研究院 (房地产数据)
//! - 各期货交易所 (商品价格)
//!
//! # Storage
//! Uses SQLite for local persistence, following the existing project pattern.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::macro_agent::types::{DataFrequency, HighFrequencyDataPoint, HighFrequencyIndicator};

// ============================================================================
// Database Schema
// ============================================================================

const CREATE_TABLES_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS high_frequency_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    indicator_code TEXT NOT NULL,
    indicator_name TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL,
    frequency TEXT NOT NULL,
    data_date DATE NOT NULL,
    yoy_change REAL,
    period_change REAL,
    source TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(indicator_code, data_date)
);

CREATE INDEX IF NOT EXISTS idx_hf_indicator_date
ON high_frequency_data(indicator_code, data_date DESC);

CREATE INDEX IF NOT EXISTS idx_hf_date
ON high_frequency_data(data_date DESC);

CREATE TABLE IF NOT EXISTS collection_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_type TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    status TEXT NOT NULL,
    records_collected INTEGER DEFAULT 0,
    error_message TEXT,
    UNIQUE(collection_type, started_at)
);
"#;

// ============================================================================
// Data Source Trait
// ============================================================================

/// Trait for high-frequency data sources
#[async_trait::async_trait]
pub trait HighFrequencyDataSource: Send + Sync {
    /// Get the source name
    fn name(&self) -> &'static str;

    /// Get the indicators this source provides
    fn provided_indicators(&self) -> Vec<HighFrequencyIndicator>;

    /// Fetch latest data for a specific indicator
    async fn fetch(&self, indicator: HighFrequencyIndicator) -> Result<Option<HighFrequencyDataPoint>>;

    /// Check if the source is available
    async fn health_check(&self) -> bool;
}

// ============================================================================
// Collector Configuration
// ============================================================================

/// Configuration for the high-frequency data collector
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Path to SQLite database
    pub db_path: PathBuf,
    /// Enable automatic collection
    pub enabled: bool,
    /// Collection schedules (cron expressions)
    pub daily_schedule: String,   // e.g., "0 18 * * 1-5" (weekdays 18:00)
    pub weekly_schedule: String,  // e.g., "0 9 * * 1" (Monday 9:00)
    pub monthly_schedule: String, // e.g., "0 10 1 * *" (1st day 10:00)
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            db_path: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("codecoder")
                .join("high_frequency.db"),
            enabled: true,
            daily_schedule: "0 18 * * 1-5".to_string(),
            weekly_schedule: "0 9 * * 1".to_string(),
            monthly_schedule: "0 10 1 * *".to_string(),
        }
    }
}

// ============================================================================
// Collection Report
// ============================================================================

/// Report from a collection run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionReport {
    /// Collection type (daily/weekly/monthly)
    pub collection_type: String,
    /// Start time
    pub started_at: chrono::DateTime<Utc>,
    /// End time
    pub completed_at: chrono::DateTime<Utc>,
    /// Number of records collected
    pub records_collected: usize,
    /// Indicators successfully fetched
    pub success: Vec<String>,
    /// Indicators that failed
    pub failures: Vec<(String, String)>, // (indicator, error)
}

// ============================================================================
// High-Frequency Data Collector
// ============================================================================

/// Collector for high-frequency economic data
pub struct HighFrequencyCollector {
    config: CollectorConfig,
    db: Arc<RwLock<Connection>>,
    sources: Vec<Box<dyn HighFrequencyDataSource>>,
}

impl HighFrequencyCollector {
    /// Create a new collector with the given configuration
    pub fn new(config: CollectorConfig) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open database connection
        let conn = Connection::open(&config.db_path)
            .context("Failed to open high-frequency database")?;

        // Create tables
        conn.execute_batch(CREATE_TABLES_SQL)
            .context("Failed to create database tables")?;

        Ok(Self {
            config,
            db: Arc::new(RwLock::new(conn)),
            sources: Vec::new(),
        })
    }

    /// Register a data source
    pub fn register_source(&mut self, source: Box<dyn HighFrequencyDataSource>) {
        tracing::info!(
            source = source.name(),
            indicators = ?source.provided_indicators().len(),
            "Registered high-frequency data source"
        );
        self.sources.push(source);
    }

    /// Collect data for all daily indicators
    pub async fn collect_daily(&self) -> Result<CollectionReport> {
        self.collect_by_frequency(DataFrequency::Daily, "daily").await
    }

    /// Collect data for all weekly indicators
    pub async fn collect_weekly(&self) -> Result<CollectionReport> {
        self.collect_by_frequency(DataFrequency::Weekly, "weekly").await
    }

    /// Collect data for all monthly indicators
    pub async fn collect_monthly(&self) -> Result<CollectionReport> {
        self.collect_by_frequency(DataFrequency::Monthly, "monthly").await
    }

    /// Collect data for a specific frequency
    async fn collect_by_frequency(
        &self,
        frequency: DataFrequency,
        collection_type: &str,
    ) -> Result<CollectionReport> {
        let started_at = Utc::now();
        let mut success = Vec::new();
        let mut failures = Vec::new();
        let mut records_collected = 0;

        // Log collection start
        self.log_collection_start(collection_type, &started_at).await?;

        // Collect from all sources
        for source in &self.sources {
            for indicator in source.provided_indicators() {
                if indicator.frequency() != frequency {
                    continue;
                }

                match source.fetch(indicator).await {
                    Ok(Some(data_point)) => {
                        if let Err(e) = self.store_data_point(&data_point).await {
                            failures.push((indicator.chinese_name().to_string(), e.to_string()));
                        } else {
                            success.push(indicator.chinese_name().to_string());
                            records_collected += 1;
                        }
                    }
                    Ok(None) => {
                        // No data available for this indicator today
                        tracing::debug!(
                            indicator = indicator.chinese_name(),
                            "No data available"
                        );
                    }
                    Err(e) => {
                        failures.push((indicator.chinese_name().to_string(), e.to_string()));
                    }
                }
            }
        }

        let completed_at = Utc::now();

        // Log collection completion
        self.log_collection_complete(
            collection_type,
            &started_at,
            &completed_at,
            records_collected,
            if failures.is_empty() { None } else { Some(&failures) },
        )
        .await?;

        Ok(CollectionReport {
            collection_type: collection_type.to_string(),
            started_at,
            completed_at,
            records_collected,
            success,
            failures,
        })
    }

    /// Store a single data point
    async fn store_data_point(&self, data: &HighFrequencyDataPoint) -> Result<()> {
        let db = self.db.read().await;

        let indicator_code = format!("{:?}", data.indicator);

        db.execute(
            r#"
            INSERT OR REPLACE INTO high_frequency_data
            (indicator_code, indicator_name, value, unit, frequency, data_date,
             yoy_change, period_change, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                indicator_code,
                data.indicator.chinese_name(),
                data.value,
                data.unit,
                format!("{}", data.indicator.frequency()),
                data.data_date.to_string(),
                data.yoy_change,
                data.period_change,
                data.source,
            ],
        )
        .context("Failed to store data point")?;

        Ok(())
    }

    /// Get latest value for an indicator
    pub async fn get_latest(&self, indicator: HighFrequencyIndicator) -> Result<Option<HighFrequencyDataPoint>> {
        let db = self.db.read().await;
        let indicator_code = format!("{:?}", indicator);

        let mut stmt = db.prepare(
            r#"
            SELECT value, unit, data_date, yoy_change, period_change, source
            FROM high_frequency_data
            WHERE indicator_code = ?1
            ORDER BY data_date DESC
            LIMIT 1
            "#,
        )?;

        let result = stmt.query_row(params![indicator_code], |row| {
            Ok(HighFrequencyDataPoint {
                indicator,
                value: row.get(0)?,
                unit: row.get(1)?,
                data_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Local::now().date_naive()),
                yoy_change: row.get(3)?,
                period_change: row.get(4)?,
                source: row.get(5)?,
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get historical values for an indicator
    pub async fn get_history(
        &self,
        indicator: HighFrequencyIndicator,
        days: u32,
    ) -> Result<Vec<HighFrequencyDataPoint>> {
        let db = self.db.read().await;
        let indicator_code = format!("{:?}", indicator);

        let mut stmt = db.prepare(
            r#"
            SELECT value, unit, data_date, yoy_change, period_change, source
            FROM high_frequency_data
            WHERE indicator_code = ?1
            ORDER BY data_date DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![indicator_code, days], |row| {
            Ok(HighFrequencyDataPoint {
                indicator,
                value: row.get(0)?,
                unit: row.get(1)?,
                data_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Local::now().date_naive()),
                yoy_change: row.get(3)?,
                period_change: row.get(4)?,
                source: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get all latest data points for building evidence chains
    pub async fn get_all_latest(&self) -> Result<Vec<HighFrequencyDataPoint>> {
        let db = self.db.read().await;

        let mut stmt = db.prepare(
            r#"
            SELECT indicator_code, value, unit, data_date, yoy_change, period_change, source
            FROM high_frequency_data hf1
            WHERE data_date = (
                SELECT MAX(data_date)
                FROM high_frequency_data hf2
                WHERE hf2.indicator_code = hf1.indicator_code
            )
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let code: String = row.get(0)?;
            Ok((code, row.get(1)?, row.get(2)?, row.get::<_, String>(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (code, value, unit, date_str, yoy, period, source): (String, f64, String, String, Option<f64>, Option<f64>, String) = row?;

            // Parse indicator from code
            if let Some(indicator) = Self::parse_indicator_code(&code) {
                results.push(HighFrequencyDataPoint {
                    indicator,
                    value,
                    unit,
                    data_date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                        .unwrap_or_else(|_| chrono::Local::now().date_naive()),
                    yoy_change: yoy,
                    period_change: period,
                    source,
                });
            }
        }
        Ok(results)
    }

    /// Parse indicator code back to enum
    fn parse_indicator_code(code: &str) -> Option<HighFrequencyIndicator> {
        match code {
            "PowerCoalConsumption" => Some(HighFrequencyIndicator::PowerCoalConsumption),
            "BlastFurnaceRate" => Some(HighFrequencyIndicator::BlastFurnaceRate),
            "TruckTireRate" => Some(HighFrequencyIndicator::TruckTireRate),
            "PassengerTireRate" => Some(HighFrequencyIndicator::PassengerTireRate),
            "PtaLoadRate" => Some(HighFrequencyIndicator::PtaLoadRate),
            "RebarPrice" => Some(HighFrequencyIndicator::RebarPrice),
            "CementPriceIndex" => Some(HighFrequencyIndicator::CementPriceIndex),
            "ThermalCoalPrice" => Some(HighFrequencyIndicator::ThermalCoalPrice),
            "ExcavatorSales" => Some(HighFrequencyIndicator::ExcavatorSales),
            "AsphaltProductionRate" => Some(HighFrequencyIndicator::AsphaltProductionRate),
            "LandTransaction100City" => Some(HighFrequencyIndicator::LandTransaction100City),
            "HouseSales30City" => Some(HighFrequencyIndicator::HouseSales30City),
            "LandPremiumRate" => Some(HighFrequencyIndicator::LandPremiumRate),
            "CityTrafficIndex" => Some(HighFrequencyIndicator::CityTrafficIndex),
            "MetroPassengers" => Some(HighFrequencyIndicator::MetroPassengers),
            "TruckFreightIndex" => Some(HighFrequencyIndicator::TruckFreightIndex),
            "ExpressDeliveryVolume" => Some(HighFrequencyIndicator::ExpressDeliveryVolume),
            "BoxOffice" => Some(HighFrequencyIndicator::BoxOffice),
            "AgriPrice200Index" => Some(HighFrequencyIndicator::AgriPrice200Index),
            "PorkPrice" => Some(HighFrequencyIndicator::PorkPrice),
            "CcfiIndex" => Some(HighFrequencyIndicator::CcfiIndex),
            "BdiIndex" => Some(HighFrequencyIndicator::BdiIndex),
            "PmiOfficial" => Some(HighFrequencyIndicator::PmiOfficial),
            "PmiCaixin" => Some(HighFrequencyIndicator::PmiCaixin),
            _ => None,
        }
    }

    /// Log collection start
    async fn log_collection_start(
        &self,
        collection_type: &str,
        started_at: &chrono::DateTime<Utc>,
    ) -> Result<()> {
        let db = self.db.read().await;
        db.execute(
            r#"
            INSERT INTO collection_log (collection_type, started_at, status)
            VALUES (?1, ?2, 'running')
            "#,
            params![collection_type, started_at.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Log collection completion
    async fn log_collection_complete(
        &self,
        collection_type: &str,
        started_at: &chrono::DateTime<Utc>,
        completed_at: &chrono::DateTime<Utc>,
        records_collected: usize,
        failures: Option<&[(String, String)]>,
    ) -> Result<()> {
        let db = self.db.read().await;

        let status = if failures.map(|f| f.is_empty()).unwrap_or(true) {
            "success"
        } else {
            "partial"
        };

        let error_msg = failures.map(|f| {
            f.iter()
                .map(|(ind, err)| format!("{}: {}", ind, err))
                .collect::<Vec<_>>()
                .join("; ")
        });

        db.execute(
            r#"
            UPDATE collection_log
            SET completed_at = ?1, status = ?2, records_collected = ?3, error_message = ?4
            WHERE collection_type = ?5 AND started_at = ?6
            "#,
            params![
                completed_at.to_rfc3339(),
                status,
                records_collected as i64,
                error_msg,
                collection_type,
                started_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Check if collection is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get collection schedules
    pub fn get_schedules(&self) -> (&str, &str, &str) {
        (
            &self.config.daily_schedule,
            &self.config.weekly_schedule,
            &self.config.monthly_schedule,
        )
    }
}

// ============================================================================
// Mock Data Source (for testing)
// ============================================================================

/// Mock data source for testing
#[derive(Default)]
pub struct MockDataSource;

#[async_trait::async_trait]
impl HighFrequencyDataSource for MockDataSource {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn provided_indicators(&self) -> Vec<HighFrequencyIndicator> {
        vec![
            HighFrequencyIndicator::PowerCoalConsumption,
            HighFrequencyIndicator::BlastFurnaceRate,
            HighFrequencyIndicator::RebarPrice,
        ]
    }

    async fn fetch(&self, indicator: HighFrequencyIndicator) -> Result<Option<HighFrequencyDataPoint>> {
        // Return mock data
        Ok(Some(HighFrequencyDataPoint {
            indicator,
            value: match indicator {
                HighFrequencyIndicator::PowerCoalConsumption => 75.5,
                HighFrequencyIndicator::BlastFurnaceRate => 82.3,
                HighFrequencyIndicator::RebarPrice => 3850.0,
                _ => 0.0,
            },
            unit: match indicator {
                HighFrequencyIndicator::PowerCoalConsumption => "万吨".to_string(),
                HighFrequencyIndicator::BlastFurnaceRate => "%".to_string(),
                HighFrequencyIndicator::RebarPrice => "元/吨".to_string(),
                _ => "".to_string(),
            },
            data_date: chrono::Local::now().date_naive(),
            yoy_change: Some(5.2),
            period_change: Some(1.3),
            source: "Mock".to_string(),
        }))
    }

    async fn health_check(&self) -> bool {
        true
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_collector_creation() {
        let dir = tempdir().unwrap();
        let config = CollectorConfig {
            db_path: dir.path().join("test_hf.db"),
            ..Default::default()
        };

        let collector = HighFrequencyCollector::new(config);
        assert!(collector.is_ok());
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let config = CollectorConfig {
            db_path: dir.path().join("test_hf.db"),
            ..Default::default()
        };

        let collector = HighFrequencyCollector::new(config).unwrap();

        let data_point = HighFrequencyDataPoint {
            indicator: HighFrequencyIndicator::BlastFurnaceRate,
            value: 82.5,
            unit: "%".to_string(),
            data_date: chrono::Local::now().date_naive(),
            yoy_change: Some(3.2),
            period_change: Some(0.5),
            source: "Test".to_string(),
        };

        collector.store_data_point(&data_point).await.unwrap();

        let retrieved = collector
            .get_latest(HighFrequencyIndicator::BlastFurnaceRate)
            .await
            .unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert!((retrieved.value - 82.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_mock_collection() {
        let dir = tempdir().unwrap();
        let config = CollectorConfig {
            db_path: dir.path().join("test_hf.db"),
            ..Default::default()
        };

        let mut collector = HighFrequencyCollector::new(config).unwrap();
        collector.register_source(Box::new(MockDataSource));

        let report = collector.collect_daily().await.unwrap();
        assert!(!report.success.is_empty());
    }

    #[test]
    fn test_parse_indicator_code() {
        assert_eq!(
            HighFrequencyCollector::parse_indicator_code("BlastFurnaceRate"),
            Some(HighFrequencyIndicator::BlastFurnaceRate)
        );
        assert_eq!(
            HighFrequencyCollector::parse_indicator_code("Unknown"),
            None
        );
    }
}
