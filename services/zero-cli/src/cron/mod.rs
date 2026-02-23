//! Cron command handlers for Zero CLI.
//!
//! This module provides CLI commands that interact with the zero-workflow
//! cron API for managing scheduled tasks.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HTTP client for zero-workflow cron API.
pub struct CronClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

/// Cron job information returned from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub next_run: String,
    pub last_run: Option<String>,
    pub last_status: Option<String>,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Request to create a new cron task.
#[derive(Debug, Serialize)]
struct CreateTaskRequest {
    id: String,
    expression: String,
    command: String,
    description: Option<String>,
}

impl CronClient {
    /// Create a new cron client with the given workflow service URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// List all scheduled tasks.
    pub fn list_tasks(&self) -> Result<Vec<CronJob>> {
        let url = format!("{}/api/v1/tasks", self.base_url);
        let resp: ApiResponse<Vec<CronJob>> = self
            .client
            .get(&url)
            .send()
            .context("Failed to connect to workflow service")?
            .json()
            .context("Failed to parse response")?;

        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Unknown error".into()))
        }
    }

    /// Add a new cron task.
    pub fn add_task(&self, expression: &str, command: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let url = format!("{}/api/v1/tasks", self.base_url);

        let req = CreateTaskRequest {
            id: id.clone(),
            expression: expression.to_string(),
            command: command.to_string(),
            description: None,
        };

        let resp: ApiResponse<String> = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .context("Failed to connect to workflow service")?
            .json()
            .context("Failed to parse response")?;

        if resp.success {
            Ok(id)
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to add task".into()))
        }
    }

    /// Remove a cron task by ID.
    pub fn remove_task(&self, id: &str) -> Result<bool> {
        let url = format!("{}/api/v1/tasks/{}", self.base_url, id);

        let resp: ApiResponse<bool> = self
            .client
            .delete(&url)
            .send()
            .context("Failed to connect to workflow service")?
            .json()
            .context("Failed to parse response")?;

        if resp.success {
            Ok(resp.data.unwrap_or(false))
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to remove task".into()))
        }
    }
}

/// Handle cron CLI commands.
#[allow(clippy::needless_pass_by_value)]
pub fn handle_command(command: crate::CronCommands, config: &crate::config::Config) -> Result<()> {
    // Get workflow service endpoint from config
    let workflow_endpoint = format!(
        "http://{}:{}",
        config.workflow_host.as_deref().unwrap_or("127.0.0.1"),
        config.workflow_port.unwrap_or(4412)
    );

    let client = CronClient::new(&workflow_endpoint);

    match command {
        crate::CronCommands::List => {
            let jobs = client.list_tasks().map_err(|e| {
                anyhow::anyhow!(
                    "Failed to list tasks: {}. Is the workflow service running at {}?",
                    e,
                    workflow_endpoint
                )
            })?;

            if jobs.is_empty() {
                println!("No scheduled tasks yet.");
                println!("\nUsage:");
                println!("  zero-bot cron add '0 9 * * *' 'echo Good morning!'");
                return Ok(());
            }

            println!("🕒 Scheduled jobs ({}):", jobs.len());
            for job in jobs {
                let last_run = job.last_run.as_deref().unwrap_or("never");
                let last_status = job.last_status.as_deref().unwrap_or("n/a");
                println!(
                    "- {} | next={} | last={} ({})\n    cmd: {}",
                    job.id,
                    job.next_run,
                    last_run,
                    last_status,
                    job.command
                );
            }
            Ok(())
        }
        crate::CronCommands::Add {
            expression,
            command,
        } => {
            let id = client.add_task(&expression, &command).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to add task: {}. Is the workflow service running at {}?",
                    e,
                    workflow_endpoint
                )
            })?;

            println!("✅ Added cron job {}", id);
            println!("  Expr: {}", expression);
            println!("  Cmd : {}", command);
            Ok(())
        }
        crate::CronCommands::Remove { id } => {
            let removed = client.remove_task(&id).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to remove task: {}. Is the workflow service running at {}?",
                    e,
                    workflow_endpoint
                )
            })?;

            if removed {
                println!("✅ Removed cron job {}", id);
            } else {
                anyhow::bail!("Cron job '{}' not found", id);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_client_creation() {
        let client = CronClient::new("http://localhost:4412");
        assert_eq!(client.base_url, "http://localhost:4412");
    }

    #[test]
    fn test_cron_client_trailing_slash() {
        let client = CronClient::new("http://localhost:4412/");
        assert_eq!(client.base_url, "http://localhost:4412");
    }
}
