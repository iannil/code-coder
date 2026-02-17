//! `CodeCoder` Tool - SSE bridge to `CodeCoder`'s 23 AI agents
//!
//! This tool allows `ZeroBot` to invoke `CodeCoder` agents via HTTP API
//! with Server-Sent Events (SSE) for real-time streaming responses.
//! Supports all 23 agents: build, plan, decision, macro, trader, etc.

#![allow(clippy::single_match_else)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::too_many_lines)]

use super::traits::{Tool, ToolResult};
use crate::agent::confirmation;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Default `CodeCoder` API endpoint
const DEFAULT_ENDPOINT: &str = "http://localhost:4096";
/// HTTP request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 300; // 5 minutes for long-running tasks
/// SSE connection timeout in seconds
const SSE_TIMEOUT_SECS: u64 = 600; // 10 minutes max for SSE stream
/// SSE per-chunk timeout in seconds (how long to wait between chunks)
const SSE_CHUNK_TIMEOUT_SECS: u64 = 120; // 2 minutes per chunk - generous for agent thinking
/// Confirmation timeout in seconds (how long to wait for user to approve/reject)
const CONFIRMATION_TIMEOUT_SECS: u64 = 120; // 2 minutes for user to respond

/// `CodeCoder` HTTP client tool with SSE support
pub struct CodeCoderTool {
    endpoint: String,
    client: reqwest::Client,
}

impl CodeCoderTool {
    pub fn new(endpoint: Option<&str>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).to_string(),
            client,
        }
    }
}

// ============================================================================
// Task API Types
// ============================================================================

/// Request body for creating a task
#[derive(Debug, Serialize)]
struct CreateTaskRequest {
    agent: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<TaskContext>,
}

/// Task context for remote execution
#[derive(Debug, Serialize)]
struct TaskContext {
    source: String,
    #[serde(rename = "userID")]
    user_id: String,
    platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
}

/// Response from task creation
#[derive(Debug, Deserialize)]
struct CreateTaskResponse {
    success: bool,
    data: Option<TaskData>,
    error: Option<String>,
}

/// Task data from API
#[derive(Debug, Deserialize)]
struct TaskData {
    id: String,
    status: String,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Response from getting task status
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetTaskResponse {
    success: bool,
    data: Option<TaskData>,
    error: Option<String>,
}

// ============================================================================
// SSE Event Types
// ============================================================================

/// SSE event types from `CodeCoder` Task API
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
enum TaskEvent {
    #[serde(rename = "progress")]
    Progress(ProgressData),
    #[serde(rename = "confirmation")]
    Confirmation(ConfirmationData),
    #[serde(rename = "finish")]
    Finish(FinishData),
}

#[derive(Debug, Deserialize)]
struct ProgressData {
    #[serde(default)]
    stage: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct ConfirmationData {
    #[serde(rename = "requestID")]
    request_id: String,
    permission: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct FinishData {
    success: bool,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

// ============================================================================
// SSE Parser
// ============================================================================

/// Parse a single SSE event from raw text
fn parse_sse_event(text: &str) -> Option<TaskEvent> {
    // SSE format:
    // event: message
    // data: {"type": "progress", "data": {...}}
    //
    // or just:
    // data: {"type": "progress", "data": {...}}

    let mut data_line: Option<&str> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("data:") {
            data_line = Some(line.strip_prefix("data:")?.trim());
        }
    }

    let data = data_line?;

    // Try to parse the event
    match serde_json::from_str::<TaskEvent>(data) {
        Ok(event) => Some(event),
        Err(_) => {
            // Try parsing as a wrapper object
            #[derive(Deserialize)]
            struct EventWrapper {
                #[serde(rename = "type")]
                event_type: String,
                data: serde_json::Value,
            }

            if let Ok(wrapper) = serde_json::from_str::<EventWrapper>(data) {
                match wrapper.event_type.as_str() {
                    "progress" => serde_json::from_value(wrapper.data)
                        .ok()
                        .map(TaskEvent::Progress),
                    "confirmation" => serde_json::from_value(wrapper.data)
                        .ok()
                        .map(TaskEvent::Confirmation),
                    "finish" => serde_json::from_value(wrapper.data)
                        .ok()
                        .map(TaskEvent::Finish),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

// ============================================================================
// Tool Implementation
// ============================================================================

#[async_trait]
impl Tool for CodeCoderTool {
    fn name(&self) -> &str {
        "codecoder"
    }

    fn description(&self) -> &str {
        "Invoke CodeCoder AI agents for specialized tasks. Available agents: \
        build (main dev), plan (planning), decision (CLOSE framework), \
        macro (economics), trader (trading), picker (product selection), \
        miniproduct (MVP design), ai-engineer (AI systems), \
        code-reviewer, security-reviewer, tdd-guide, architect, \
        writer, proofreader, observer (philosophy), explore, general."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Agent to invoke. Options: build, plan, decision, macro, \
                                  trader, picker, miniproduct, ai-engineer, code-reviewer, \
                                  security-reviewer, tdd-guide, architect, writer, proofreader, \
                                  observer, explore, general",
                    "enum": [
                        "build", "plan", "decision", "macro", "trader", "picker",
                        "miniproduct", "ai-engineer", "code-reviewer", "security-reviewer",
                        "tdd-guide", "architect", "writer", "proofreader", "observer",
                        "explore", "general", "code-reverse", "jar-code-reverse"
                    ]
                },
                "prompt": {
                    "type": "string",
                    "description": "The message/prompt to send to the agent"
                },
                "model": {
                    "type": "string",
                    "description": "Optional: specific model to use (e.g., 'anthropic/claude-sonnet-4')"
                },
                "auto_approve": {
                    "type": "boolean",
                    "description": "Optional: automatically approve permission requests (default: false)"
                }
            },
            "required": ["agent", "prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let agent = args
            .get("agent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'agent' parameter"))?;

        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;

        let model = args.get("model").and_then(|v| v.as_str()).map(String::from);
        let auto_approve = args.get("auto_approve").and_then(|v| v.as_bool()).unwrap_or(false);

        // Extract context info if available (injected by AgentExecutor)
        let (platform, user_id) = if let Some(ctx) = args.get("_context") {
            let channel = ctx.get("channel").and_then(|v| v.as_str()).unwrap_or("zerobot");
            let sender = ctx.get("sender_id").and_then(|v| v.as_str()).unwrap_or("unknown");
            (channel.to_string(), sender.to_string())
        } else {
            ("zerobot".to_string(), "unknown".to_string())
        };

        // Step 1: Create task via Task API
        let task_id = self.create_task(agent, prompt, model.as_deref(), &platform, &user_id).await?;
        tracing::info!("Created CodeCoder task: {} (platform: {}, user: {})", task_id, platform, user_id);

        // Step 2: Connect to SSE stream and process events
        let result = self.stream_task_events(&task_id, agent, auto_approve, &platform, &user_id).await;

        match result {
            Ok(output) => Ok(ToolResult {
                success: true,
                output: format!("[Agent: {agent}]\n[Task: {task_id}]\n\n{output}"),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: format!("Task ID: {task_id}"),
                error: Some(e.to_string()),
            }),
        }
    }
}

impl CodeCoderTool {
    /// Create a new task via the Task API
    async fn create_task(
        &self,
        agent: &str,
        prompt: &str,
        model: Option<&str>,
        platform: &str,
        user_id: &str,
    ) -> anyhow::Result<String> {
        let request = CreateTaskRequest {
            agent: agent.to_string(),
            prompt: prompt.to_string(),
            model: model.map(String::from),
            context: Some(TaskContext {
                source: "remote".to_string(),
                user_id: user_id.to_string(),
                platform: platform.to_string(),
                client_id: Some("zerobot".to_string()),
            }),
        };

        let resp = self
            .client
            .post(format!("{}/api/v1/tasks", self.endpoint))
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create task: {e}"))?;

        let body: CreateTaskResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse task response: {e}"))?;

        if !body.success {
            return Err(anyhow::anyhow!(
                body.error.unwrap_or_else(|| "Unknown error creating task".into())
            ));
        }

        body.data
            .ok_or_else(|| anyhow::anyhow!("No task data in response"))
            .map(|d| d.id)
    }

    /// Connect to SSE stream and process events
    async fn stream_task_events(
        &self,
        task_id: &str,
        agent: &str,
        auto_approve: bool,
        platform: &str,
        user_id: &str,
    ) -> anyhow::Result<String> {
        let url = format!("{}/api/v1/tasks/{}/events", self.endpoint, task_id);

        tracing::info!("Connecting to SSE stream: {}", url);

        // Create a client with longer timeout for SSE
        let sse_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(SSE_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        let resp = match sse_client
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("SSE connection error: {}, falling back to polling", e);
                return self.poll_task_result(task_id).await;
            }
        };

        let status = resp.status();
        tracing::info!("SSE response status: {}", status);

        if !status.is_success() {
            // If SSE connection fails, fall back to polling
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("SSE connection failed with status {}: {}", status, body);
            return self.poll_task_result(task_id).await;
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut progress_messages: Vec<String> = Vec::new();

        // Use per-chunk timeout to prevent indefinite blocking
        let chunk_timeout = Duration::from_secs(SSE_CHUNK_TIMEOUT_SECS);

        loop {
            // Wait for next chunk with timeout
            let chunk_result = match tokio::time::timeout(chunk_timeout, stream.next()).await {
                Ok(Some(result)) => {
                    tracing::debug!("Received SSE chunk");
                    result
                }
                Ok(None) => {
                    // Stream ended normally
                    tracing::info!("SSE stream ended normally, polling for final status");
                    return self.poll_task_result(task_id).await;
                }
                Err(_) => {
                    // Timeout waiting for chunk - fall back to polling
                    tracing::warn!(
                        "SSE chunk timeout after {} seconds, falling back to polling",
                        SSE_CHUNK_TIMEOUT_SECS
                    );
                    return self.poll_task_result(task_id).await;
                }
            };

            // Process the chunk
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&text);

                    // Process complete events (separated by double newline)
                    while let Some(pos) = buffer.find("\n\n") {
                        let event_text = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        if event_text.trim().is_empty() {
                            continue;
                        }

                        if let Some(event) = parse_sse_event(&event_text) {
                            match event {
                                TaskEvent::Progress(data) => {
                                    tracing::debug!(
                                        "[{}] Progress: {} - {}",
                                        agent,
                                        data.stage,
                                        data.message
                                    );
                                    if !data.message.is_empty() {
                                        progress_messages.push(format!(
                                            "[{}] {}",
                                            data.stage,
                                            data.message
                                        ));
                                    }
                                }
                                TaskEvent::Confirmation(data) => {
                                    tracing::info!(
                                        "[{}] Confirmation needed: {} - {}",
                                        agent,
                                        data.permission,
                                        data.message
                                    );

                                    if auto_approve {
                                        // Auto-approve the permission
                                        let notification_msg = format!(
                                            "🔐 *CodeCoder 授权请求*\n\n\
                                            📋 *操作*: {}\n\
                                            📝 *详情*: {}\n\n\
                                            ✅ 已自动批准",
                                            data.permission,
                                            data.message
                                        );
                                        confirmation::notify(platform, user_id, &notification_msg).await;
                                        self.approve_task(task_id, &data.request_id).await?;
                                        progress_messages.push(format!(
                                            "[auto-approved] {}",
                                            data.message
                                        ));
                                    } else {
                                        // Request interactive confirmation from user
                                        tracing::info!(
                                            "Requesting interactive confirmation for {} from {} on {}",
                                            data.request_id,
                                            user_id,
                                            platform
                                        );

                                        match confirmation::request_confirmation_and_wait(
                                            platform,
                                            user_id,
                                            &data.request_id,
                                            &data.permission,
                                            &data.message,
                                            Some(CONFIRMATION_TIMEOUT_SECS),
                                        )
                                        .await
                                        {
                                            Ok(true) => {
                                                // User approved
                                                tracing::info!(
                                                    "User {} approved confirmation {}",
                                                    user_id,
                                                    data.request_id
                                                );
                                                self.approve_task(task_id, &data.request_id).await?;
                                                progress_messages.push(format!(
                                                    "[user-approved] {}",
                                                    data.message
                                                ));
                                            }
                                            Ok(false) => {
                                                // User rejected
                                                tracing::info!(
                                                    "User {} rejected confirmation {}",
                                                    user_id,
                                                    data.request_id
                                                );
                                                return Err(anyhow::anyhow!(
                                                    "用户拒绝了操作: {} - {}",
                                                    data.permission,
                                                    data.message
                                                ));
                                            }
                                            Err(e) => {
                                                // Timeout or error - no registry/sink initialized
                                                // Fall back to old behavior (return error)
                                                tracing::warn!(
                                                    "Interactive confirmation failed: {}, falling back to error",
                                                    e
                                                );
                                                return Err(anyhow::anyhow!(
                                                    "需要授权: {} - {}。请使用 auto_approve=true 或通过 API 手动批准。",
                                                    data.permission,
                                                    data.message
                                                ));
                                            }
                                        }
                                    }
                                }
                                TaskEvent::Finish(data) => {
                                    tracing::info!(
                                        "[{}] Task finished. Success: {}",
                                        agent,
                                        data.success
                                    );

                                    if data.success {
                                        return Ok(data.output.unwrap_or_else(|| {
                                            if progress_messages.is_empty() {
                                                "Task completed successfully".to_string()
                                            } else {
                                                progress_messages.join("\n")
                                            }
                                        }));
                                    }
                                    return Err(anyhow::anyhow!(
                                        data.error.unwrap_or_else(|| "Task failed".into())
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("SSE stream error: {}, falling back to polling", e);
                    return self.poll_task_result(task_id).await;
                }
            }
        }
    }

    /// Approve a permission request
    async fn approve_task(&self, task_id: &str, request_id: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/api/v1/tasks/{}/interact", self.endpoint, task_id))
            .json(&json!({
                "action": "approve",
                "reply": "session",
                "requestID": request_id
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to approve task: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to approve task: {}", text));
        }

        Ok(())
    }

    /// Reject a permission request (user declined)
    #[allow(dead_code)]
    async fn reject_task(&self, task_id: &str, request_id: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/api/v1/tasks/{}/interact", self.endpoint, task_id))
            .json(&json!({
                "action": "reject",
                "reply": "session",
                "requestID": request_id
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to reject task: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to reject task: {}", text));
        }

        Ok(())
    }

    /// Fallback: Poll for task result
    async fn poll_task_result(&self, task_id: &str) -> anyhow::Result<String> {
        tracing::info!("Starting to poll task {} for result", task_id);
        let max_attempts = 180; // 180 * 2s = 6 minutes
        let poll_interval = Duration::from_secs(2);

        for attempt in 0..max_attempts {
            tokio::time::sleep(poll_interval).await;

            let resp = self
                .client
                .get(format!("{}/api/v1/tasks/{}", self.endpoint, task_id))
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get task status: {e}"))?;

            let body: GetTaskResponse = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse task response: {e}"))?;

            if let Some(task) = body.data {
                tracing::info!(
                    "Poll attempt {}: task {} status = {}",
                    attempt + 1,
                    task_id,
                    task.status
                );
                match task.status.as_str() {
                    "completed" => {
                        tracing::info!("Task {} completed with output", task_id);
                        return Ok(task.output.unwrap_or_else(|| "Task completed".into()));
                    }
                    "failed" => {
                        tracing::warn!("Task {} failed: {:?}", task_id, task.error);
                        return Err(anyhow::anyhow!(
                            task.error.unwrap_or_else(|| "Task failed".into())
                        ));
                    }
                    "awaiting_approval" => {
                        tracing::info!("Task {} awaiting approval", task_id);
                        return Err(anyhow::anyhow!(
                            "Task is awaiting approval. Use auto_approve=true or approve manually."
                        ));
                    }
                    _ => {
                        // Still running, continue polling
                        tracing::debug!(
                            "Task {} status: {} (attempt {}/{})",
                            task_id,
                            task.status,
                            attempt + 1,
                            max_attempts
                        );
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Timeout waiting for task completion"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codecoder_tool_name() {
        let tool = CodeCoderTool::new(None);
        assert_eq!(tool.name(), "codecoder");
    }

    #[test]
    fn codecoder_tool_description() {
        let tool = CodeCoderTool::new(None);
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("build"));
        assert!(tool.description().contains("decision"));
    }

    #[test]
    fn codecoder_tool_schema_has_required_fields() {
        let tool = CodeCoderTool::new(None);
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["agent"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
        assert!(schema["properties"]["model"].is_object());
        assert!(schema["properties"]["auto_approve"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("agent")));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("prompt")));
    }

    #[test]
    fn codecoder_tool_custom_endpoint() {
        let tool = CodeCoderTool::new(Some("http://custom:8080"));
        assert_eq!(tool.endpoint, "http://custom:8080");
    }

    #[tokio::test]
    async fn codecoder_missing_agent() {
        let tool = CodeCoderTool::new(None);
        let result = tool.execute(json!({"prompt": "hello"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("agent"));
    }

    #[tokio::test]
    async fn codecoder_missing_prompt() {
        let tool = CodeCoderTool::new(None);
        let result = tool.execute(json!({"agent": "build"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("prompt"));
    }

    // ── SSE Parsing Tests ───────────────────────────────────────

    #[test]
    fn parse_sse_progress_event() {
        let sse_text = r#"event: message
data: {"type": "progress", "data": {"stage": "starting", "message": "Starting agent..."}}

"#;
        let event = parse_sse_event(sse_text);
        assert!(event.is_some());
        match event.unwrap() {
            TaskEvent::Progress(data) => {
                assert_eq!(data.stage, "starting");
                assert_eq!(data.message, "Starting agent...");
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn parse_sse_finish_event() {
        let sse_text = r#"data: {"type": "finish", "data": {"success": true, "output": "Done!"}}

"#;
        let event = parse_sse_event(sse_text);
        assert!(event.is_some());
        match event.unwrap() {
            TaskEvent::Finish(data) => {
                assert!(data.success);
                assert_eq!(data.output.as_deref(), Some("Done!"));
            }
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn parse_sse_confirmation_event() {
        let sse_text = r#"data: {"type": "confirmation", "data": {"requestID": "req-123", "permission": "edit", "message": "Allow editing?"}}

"#;
        let event = parse_sse_event(sse_text);
        assert!(event.is_some());
        match event.unwrap() {
            TaskEvent::Confirmation(data) => {
                assert_eq!(data.request_id, "req-123");
                assert_eq!(data.permission, "edit");
                assert_eq!(data.message, "Allow editing?");
            }
            _ => panic!("Expected Confirmation event"),
        }
    }

    #[test]
    fn parse_sse_empty_data() {
        let sse_text = "";
        let event = parse_sse_event(sse_text);
        assert!(event.is_none());
    }

    #[test]
    fn parse_sse_invalid_json() {
        let sse_text = "data: not-json\n\n";
        let event = parse_sse_event(sse_text);
        assert!(event.is_none());
    }
}
