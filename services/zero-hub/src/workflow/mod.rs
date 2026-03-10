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
pub mod forum;
pub mod github;
pub mod gitlab;
pub mod hands;
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
use tower_http::cors::{Any, CorsLayer};
use zero_core::common::config::Config;

pub use routes::{build_router, create_state, create_state_with_channels, create_isolated_test_state, WorkflowState};
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
// Hands exports
pub use hands::{
    HandExecutor, HandManifest, HandConfig, HandSummary, HandsScheduler,
    HandExecution, HandState, StateStore,
    state::ExecutionStatus as HandExecutionStatus,
};

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

        let codecoder_endpoint = self.config.codecoder_endpoint();
        let channels_endpoint = self.config.channels_endpoint();

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

    /// Build the workflow router with an existing state.
    fn build_router_with_state(&self, state: Arc<WorkflowState>) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let codecoder_endpoint = self.config.codecoder_endpoint();

        // Build webhook state with optional review bridge
        let webhook_state = WebhookState::new(
            self.config.workflow.webhook.secret.clone().map(Arc::new),
            self.config.workflow.git.github_secret.clone().map(Arc::new),
            self.config.workflow.git.gitlab_token.clone().map(Arc::new),
        );

        // Add review bridge if Git integration is enabled
        let webhook_state = if self.config.workflow.git.enabled {
            let review_bridge = Arc::new(ReviewBridge::new(codecoder_endpoint.clone()));
            webhook_state.with_review_bridge(review_bridge)
        } else {
            webhook_state
        };

        // Build ticket bridge if ticket automation is enabled
        let channels_endpoint = self.config.channels_endpoint();
        let webhook_state = if self.config.workflow.ticket.enabled {
            if let Some(ref github_config) = self.config.workflow.ticket.github {
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

                        webhook_state.with_ticket_bridge(Arc::new(ticket_bridge))
                    } else {
                        webhook_state
                    }
                } else {
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
            .merge(build_router(state))
            .merge(webhook_routes(webhook_state))
            .layer(cors)
    }

    /// Start the workflow service.
    pub async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Starting Zero Workflow service");

        let codecoder_endpoint = self.config.codecoder_endpoint();
        let channels_endpoint = self.config.channels_endpoint();

        let state = create_state_with_channels(codecoder_endpoint.clone(), channels_endpoint);

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
            let scheduler = Arc::clone(&state.scheduler);
            let cron_codecoder_endpoint = codecoder_endpoint.clone();
            let cron_channels_endpoint = self.config.channels_endpoint();

            tokio::spawn(async move {
                let codecoder_endpoint = cron_codecoder_endpoint;
                let channels_endpoint = cron_channels_endpoint;

                // Check for tasks periodically
                loop {
                    let now = chrono::Utc::now();

                    // Get tasks that are due to run
                    match scheduler.due_tasks(now) {
                        Ok(due_tasks) => {
                            for job in due_tasks {
                                tracing::info!(task_id = %job.id, "Running scheduled task");

                                let cmd = job.command.clone();
                                let cc_endpoint = codecoder_endpoint.clone();
                                let ch_endpoint = channels_endpoint.clone();
                                let task_id = job.id.clone();

                                // Execute the command
                                let (success, output) = match execute_cron_command(&cmd, &cc_endpoint, &ch_endpoint).await {
                                    Ok(output) => {
                                        tracing::info!(task_id = %task_id, "Cron command executed successfully");
                                        (true, output)
                                    }
                                    Err(e) => {
                                        let error_msg = format!("{}", e);
                                        tracing::error!(task_id = %task_id, command = %cmd, error = %error_msg, "Cron command failed");
                                        (false, error_msg)
                                    }
                                };

                                // Reschedule the task for next run
                                if let Err(e) = scheduler.reschedule_after_run(&job, success, &output) {
                                    tracing::error!(task_id = %task_id, error = %e, "Failed to reschedule task");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to get due tasks");
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });

            tracing::info!("Cron scheduler started");
        }

        // Initialize Hands scheduler if enabled
        if self.config.workflow.hands.enabled {
            tracing::info!("Initializing Hands scheduler");
            match HandsScheduler::new(self.config.codecoder_endpoint()) {
                Ok(mut scheduler) => {
                    // Start the scheduler (loads hands and begins scheduling loop)
                    match scheduler.start().await {
                        Ok(()) => {
                            tracing::info!("Hands scheduler started");
                            // Store in state for API access
                            let mut hands_scheduler = state.hands_scheduler.write().await;
                            *hands_scheduler = Some(scheduler);
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to start Hands scheduler");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create Hands scheduler");
                }
            }
        }

        // Build and start HTTP server
        let router = self.build_router_with_state(state.clone());

        // Use the centralized config accessors for port and bind address
        let port = self.config.workflow_port();
        let host: std::net::IpAddr = self.config.bind_address().parse().unwrap_or([127, 0, 0, 1].into());
        let addr = SocketAddr::from((host, port));

        tracing::info!("Starting Zero Workflow HTTP server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }

    /// Run the monitor scheduler.
    async fn run_monitor_scheduler(
        monitor_tasks: Arc<tokio::sync::RwLock<std::collections::HashMap<String, zero_core::common::config::MonitorTask>>>,
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

// ============================================================================
// Cron Command Types and Executor
// ============================================================================

/// Cron command types matching the TypeScript scheduler tool.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CronCommand {
    /// Execute an agent with a prompt
    Agent {
        agent: String,
        prompt: String,
        /// Optional callback channel type for sending results back to IM (e.g., "telegram")
        #[serde(default)]
        callback_channel_type: Option<String>,
        /// Optional callback channel ID for sending results back to IM
        #[serde(default)]
        callback_channel_id: Option<String>,
    },
    /// Make an HTTP API call
    Api {
        endpoint: String,
        method: String,
        #[serde(default)]
        body: Option<serde_json::Value>,
    },
    /// Send a message to an IM channel (Telegram, Feishu, etc.)
    ChannelMessage {
        channel_type: String,
        channel_id: String,
        message: String,
    },
    /// Execute a shell command (not explicitly tagged, fallback)
    #[serde(skip)]
    #[allow(dead_code)]
    Shell {
        command: String,
    },
}

/// Execute a cron command, dispatching based on command type.
///
/// Supports four command types:
/// - `agent`: Invokes a CodeCoder agent via the API
/// - `api`: Makes an HTTP request to an external endpoint
/// - `channel_message`: Sends a message to an IM channel via zero-channels
/// - `shell`: Executes a shell command (default for non-JSON commands)
async fn execute_cron_command(command_str: &str, codecoder_endpoint: &str, channels_endpoint: &str) -> anyhow::Result<String> {
    // Try to parse as JSON command
    if let Ok(cmd) = serde_json::from_str::<CronCommand>(command_str) {
        match cmd {
            CronCommand::Agent { agent, prompt, callback_channel_type, callback_channel_id } => {
                execute_agent_command(&agent, &prompt, callback_channel_type.as_deref(), callback_channel_id.as_deref(), codecoder_endpoint, channels_endpoint).await
            }
            CronCommand::Api { endpoint, method, body } => {
                execute_api_command(&endpoint, &method, body).await
            }
            CronCommand::ChannelMessage { channel_type, channel_id, message } => {
                execute_channel_message_command(&channel_type, &channel_id, &message, channels_endpoint).await
            }
            CronCommand::Shell { command } => {
                execute_shell_command(&command).await
            }
        }
    } else {
        // Not JSON, treat as shell command
        execute_shell_command(command_str).await
    }
}

/// Execute an agent command by calling the CodeCoder API.
/// If callback_channel_type and callback_channel_id are provided, sends the result back to the IM channel.
async fn execute_agent_command(
    agent: &str,
    prompt: &str,
    callback_channel_type: Option<&str>,
    callback_channel_id: Option<&str>,
    codecoder_endpoint: &str,
    channels_endpoint: &str,
) -> anyhow::Result<String> {
    use std::time::Duration;

    tracing::info!(agent = %agent, prompt_len = prompt.len(), "Executing agent command");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300)) // 5 minute timeout for agent execution
        .build()?;

    let url = format!("{}/api/agent/invoke", codecoder_endpoint.trim_end_matches('/'));

    let payload = serde_json::json!({
        "agent": agent,
        "prompt": prompt
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call agent API: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Agent API returned {}: {}", status, body));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse agent response: {}", e))?;

    // Check for success in response
    if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let error = result.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Agent execution failed: {}", error));
    }

    // Extract useful info from response
    let session_id = result.get("data")
        .and_then(|d| d.get("sessionId"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let message_id = result.get("data")
        .and_then(|d| d.get("messageId"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    // Extract agent response content for callback
    let response_content = result.get("data")
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    tracing::info!(
        agent = %agent,
        session_id = %session_id,
        message_id = %message_id,
        has_callback = callback_channel_type.is_some() && callback_channel_id.is_some(),
        "Agent command completed"
    );

    // If callback channel is configured, send the result back to IM
    if let (Some(ch_type), Some(ch_id)) = (callback_channel_type, callback_channel_id) {
        if !response_content.is_empty() {
            tracing::info!(
                channel_type = %ch_type,
                channel_id = %ch_id,
                content_len = response_content.len(),
                "Sending agent result to callback channel"
            );

            if let Err(e) = execute_channel_message_command(ch_type, ch_id, response_content, channels_endpoint).await {
                tracing::warn!(
                    error = %e,
                    channel_type = %ch_type,
                    channel_id = %ch_id,
                    "Failed to send callback message to IM channel"
                );
            }
        } else {
            tracing::debug!("Agent response content is empty, skipping callback");
        }
    }

    Ok(format!("Agent '{}' executed successfully. Session: {}, Message: {}", agent, session_id, message_id))
}

/// Execute an API command by making an HTTP request.
async fn execute_api_command(endpoint: &str, method: &str, body: Option<serde_json::Value>) -> anyhow::Result<String> {
    use std::time::Duration;

    tracing::info!(endpoint = %endpoint, method = %method, "Executing API command");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let method = match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
    };

    let mut request = client.request(method.clone(), endpoint);

    if let Some(body_value) = body {
        request = request
            .header("Content-Type", "application/json")
            .json(&body_value);
    }

    let response = request
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("API request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    tracing::info!(endpoint = %endpoint, status = %status, "API command completed");

    if status.is_success() {
        Ok(body)
    } else {
        Err(anyhow::anyhow!("API returned {}: {}", status, body))
    }
}

/// Execute a channel message command by sending a message to an IM channel via zero-channels.
async fn execute_channel_message_command(
    channel_type: &str,
    channel_id: &str,
    message: &str,
    channels_endpoint: &str,
) -> anyhow::Result<String> {
    use std::time::Duration;

    tracing::info!(
        channel_type = %channel_type,
        channel_id = %channel_id,
        message_len = message.len(),
        "Executing channel message command"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let url = format!("{}/api/v1/send", channels_endpoint.trim_end_matches('/'));

    let payload = serde_json::json!({
        "channel_type": channel_type,
        "channel_id": channel_id,
        "content": {
            "type": "text",
            "text": message
        }
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call channels API: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Channels API returned {}: {}", status, body));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse channels response: {}", e))?;

    // Check for success in response
    if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let error = result.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Channel message failed: {}", error));
    }

    let message_id = result.get("message_id")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");

    tracing::info!(
        channel_type = %channel_type,
        channel_id = %channel_id,
        message_id = %message_id,
        "Channel message sent successfully"
    );

    Ok(format!("Message sent to {} channel {}. Message ID: {}", channel_type, channel_id, message_id))
}

/// Execute a shell command asynchronously.
async fn execute_shell_command(command: &str) -> anyhow::Result<String> {
    use std::process::Stdio;
    use tokio::process::Command;

    tracing::debug!(command = %command, "Executing shell command");

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

/// Legacy function for backwards compatibility.
#[allow(dead_code)]
async fn execute_command(command: &str) -> anyhow::Result<String> {
    execute_shell_command(command).await
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
