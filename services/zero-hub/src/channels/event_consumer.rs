//! Unified Event Consumer for Redis Streams
//!
//! This module provides a unified interface for consuming task events from
//! Redis Streams and dispatching them to IM progress handlers.
//!
//! # Architecture
//!
//! ```text
//! Redis Stream (tasks:events:{id})
//!         │
//!         ▼
//! ┌───────────────────┐
//! │  EventConsumer    │ ←── Converts StreamEvent to SSE TaskEvent
//! └─────────┬─────────┘
//!           │
//!           ▼
//! ┌───────────────────┐
//! │ ImProgressHandler │ ←── Existing handler (unchanged)
//! └───────────────────┘
//!           │
//!           ▼
//!     IM Platform
//! ```
//!
//! # Event Conversion
//!
//! Redis Streams events (from `zero_core::common::events`) are converted to
//! SSE-compatible events (from `crate::sse`) for the progress handler.

use super::checkpoint::CheckpointManager;
use super::message::ChannelMessage;
use super::progress::{ImProgressHandler, ProgressHandler};
use super::sse::{
    AgentInfoData as SseAgentInfoData, ConfirmationData as SseConfirmationData,
    DebugInfoData as SseDebugInfoData, FinishData, ProgressData as SseProgressData,
    SkillUseData as SseSkillUseData, TaskEvent as SseTaskEvent, ToolUseData as SseToolUseData,
};
use anyhow::Result;
use std::sync::Arc;
use zero_core::common::{
    stream_keys, RedisStreamClient, StreamEvent, TaskEvent as CommonTaskEvent,
};

// ============================================================================
// Event Conversion
// ============================================================================

/// Convert a Redis Streams TaskEvent to an SSE TaskEvent.
///
/// This bridges the gap between the canonical event format in `zero_common`
/// and the SSE-specific format used by `ImProgressHandler`.
pub fn convert_to_sse_event(event: &CommonTaskEvent) -> Option<SseTaskEvent> {
    match event {
        CommonTaskEvent::Thought(data) => Some(SseTaskEvent::Thought(data.content.clone())),

        CommonTaskEvent::ToolUse(data) => Some(SseTaskEvent::ToolUse(SseToolUseData {
            tool: data.tool.clone(),
            args: data.args.clone(),
            result: data.result.clone(),
        })),

        CommonTaskEvent::Progress(data) => Some(SseTaskEvent::Progress(SseProgressData {
            stage: data.stage.clone(),
            message: data.message.clone(),
            percentage: data.percentage.map(|p| p as u32),
        })),

        CommonTaskEvent::Output(data) => Some(SseTaskEvent::Output(data.content.clone())),

        CommonTaskEvent::Confirmation(data) => {
            Some(SseTaskEvent::Confirmation(SseConfirmationData {
                request_id: data.request_id.clone(),
                tool: data.tool.clone(),
                description: data.description.clone(),
                args: data.args.clone(),
                actions: data.actions.clone(),
            }))
        }

        CommonTaskEvent::DebugInfo(data) => Some(SseTaskEvent::DebugInfo(SseDebugInfoData {
            model: data.model.clone(),
            provider: data.provider.clone(),
            input_tokens: data.input_tokens,
            output_tokens: data.output_tokens,
            total_tokens: data.total_tokens,
            duration_ms: data.duration_ms,
            request_bytes: data.request_bytes,
            response_bytes: data.response_bytes,
        })),

        CommonTaskEvent::AgentInfo(data) => Some(SseTaskEvent::AgentInfo(SseAgentInfoData {
            agent: data.agent.clone(),
            display_name: data.display_name.clone(),
            is_primary: Some(data.is_primary),
            duration_ms: data.duration_ms,
        })),

        CommonTaskEvent::SkillUse(data) => Some(SseTaskEvent::SkillUse(SseSkillUseData {
            skill: data.skill.clone(),
            args: data.args.clone(),
            duration_ms: data.duration_ms,
        })),

        CommonTaskEvent::TaskCompleted(data) => Some(SseTaskEvent::Finish(FinishData {
            success: true,
            output: Some(data.output.clone()),
            error: None,
        })),

        CommonTaskEvent::TaskFailed(data) => Some(SseTaskEvent::Finish(FinishData {
            success: false,
            output: None,
            error: Some(data.error.clone()),
        })),

        // Events that don't have SSE equivalents
        CommonTaskEvent::TaskCreated(_) => None,
        CommonTaskEvent::TaskStarted(_) => None,
        CommonTaskEvent::AgentSwitch(_) => None,
        CommonTaskEvent::Heartbeat(_) => None,
    }
}

// ============================================================================
// Event Consumer
// ============================================================================

/// Configuration for the event consumer.
#[derive(Debug, Clone)]
pub struct EventConsumerConfig {
    /// Read block timeout in milliseconds.
    pub block_ms: u64,
    /// Maximum events per read.
    pub count: usize,
    /// Checkpoint save interval (events).
    pub checkpoint_interval: u64,
}

impl Default for EventConsumerConfig {
    fn default() -> Self {
        Self {
            block_ms: 5000,      // 5 second block
            count: 100,         // 100 events per read
            checkpoint_interval: 10, // Save checkpoint every 10 events
        }
    }
}

/// Unified event consumer for Redis Streams.
///
/// Subscribes to task event streams and dispatches events to the
/// IM progress handler, handling checkpointing and recovery.
pub struct EventConsumer {
    client: Arc<RedisStreamClient>,
    checkpoint_mgr: CheckpointManager,
    config: EventConsumerConfig,
}

impl EventConsumer {
    /// Create a new event consumer.
    pub fn new(client: Arc<RedisStreamClient>) -> Self {
        Self {
            checkpoint_mgr: CheckpointManager::new(client.clone()),
            client,
            config: EventConsumerConfig::default(),
        }
    }

    /// Set the consumer configuration.
    pub fn with_config(mut self, config: EventConsumerConfig) -> Self {
        self.config = config;
        self
    }

    /// Subscribe to a task's event stream and process events.
    ///
    /// This method:
    /// 1. Loads checkpoint for resume support
    /// 2. Reads events from the task's stream
    /// 3. Converts and dispatches to the progress handler
    /// 4. Saves checkpoints periodically
    /// 5. Clears checkpoint on completion
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID to subscribe to
    /// * `message` - The original channel message (for handler context)
    /// * `handler` - The IM progress handler
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the task completed, `Ok(false)` if interrupted,
    /// `Err` on fatal error.
    pub async fn subscribe(
        &self,
        task_id: &str,
        message: &ChannelMessage,
        handler: &ImProgressHandler,
    ) -> Result<bool> {
        let stream_key = stream_keys::task_events(task_id);

        // Load checkpoint for resume
        let checkpoint = self.checkpoint_mgr.load(task_id).await?;
        let mut last_id = checkpoint.last_id.clone();
        let mut event_count = checkpoint.event_count;

        tracing::info!(
            task_id = %task_id,
            trace_id = %message.trace_id,
            last_id = %last_id,
            event_count = event_count,
            "📡 Starting event stream subscription (Redis Streams)"
        );

        // Notify handler of task start
        handler.on_start(message, task_id).await?;

        loop {
            // Read events from stream
            let events = self
                .client
                .xread(&stream_key, &last_id, self.config.count, Some(self.config.block_ms))
                .await?;

            if events.is_empty() {
                // Timeout, no new events - check if task is still running
                // In a real implementation, we'd check task state here
                continue;
            }

            for msg in events {
                // Parse the event from fields
                let stream_id = msg.id.clone();
                let event_json = msg.fields.get("event").cloned().unwrap_or_default();

                let stream_event: StreamEvent = match serde_json::from_str(&event_json) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(
                            task_id = %task_id,
                            stream_id = %stream_id,
                            error = %e,
                            "Failed to parse stream event"
                        );
                        continue;
                    }
                };

                // Convert to SSE event
                if let Some(sse_event) = convert_to_sse_event(&stream_event.event) {
                    // Dispatch to handler
                    let is_terminal = handler.handle_event(message, sse_event).await?;

                    // Update tracking
                    last_id = stream_id.clone();
                    event_count += 1;

                    // Save checkpoint periodically
                    if event_count % self.config.checkpoint_interval == 0 {
                        self.checkpoint_mgr
                            .update(task_id, &last_id, true)
                            .await?;
                    }

                    // Check if task completed
                    if is_terminal {
                        tracing::info!(
                            task_id = %task_id,
                            trace_id = %message.trace_id,
                            total_events = event_count,
                            "✅ Task completed, clearing checkpoint"
                        );
                        self.checkpoint_mgr.clear(task_id).await?;
                        return Ok(true);
                    }
                }
            }
        }
    }

    /// Subscribe with timeout.
    ///
    /// Like `subscribe` but with a maximum duration.
    pub async fn subscribe_with_timeout(
        &self,
        task_id: &str,
        message: &ChannelMessage,
        handler: &ImProgressHandler,
        timeout: std::time::Duration,
    ) -> Result<bool> {
        tokio::time::timeout(timeout, self.subscribe(task_id, message, handler))
            .await
            .map_err(|_| anyhow::anyhow!("Task subscription timeout after {:?}", timeout))?
    }
}

// ============================================================================
// Stream Subscription Handle
// ============================================================================

/// Handle for an active stream subscription.
///
/// Allows cancellation and status checking.
pub struct SubscriptionHandle {
    task_id: String,
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

impl SubscriptionHandle {
    /// Cancel the subscription.
    pub fn cancel(self) {
        let _ = self.cancel_tx.send(());
    }

    /// Get the task ID.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use zero_core::common::{
        OutputData, ProgressData as CommonProgressData, TaskCompletedData, TaskFailedData,
        ThoughtData, ToolUseData as CommonToolUseData,
    };

    #[test]
    fn test_convert_thought_event() {
        let event = CommonTaskEvent::Thought(ThoughtData {
            content: "Analyzing the data...".to_string(),
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::Thought(content) => {
                assert_eq!(content, "Analyzing the data...");
            }
            _ => panic!("Expected Thought event"),
        }
    }

    #[test]
    fn test_convert_progress_event() {
        let event = CommonTaskEvent::Progress(CommonProgressData {
            stage: "processing".to_string(),
            message: "Working...".to_string(),
            percentage: Some(50),
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::Progress(data) => {
                assert_eq!(data.stage, "processing");
                assert_eq!(data.percentage, Some(50));
            }
            _ => panic!("Expected Progress event"),
        }
    }

    #[test]
    fn test_convert_output_event() {
        let event = CommonTaskEvent::Output(OutputData {
            content: "Here is the result...".to_string(),
            is_partial: false,
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::Output(content) => {
                assert_eq!(content, "Here is the result...");
            }
            _ => panic!("Expected Output event"),
        }
    }

    #[test]
    fn test_convert_tool_use_event() {
        let event = CommonTaskEvent::ToolUse(CommonToolUseData {
            tool: "Read".to_string(),
            args: serde_json::json!({"file_path": "/test.txt"}),
            result: Some(serde_json::json!({"content": "file data"})),
            duration_ms: Some(100),
            is_result_tool: false,
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::ToolUse(data) => {
                assert_eq!(data.tool, "Read");
                assert!(data.result.is_some());
            }
            _ => panic!("Expected ToolUse event"),
        }
    }

    #[test]
    fn test_convert_completed_event() {
        let event = CommonTaskEvent::TaskCompleted(TaskCompletedData {
            output: "All done!".to_string(),
            summary: None,
            usage: None,
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::Finish(data) => {
                assert!(data.success);
                assert_eq!(data.output, Some("All done!".to_string()));
                assert!(data.error.is_none());
            }
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn test_convert_failed_event() {
        let event = CommonTaskEvent::TaskFailed(TaskFailedData {
            error: "Something went wrong".to_string(),
            recoverable: false,
            code: None,
        });

        let sse = convert_to_sse_event(&event).unwrap();
        match sse {
            SseTaskEvent::Finish(data) => {
                assert!(!data.success);
                assert!(data.output.is_none());
                assert_eq!(data.error, Some("Something went wrong".to_string()));
            }
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn test_convert_heartbeat_returns_none() {
        let event = CommonTaskEvent::Heartbeat(zero_core::common::HeartbeatData {
            stage: Some("running".to_string()),
            elapsed_ms: 5000,
        });

        assert!(convert_to_sse_event(&event).is_none());
    }

    #[test]
    fn test_event_consumer_config_default() {
        let config = EventConsumerConfig::default();
        assert_eq!(config.block_ms, 5000);
        assert_eq!(config.count, 100);
        assert_eq!(config.checkpoint_interval, 10);
    }
}
