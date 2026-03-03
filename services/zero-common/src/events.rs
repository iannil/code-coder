//! Task Event Types for Event Sourcing
//!
//! Defines all event types that flow through the task execution system.
//! These events are stored in Redis Streams and can be replayed for state reconstruction.
//!
//! # Event Flow
//!
//! ```text
//! IM Message → TaskCreated → TaskStarted → [Events...] → TaskCompleted/TaskFailed
//!                                              ↓
//!                              Thought, ToolUse, Progress, Output,
//!                              Confirmation, AgentSwitch, Heartbeat
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

// ============================================================================
// Task Event Types
// ============================================================================

/// All task event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    /// Task has been created and queued.
    TaskCreated(TaskCreatedData),

    /// Task execution has started.
    TaskStarted(TaskStartedData),

    /// Agent is thinking/reasoning.
    Thought(ThoughtData),

    /// Tool invocation.
    ToolUse(ToolUseData),

    /// Progress update.
    Progress(ProgressData),

    /// Text output from the agent.
    Output(OutputData),

    /// Waiting for user confirmation.
    Confirmation(ConfirmationData),

    /// Agent switching to another agent.
    AgentSwitch(AgentSwitchData),

    /// Heartbeat to indicate task is still alive.
    Heartbeat(HeartbeatData),

    /// Debug information.
    DebugInfo(DebugInfoData),

    /// Agent information.
    AgentInfo(AgentInfoData),

    /// Skill usage.
    SkillUse(SkillUseData),

    /// Task completed successfully.
    TaskCompleted(TaskCompletedData),

    /// Task failed.
    TaskFailed(TaskFailedData),
}

impl TaskEvent {
    /// Get the event type as a string.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TaskCreated(_) => "task_created",
            Self::TaskStarted(_) => "task_started",
            Self::Thought(_) => "thought",
            Self::ToolUse(_) => "tool_use",
            Self::Progress(_) => "progress",
            Self::Output(_) => "output",
            Self::Confirmation(_) => "confirmation",
            Self::AgentSwitch(_) => "agent_switch",
            Self::Heartbeat(_) => "heartbeat",
            Self::DebugInfo(_) => "debug_info",
            Self::AgentInfo(_) => "agent_info",
            Self::SkillUse(_) => "skill_use",
            Self::TaskCompleted(_) => "task_completed",
            Self::TaskFailed(_) => "task_failed",
        }
    }

    /// Check if this is a terminal event (task_completed or task_failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::TaskCompleted(_) | Self::TaskFailed(_))
    }
}

// ============================================================================
// Event Data Types
// ============================================================================

/// Task created event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskCreatedData {
    /// Task ID.
    pub task_id: String,
    /// User ID.
    pub user_id: String,
    /// Channel type (telegram, discord, etc.).
    pub channel: String,
    /// Channel/chat ID.
    pub channel_id: String,
    /// User prompt.
    pub prompt: String,
    /// Agent name.
    pub agent: String,
    /// Trace ID for distributed tracing.
    pub trace_id: String,
    /// Message history for context.
    #[serde(default)]
    pub chat_history: Vec<serde_json::Value>,
}

/// Task started event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskStartedData {
    /// Agent executing the task.
    pub agent: String,
    /// Session ID.
    pub session_id: String,
    /// Trace ID.
    pub trace_id: String,
}

/// Thought event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct ThoughtData {
    /// Thought content.
    pub content: String,
}

/// Tool use event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct ToolUseData {
    /// Tool name.
    pub tool: String,
    /// Tool arguments.
    pub args: serde_json::Value,
    /// Tool result (if completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether this is a result tool (vs intermediate).
    #[serde(default)]
    pub is_result_tool: bool,
}

/// Progress event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct ProgressData {
    /// Current stage.
    pub stage: String,
    /// Progress message.
    pub message: String,
    /// Percentage (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u8>,
}

/// Output event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct OutputData {
    /// Output content.
    pub content: String,
    /// Whether this is a partial output (streaming).
    #[serde(default)]
    pub is_partial: bool,
}

/// Confirmation request event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct ConfirmationData {
    /// Unique request ID.
    pub request_id: String,
    /// Tool requiring confirmation.
    pub tool: String,
    /// Human-readable description.
    pub description: String,
    /// Tool arguments.
    pub args: serde_json::Value,
    /// Available actions (once, always, reject).
    pub actions: Vec<String>,
}

/// Agent switch event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct AgentSwitchData {
    /// Previous agent.
    pub from: String,
    /// New agent.
    pub to: String,
    /// Reason for switch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Heartbeat event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct HeartbeatData {
    /// Current stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
}

/// Debug info event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct DebugInfoData {
    /// Model name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Total tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Request bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_bytes: Option<u64>,
    /// Response bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_bytes: Option<u64>,
}

/// Agent info event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct AgentInfoData {
    /// Agent name.
    pub agent: String,
    /// Display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Whether this is the primary agent.
    #[serde(default)]
    pub is_primary: bool,
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Skill use event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct SkillUseData {
    /// Skill name.
    pub skill: String,
    /// Skill arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Task completed event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskCompletedData {
    /// Final output.
    pub output: String,
    /// Summary (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Token usage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TaskUsage>,
}

/// Task failed event data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskFailedData {
    /// Error message.
    pub error: String,
    /// Whether the task can be recovered.
    #[serde(default)]
    pub recoverable: bool,
    /// Error code (for categorization).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskUsage {
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

// ============================================================================
// Stream Event Envelope
// ============================================================================

/// Event envelope for Redis Stream storage.
///
/// Contains the event plus metadata for stream operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct StreamEvent {
    /// Monotonically increasing sequence number (per task).
    pub seq: u64,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// The actual event.
    pub event: TaskEvent,
    /// Trace ID for distributed tracing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Span ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl StreamEvent {
    /// Create a new stream event.
    pub fn new(seq: u64, event: TaskEvent) -> Self {
        Self {
            seq,
            timestamp: Utc::now(),
            event,
            trace_id: None,
            span_id: None,
        }
    }

    /// Add trace context.
    pub fn with_trace_context(mut self, trace_id: String, span_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self.span_id = Some(span_id);
        self
    }
}

// ============================================================================
// Task State Projection
// ============================================================================

/// Task state projection (materialized from events).
///
/// This is stored in Redis Hash for quick access.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
pub struct TaskState {
    /// Task ID.
    pub task_id: String,
    /// Current status.
    pub status: TaskStatus,
    /// Current agent.
    pub current_agent: Option<String>,
    /// Progress percentage (0-100).
    pub progress_pct: u8,
    /// Last event sequence number.
    pub last_event_seq: u64,
    /// Partial output buffer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_buffer: Option<String>,
    /// Task start time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
    /// Last heartbeat time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<DateTime<Utc>>,
    /// Error message (if failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Task status values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "events/"))]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued but not started.
    #[default]
    Pending,
    /// Task is currently running.
    Running,
    /// Task is waiting for user confirmation.
    AwaitingApproval,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
}

impl TaskState {
    /// Create a new task state.
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            status: TaskStatus::Pending,
            updated_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Apply an event to update the state.
    pub fn apply(&mut self, event: &StreamEvent) {
        self.last_event_seq = event.seq;
        self.updated_at = event.timestamp;

        match &event.event {
            TaskEvent::TaskStarted(data) => {
                self.status = TaskStatus::Running;
                self.current_agent = Some(data.agent.clone());
                self.started_at = Some(event.timestamp);
            }
            TaskEvent::Progress(data) => {
                if let Some(pct) = data.percentage {
                    self.progress_pct = pct;
                }
            }
            TaskEvent::Output(data) => {
                if data.is_partial {
                    // Append to buffer
                    let buffer = self.output_buffer.get_or_insert_with(String::new);
                    buffer.push_str(&data.content);
                } else {
                    // Final output
                    self.output_buffer = Some(data.content.clone());
                }
            }
            TaskEvent::Confirmation(_) => {
                self.status = TaskStatus::AwaitingApproval;
            }
            TaskEvent::AgentSwitch(data) => {
                self.current_agent = Some(data.to.clone());
            }
            TaskEvent::Heartbeat(_) => {
                self.last_heartbeat = Some(event.timestamp);
            }
            TaskEvent::TaskCompleted(_) => {
                self.status = TaskStatus::Completed;
                self.progress_pct = 100;
            }
            TaskEvent::TaskFailed(data) => {
                self.status = TaskStatus::Failed;
                self.error = Some(data.error.clone());
            }
            _ => {}
        }
    }

    /// Convert state to Redis hash fields.
    pub fn to_hash_fields(&self) -> Vec<(String, String)> {
        let mut fields = vec![
            ("task_id".to_string(), self.task_id.clone()),
            ("status".to_string(), format!("{:?}", self.status).to_lowercase()),
            ("progress_pct".to_string(), self.progress_pct.to_string()),
            ("last_event_seq".to_string(), self.last_event_seq.to_string()),
            ("updated_at".to_string(), self.updated_at.to_rfc3339()),
        ];

        if let Some(agent) = &self.current_agent {
            fields.push(("current_agent".to_string(), agent.clone()));
        }
        if let Some(output) = &self.output_buffer {
            fields.push(("output_buffer".to_string(), output.clone()));
        }
        if let Some(started) = &self.started_at {
            fields.push(("started_at".to_string(), started.to_rfc3339()));
        }
        if let Some(hb) = &self.last_heartbeat {
            fields.push(("last_heartbeat".to_string(), hb.to_rfc3339()));
        }
        if let Some(err) = &self.error {
            fields.push(("error".to_string(), err.clone()));
        }

        fields
    }

    /// Create state from Redis hash fields.
    pub fn from_hash_fields(fields: HashMap<String, String>) -> Self {
        let mut state = Self::default();

        if let Some(v) = fields.get("task_id") {
            state.task_id = v.clone();
        }
        if let Some(v) = fields.get("status") {
            state.status = match v.as_str() {
                "pending" => TaskStatus::Pending,
                "running" => TaskStatus::Running,
                "awaiting_approval" => TaskStatus::AwaitingApproval,
                "completed" => TaskStatus::Completed,
                "failed" => TaskStatus::Failed,
                _ => TaskStatus::Pending,
            };
        }
        if let Some(v) = fields.get("progress_pct") {
            state.progress_pct = v.parse().unwrap_or(0);
        }
        if let Some(v) = fields.get("last_event_seq") {
            state.last_event_seq = v.parse().unwrap_or(0);
        }
        if let Some(v) = fields.get("current_agent") {
            state.current_agent = Some(v.clone());
        }
        if let Some(v) = fields.get("output_buffer") {
            state.output_buffer = Some(v.clone());
        }
        if let Some(v) = fields.get("started_at") {
            state.started_at = DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.with_timezone(&Utc));
        }
        if let Some(v) = fields.get("updated_at") {
            state.updated_at = DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
        }
        if let Some(v) = fields.get("last_heartbeat") {
            state.last_heartbeat = DateTime::parse_from_rfc3339(v)
                .ok()
                .map(|dt| dt.with_timezone(&Utc));
        }
        if let Some(v) = fields.get("error") {
            state.error = Some(v.clone());
        }

        state
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_event_serialization() {
        let event = TaskEvent::Progress(ProgressData {
            stage: "analyzing".to_string(),
            message: "Processing file".to_string(),
            percentage: Some(50),
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"percentage\":50"));

        let parsed: TaskEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn test_task_event_type() {
        assert_eq!(
            TaskEvent::TaskStarted(TaskStartedData {
                agent: "test".to_string(),
                session_id: "s".to_string(),
                trace_id: "t".to_string(),
            })
            .event_type(),
            "task_started"
        );

        assert_eq!(
            TaskEvent::TaskCompleted(TaskCompletedData {
                output: "done".to_string(),
                summary: None,
                usage: None,
            })
            .event_type(),
            "task_completed"
        );
    }

    #[test]
    fn test_task_event_is_terminal() {
        assert!(!TaskEvent::Progress(ProgressData {
            stage: "test".to_string(),
            message: "msg".to_string(),
            percentage: None,
        })
        .is_terminal());

        assert!(TaskEvent::TaskCompleted(TaskCompletedData {
            output: "done".to_string(),
            summary: None,
            usage: None,
        })
        .is_terminal());

        assert!(TaskEvent::TaskFailed(TaskFailedData {
            error: "error".to_string(),
            recoverable: false,
            code: None,
        })
        .is_terminal());
    }

    #[test]
    fn test_stream_event() {
        let event = TaskEvent::Thought(ThoughtData {
            content: "thinking...".to_string(),
        });

        let stream_event = StreamEvent::new(1, event)
            .with_trace_context("trace-123".to_string(), "span-456".to_string());

        assert_eq!(stream_event.seq, 1);
        assert_eq!(stream_event.trace_id, Some("trace-123".to_string()));
        assert_eq!(stream_event.span_id, Some("span-456".to_string()));
    }

    #[test]
    fn test_task_state_apply() {
        let mut state = TaskState::new("task-123".to_string());
        assert_eq!(state.status, TaskStatus::Pending);

        // Apply started event
        let started_event = StreamEvent::new(
            1,
            TaskEvent::TaskStarted(TaskStartedData {
                agent: "code-reviewer".to_string(),
                session_id: "session-1".to_string(),
                trace_id: "trace-1".to_string(),
            }),
        );
        state.apply(&started_event);
        assert_eq!(state.status, TaskStatus::Running);
        assert_eq!(state.current_agent, Some("code-reviewer".to_string()));
        assert_eq!(state.last_event_seq, 1);

        // Apply progress event
        let progress_event = StreamEvent::new(
            2,
            TaskEvent::Progress(ProgressData {
                stage: "analyzing".to_string(),
                message: "50% done".to_string(),
                percentage: Some(50),
            }),
        );
        state.apply(&progress_event);
        assert_eq!(state.progress_pct, 50);
        assert_eq!(state.last_event_seq, 2);

        // Apply completed event
        let completed_event = StreamEvent::new(
            3,
            TaskEvent::TaskCompleted(TaskCompletedData {
                output: "All done!".to_string(),
                summary: None,
                usage: None,
            }),
        );
        state.apply(&completed_event);
        assert_eq!(state.status, TaskStatus::Completed);
        assert_eq!(state.progress_pct, 100);
    }

    #[test]
    fn test_task_state_hash_conversion() {
        let mut state = TaskState::new("task-123".to_string());
        state.status = TaskStatus::Running;
        state.current_agent = Some("test-agent".to_string());
        state.progress_pct = 75;
        state.last_event_seq = 10;

        // Convert to hash fields
        let fields = state.to_hash_fields();
        assert!(fields.iter().any(|(k, v)| k == "task_id" && v == "task-123"));
        assert!(fields.iter().any(|(k, v)| k == "status" && v == "running"));
        assert!(fields.iter().any(|(k, v)| k == "progress_pct" && v == "75"));

        // Convert back from hash
        let field_map: HashMap<String, String> = fields.into_iter().collect();
        let restored = TaskState::from_hash_fields(field_map);

        assert_eq!(restored.task_id, state.task_id);
        assert_eq!(restored.status, state.status);
        assert_eq!(restored.current_agent, state.current_agent);
        assert_eq!(restored.progress_pct, state.progress_pct);
    }
}
