//! Zero Workflow - Workflow engine for the Zero ecosystem.
//!
//! This crate provides:
//! - Webhook triggers (Git, custom)
//! - Cron scheduler
//! - Workflow orchestration
//! - HTTP API for management
//! - Automated code review via CodeCoder
//! - Ticket/Issue automation from user feedback
//! - Competitive intelligence monitoring

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod dsl;
pub mod economic_bridge;
pub mod github;
pub mod gitlab;
pub mod monitor_bridge;
pub mod review_bridge;
pub mod risk_monitor;
pub mod routes;
pub mod scheduler;
pub mod ticket_bridge;
pub mod trading_review;
pub mod webhook;
pub mod workflow;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use zero_common::config::Config;

pub use routes::{build_router, create_state, create_state_with_channels, WorkflowState};
pub use scheduler::{Scheduler, TaskInfo};
pub use webhook::{webhook_routes, WebhookEvent, WebhookState};
pub use workflow::{ExecutionStatus, Step, StepType, Trigger, Workflow, WorkflowExecutor, WorkflowResult};
pub use github::{GitHubClient, IssueResponse, PullRequestEvent};
pub use gitlab::{GitLabClient, MergeRequestEvent};
pub use review_bridge::{ReviewBridge, ReviewResult};
pub use ticket_bridge::{Feedback, FeedbackCategory, TicketBridge, TicketIMConfig, TicketResult};
pub use monitor_bridge::{MonitorBridge, MonitorIMConfig, MonitorReport, MonitorRunResult, SourceContent, SourceSummary};
pub use economic_bridge::{AnomalyAlert, AlertType, DataSource, EconomicDataBridge, EconomicDataPoint, IndicatorConfig, IndicatorType};
pub use risk_monitor::{AlertSeverity, AlertThreshold, Margin, MarginCategory, RiskAlert, RiskAlertType, RiskMonitor, RiskMonitorConfig};
pub use trading_review::{AssetClass, JournalEntry, ReminderSchedule, ReviewPeriod, ReviewStats, TradeDirection, TradeEntry, TradeOutcome, TradingReview, TradingReviewSystem};
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
        let channels_endpoint = format!(
            "http://{}:{}",
            self.config.channels.host, self.config.channels.port
        );

        // Create state with channels endpoint for monitor IM notifications
        let state = create_state_with_channels(codecoder_endpoint.clone(), channels_endpoint.clone());

        // Load monitor tasks from configuration
        if self.config.workflow.monitor.enabled {
            let tasks = state.monitor_tasks.clone();
            let monitor_tasks = self.config.workflow.monitor.tasks.clone();

            tokio::spawn(async move {
                let mut tasks = tasks.write().await;
                for task in monitor_tasks {
                    tracing::info!(task_id = %task.id, task_name = %task.name, "Loaded monitor task");
                    tasks.insert(task.id.clone(), task);
                }
            });
        }

        // Build webhook state with optional review bridge
        let webhook_state = WebhookState::new(
            self.config.workflow.webhook.secret.clone().map(Arc::new),
            self.config.workflow.git.github_secret.clone().map(Arc::new),
            self.config.workflow.git.gitlab_token.clone().map(Arc::new),
        );

        // Add review bridge if Git integration is enabled
        let webhook_state = if self.config.workflow.git.enabled {
            let review_bridge = Arc::new(ReviewBridge::new(codecoder_endpoint.clone()));
            // Note: GitHub/GitLab clients would be configured here with tokens
            webhook_state.with_review_bridge(review_bridge)
        } else {
            webhook_state
        };

        // Build ticket bridge if ticket automation is enabled
        let webhook_state = if self.config.workflow.ticket.enabled {
            if let Some(ref github_config) = self.config.workflow.ticket.github {
                // Get GitHub token from ticket config or fall back to git config
                let github_token = github_config
                    .token
                    .clone()
                    .or_else(|| self.config.workflow.git.github_token.clone());

                if let Some(token) = github_token {
                    if let Ok(github_client) = GitHubClient::new(&token) {
                        let mut ticket_bridge = TicketBridge::new(codecoder_endpoint)
                            .with_github(Arc::new(github_client))
                            .with_default_repo(&github_config.default_repo)
                            .with_bug_labels(github_config.bug_labels.clone())
                            .with_feature_labels(github_config.feature_labels.clone());

                        // Configure IM notifications if enabled
                        if let Some(ref notification) = self.config.workflow.ticket.notification {
                            if notification.enabled {
                                ticket_bridge = ticket_bridge.with_im_config(TicketIMConfig {
                                    enabled: true,
                                    channels_endpoint: Some(channels_endpoint),
                                    channel_type: notification.channel_type.clone(),
                                    channel_id: Some(notification.channel_id.clone()),
                                });
                            }
                        }

                        tracing::info!(
                            repo = %github_config.default_repo,
                            "Ticket bridge initialized with GitHub integration"
                        );

                        webhook_state.with_ticket_bridge(Arc::new(ticket_bridge))
                    } else {
                        tracing::warn!("Failed to create GitHub client for ticket bridge");
                        webhook_state
                    }
                } else {
                    tracing::warn!("Ticket automation enabled but no GitHub token configured");
                    webhook_state
                }
            } else {
                webhook_state
            }
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
        let channels_endpoint = format!(
            "http://{}:{}",
            self.config.channels.host, self.config.channels.port
        );

        let state = create_state_with_channels(codecoder_endpoint, channels_endpoint);

        // Load monitor tasks from config
        if self.config.workflow.monitor.enabled {
            tracing::info!("Loading monitor tasks from configuration");
            let mut tasks = state.monitor_tasks.write().await;
            for task in &self.config.workflow.monitor.tasks {
                tracing::info!(
                    task_id = %task.id,
                    task_name = %task.name,
                    schedule = %task.schedule,
                    sources_count = task.sources.len(),
                    "Loaded monitor task"
                );
                tasks.insert(task.id.clone(), task.clone());
            }
            drop(tasks);

            // Start monitor scheduler
            let monitor_tasks = state.monitor_tasks.clone();
            let monitor_bridge = state.monitor_bridge.clone();

            tokio::spawn(async move {
                Self::run_monitor_scheduler(monitor_tasks, monitor_bridge).await;
            });

            tracing::info!("Monitor scheduler started");
        }

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

        // Default port 4412 for workflow service
        let port = self.config.workflow.webhook.port.unwrap_or(4412);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        tracing::info!("Starting Zero Workflow HTTP server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }

    /// Run the monitor scheduler.
    async fn run_monitor_scheduler(
        monitor_tasks: Arc<tokio::sync::RwLock<std::collections::HashMap<String, zero_common::config::MonitorTask>>>,
        monitor_bridge: Arc<MonitorBridge>,
    ) {
        use std::collections::HashMap;

        // Track last run times for each task
        let mut last_runs: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();

        loop {
            let tasks = monitor_tasks.read().await;
            let now = chrono::Utc::now();

            for (task_id, task) in tasks.iter() {
                // Parse cron expression
                let schedule = match task.schedule.parse::<cron::Schedule>() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(task_id = %task_id, error = %e, "Invalid cron expression");
                        continue;
                    }
                };

                // Get last run time or epoch
                let last_run = last_runs
                    .get(task_id)
                    .cloned()
                    .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH.into());

                // Check if task should run
                if let Some(next_run) = schedule.after(&last_run).next() {
                    if next_run <= now {
                        tracing::info!(
                            task_id = %task_id,
                            task_name = %task.name,
                            "Running scheduled monitor task"
                        );

                        let task_clone = task.clone();
                        let bridge_clone = monitor_bridge.clone();

                        // Run monitor task
                        match bridge_clone.run_monitor(&task_clone).await {
                            Ok(result) => {
                                match result {
                                    MonitorRunResult::Success { report, notification_sent } => {
                                        tracing::info!(
                                            task_id = %task_id,
                                            report_id = %report.id,
                                            notification_sent = notification_sent,
                                            "Monitor task completed successfully"
                                        );
                                    }
                                    MonitorRunResult::Partial { report, failed_sources, notification_sent } => {
                                        tracing::warn!(
                                            task_id = %task_id,
                                            report_id = %report.id,
                                            failed_count = failed_sources.len(),
                                            notification_sent = notification_sent,
                                            "Monitor task completed with some failures"
                                        );
                                    }
                                    MonitorRunResult::Failed { reason } => {
                                        tracing::error!(
                                            task_id = %task_id,
                                            reason = %reason,
                                            "Monitor task failed"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    task_id = %task_id,
                                    error = %e,
                                    "Monitor task execution error"
                                );
                            }
                        }

                        // Update last run time
                        last_runs.insert(task_id.clone(), now);
                    }
                }
            }

            drop(tasks);

            // Check every minute
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
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
