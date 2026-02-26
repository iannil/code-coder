//! High-frequency data collection scheduler.
//!
//! This module provides cron-based scheduling for high-frequency economic data
//! collection. It runs independently from the trading session scheduler.
//!
//! # Collection Schedule (Beijing Time)
//!
//! - **Daily**: 18:00 on weekdays (after market close)
//! - **Weekly**: 09:00 on Mondays (start of week)
//! - **Monthly**: 10:00 on 1st day of month (after monthly data release)
//!
//! # Configuration
//!
//! ```json
//! {
//!   "trading": {
//!     "high_frequency": {
//!       "enabled": true,
//!       "daily_schedule": "0 0 18 * * 1-5",
//!       "weekly_schedule": "0 0 9 * * 1",
//!       "monthly_schedule": "0 0 10 1 * *"
//!     }
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use super::high_frequency::{CollectionReport, HighFrequencyCollector};

// ============================================================================
// Scheduled Task Types
// ============================================================================

/// High-frequency data collection task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HfScheduledTask {
    /// Collect daily indicators (发电耗煤、螺纹钢价格等)
    DailyCollection,
    /// Collect weekly indicators (高炉开工率、商品房成交等)
    WeeklyCollection,
    /// Collect monthly indicators (PMI等)
    MonthlyCollection,
}

impl HfScheduledTask {
    /// Get task name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Self::DailyCollection => "hf_daily_collection",
            Self::WeeklyCollection => "hf_weekly_collection",
            Self::MonthlyCollection => "hf_monthly_collection",
        }
    }

    /// Get Chinese description
    pub fn description(&self) -> &'static str {
        match self {
            Self::DailyCollection => "日频高频数据采集",
            Self::WeeklyCollection => "周频高频数据采集",
            Self::MonthlyCollection => "月频高频数据采集",
        }
    }
}

// ============================================================================
// Scheduler State
// ============================================================================

/// Scheduler state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HfSchedulerState {
    /// Scheduler not started
    Stopped,
    /// Scheduler running
    Running,
    /// Scheduler paused (e.g., holiday)
    Paused,
}

/// A parsed schedule with its task type
struct ParsedHfSchedule {
    task: HfScheduledTask,
    schedule: Schedule,
}

// ============================================================================
// High-Frequency Data Scheduler
// ============================================================================

/// Scheduler for high-frequency economic data collection
pub struct HighFrequencyScheduler {
    /// Collector instance
    collector: Arc<RwLock<HighFrequencyCollector>>,
    /// Current state
    state: Arc<RwLock<HfSchedulerState>>,
    /// Parsed schedules
    schedules: Vec<ParsedHfSchedule>,
    /// Last execution times for each task
    last_executions: Arc<RwLock<HashMap<HfScheduledTask, DateTime<Utc>>>>,
    /// Collection reports history
    reports: Arc<RwLock<Vec<CollectionReport>>>,
    /// Max reports to keep
    max_reports: usize,
}

impl HighFrequencyScheduler {
    /// Create a new scheduler with the given collector
    pub fn new(collector: HighFrequencyCollector) -> Result<Self> {
        let schedules = Self::parse_schedules(&collector)?;

        Ok(Self {
            collector: Arc::new(RwLock::new(collector)),
            state: Arc::new(RwLock::new(HfSchedulerState::Stopped)),
            schedules,
            last_executions: Arc::new(RwLock::new(HashMap::new())),
            reports: Arc::new(RwLock::new(Vec::new())),
            max_reports: 100,
        })
    }

    /// Parse cron schedules from collector config
    fn parse_schedules(collector: &HighFrequencyCollector) -> Result<Vec<ParsedHfSchedule>> {
        let (daily, weekly, monthly) = collector.get_schedules();
        let mut schedules = Vec::new();

        schedules.push(ParsedHfSchedule {
            task: HfScheduledTask::DailyCollection,
            schedule: Schedule::from_str(daily)
                .with_context(|| format!("Invalid daily schedule cron: {}", daily))?,
        });

        schedules.push(ParsedHfSchedule {
            task: HfScheduledTask::WeeklyCollection,
            schedule: Schedule::from_str(weekly)
                .with_context(|| format!("Invalid weekly schedule cron: {}", weekly))?,
        });

        schedules.push(ParsedHfSchedule {
            task: HfScheduledTask::MonthlyCollection,
            schedule: Schedule::from_str(monthly)
                .with_context(|| format!("Invalid monthly schedule cron: {}", monthly))?,
        });

        info!(
            daily = %daily,
            weekly = %weekly,
            monthly = %monthly,
            "High-frequency scheduler configured"
        );

        Ok(schedules)
    }

    /// Get current scheduler state
    pub async fn get_state(&self) -> HfSchedulerState {
        *self.state.read().await
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = HfSchedulerState::Stopped;
        info!("High-frequency scheduler stopped");
    }

    /// Pause the scheduler (e.g., for holidays)
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        *state = HfSchedulerState::Paused;
        info!("High-frequency scheduler paused");
    }

    /// Resume the scheduler
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        *state = HfSchedulerState::Running;
        info!("High-frequency scheduler resumed");
    }

    /// Run the scheduler loop
    pub async fn run(&self) -> Result<()> {
        let collector = self.collector.read().await;
        if !collector.is_enabled() {
            info!("High-frequency collection disabled, not starting scheduler");
            return Ok(());
        }
        drop(collector);

        {
            let mut state = self.state.write().await;
            *state = HfSchedulerState::Running;
        }

        info!("High-frequency scheduler started");

        // Check every 30 seconds (less frequent than trading scheduler)
        let mut check_interval = interval(Duration::from_secs(30));

        loop {
            check_interval.tick().await;

            let current_state = *self.state.read().await;
            match current_state {
                HfSchedulerState::Stopped => break,
                HfSchedulerState::Paused => continue,
                HfSchedulerState::Running => {
                    if let Err(e) = self.check_and_execute().await {
                        error!(error = %e, "High-frequency scheduler check failed");
                    }
                }
            }
        }

        Ok(())
    }

    /// Check schedules and execute due tasks
    async fn check_and_execute(&self) -> Result<()> {
        let now = Utc::now();

        for parsed in &self.schedules {
            if self.should_execute(&parsed.task, &parsed.schedule, now).await {
                self.execute_task(parsed.task).await;
            }
        }

        Ok(())
    }

    /// Check if a task should be executed
    async fn should_execute(
        &self,
        task: &HfScheduledTask,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> bool {
        let last_exec = {
            let executions = self.last_executions.read().await;
            executions.get(task).copied()
        };

        // Check if there's a scheduled time between last execution and now
        let after = last_exec.unwrap_or_else(|| now - chrono::Duration::hours(1));

        for scheduled in schedule.after(&after).take(10) {
            if scheduled <= now {
                // This scheduled time has passed, check if it's recent (within 2 minutes)
                let since_scheduled = now.signed_duration_since(scheduled);
                if since_scheduled < chrono::Duration::seconds(120) {
                    // Check if we already executed for this schedule
                    if let Some(last) = last_exec {
                        if last >= scheduled {
                            continue; // Already executed
                        }
                    }
                    return true;
                }
            } else {
                // Future scheduled time
                break;
            }
        }

        false
    }

    /// Execute a scheduled task
    async fn execute_task(&self, task: HfScheduledTask) {
        info!(
            task = task.name(),
            description = task.description(),
            "Executing high-frequency collection task"
        );

        // Record execution time
        {
            let mut executions = self.last_executions.write().await;
            executions.insert(task, Utc::now());
        }

        let collector = self.collector.read().await;
        let result = match task {
            HfScheduledTask::DailyCollection => collector.collect_daily().await,
            HfScheduledTask::WeeklyCollection => collector.collect_weekly().await,
            HfScheduledTask::MonthlyCollection => collector.collect_monthly().await,
        };

        match result {
            Ok(report) => {
                info!(
                    task = task.name(),
                    records = report.records_collected,
                    successes = report.success.len(),
                    failures = report.failures.len(),
                    "Collection task completed"
                );

                // Store report
                let mut reports = self.reports.write().await;
                reports.push(report);
                // Keep only recent reports
                if reports.len() > self.max_reports {
                    reports.remove(0);
                }
            }
            Err(e) => {
                error!(
                    task = task.name(),
                    error = %e,
                    "Collection task failed"
                );
            }
        }
    }

    /// Manually trigger a collection task
    pub async fn trigger(&self, task: HfScheduledTask) -> Result<CollectionReport> {
        info!(
            task = task.name(),
            description = task.description(),
            "Manually triggering collection task"
        );

        let collector = self.collector.read().await;
        let report = match task {
            HfScheduledTask::DailyCollection => collector.collect_daily().await?,
            HfScheduledTask::WeeklyCollection => collector.collect_weekly().await?,
            HfScheduledTask::MonthlyCollection => collector.collect_monthly().await?,
        };

        // Store report
        {
            let mut reports = self.reports.write().await;
            reports.push(report.clone());
            if reports.len() > self.max_reports {
                reports.remove(0);
            }
        }

        Ok(report)
    }

    /// Get next scheduled times for each task
    pub fn get_next_schedules(&self) -> Vec<(HfScheduledTask, DateTime<Utc>)> {
        let mut next_times = Vec::new();

        for parsed in &self.schedules {
            if let Some(next) = parsed.schedule.upcoming(Utc).next() {
                next_times.push((parsed.task, next));
            }
        }

        next_times.sort_by_key(|(_, time)| *time);
        next_times
    }

    /// Get recent collection reports
    pub async fn get_reports(&self, limit: usize) -> Vec<CollectionReport> {
        let reports = self.reports.read().await;
        reports.iter().rev().take(limit).cloned().collect()
    }

    /// Get the collector for direct access
    pub fn collector(&self) -> Arc<RwLock<HighFrequencyCollector>> {
        Arc::clone(&self.collector)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::high_frequency::CollectorConfig;
    use tempfile::tempdir;

    fn create_test_collector() -> HighFrequencyCollector {
        let dir = tempdir().unwrap();
        let config = CollectorConfig {
            db_path: dir.into_path().join("test_hf.db"),
            enabled: true,
            daily_schedule: "0 0 18 * * 1-5".to_string(),
            weekly_schedule: "0 0 9 * * 1".to_string(),
            monthly_schedule: "0 0 10 1 * *".to_string(),
        };
        HighFrequencyCollector::new(config).unwrap()
    }

    #[test]
    fn test_hf_scheduled_task_name() {
        assert_eq!(HfScheduledTask::DailyCollection.name(), "hf_daily_collection");
        assert_eq!(HfScheduledTask::WeeklyCollection.name(), "hf_weekly_collection");
        assert_eq!(HfScheduledTask::MonthlyCollection.name(), "hf_monthly_collection");
    }

    #[test]
    fn test_hf_scheduled_task_description() {
        assert!(HfScheduledTask::DailyCollection.description().contains("日频"));
        assert!(HfScheduledTask::WeeklyCollection.description().contains("周频"));
        assert!(HfScheduledTask::MonthlyCollection.description().contains("月频"));
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let collector = create_test_collector();
        let scheduler = HighFrequencyScheduler::new(collector);
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_state() {
        let collector = create_test_collector();
        let scheduler = HighFrequencyScheduler::new(collector).unwrap();

        assert_eq!(scheduler.get_state().await, HfSchedulerState::Stopped);

        scheduler.resume().await;
        assert_eq!(scheduler.get_state().await, HfSchedulerState::Running);

        scheduler.pause().await;
        assert_eq!(scheduler.get_state().await, HfSchedulerState::Paused);

        scheduler.stop().await;
        assert_eq!(scheduler.get_state().await, HfSchedulerState::Stopped);
    }

    #[tokio::test]
    async fn test_get_next_schedules() {
        let collector = create_test_collector();
        let scheduler = HighFrequencyScheduler::new(collector).unwrap();

        let schedules = scheduler.get_next_schedules();
        assert_eq!(schedules.len(), 3);
    }

    #[tokio::test]
    async fn test_manual_trigger() {
        let collector = create_test_collector();
        let scheduler = HighFrequencyScheduler::new(collector).unwrap();

        // Trigger daily collection (should work even with no sources registered)
        let report = scheduler.trigger(HfScheduledTask::DailyCollection).await;
        assert!(report.is_ok());

        let report = report.unwrap();
        assert_eq!(report.collection_type, "daily");
    }
}
