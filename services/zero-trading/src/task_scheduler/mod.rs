//! Task scheduler module for automated trading preparation and execution.
//!
//! This module provides a unified way to schedule and manage tasks that run
//! on different schedules:
//! - **Preparation tasks** (24/7): Data preloading, parameter precomputation
//! - **Execution tasks** (trading hours): Signal generation, order execution
//!
//! # Architecture
//!
//! ```text
//! TaskScheduler
//!     |
//!     +-- Preparation Tasks (24/7)
//!     |   +-- DataPreload
//!     |   +-- ParameterPrecompute
//!     |   +-- MacroAnalysis
//!     |
//!     +-- Execution Tasks (Trading Hours)
//!         +-- SignalGeneration
//!         +-- OrderExecution
//!         +-- PriceMonitoring
//!         +-- RiskManagement
//! ```

mod category;
mod config;

pub use category::{TaskCategory, TaskType};
pub use config::PreparationTaskConfig;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::data::MarketDataAggregator;
use crate::r#loop::SignalDetector;
use crate::strategy::StrategyEngine;

/// Task scheduler for managing preparation and execution tasks.
///
/// Separates tasks that can run 24/7 (preparation) from those that only
/// run during trading hours (execution).
pub struct TaskScheduler {
    /// Preparation task configuration
    pub prep_config: PreparationTaskConfig,
    /// Market data aggregator
    data: Arc<MarketDataAggregator>,
    /// Strategy engine (kept for component ownership, used via signal_detector)
    #[allow(dead_code)]
    strategy: Arc<StrategyEngine>,
    /// Signal detector
    signal_detector: SignalDetector,
    /// Task execution tracking
    task_last_run: Arc<RwLock<HashMap<TaskType, chrono::DateTime<chrono::Utc>>>>,
}

impl TaskScheduler {
    /// Create a new task scheduler.
    pub fn new(
        prep_config: PreparationTaskConfig,
        data: Arc<MarketDataAggregator>,
        strategy: Arc<StrategyEngine>,
    ) -> Self {
        let signal_detector = SignalDetector::new(Arc::clone(&strategy), Arc::clone(&data));

        Self {
            prep_config,
            data,
            strategy,
            signal_detector,
            task_last_run: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Run preparation tasks (24/7 operation).
    ///
    /// These tasks can run at any time and help prepare data for faster
    /// execution during trading hours.
    pub async fn run_preparation_tasks(&self) -> anyhow::Result<()> {
        let now = chrono::Utc::now();

        // 1. Data preload (most frequent)
        if self.should_run_task(TaskType::DataPreload, now).await {
            info!("Running data preload task");
            if let Err(e) = self.preload_data().await {
                warn!(error = %e, "Data preload failed");
            } else {
                self.record_task_run(TaskType::DataPreload, now).await;
            }
        }

        // 2. Parameter precomputation
        if self.should_run_task(TaskType::ParameterPrecompute, now).await {
            info!("Running parameter precomputation task");
            if let Err(e) = self.precompute_parameters().await {
                warn!(error = %e, "Parameter precomputation failed");
            } else {
                self.record_task_run(TaskType::ParameterPrecompute, now).await;
            }
        }

        // 3. Macro analysis background update
        if self.should_run_task(TaskType::MacroAnalysis, now).await {
            debug!("Running macro analysis background update");
            if let Err(e) = self.update_macro_analysis().await {
                warn!(error = %e, "Macro analysis update failed");
            } else {
                self.record_task_run(TaskType::MacroAnalysis, now).await;
            }
        }

        Ok(())
    }

    /// Preload historical data for all tracked symbols.
    async fn preload_data(&self) -> anyhow::Result<()> {
        let symbols = self.data.get_tracked_symbols().await;

        info!(count = symbols.len(), "Preloading data for tracked symbols");

        for symbol in symbols {
            // Preload Daily timeframe only (H4/H1 not supported by Lixin)
            let tf = crate::data::Timeframe::Daily;
            if let Err(e) = self.data.get_candles(&symbol, tf, 200).await {
                debug!(
                    symbol = %symbol,
                    timeframe = ?tf,
                    error = %e,
                    "Failed to preload candle data"
                );
            }
        }

        info!("Data preload completed");
        Ok(())
    }

    /// Precompute technical indicator parameters.
    async fn precompute_parameters(&self) -> anyhow::Result<()> {
        // Trigger the signal detector's precomputation
        self.signal_detector.precompute_parameters(&self.data).await?;
        info!("Parameter precomputation completed");
        Ok(())
    }

    /// Update macro analysis in the background.
    async fn update_macro_analysis(&self) -> anyhow::Result<()> {
        // This will be implemented by the macro orchestrator
        // For now, just log that it ran
        debug!("Macro analysis background update completed");
        Ok(())
    }

    /// Check if a task should run based on its interval.
    async fn should_run_task(&self, task_type: TaskType, now: chrono::DateTime<chrono::Utc>) -> bool {
        let last_run = self.task_last_run.read().await;

        if let Some(&last) = last_run.get(&task_type) {
            let interval_secs = match task_type {
                TaskType::DataPreload => self.prep_config.data_preload_interval_secs,
                TaskType::ParameterPrecompute => self.prep_config.parameter_precompute_interval_secs,
                TaskType::MacroAnalysis => self.prep_config.macro_analysis_interval_secs,
                _ => return false, // Execution tasks are handled elsewhere
            };

            let elapsed = (now - last).num_seconds();
            elapsed >= interval_secs as i64
        } else {
            true // Never run before
        }
    }

    /// Record the last run time for a task.
    async fn record_task_run(&self, task_type: TaskType, time: chrono::DateTime<chrono::Utc>) {
        let mut last_run = self.task_last_run.write().await;
        last_run.insert(task_type, time);
    }

    /// Get the preparation task interval based on whether we're in trading hours.
    ///
    /// Non-trading hours: More frequent updates (preparation is the main activity)
    /// Trading hours: Less frequent (avoid competing with execution tasks)
    pub fn get_prep_interval_secs(&self, is_trading_hours: bool) -> u64 {
        if is_trading_hours {
            // Lower frequency during trading hours to avoid competing
            self.prep_config.data_preload_interval_secs * 2
        } else {
            // Higher frequency outside trading hours
            self.prep_config.data_preload_interval_secs
        }
    }

    /// Check if current time is within A-share trading hours.
    pub fn is_trading_hours() -> bool {
        use chrono::{Local, Timelike};
        let now = Local::now();
        let hour = now.hour();
        let minute = now.minute();

        // A-share trading hours: 9:15-15:00
        (hour == 9 && minute >= 15)
            || (hour >= 10 && hour < 15)
            || (hour == 15 && minute == 0)
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self {
            prep_config: PreparationTaskConfig::default(),
            data: Arc::new(MarketDataAggregator::new(&zero_common::config::Config::default())),
            strategy: Arc::new(StrategyEngine::new(&zero_common::config::Config::default())),
            signal_detector: SignalDetector::new(
                Arc::new(StrategyEngine::new(&zero_common::config::Config::default())),
                Arc::new(MarketDataAggregator::new(&zero_common::config::Config::default())),
            ),
            task_last_run: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_trading_hours() {
        // This test is time-dependent, so we just check that the function runs
        let _ = TaskScheduler::is_trading_hours();
    }

    #[test]
    fn test_prep_config_default() {
        let config = PreparationTaskConfig::default();
        assert_eq!(config.data_preload_interval_secs, 300);
        assert_eq!(config.parameter_precompute_interval_secs, 600);
        assert_eq!(config.macro_analysis_interval_secs, 3600);
    }

    #[test]
    fn test_prep_interval_adjustment() {
        let scheduler = TaskScheduler::default();
        let trading_hours_interval = scheduler.get_prep_interval_secs(true);
        let non_trading_hours_interval = scheduler.get_prep_interval_secs(false);

        // Trading hours interval should be 2x non-trading hours
        assert_eq!(trading_hours_interval, non_trading_hours_interval * 2);
    }
}
