//! CodeCoder tool - HTTP client for CodeCoder AI agents.
//!
//! Invokes CodeCoder agents via HTTP API with SSE streaming support.

use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Default CodeCoder API endpoint.
const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4400";
/// HTTP request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 300;
/// SSE per-chunk timeout in seconds.
const SSE_CHUNK_TIMEOUT_SECS: u64 = 120;
/// SSE connection retry attempts.
const SSE_MAX_RETRIES: u32 = 3;
/// SSE connection timeout in seconds.
const SSE_CONNECT_TIMEOUT_SECS: u64 = 30;

/// CodeCoder HTTP client tool.
pub struct CodeCoderTool {
    endpoint: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl CodeCoderTool {
    /// Create a new CodeCoder tool.
    pub fn new(endpoint: Option<&str>, api_key: Option<&str>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.unwrap_or(DEFAULT_ENDPOINT).to_string(),
            api_key: api_key.map(String::from),
            client,
        }
    }

    fn add_auth_headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.api_key {
            request.header("Authorization", format!("Bearer {}", key))
        } else {
            request
        }
    }
}

/// Request body for creating a task.
#[derive(Debug, Serialize)]
struct CreateTaskRequest {
    agent: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<TaskContext>,
}

/// Task context for remote execution.
#[derive(Debug, Serialize)]
struct TaskContext {
    source: String,
    #[serde(rename = "userID")]
    user_id: String,
    platform: String,
}

/// Response from task creation.
#[derive(Debug, Deserialize)]
struct CreateTaskResponse {
    success: bool,
    data: Option<TaskData>,
    error: Option<String>,
}

/// Task data from API.
#[derive(Debug, Deserialize)]
struct TaskData {
    id: String,
    status: String,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// SSE event types from CodeCoder Task API.
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

/// Parse a single SSE event from raw text.
fn parse_sse_event(text: &str) -> Option<TaskEvent> {
    let mut data_line: Option<&str> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("data:") {
            data_line = Some(line.strip_prefix("data:")?.trim());
        }
    }

    let data = data_line?;

    match serde_json::from_str::<TaskEvent>(data) {
        Ok(event) => Some(event),
        Err(_) => {
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
                    "description": "Agent to invoke",
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
                    "description": "Optional: specific model to use"
                },
                "auto_approve": {
                    "type": "boolean",
                    "description": "Automatically approve permission requests (default: false)"
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

        // Create task
        let task_id = self.create_task(agent, prompt, model.as_deref()).await?;
        tracing::info!("Created CodeCoder task: {}", task_id);

        // Stream task events
        let result = self.stream_task_events(&task_id, agent, auto_approve).await;

        match result {
            Ok(output) => Ok(ToolResult::success(format!(
                "[Agent: {}]\n[Task: {}]\n\n{}",
                agent, task_id, output
            ))),
            Err(e) => Ok(ToolResult::failure_with_output(
                format!("Task ID: {}", task_id),
                e.to_string(),
            )),
        }
    }
}

impl CodeCoderTool {
    /// Create a new task via the Task API.
    async fn create_task(
        &self,
        agent: &str,
        prompt: &str,
        model: Option<&str>,
    ) -> anyhow::Result<String> {
        let request = CreateTaskRequest {
            agent: agent.to_string(),
            prompt: prompt.to_string(),
            model: model.map(String::from),
            context: Some(TaskContext {
                source: "remote".to_string(),
                user_id: "zero-tools".to_string(),
                platform: "api".to_string(),
            }),
        };

        let req = self
            .client
            .post(format!("{}/api/v1/tasks", self.endpoint))
            .json(&request);
        let resp = self
            .add_auth_headers(req)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create task: {}", e))?;

        let body: CreateTaskResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse task response: {}", e))?;

        if !body.success {
            return Err(anyhow::anyhow!(
                body.error.unwrap_or_else(|| "Unknown error creating task".into())
            ));
        }

        body.data
            .ok_or_else(|| anyhow::anyhow!("No task data in response"))
            .map(|d| d.id)
    }

    /// Connect to SSE stream and process events.
    async fn stream_task_events(
        &self,
        task_id: &str,
        _agent: &str,
        auto_approve: bool,
    ) -> anyhow::Result<String> {
        let url = format!("{}/api/v1/tasks/{}/events", self.endpoint, task_id);

        tracing::info!("Connecting to SSE stream: {}", url);

        let sse_client = reqwest::Client::builder()
            .http1_only()
            .connect_timeout(Duration::from_secs(SSE_CONNECT_TIMEOUT_SECS))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(0)
            .build()
            .unwrap_or_default();

        // Retry SSE connection
        let mut resp: Option<reqwest::Response> = None;

        for attempt in 1..=SSE_MAX_RETRIES {
            let mut req = sse_client
                .get(&url)
                .header("Accept", "text/event-stream")
                .header("Cache-Control", "no-cache");

            if let Some(ref key) = self.api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            match req.send().await {
                Ok(r) if r.status().is_success() => {
                    resp = Some(r);
                    break;
                }
                Ok(_) | Err(_) => {}
            }

            if attempt < SSE_MAX_RETRIES {
                tokio::time::sleep(Duration::from_secs(2 * u64::from(attempt))).await;
            }
        }

        let Some(resp) = resp else {
            tracing::warn!("SSE connection failed, falling back to polling");
            return self.poll_task_result(task_id).await;
        };

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut progress_messages: Vec<String> = Vec::new();
        let chunk_timeout = Duration::from_secs(SSE_CHUNK_TIMEOUT_SECS);

        loop {
            let chunk_result = match tokio::time::timeout(chunk_timeout, stream.next()).await {
                Ok(Some(result)) => result,
                Ok(None) => return self.poll_task_result(task_id).await,
                Err(_) => return self.poll_task_result(task_id).await,
            };

            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&text);

                    while let Some(pos) = buffer.find("\n\n") {
                        let event_text = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        if event_text.trim().is_empty() {
                            continue;
                        }

                        if let Some(event) = parse_sse_event(&event_text) {
                            match event {
                                TaskEvent::Progress(data) => {
                                    if !data.message.is_empty() {
                                        progress_messages.push(format!(
                                            "[{}] {}",
                                            data.stage, data.message
                                        ));
                                    }
                                }
                                TaskEvent::Confirmation(data) => {
                                    if auto_approve {
                                        self.approve_task(task_id, &data.request_id).await?;
                                        progress_messages
                                            .push(format!("[auto-approved] {}", data.message));
                                    } else {
                                        return Err(anyhow::anyhow!(
                                            "Confirmation required: {} - {}. Use auto_approve=true.",
                                            data.permission,
                                            data.message
                                        ));
                                    }
                                }
                                TaskEvent::Finish(data) => {
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
                    tracing::warn!("SSE stream error: {}", e);
                    return self.poll_task_result(task_id).await;
                }
            }
        }
    }

    /// Approve a permission request.
    async fn approve_task(&self, task_id: &str, request_id: &str) -> anyhow::Result<()> {
        let req = self
            .client
            .post(format!("{}/api/v1/tasks/{}/interact", self.endpoint, task_id))
            .json(&json!({
                "action": "approve",
                "reply": "once",
                "requestID": request_id
            }));
        let resp = self
            .add_auth_headers(req)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to approve task: {}", e))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to approve task: {}", text));
        }

        Ok(())
    }

    /// Fallback: Poll for task result.
    async fn poll_task_result(&self, task_id: &str) -> anyhow::Result<String> {
        let max_attempts = 180;
        let poll_interval = Duration::from_secs(2);

        for _ in 0..max_attempts {
            tokio::time::sleep(poll_interval).await;

            let req = self
                .client
                .get(format!("{}/api/v1/tasks/{}", self.endpoint, task_id));
            let resp = self
                .add_auth_headers(req)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get task status: {}", e))?;

            #[derive(Deserialize)]
            struct GetTaskResponse {
                #[allow(dead_code)]
                success: bool,
                data: Option<TaskData>,
            }

            let body: GetTaskResponse = resp.json().await?;

            if let Some(task) = body.data {
                match task.status.as_str() {
                    "completed" => return Ok(task.output.unwrap_or_else(|| "Task completed".into())),
                    "failed" => {
                        return Err(anyhow::anyhow!(
                            task.error.unwrap_or_else(|| "Task failed".into())
                        ))
                    }
                    "awaiting_approval" => {
                        return Err(anyhow::anyhow!(
                            "Task is awaiting approval. Use auto_approve=true."
                        ))
                    }
                    _ => {}
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
        let tool = CodeCoderTool::new(None, None);
        assert_eq!(tool.name(), "codecoder");
    }

    #[test]
    fn codecoder_tool_description() {
        let tool = CodeCoderTool::new(None, None);
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("build"));
        assert!(tool.description().contains("decision"));
    }

    #[test]
    fn codecoder_tool_schema() {
        let tool = CodeCoderTool::new(None, None);
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["agent"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
    }

    #[test]
    fn codecoder_tool_custom_endpoint() {
        let tool = CodeCoderTool::new(Some("http://custom:8080"), Some("test-api-key"));
        assert_eq!(tool.endpoint, "http://custom:8080");
        assert_eq!(tool.api_key.as_deref(), Some("test-api-key"));
    }

    #[tokio::test]
    async fn codecoder_missing_agent() {
        let tool = CodeCoderTool::new(None, None);
        let result = tool.execute(json!({"prompt": "hello"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("agent"));
    }

    #[tokio::test]
    async fn codecoder_missing_prompt() {
        let tool = CodeCoderTool::new(None, None);
        let result = tool.execute(json!({"agent": "build"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("prompt"));
    }

    #[test]
    fn parse_sse_progress_event() {
        let sse_text = r#"data: {"type": "progress", "data": {"stage": "starting", "message": "Starting agent..."}}"#;
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
        let sse_text = r#"data: {"type": "finish", "data": {"success": true, "output": "Done!"}}"#;
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
    fn parse_sse_empty_data() {
        let event = parse_sse_event("");
        assert!(event.is_none());
    }

    #[test]
    fn parse_sse_invalid_json() {
        let event = parse_sse_event("data: not-json\n\n");
        assert!(event.is_none());
    }
}
