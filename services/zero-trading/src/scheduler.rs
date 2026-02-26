//! Trading session scheduler for automated lifecycle control.
//!
//! This module provides cron-based scheduling for trading sessions,
//! automatically starting, pausing, resuming, and stopping sessions
//! based on A-share market hours.
//!
//! # A-Share Market Hours (Beijing Time)
//!
//! - 09:15-09:25: Pre-market auction
//! - 09:25-09:30: No trading (order matching)
//! - 09:30-11:30: Morning session
//! - 11:30-13:00: Lunch break
//! - 13:00-15:00: Afternoon session
//!
//! # Schedule Configuration
//!
//! ```json
//! {
//!   "trading": {
//!     "schedule": {
//!       "enabled": true,
//!       "session_start": "0 25 9 * * 1-5",
//!       "session_pause": "0 30 11 * * 1-5",
//!       "session_resume": "0 0 13 * * 1-5",
//!       "session_stop": "0 0 15 * * 1-5"
//!     }
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

use crate::session::{SessionConfig, TradingSessionManager};
use crate::r#loop::TradingMode;
use zero_common::config::TradingScheduleConfig;

/// Scheduled task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScheduledTask {
    /// Start trading session
    SessionStart,
    /// Pause trading session (lunch break)
    SessionPause,
    /// Resume trading session
    SessionResume,
    /// Stop trading session (market close)
    SessionStop,
    /// Run daily review/report
    DailyReview,
}

impl ScheduledTask {
    /// Get task name for logging
    pub fn name(&self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::SessionPause => "session_pause",
            Self::SessionResume => "session_resume",
            Self::SessionStop => "session_stop",
            Self::DailyReview => "daily_review",
        }
    }
}

/// Scheduler state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
    /// Scheduler not started
    Stopped,
    /// Scheduler running
    Running,
    /// Scheduler paused (e.g., holiday)
    Paused,
}

/// A parsed schedule with its task type
struct ParsedSchedule {
    task: ScheduledTask,
    schedule: Schedule,
}

/// Trading session scheduler
pub struct TradingScheduler {
    /// Configuration
    config: TradingScheduleConfig,
    /// Session manager
    session_manager: Arc<TradingSessionManager>,
    /// Current state
    state: Arc<RwLock<SchedulerState>>,
    /// Parsed schedules
    schedules: Vec<ParsedSchedule>,
    /// Last execution times for each task
    last_executions: Arc<RwLock<std::collections::HashMap<ScheduledTask, DateTime<Utc>>>>,
    /// Consecutive failure counts for each task
    failure_counts: Arc<RwLock<std::collections::HashMap<ScheduledTask, u32>>>,
    /// Maximum retries for a failed task
    max_retries: u32,
    /// Failure threshold before alerting
    alert_threshold: u32,
}

impl TradingScheduler {
    /// Create a new scheduler
    pub fn new(
        config: TradingScheduleConfig,
        session_manager: Arc<TradingSessionManager>,
    ) -> Result<Self> {
        let mut schedules = Vec::new();

        // Parse all cron expressions
        if config.enabled {
            schedules.push(ParsedSchedule {
                task: ScheduledTask::SessionStart,
                schedule: Schedule::from_str(&config.session_start)
                    .with_context(|| format!("Invalid session_start cron: {}", config.session_start))?,
            });

            schedules.push(ParsedSchedule {
                task: ScheduledTask::SessionPause,
                schedule: Schedule::from_str(&config.session_pause)
                    .with_context(|| format!("Invalid session_pause cron: {}", config.session_pause))?,
            });

            schedules.push(ParsedSchedule {
                task: ScheduledTask::SessionResume,
                schedule: Schedule::from_str(&config.session_resume)
                    .with_context(|| format!("Invalid session_resume cron: {}", config.session_resume))?,
            });

            schedules.push(ParsedSchedule {
                task: ScheduledTask::SessionStop,
                schedule: Schedule::from_str(&config.session_stop)
                    .with_context(|| format!("Invalid session_stop cron: {}", config.session_stop))?,
            });

            schedules.push(ParsedSchedule {
                task: ScheduledTask::DailyReview,
                schedule: Schedule::from_str(&config.daily_review)
                    .with_context(|| format!("Invalid daily_review cron: {}", config.daily_review))?,
            });

            info!(
                start = %config.session_start,
                pause = %config.session_pause,
                resume = %config.session_resume,
                stop = %config.session_stop,
                "Scheduler configured"
            );
        }

        Ok(Self {
            config,
            session_manager,
            state: Arc::new(RwLock::new(SchedulerState::Stopped)),
            schedules,
            last_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            failure_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_retries: 3,
            alert_threshold: 5,
        })
    }

    /// Get current scheduler state
    pub async fn get_state(&self) -> SchedulerState {
        *self.state.read().await
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = SchedulerState::Stopped;
        info!("Scheduler stopped");
    }

    /// Pause the scheduler (e.g., for holidays)
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        *state = SchedulerState::Paused;
        info!("Scheduler paused");
    }

    /// Resume the scheduler
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        *state = SchedulerState::Running;
        info!("Scheduler resumed");
    }

    /// Run the scheduler loop
    pub async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Scheduler disabled, not starting");
            return Ok(());
        }

        {
            let mut state = self.state.write().await;
            *state = SchedulerState::Running;
        }

        info!("Scheduler started");

        // Check every 10 seconds
        let mut check_interval = interval(Duration::from_secs(10));

        loop {
            check_interval.tick().await;

            let current_state = *self.state.read().await;
            match current_state {
                SchedulerState::Stopped => break,
                SchedulerState::Paused => continue,
                SchedulerState::Running => {
                    if let Err(e) = self.check_and_execute().await {
                        error!(error = %e, "Scheduler check failed");
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
        task: &ScheduledTask,
        schedule: &Schedule,
        now: DateTime<Utc>,
    ) -> bool {
        // Get the next scheduled time after last execution (or beginning of time)
        let last_exec = {
            let executions = self.last_executions.read().await;
            executions.get(task).copied()
        };

        // Check if there's a scheduled time between last execution and now
        let after = last_exec.unwrap_or_else(|| now - chrono::Duration::hours(1));

        for scheduled in schedule.after(&after).take(10) {
            if scheduled <= now {
                // This scheduled time has passed, check if it's recent (within last minute)
                let since_scheduled = now.signed_duration_since(scheduled);
                if since_scheduled < chrono::Duration::seconds(60) {
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

    /// Execute a scheduled task with retry logic
    async fn execute_task(&self, task: ScheduledTask) {
        info!(task = task.name(), "Executing scheduled task");

        // Record execution time
        {
            let mut executions = self.last_executions.write().await;
            executions.insert(task, Utc::now());
        }

        // Execute with retries
        let mut last_error = None;
        for attempt in 1..=self.max_retries {
            let result = match task {
                ScheduledTask::SessionStart => self.handle_session_start().await,
                ScheduledTask::SessionPause => self.handle_session_pause().await,
                ScheduledTask::SessionResume => self.handle_session_resume().await,
                ScheduledTask::SessionStop => self.handle_session_stop().await,
                ScheduledTask::DailyReview => self.handle_daily_review().await,
            };

            match result {
                Ok(()) => {
                    // Reset failure count on success
                    let mut failures = self.failure_counts.write().await;
                    failures.insert(task, 0);
                    return;
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        let backoff_ms = 1000 * (1 << (attempt - 1)); // Exponential backoff
                        debug!(
                            task = task.name(),
                            attempt,
                            backoff_ms,
                            "Task failed, retrying..."
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        // All retries failed
        if let Some(e) = last_error {
            error!(
                task = task.name(),
                error = %e,
                max_retries = self.max_retries,
                "Task execution failed after all retries"
            );

            // Track failures and check threshold
            self.track_failure(task).await;
        }
    }

    /// Track task failure and alert if threshold exceeded
    async fn track_failure(&self, task: ScheduledTask) {
        let failure_count = {
            let mut failures = self.failure_counts.write().await;
            let count = failures.entry(task).or_insert(0);
            *count += 1;
            *count
        };

        if failure_count >= self.alert_threshold {
            error!(
                task = task.name(),
                failure_count,
                threshold = self.alert_threshold,
                "ALERT: Task failure threshold exceeded! System may need attention."
            );

            // Reset count after alert to avoid spam
            let mut failures = self.failure_counts.write().await;
            failures.insert(task, 0);
        }
    }

    /// Handle session start
    async fn handle_session_start(&self) -> Result<()> {
        // Check if session already running
        if let Some(session) = self.session_manager.current_session().await? {
            if session.state == crate::session::SessionState::Running {
                debug!("Session already running, skipping start");
                return Ok(());
            }
        }

        // Create session config
        let mode = match self.config.default_mode.as_str() {
            "live" => TradingMode::Live,
            _ => TradingMode::Paper,
        };

        let config = SessionConfig {
            mode,
            auto_start: true,
            name: Some(format!("Scheduled session {}", Local::now().format("%Y-%m-%d"))),
            ..Default::default()
        };

        let session_id = self.session_manager.start_session(Some(config)).await?;
        info!(session_id = %session_id, "Scheduled session started");

        Ok(())
    }

    /// Handle session pause (lunch break)
    async fn handle_session_pause(&self) -> Result<()> {
        if let Some(session) = self.session_manager.current_session().await? {
            if session.state == crate::session::SessionState::Running {
                self.session_manager.pause_session().await?;
                info!("Session paused for lunch break");
            }
        }
        Ok(())
    }

    /// Handle session resume (afternoon)
    async fn handle_session_resume(&self) -> Result<()> {
        if let Some(session) = self.session_manager.current_session().await? {
            if session.state == crate::session::SessionState::Paused {
                self.session_manager.resume_session().await?;
                info!("Session resumed for afternoon trading");
            }
        }
        Ok(())
    }

    /// Handle session stop (market close)
    async fn handle_session_stop(&self) -> Result<()> {
        if self.session_manager.current_session_id().await.is_some() {
            self.session_manager.stop_session().await?;
            info!("Session stopped at market close");
        }
        Ok(())
    }

    /// Handle daily review
    async fn handle_daily_review(&self) -> Result<()> {
        info!("Running daily review...");

        // Get recent sessions
        let sessions = self.session_manager.get_recent_sessions(1).await?;

        if let Some(session) = sessions.first() {
            info!(
                session_id = %session.id,
                state = ?session.state,
                total_pnl = session.total_pnl,
                open_positions = session.open_positions,
                "Daily review: Today's session summary"
            );

            // Get positions for detailed review
            let positions = self.session_manager.get_positions(&session.id).await?;

            let total_trades = positions.len();
            let profitable = positions.iter().filter(|p| p.realized_pnl.unwrap_or(0.0) > 0.0).count();
            let win_rate = if total_trades > 0 {
                (profitable as f64 / total_trades as f64) * 100.0
            } else {
                0.0
            };

            info!(
                total_trades,
                profitable,
                win_rate = format!("{:.1}%", win_rate),
                "Daily review: Trade statistics"
            );
        } else {
            info!("Daily review: No session today");
        }

        Ok(())
    }

    /// Get next scheduled times for each task
    pub fn get_next_schedules(&self) -> Vec<(ScheduledTask, DateTime<Utc>)> {
        let _now = Utc::now(); // Reserved for filtering past schedules
        let mut next_times = Vec::new();

        for parsed in &self.schedules {
            if let Some(next) = parsed.schedule.upcoming(Utc).next() {
                next_times.push((parsed.task, next));
            }
        }

        next_times.sort_by_key(|(_, time)| *time);
        next_times
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduled_task_name() {
        assert_eq!(ScheduledTask::SessionStart.name(), "session_start");
        assert_eq!(ScheduledTask::DailyReview.name(), "daily_review");
    }

    #[test]
    fn test_schedule_config_parsing() {
        let config = TradingScheduleConfig {
            enabled: true,
            session_start: "0 25 9 * * 1-5".to_string(),
            session_pause: "0 30 11 * * 1-5".to_string(),
            session_resume: "0 0 13 * * 1-5".to_string(),
            session_stop: "0 0 15 * * 1-5".to_string(),
            daily_review: "0 30 15 * * 1-5".to_string(),
            auto_start: true,
            persist_state: true,
            default_mode: "paper".to_string(),
        };

        // Verify cron expressions are valid
        assert!(Schedule::from_str(&config.session_start).is_ok());
        assert!(Schedule::from_str(&config.session_pause).is_ok());
        assert!(Schedule::from_str(&config.session_resume).is_ok());
        assert!(Schedule::from_str(&config.session_stop).is_ok());
        assert!(Schedule::from_str(&config.daily_review).is_ok());
    }

    #[test]
    fn test_scheduler_state() {
        assert_ne!(SchedulerState::Running, SchedulerState::Stopped);
        assert_ne!(SchedulerState::Paused, SchedulerState::Running);
    }
}
