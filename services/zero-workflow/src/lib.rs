//! Zero Workflow - Workflow engine for the Zero ecosystem.
//!
//! This crate provides:
//! - Webhook triggers (Git, custom)
//! - Cron scheduler
//! - Workflow orchestration
//! - HTTP API for management
//! - Automated code review via CodeCoder

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod dsl;
pub mod github;
pub mod gitlab;
pub mod review_bridge;
pub mod routes;
pub mod scheduler;
pub mod webhook;
pub mod workflow;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use zero_common::config::Config;

pub use routes::{build_router, create_state, WorkflowState};
pub use scheduler::{Scheduler, TaskInfo};
pub use webhook::{webhook_routes, WebhookEvent, WebhookState};
pub use workflow::{ExecutionStatus, Step, StepType, Trigger, Workflow, WorkflowExecutor, WorkflowResult};
pub use github::{GitHubClient, PullRequestEvent};
pub use gitlab::{GitLabClient, MergeRequestEvent};
pub use review_bridge::{ReviewBridge, ReviewResult};
pub use dsl::{evaluate_expression, interpolate, ControlFlow, EvalContext, EvalError, ExecutionState};

// ============================================================================
// Workflow Service
// ============================================================================

/// Workflow service that runs the HTTP server, cron scheduler, and webhook handlers.
pub struct WorkflowService {
    config: Config,
}

impl WorkflowService {
    /// Create a new workflow service.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Build the workflow router with all routes.
    pub fn build_router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let codecoder_endpoint = self.config.codecoder.endpoint.clone();
        let state = create_state(codecoder_endpoint.clone());

        // Build webhook state with optional review bridge
        let webhook_state = WebhookState::new(
            self.config.workflow.webhook.secret.clone().map(Arc::new),
            self.config.workflow.git.github_secret.clone().map(Arc::new),
            self.config.workflow.git.gitlab_token.clone().map(Arc::new),
        );

        // Add review bridge if Git integration is enabled
        let webhook_state = if self.config.workflow.git.enabled {
            let review_bridge = Arc::new(ReviewBridge::new(codecoder_endpoint));
            // Note: GitHub/GitLab clients would be configured here with tokens
            webhook_state.with_review_bridge(review_bridge)
        } else {
            webhook_state
        };

        // Combine routes
        Router::new()
            .merge(build_router(state.clone()))
            .merge(webhook_routes(webhook_state))
            .layer(cors)
    }

    /// Start the workflow service.
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting Zero Workflow service");

        let codecoder_endpoint = self.config.codecoder.endpoint.clone();
        let state = create_state(codecoder_endpoint);

        // Load cron tasks from config
        if self.config.workflow.cron.enabled {
            tracing::info!("Loading cron tasks from configuration");
            for task in &self.config.workflow.cron.tasks {
                match state.scheduler.add_task(task.clone()) {
                    Ok(()) => {
                        tracing::info!(task_id = %task.id, "Loaded cron task");
                    }
                    Err(e) => {
                        tracing::error!(task_id = %task.id, error = %e, "Failed to load cron task");
                    }
                }
            }

            // Start the scheduler
            let (tx, _rx) = mpsc::channel::<String>(100);
            let scheduler = Arc::clone(&state.scheduler);

            // Create a mutable scheduler for starting
            // Note: The scheduler needs to be started with a mutable reference
            // We use interior mutability pattern here
            tokio::spawn(async move {
                let executor = move |command: &str| {
                    let cmd = command.to_string();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        match execute_command(&cmd).await {
                            Ok(output) => {
                                tracing::info!(command = %cmd, "Cron command executed successfully");
                                let _ = tx.send(output).await;
                            }
                            Err(e) => {
                                tracing::error!(command = %cmd, error = %e, "Cron command failed");
                            }
                        }
                    });
                };

                // Check for tasks periodically
                loop {
                    if let Ok(tasks) = scheduler.list_tasks() {
                        let now = chrono::Utc::now();
                        for task in tasks {
                            if task.next_run <= now {
                                tracing::info!(task_id = %task.id, "Running scheduled task");
                                executor(&task.command);
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });

            tracing::info!("Cron scheduler started");
        }

        // Build and start HTTP server
        let router = self.build_router();

        // Default port 4405 for workflow service
        let port = self.config.workflow.webhook.port.unwrap_or(4405);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        tracing::info!("Starting Zero Workflow HTTP server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}

/// Execute a shell command asynchronously.
async fn execute_command(command: &str) -> anyhow::Result<String> {
    use std::process::Stdio;
    use tokio::process::Command;

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Command failed: {}", stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_service_creation() {
        let config = Config::default();
        let _service = WorkflowService::new(config);
    }
}
