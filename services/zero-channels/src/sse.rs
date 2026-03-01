//! SSE (Server-Sent Events) client for consuming CodeCoder task events.
//!
//! This module provides an async SSE client that connects to the CodeCoder
//! task events endpoint (`/api/v1/tasks/{id}/events`) and streams events
//! for real-time progress feedback.
//!
//! # Architecture
//!
//! ```text
//! CodeCoder Task API                    zero-channels
//! ┌─────────────────┐                   ┌─────────────────┐
//! │ /tasks/{id}/    │   SSE stream     │   SseClient     │
//! │     events      │ ─────────────────▶ │                │
//! │                 │   TaskEvent       │ ─▶ IM Progress  │
//! └─────────────────┘                   └─────────────────┘
//! ```

use anyhow::{Context, Result};
use eventsource_client::{Client, SSE};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

// ============================================================================
// Task Event Types (matching TypeScript definitions)
// ============================================================================

/// Event types emitted during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TaskEvent {
    /// Thinking/reasoning event
    #[serde(rename = "thought")]
    Thought(String),

    /// Tool invocation event
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseData),

    /// Output text event
    #[serde(rename = "output")]
    Output(String),

    /// Permission confirmation request
    #[serde(rename = "confirmation")]
    Confirmation(ConfirmationData),

    /// Progress update event
    #[serde(rename = "progress")]
    Progress(ProgressData),

    /// Debug information event (triggered by @@debug)
    #[serde(rename = "debug_info")]
    DebugInfo(DebugInfoData),

    /// Agent information event (which agents were invoked)
    #[serde(rename = "agent_info")]
    AgentInfo(AgentInfoData),

    /// Skill use event (which skills were invoked)
    #[serde(rename = "skill_use")]
    SkillUse(SkillUseData),

    /// Task completion event (success or failure)
    #[serde(rename = "finish")]
    Finish(FinishData),
}

/// Tool use event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseData {
    pub tool: String,
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

/// Confirmation request data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationData {
    #[serde(rename = "requestID")]
    pub request_id: String,
    pub tool: String,
    pub description: String,
    pub args: serde_json::Value,
    pub actions: Vec<String>,
}

/// Progress update data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u32>,
}

/// Finish event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishData {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Debug information event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfoData {
    /// Model identifier (e.g., "claude-opus-4.5")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider (e.g., "anthropic", "openai")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Input tokens consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Output tokens generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Total tokens (may be provided by some providers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Request size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_bytes: Option<u64>,
    /// Response size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_bytes: Option<u64>,
}

/// Agent information event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfoData {
    /// Agent name (e.g., "build", "code-reviewer", "macro")
    pub agent: String,
    /// Display name for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Whether this is the primary agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Skill use event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUseData {
    /// Skill name (e.g., "tdd", "brainstorming", "frontend-design")
    pub skill: String,
    /// Arguments passed to the skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

// ============================================================================
// SSE Client
// ============================================================================

/// Configuration for the SSE client.
#[derive(Debug, Clone)]
pub struct SseClientConfig {
    /// CodeCoder API base endpoint (e.g., "http://127.0.0.1:4400")
    pub endpoint: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Maximum number of reconnection attempts
    pub max_retries: u32,
    /// Initial backoff duration for reconnection
    pub initial_backoff: Duration,
}

impl Default for SseClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:4400".to_string(),
            connect_timeout: Duration::from_secs(10),
            max_retries: 3,
            initial_backoff: Duration::from_secs(1),
        }
    }
}

/// SSE client for consuming task events from CodeCoder.
pub struct SseTaskClient {
    config: SseClientConfig,
}

impl SseTaskClient {
    /// Create a new SSE client with the given configuration.
    pub fn new(config: SseClientConfig) -> Self {
        Self { config }
    }

    /// Create a new SSE client with default configuration and custom endpoint.
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            config: SseClientConfig {
                endpoint: endpoint.into(),
                ..Default::default()
            },
        }
    }

    /// Subscribe to events for a specific task.
    ///
    /// Returns a channel receiver that yields `TaskEvent` values until
    /// the task completes or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID to subscribe to
    ///
    /// # Returns
    ///
    /// A tuple of (receiver, join_handle) where:
    /// - receiver yields `TaskEvent` values
    /// - join_handle can be awaited to get the final result
    pub async fn subscribe(
        &self,
        task_id: &str,
    ) -> Result<(mpsc::Receiver<TaskEvent>, tokio::task::JoinHandle<Result<()>>)> {
        let url = format!("{}/api/v1/tasks/{}/events", self.config.endpoint, task_id);
        let (tx, rx) = mpsc::channel(32);
        let max_retries = self.config.max_retries;
        let initial_backoff = self.config.initial_backoff;
        let task_id_owned = task_id.to_string();

        tracing::info!(
            task_id = %task_id,
            endpoint = %self.config.endpoint,
            max_retries = max_retries,
            "📡 Starting SSE subscription"
        );

        let handle = tokio::spawn(async move {
            let mut retries = 0;
            let mut backoff = initial_backoff;

            loop {
                match Self::connect_and_stream(&url, &tx).await {
                    Ok(finished) => {
                        if finished {
                            tracing::info!(task_id = %task_id_owned, "📡 SSE stream finished normally");
                            return Ok(());
                        }
                        // Stream ended without finish event, try reconnecting
                        retries += 1;
                        if retries > max_retries {
                            tracing::error!(
                                task_id = %task_id_owned,
                                retries = retries,
                                "Max SSE reconnection attempts exceeded"
                            );
                            return Err(anyhow::anyhow!("SSE connection failed after {} retries", max_retries));
                        }
                        tracing::warn!(
                            task_id = %task_id_owned,
                            retry = retries,
                            backoff_secs = backoff.as_secs(),
                            "SSE stream ended, reconnecting..."
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
                    }
                    Err(e) => {
                        retries += 1;
                        if retries > max_retries {
                            tracing::error!(
                                task_id = %task_id_owned,
                                error = %e,
                                "SSE connection error, max retries exceeded"
                            );
                            return Err(e);
                        }
                        tracing::warn!(
                            task_id = %task_id_owned,
                            error = %e,
                            retry = retries,
                            backoff_secs = backoff.as_secs(),
                            "SSE connection error, reconnecting..."
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
                    }
                }
            }
        });

        Ok((rx, handle))
    }

    /// Connect to SSE endpoint and stream events.
    ///
    /// Returns Ok(true) if the stream finished with a finish event,
    /// Ok(false) if the stream ended without a finish event,
    /// Err if there was a connection error.
    async fn connect_and_stream(url: &str, tx: &mpsc::Sender<TaskEvent>) -> Result<bool> {
        tracing::debug!(url = %url, "Connecting to SSE endpoint");

        let client = eventsource_client::ClientBuilder::for_url(url)
            .context("Failed to create SSE client")?
            .build();

        let mut stream = client.stream();
        let mut finished = false;
        let mut event_count = 0u32;

        while let Some(event) = stream.next().await {
            match event {
                Ok(SSE::Event(ev)) => {
                    event_count += 1;
                    // Parse the event data
                    match serde_json::from_str::<TaskEvent>(&ev.data) {
                        Ok(task_event) => {
                            // Check if this is a finish event
                            if matches!(&task_event, TaskEvent::Finish(_)) {
                                finished = true;
                            }

                            // Send to channel, stop if receiver is dropped
                            if tx.send(task_event).await.is_err() {
                                tracing::warn!(
                                    url = %url,
                                    event_count = event_count,
                                    "⚠️ SSE event receiver dropped, closing stream"
                                );
                                return Ok(finished);
                            }

                            if finished {
                                tracing::debug!(
                                    url = %url,
                                    event_count = event_count,
                                    "SSE stream completed with finish event"
                                );
                                return Ok(true);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                event_type = %ev.event_type,
                                data = %ev.data,
                                error = %e,
                                "Failed to parse SSE event data"
                            );
                        }
                    }
                }
                Ok(SSE::Comment(_)) => {
                    // Ignore comments (heartbeats)
                }
                Ok(SSE::Connected(_)) => {
                    // Connection established, ignore
                    tracing::debug!("SSE connection established");
                }
                Err(e) => {
                    tracing::error!(error = %e, "SSE stream error");
                    return Err(anyhow::anyhow!("SSE stream error: {}", e));
                }
            }
        }

        Ok(finished)
    }
}

// ============================================================================
// Task API Types
// ============================================================================

/// Request to create a new task.
#[derive(Debug, Clone, Serialize)]
pub struct CreateTaskRequest {
    /// Agent name to invoke (e.g., "macro", "decision", "build")
    pub agent: String,
    /// User prompt/request
    pub prompt: String,
    /// Remote context information
    pub context: TaskContext,
    /// Optional existing session ID for continuity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional model override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Remote task context.
#[derive(Debug, Clone, Serialize)]
pub struct TaskContext {
    /// User identifier from IM platform
    #[serde(rename = "userID")]
    pub user_id: String,
    /// Platform source (telegram, discord, etc.)
    pub platform: String,
    /// Conversation identifier for session continuity (e.g., "telegram:765318302")
    #[serde(rename = "conversationId", skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Marker for remote calls
    pub source: &'static str,
}

impl TaskContext {
    /// Create a new remote task context.
    ///
    /// # Arguments
    /// * `user_id` - User identifier from the IM platform
    /// * `channel_id` - Channel/chat identifier (may differ from user_id in group chats)
    /// * `platform` - Platform name (telegram, discord, slack, etc.)
    pub fn new(
        user_id: impl Into<String>,
        channel_id: impl Into<String>,
        platform: impl Into<String>,
    ) -> Self {
        let plat = platform.into();
        let chan = channel_id.into();
        // Generate conversation_id from platform and channel_id for session continuity
        // Format: "platform:channel_id" (e.g., "telegram:765318302")
        let conversation_id = format!("{}:{}", plat, chan);
        Self {
            user_id: user_id.into(),
            platform: plat,
            conversation_id: Some(conversation_id),
            source: "remote",
        }
    }
}

/// Response from creating a task.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskResponse {
    pub success: bool,
    pub data: Option<TaskData>,
    pub error: Option<String>,
}

/// Task data from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskData {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub status: String,
    pub agent: String,
    pub prompt: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ────────────────────────────────────────────────────────────────────────────
    // TaskEvent Parsing Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_thought_event() {
        let json = r#"{"type": "thought", "data": "Analyzing the PMI data..."}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, TaskEvent::Thought(s) if s == "Analyzing the PMI data..."));
    }

    #[test]
    fn test_parse_thought_event_empty() {
        let json = r#"{"type": "thought", "data": ""}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, TaskEvent::Thought(s) if s.is_empty()));
    }

    #[test]
    fn test_parse_thought_event_chinese() {
        let json = r#"{"type": "thought", "data": "我正在分析数据..."}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, TaskEvent::Thought(s) if s == "我正在分析数据..."));
    }

    #[test]
    fn test_parse_output_event() {
        let json = r#"{"type": "output", "data": "Here is the result..."}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, TaskEvent::Output(s) if s == "Here is the result..."));
    }

    #[test]
    fn test_parse_output_event_multiline() {
        let json = r#"{"type": "output", "data": "Line 1\nLine 2\nLine 3"}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Output(s) => {
                assert!(s.contains("Line 1"));
                assert!(s.contains("Line 2"));
                assert!(s.contains("Line 3"));
            }
            _ => panic!("Expected Output event"),
        }
    }

    #[test]
    fn test_parse_tool_use_event() {
        let json = r#"{
            "type": "tool_use",
            "data": {
                "tool": "web_search",
                "args": {"query": "PMI data 2026"},
                "result": null
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::ToolUse(data) => {
                assert_eq!(data.tool, "web_search");
                assert!(data.result.is_none());
            }
            _ => panic!("Expected ToolUse event"),
        }
    }

    #[test]
    fn test_parse_tool_use_event_with_result() {
        let json = r#"{
            "type": "tool_use",
            "data": {
                "tool": "Read",
                "args": {"file_path": "/path/to/file.txt"},
                "result": {"content": "file contents here"}
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::ToolUse(data) => {
                assert_eq!(data.tool, "Read");
                assert!(data.result.is_some());
                let result = data.result.unwrap();
                assert_eq!(result["content"], "file contents here");
            }
            _ => panic!("Expected ToolUse event"),
        }
    }

    #[test]
    fn test_parse_progress_event() {
        let json = r#"{
            "type": "progress",
            "data": {
                "stage": "processing",
                "message": "Processing with macro agent...",
                "percentage": 50
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Progress(data) => {
                assert_eq!(data.stage, "processing");
                assert_eq!(data.message, "Processing with macro agent...");
                assert_eq!(data.percentage, Some(50));
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_parse_progress_event_without_percentage() {
        let json = r#"{
            "type": "progress",
            "data": {
                "stage": "starting",
                "message": "Initializing..."
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Progress(data) => {
                assert_eq!(data.stage, "starting");
                assert!(data.percentage.is_none());
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_parse_confirmation_event() {
        let json = r#"{
            "type": "confirmation",
            "data": {
                "requestID": "req-123",
                "tool": "Bash",
                "description": "Execute rm -rf command",
                "args": {"command": "rm -rf /tmp/test"},
                "actions": ["allow", "deny", "always_allow"]
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Confirmation(data) => {
                assert_eq!(data.request_id, "req-123");
                assert_eq!(data.tool, "Bash");
                assert_eq!(data.description, "Execute rm -rf command");
                assert_eq!(data.actions.len(), 3);
                assert!(data.actions.contains(&"allow".to_string()));
            }
            _ => panic!("Expected Confirmation event"),
        }
    }

    #[test]
    fn test_parse_debug_info_event() {
        let json = r#"{
            "type": "debug_info",
            "data": {
                "model": "claude-opus-4.5",
                "provider": "anthropic",
                "input_tokens": 1234,
                "output_tokens": 567,
                "duration_ms": 1250
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::DebugInfo(data) => {
                assert_eq!(data.model, Some("claude-opus-4.5".to_string()));
                assert_eq!(data.provider, Some("anthropic".to_string()));
                assert_eq!(data.input_tokens, Some(1234));
                assert_eq!(data.output_tokens, Some(567));
                assert_eq!(data.duration_ms, Some(1250));
            }
            _ => panic!("Expected DebugInfo event"),
        }
    }

    #[test]
    fn test_parse_debug_info_event_minimal() {
        let json = r#"{
            "type": "debug_info",
            "data": {}
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::DebugInfo(data) => {
                assert!(data.model.is_none());
                assert!(data.provider.is_none());
                assert!(data.input_tokens.is_none());
                assert!(data.output_tokens.is_none());
            }
            _ => panic!("Expected DebugInfo event"),
        }
    }

    #[test]
    fn test_parse_debug_info_event_with_bytes() {
        let json = r#"{
            "type": "debug_info",
            "data": {
                "model": "claude-sonnet-4.5",
                "request_bytes": 5000,
                "response_bytes": 10000,
                "total_tokens": 2000
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::DebugInfo(data) => {
                assert_eq!(data.model, Some("claude-sonnet-4.5".to_string()));
                assert_eq!(data.request_bytes, Some(5000));
                assert_eq!(data.response_bytes, Some(10000));
                assert_eq!(data.total_tokens, Some(2000));
            }
            _ => panic!("Expected DebugInfo event"),
        }
    }

    #[test]
    fn test_parse_agent_info_event() {
        let json = r#"{
            "type": "agent_info",
            "data": {
                "agent": "macro",
                "display_name": "Macro Economist",
                "is_primary": true,
                "duration_ms": 5000
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::AgentInfo(data) => {
                assert_eq!(data.agent, "macro");
                assert_eq!(data.display_name, Some("Macro Economist".to_string()));
                assert_eq!(data.is_primary, Some(true));
                assert_eq!(data.duration_ms, Some(5000));
            }
            _ => panic!("Expected AgentInfo event"),
        }
    }

    #[test]
    fn test_parse_agent_info_event_minimal() {
        let json = r#"{
            "type": "agent_info",
            "data": {
                "agent": "build"
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::AgentInfo(data) => {
                assert_eq!(data.agent, "build");
                assert!(data.display_name.is_none());
                assert!(data.is_primary.is_none());
                assert!(data.duration_ms.is_none());
            }
            _ => panic!("Expected AgentInfo event"),
        }
    }

    #[test]
    fn test_parse_finish_event() {
        let json = r#"{
            "type": "finish",
            "data": {
                "success": true,
                "output": "PMI analysis complete...",
                "error": null
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Finish(data) => {
                assert!(data.success);
                assert!(data.output.is_some());
                assert!(data.error.is_none());
            }
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn test_parse_finish_event_failure() {
        let json = r#"{
            "type": "finish",
            "data": {
                "success": false,
                "output": null,
                "error": "Rate limit exceeded"
            }
        }"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        match event {
            TaskEvent::Finish(data) => {
                assert!(!data.success);
                assert!(data.output.is_none());
                assert_eq!(data.error, Some("Rate limit exceeded".to_string()));
            }
            _ => panic!("Expected Finish event"),
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // TaskContext Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_create_task_context() {
        let ctx = TaskContext::new("user123", "channel456", "telegram");
        assert_eq!(ctx.user_id, "user123");
        assert_eq!(ctx.platform, "telegram");
        assert_eq!(ctx.source, "remote");
    }

    #[test]
    fn test_task_context_conversation_id() {
        let ctx = TaskContext::new("user123", "channel456", "telegram");
        assert_eq!(ctx.conversation_id, Some("telegram:channel456".to_string()));
    }

    #[test]
    fn test_task_context_different_platforms() {
        let telegram = TaskContext::new("user1", "chat1", "telegram");
        assert_eq!(telegram.conversation_id, Some("telegram:chat1".to_string()));

        let discord = TaskContext::new("user2", "guild#channel", "discord");
        assert_eq!(discord.conversation_id, Some("discord:guild#channel".to_string()));

        let slack = TaskContext::new("user3", "C123456", "slack");
        assert_eq!(slack.conversation_id, Some("slack:C123456".to_string()));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // SseClientConfig Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_sse_client_config_default() {
        let config = SseClientConfig::default();
        assert_eq!(config.endpoint, "http://127.0.0.1:4400");
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff, Duration::from_secs(1));
    }

    #[test]
    fn test_sse_task_client_with_endpoint() {
        let client = SseTaskClient::with_endpoint("http://localhost:8080");
        assert_eq!(client.config.endpoint, "http://localhost:8080");
        // Other config values should be default
        assert_eq!(client.config.max_retries, 3);
    }

    #[test]
    fn test_sse_task_client_new() {
        let config = SseClientConfig {
            endpoint: "https://api.example.com".to_string(),
            connect_timeout: Duration::from_secs(30),
            max_retries: 5,
            initial_backoff: Duration::from_secs(2),
        };
        let client = SseTaskClient::new(config);
        assert_eq!(client.config.endpoint, "https://api.example.com");
        assert_eq!(client.config.max_retries, 5);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // CreateTaskRequest Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_create_task_request_serialization() {
        let request = CreateTaskRequest {
            agent: "macro".into(),
            prompt: "Analyze PMI data".into(),
            context: TaskContext::new("user1", "chat1", "telegram"),
            session_id: None,
            model: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"agent\":\"macro\""));
        assert!(json.contains("\"prompt\":\"Analyze PMI data\""));
        assert!(json.contains("\"source\":\"remote\""));
        assert!(!json.contains("\"session_id\""));
        assert!(!json.contains("\"model\""));
    }

    #[test]
    fn test_create_task_request_with_session() {
        let request = CreateTaskRequest {
            agent: "build".into(),
            prompt: "Fix the bug".into(),
            context: TaskContext::new("user1", "chat1", "telegram"),
            session_id: Some("session-abc-123".into()),
            model: Some("claude-opus-4.5".into()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"session_id\":\"session-abc-123\""));
        assert!(json.contains("\"model\":\"claude-opus-4.5\""));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // CreateTaskResponse Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_create_task_response_success() {
        let json = r#"{
            "success": true,
            "data": {
                "id": "task-123",
                "sessionID": "session-456",
                "status": "pending",
                "agent": "macro",
                "prompt": "Test prompt",
                "createdAt": "2026-02-28T12:00:00Z",
                "updatedAt": "2026-02-28T12:00:00Z"
            }
        }"#;

        let response: CreateTaskResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());

        let data = response.data.unwrap();
        assert_eq!(data.id, "task-123");
        assert_eq!(data.session_id, "session-456");
        assert_eq!(data.status, "pending");
        assert_eq!(data.agent, "macro");
    }

    #[test]
    fn test_create_task_response_error() {
        let json = r#"{
            "success": false,
            "error": "Invalid agent name"
        }"#;

        let response: CreateTaskResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Invalid agent name".to_string()));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Data Structure Serialization Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_tool_use_data_serialization() {
        let data = ToolUseData {
            tool: "Read".into(),
            args: serde_json::json!({"file_path": "/test.txt"}),
            result: None,
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"tool\":\"Read\""));
        assert!(json.contains("\"file_path\""));
        assert!(!json.contains("\"result\""));

        let data_with_result = ToolUseData {
            tool: "Read".into(),
            args: serde_json::json!({"file_path": "/test.txt"}),
            result: Some(serde_json::json!({"content": "file data"})),
        };

        let json = serde_json::to_string(&data_with_result).unwrap();
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_progress_data_serialization() {
        let data = ProgressData {
            stage: "processing".into(),
            message: "Working...".into(),
            percentage: Some(75),
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"stage\":\"processing\""));
        assert!(json.contains("\"percentage\":75"));
    }

    #[test]
    fn test_finish_data_serialization() {
        let success_data = FinishData {
            success: true,
            output: Some("Done!".into()),
            error: None,
        };

        let json = serde_json::to_string(&success_data).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"output\":\"Done!\""));
        assert!(!json.contains("\"error\""));

        let error_data = FinishData {
            success: false,
            output: None,
            error: Some("Failed".into()),
        };

        let json = serde_json::to_string(&error_data).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Failed\""));
    }
}
