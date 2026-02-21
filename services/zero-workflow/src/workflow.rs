//! Workflow definition and execution.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name
    pub name: String,
    /// Workflow description
    #[serde(default)]
    pub description: Option<String>,
    /// Trigger configuration
    pub trigger: Trigger,
    /// Workflow steps
    pub steps: Vec<Step>,
    /// Global variables
    #[serde(default)]
    pub vars: HashMap<String, serde_json::Value>,
}

/// Workflow trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Trigger {
    /// Webhook trigger
    Webhook {
        /// Event types to match
        #[serde(default)]
        events: Vec<String>,
        /// Filter conditions
        #[serde(default)]
        filter: Option<TriggerFilter>,
    },
    /// Cron trigger
    Cron {
        /// Cron expression
        expression: String,
    },
    /// Manual trigger
    Manual,
}

/// Trigger filter for webhook events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerFilter {
    /// Branch filter (for git events)
    #[serde(default)]
    pub branch: Option<String>,
    /// Action filter (e.g., "opened", "closed")
    #[serde(default)]
    pub action: Option<Vec<String>>,
    /// Custom JSONPath conditions
    #[serde(default)]
    pub conditions: Vec<String>,
}

/// Workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step name
    pub name: String,
    /// Step type
    #[serde(flatten)]
    pub step_type: StepType,
    /// Condition for running this step (expression)
    #[serde(default)]
    pub condition: Option<String>,
    /// Continue on error
    #[serde(default)]
    pub continue_on_error: bool,
    /// Timeout in seconds
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Step type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    /// Call a CodeCoder agent
    Agent {
        /// Agent name
        agent: String,
        /// Input data
        input: serde_json::Value,
    },
    /// Send a notification
    Notify {
        /// Channel type
        channel: String,
        /// Message template
        template: String,
    },
    /// Run a shell command
    Shell {
        /// Command to run
        command: String,
        /// Working directory
        #[serde(default)]
        cwd: Option<String>,
    },
    /// HTTP request
    Http {
        /// HTTP method
        method: String,
        /// URL
        url: String,
        /// Request body
        #[serde(default)]
        body: Option<serde_json::Value>,
        /// Headers
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Workflow execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow name
    pub workflow: String,
    /// Execution ID
    pub execution_id: String,
    /// Overall status
    pub status: ExecutionStatus,
    /// Step results
    pub steps: Vec<StepResult>,
    /// Start time
    pub started_at: i64,
    /// End time
    pub ended_at: Option<i64>,
}

/// Execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Cancelled,
}

/// Step execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step name
    pub name: String,
    /// Step status
    pub status: ExecutionStatus,
    /// Step output
    pub output: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Workflow executor.
pub struct WorkflowExecutor {
    http_client: reqwest::Client,
    codecoder_endpoint: Option<String>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor.
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            codecoder_endpoint: None,
        }
    }

    /// Create a workflow executor with CodeCoder endpoint.
    pub fn with_codecoder(endpoint: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            codecoder_endpoint: Some(endpoint),
        }
    }

    /// Execute a workflow.
    pub async fn execute(&self, workflow: &Workflow, context: serde_json::Value) -> Result<WorkflowResult> {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().timestamp_millis();

        tracing::info!(
            workflow = %workflow.name,
            execution_id = %execution_id,
            "Starting workflow execution"
        );

        let mut step_results = Vec::new();
        let mut overall_status = ExecutionStatus::Success;
        let mut step_outputs: HashMap<String, serde_json::Value> = HashMap::new();

        // Add context to step outputs for variable interpolation
        step_outputs.insert("event".to_string(), context.clone());

        for step in &workflow.steps {
            let step_start = std::time::Instant::now();

            // TODO: Evaluate condition using step_outputs

            tracing::info!(
                workflow = %workflow.name,
                step = %step.name,
                "Executing step"
            );

            let result = match &step.step_type {
                StepType::Shell { command, cwd } => {
                    self.execute_shell(command, cwd.as_deref(), step.timeout_secs)
                        .await
                }
                StepType::Http {
                    method,
                    url,
                    body,
                    headers,
                } => {
                    self.execute_http(method, url, body.as_ref(), headers)
                        .await
                }
                StepType::Agent { agent, input } => self.execute_agent(agent, input).await,
                StepType::Notify { channel, template } => {
                    self.execute_notify(channel, template, &step_outputs).await
                }
            };

            let duration_ms = step_start.elapsed().as_millis() as u64;

            let step_result = match result {
                Ok(output) => {
                    step_outputs.insert(step.name.clone(), output.clone());
                    StepResult {
                        name: step.name.clone(),
                        status: ExecutionStatus::Success,
                        output: Some(output),
                        error: None,
                        duration_ms,
                    }
                }
                Err(e) => {
                    tracing::error!(
                        workflow = %workflow.name,
                        step = %step.name,
                        error = %e,
                        "Step execution failed"
                    );
                    StepResult {
                        name: step.name.clone(),
                        status: ExecutionStatus::Failed,
                        output: None,
                        error: Some(e.to_string()),
                        duration_ms,
                    }
                }
            };

            if step_result.status == ExecutionStatus::Failed && !step.continue_on_error {
                overall_status = ExecutionStatus::Failed;
                step_results.push(step_result);
                break;
            }

            step_results.push(step_result);
        }

        let ended_at = Some(chrono::Utc::now().timestamp_millis());

        tracing::info!(
            workflow = %workflow.name,
            execution_id = %execution_id,
            status = ?overall_status,
            "Workflow execution completed"
        );

        Ok(WorkflowResult {
            workflow: workflow.name.clone(),
            execution_id,
            status: overall_status,
            steps: step_results,
            started_at,
            ended_at,
        })
    }

    /// Execute a shell command.
    async fn execute_shell(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: Option<u64>,
    ) -> Result<serde_json::Value> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(300));

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .context("Command timed out")?
            .context("Failed to execute command")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": 0
            }))
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            Err(anyhow::anyhow!(
                "Command failed with exit code {}: {}",
                exit_code,
                stderr
            ))
        }
    }

    /// Execute an HTTP request.
    async fn execute_http(
        &self,
        method: &str,
        url: &str,
        body: Option<&serde_json::Value>,
        headers: &HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        let method = method.to_uppercase();
        let mut request = match method.as_str() {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url),
            "PUT" => self.http_client.put(url),
            "DELETE" => self.http_client.delete(url),
            "PATCH" => self.http_client.patch(url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
        };

        for (key, value) in headers {
            request = request.header(key, value);
        }

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request
            .send()
            .await
            .context("HTTP request failed")?;

        let status = response.status().as_u16();
        let response_body: serde_json::Value = response
            .json()
            .await
            .unwrap_or(serde_json::Value::Null);

        if status >= 200 && status < 300 {
            Ok(serde_json::json!({
                "status": status,
                "body": response_body
            }))
        } else {
            Err(anyhow::anyhow!(
                "HTTP request failed with status {}: {:?}",
                status,
                response_body
            ))
        }
    }

    /// Execute a CodeCoder agent.
    async fn execute_agent(
        &self,
        agent: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let endpoint = self
            .codecoder_endpoint
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CodeCoder endpoint not configured"))?;

        let url = format!("{}/api/v1/agent/{}/execute", endpoint, agent);

        let response = self
            .http_client
            .post(&url)
            .json(input)
            .send()
            .await
            .context("Failed to call CodeCoder agent")?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse agent response")?;

        if status.is_success() {
            Ok(body)
        } else {
            Err(anyhow::anyhow!(
                "Agent execution failed: {:?}",
                body
            ))
        }
    }

    /// Execute a notification.
    async fn execute_notify(
        &self,
        channel: &str,
        template: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Simple template interpolation (replace {{ key }} with values)
        let mut message = template.to_string();
        for (key, value) in context {
            let placeholder = format!("{{{{ {} }}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            message = message.replace(&placeholder, &replacement);
        }

        // For now, just log the notification
        // In production, this would call the channels service
        tracing::info!(channel = %channel, message = %message, "Sending notification");

        Ok(serde_json::json!({
            "channel": channel,
            "message": message,
            "sent": true
        }))
    }
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_parsing() {
        let yaml = r#"
name: test-workflow
description: A test workflow
trigger:
  type: webhook
  events: ["push"]
steps:
  - name: code-review
    type: agent
    agent: code-reviewer
    input:
      diff_url: "{{ event.diff_url }}"
  - name: notify
    type: notify
    channel: slack
    template: "Review complete"
"#;

        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_workflow_with_shell() {
        let yaml = r#"
name: build-workflow
trigger:
  type: manual
steps:
  - name: build
    type: shell
    command: "echo 'Building...'"
    cwd: "/tmp"
"#;

        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workflow.name, "build-workflow");
        if let StepType::Shell { command, cwd } = &workflow.steps[0].step_type {
            assert_eq!(command, "echo 'Building...'");
            assert_eq!(cwd.as_deref(), Some("/tmp"));
        } else {
            panic!("Expected Shell step");
        }
    }

    #[tokio::test]
    async fn test_execute_shell_command() {
        let executor = WorkflowExecutor::new();
        let result = executor
            .execute_shell("echo 'hello world'", None, None)
            .await
            .unwrap();

        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_execute_shell_command_failure() {
        let executor = WorkflowExecutor::new();
        let result = executor
            .execute_shell("exit 1", None, None)
            .await;

        assert!(result.is_err());
    }
}
