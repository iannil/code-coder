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

        let handle = tokio::spawn(async move {
            let mut retries = 0;
            let mut backoff = initial_backoff;

            loop {
                match Self::connect_and_stream(&url, &tx).await {
                    Ok(finished) => {
                        if finished {
                            tracing::info!(task_id = %task_id_owned, "SSE stream finished normally");
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
        let client = eventsource_client::ClientBuilder::for_url(url)
            .context("Failed to create SSE client")?
            .build();

        let mut stream = client.stream();
        let mut finished = false;

        while let Some(event) = stream.next().await {
            match event {
                Ok(SSE::Event(ev)) => {
                    // Parse the event data
                    match serde_json::from_str::<TaskEvent>(&ev.data) {
                        Ok(task_event) => {
                            // Check if this is a finish event
                            if matches!(&task_event, TaskEvent::Finish(_)) {
                                finished = true;
                            }

                            // Send to channel, stop if receiver is dropped
                            if tx.send(task_event).await.is_err() {
                                tracing::debug!("SSE event receiver dropped");
                                return Ok(finished);
                            }

                            if finished {
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

    #[test]
    fn test_parse_thought_event() {
        let json = r#"{"type": "thought", "data": "Analyzing the PMI data..."}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, TaskEvent::Thought(s) if s == "Analyzing the PMI data..."));
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
                assert_eq!(data.percentage, Some(50));
            }
            _ => panic!("Expected Progress event"),
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
            }
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn test_create_task_context() {
        let ctx = TaskContext::new("user123", "telegram");
        assert_eq!(ctx.user_id, "user123");
        assert_eq!(ctx.platform, "telegram");
        assert_eq!(ctx.source, "remote");
    }
}
