//! Cron scheduler for Zero Workflow.

use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use zero_common::config::CronTask;

/// Cron scheduler.
pub struct Scheduler {
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// A scheduled task with its schedule and next run time.
struct ScheduledTask {
    id: String,
    schedule: Schedule,
    command: String,
    description: Option<String>,
    next_run: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
}

impl Scheduler {
    /// Create a new scheduler.
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Add a task to the scheduler.
    pub fn add_task(&self, task: CronTask) -> Result<()> {
        let schedule = Schedule::from_str(&task.expression)
            .map_err(|e| anyhow::anyhow!("Invalid cron expression: {}", e))?;

        let next_run = schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No upcoming runs for cron expression"))?;

        let scheduled_task = ScheduledTask {
            id: task.id.clone(),
            schedule,
            command: task.command,
            description: task.description,
            next_run,
            last_run: None,
        };

        self.tasks
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .insert(task.id, scheduled_task);

        Ok(())
    }

    /// Remove a task from the scheduler.
    pub fn remove_task(&self, id: &str) -> Result<bool> {
        let removed = self
            .tasks
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .remove(id)
            .is_some();
        Ok(removed)
    }

    /// List all scheduled tasks.
    pub fn list_tasks(&self) -> Result<Vec<TaskInfo>> {
        let tasks = self.tasks.read().map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(tasks
            .values()
            .map(|t| TaskInfo {
                id: t.id.clone(),
                command: t.command.clone(),
                description: t.description.clone(),
                next_run: t.next_run,
                last_run: t.last_run,
            })
            .collect())
    }

    /// Start the scheduler loop.
    pub async fn start<F>(&mut self, executor: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let tasks = Arc::clone(&self.tasks);
        let executor = Arc::new(executor);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Scheduler shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        // Check for tasks to run
                        let now = Utc::now();
                        let mut tasks_to_run = Vec::new();

                        {
                            let tasks = tasks.read().unwrap();
                            for task in tasks.values() {
                                if task.next_run <= now {
                                    tasks_to_run.push((task.id.clone(), task.command.clone()));
                                }
                            }
                        }

                        // Run tasks
                        for (id, command) in tasks_to_run {
                            tracing::info!(task_id = %id, "Running scheduled task");
                            executor(&command);

                            // Update next run time
                            if let Ok(mut tasks) = tasks.write() {
                                if let Some(task) = tasks.get_mut(&id) {
                                    task.last_run = Some(now);
                                    if let Some(next) = task.schedule.upcoming(Utc).next() {
                                        task.next_run = next;
                                    }
                                }
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
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Task information for display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_add_task() {
        let scheduler = Scheduler::new();
        let task = CronTask {
            id: "test".into(),
            expression: "0 0 * * * *".into(), // Every hour (6-field format)
            command: "echo hello".into(),
            description: Some("Test task".into()),
        };

        scheduler.add_task(task).unwrap();
        let tasks = scheduler.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "test");
    }

    #[test]
    fn test_scheduler_invalid_cron() {
        let scheduler = Scheduler::new();
        let task = CronTask {
            id: "bad".into(),
            expression: "invalid cron".into(),
            command: "echo".into(),
            description: None,
        };

        assert!(scheduler.add_task(task).is_err());
    }
}
