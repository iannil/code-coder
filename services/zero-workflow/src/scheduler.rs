//! SQLite-backed cron scheduler for Zero Workflow.
//!
//! Provides persistent cron job storage with:
//! - SQLite database for durability
//! - Standard crontab-like expressions
//! - Execution history tracking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use cron::Schedule;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use zero_common::config::CronTask;

/// Cron scheduler with SQLite persistence.
pub struct Scheduler {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// A scheduled cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub expression: String,
    pub command: String,
    pub description: Option<String>,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_output: Option<String>,
}

/// Task information for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
}

impl Scheduler {
    /// Create a new scheduler with default data directory.
    pub fn new() -> Self {
        let data_dir = zero_common::config::config_dir().join("workflow");
        Self::with_data_dir(data_dir)
    }

    /// Create a new scheduler with a specific data directory.
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        let db_path = data_dir.join("cron.db");

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Open database connection
        let conn = Connection::open(&db_path).expect("Failed to open cron database");

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cron_jobs (
                id          TEXT PRIMARY KEY,
                expression  TEXT NOT NULL,
                command     TEXT NOT NULL,
                description TEXT,
                created_at  TEXT NOT NULL,
                next_run    TEXT NOT NULL,
                last_run    TEXT,
                last_status TEXT,
                last_output TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_cron_jobs_next_run ON cron_jobs(next_run);",
        )
        .expect("Failed to initialize cron schema");

        Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
            shutdown_tx: None,
        }
    }

    /// Add a task to the scheduler.
    pub fn add_task(&self, task: CronTask) -> Result<()> {
        let now = Utc::now();
        let next_run = self.next_run_for(&task.expression, now)?;

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO cron_jobs (id, expression, command, description, created_at, next_run)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task.id,
                task.expression,
                task.command,
                task.description,
                now.to_rfc3339(),
                next_run.to_rfc3339()
            ],
        )
        .context("Failed to insert cron job")?;

        tracing::info!(task_id = %task.id, next_run = %next_run.to_rfc3339(), "Added cron task");
        Ok(())
    }

    /// Remove a task from the scheduler.
    pub fn remove_task(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let changed = conn
            .execute("DELETE FROM cron_jobs WHERE id = ?1", params![id])
            .context("Failed to delete cron job")?;

        if changed > 0 {
            tracing::info!(task_id = %id, "Removed cron task");
        }

        Ok(changed > 0)
    }

    /// List all scheduled tasks.
    pub fn list_tasks(&self) -> Result<Vec<TaskInfo>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, command, description, next_run, last_run, last_status
             FROM cron_jobs ORDER BY next_run ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let next_run_raw: String = row.get(3)?;
            let last_run_raw: Option<String> = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                next_run_raw,
                last_run_raw,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            let (id, command, description, next_run_raw, last_run_raw, last_status) = row?;
            tasks.push(TaskInfo {
                id,
                command,
                description,
                next_run: Self::parse_rfc3339(&next_run_raw)?,
                last_run: match last_run_raw {
                    Some(raw) => Some(Self::parse_rfc3339(&raw)?),
                    None => None,
                },
                last_status,
            });
        }

        Ok(tasks)
    }

    /// Get tasks that are due to run.
    pub fn due_tasks(&self, now: DateTime<Utc>) -> Result<Vec<CronJob>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, expression, command, description, next_run, last_run, last_status, last_output
             FROM cron_jobs WHERE next_run <= ?1 ORDER BY next_run ASC",
        )?;

        let rows = stmt.query_map(params![now.to_rfc3339()], |row| {
            let next_run_raw: String = row.get(4)?;
            let last_run_raw: Option<String> = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                next_run_raw,
                last_run_raw,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            let (id, expression, command, description, next_run_raw, last_run_raw, last_status, last_output) = row?;
            jobs.push(CronJob {
                id,
                expression,
                command,
                description,
                next_run: Self::parse_rfc3339(&next_run_raw)?,
                last_run: match last_run_raw {
                    Some(raw) => Some(Self::parse_rfc3339(&raw)?),
                    None => None,
                },
                last_status,
                last_output,
            });
        }

        Ok(jobs)
    }

    /// Update a job after execution.
    pub fn reschedule_after_run(
        &self,
        job: &CronJob,
        success: bool,
        output: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let next_run = self.next_run_for(&job.expression, now)?;
        let status = if success { "ok" } else { "error" };

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        conn.execute(
            "UPDATE cron_jobs
             SET next_run = ?1, last_run = ?2, last_status = ?3, last_output = ?4
             WHERE id = ?5",
            params![
                next_run.to_rfc3339(),
                now.to_rfc3339(),
                status,
                output,
                job.id
            ],
        )
        .context("Failed to update cron job run state")?;

        tracing::debug!(
            task_id = %job.id,
            status = status,
            next_run = %next_run.to_rfc3339(),
            "Rescheduled cron task after run"
        );

        Ok(())
    }

    /// Start the scheduler loop.
    pub async fn start<F>(&mut self, executor: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let conn = Arc::clone(&self.conn);
        let db_path = self.db_path.clone();
        let executor = Arc::new(executor);

        tokio::spawn(async move {
            // Create a separate scheduler instance for the task
            let scheduler = Scheduler {
                db_path,
                conn,
                shutdown_tx: None,
            };

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Scheduler shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        // Check for tasks to run
                        let now = Utc::now();
                        let due_tasks = match scheduler.due_tasks(now) {
                            Ok(tasks) => tasks,
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to get due tasks");
                                continue;
                            }
                        };

                        // Run tasks
                        for job in due_tasks {
                            tracing::info!(task_id = %job.id, "Running scheduled task");
                            executor(&job.command);

                            // Update next run time
                            if let Err(e) = scheduler.reschedule_after_run(&job, true, "") {
                                tracing::error!(task_id = %job.id, error = %e, "Failed to reschedule task");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the scheduler.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(()).await?;
        }
        Ok(())
    }

    /// Calculate the next run time for a cron expression.
    fn next_run_for(&self, expression: &str, from: DateTime<Utc>) -> Result<DateTime<Utc>> {
        let normalized = Self::normalize_expression(expression)?;
        let schedule = Schedule::from_str(&normalized)
            .with_context(|| format!("Invalid cron expression: {expression}"))?;
        schedule
            .after(&from)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No future occurrence for expression: {expression}"))
    }

    /// Normalize cron expression to 6-field format.
    fn normalize_expression(expression: &str) -> Result<String> {
        let expression = expression.trim();
        let field_count = expression.split_whitespace().count();

        match field_count {
            // Standard crontab syntax: minute hour day month weekday
            5 => Ok(format!("0 {expression}")),
            // Crate-native syntax includes seconds (+ optional year)
            6 | 7 => Ok(expression.to_string()),
            _ => anyhow::bail!(
                "Invalid cron expression: {expression} (expected 5, 6, or 7 fields, got {field_count})"
            ),
        }
    }

    /// Parse an RFC3339 timestamp.
    fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>> {
        let parsed = DateTime::parse_from_rfc3339(raw)
            .with_context(|| format!("Invalid RFC3339 timestamp: {raw}"))?;
        Ok(parsed.with_timezone(&Utc))
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_scheduler(tmp: &TempDir) -> Scheduler {
        Scheduler::with_data_dir(tmp.path().to_path_buf())
    }

    #[test]
    fn test_add_task() {
        let tmp = TempDir::new().unwrap();
        let scheduler = test_scheduler(&tmp);

        let task = CronTask {
            id: "test".into(),
            expression: "*/5 * * * *".into(),
            command: "echo hello".into(),
            description: Some("Test task".into()),
        };

        scheduler.add_task(task).unwrap();
        let tasks = scheduler.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "test");
    }

    #[test]
    fn test_invalid_cron_expression() {
        let tmp = TempDir::new().unwrap();
        let scheduler = test_scheduler(&tmp);

        let task = CronTask {
            id: "bad".into(),
            expression: "invalid cron".into(),
            command: "echo".into(),
            description: None,
        };

        assert!(scheduler.add_task(task).is_err());
    }

    #[test]
    fn test_remove_task() {
        let tmp = TempDir::new().unwrap();
        let scheduler = test_scheduler(&tmp);

        let task = CronTask {
            id: "to-remove".into(),
            expression: "0 0 * * *".into(),
            command: "echo remove me".into(),
            description: None,
        };

        scheduler.add_task(task).unwrap();
        assert!(scheduler.remove_task("to-remove").unwrap());
        assert!(!scheduler.remove_task("nonexistent").unwrap());
    }

    #[test]
    fn test_due_tasks() {
        let tmp = TempDir::new().unwrap();
        let scheduler = test_scheduler(&tmp);

        let task = CronTask {
            id: "due-test".into(),
            expression: "* * * * *".into(), // Every minute
            command: "echo due".into(),
            description: None,
        };

        scheduler.add_task(task).unwrap();

        // Should not be due immediately (next_run is in the future)
        let due_now = scheduler.due_tasks(Utc::now()).unwrap();
        assert!(due_now.is_empty());

        // Should be due in the far future
        let far_future = Utc::now() + chrono::Duration::days(365);
        let due_future = scheduler.due_tasks(far_future).unwrap();
        assert_eq!(due_future.len(), 1);
    }

    #[test]
    fn test_reschedule_after_run() {
        let tmp = TempDir::new().unwrap();
        let scheduler = test_scheduler(&tmp);

        let task = CronTask {
            id: "reschedule-test".into(),
            expression: "*/15 * * * *".into(),
            command: "echo run".into(),
            description: None,
        };

        scheduler.add_task(task).unwrap();

        // Get the job
        let far_future = Utc::now() + chrono::Duration::days(365);
        let jobs = scheduler.due_tasks(far_future).unwrap();
        let job = &jobs[0];

        // Reschedule
        scheduler.reschedule_after_run(job, false, "failed output").unwrap();

        // Verify
        let tasks = scheduler.list_tasks().unwrap();
        assert_eq!(tasks[0].last_status.as_deref(), Some("error"));
        assert!(tasks[0].last_run.is_some());
    }

    #[test]
    fn test_normalize_five_field_expression() {
        let result = Scheduler::normalize_expression("*/5 * * * *").unwrap();
        assert_eq!(result, "0 */5 * * * *");
    }

    #[test]
    fn test_normalize_six_field_expression() {
        let result = Scheduler::normalize_expression("0 */5 * * * *").unwrap();
        assert_eq!(result, "0 */5 * * * *");
    }

    #[test]
    fn test_invalid_field_count() {
        let result = Scheduler::normalize_expression("* * * *");
        assert!(result.is_err());
    }
}
