//! Task Dispatcher for zero-channels
//!
//! Routes incoming IM messages to the persistent task queue (Redis Streams).
//! This replaces direct HTTP calls to CodeCoder with queue-based processing.
//!
//! # Architecture
//!
//! ```text
//! IM Message → TaskDispatcher → Redis Stream (tasks:pending)
//!                   ↓                        ↓
//!              Immediate ACK            ccode Worker
//!              to user                  processes task
//! ```
//!
//! # Benefits
//!
//! - **Reliability**: Messages are persisted before acknowledgement
//! - **Scalability**: Multiple workers can process tasks
//! - **Recovery**: Failed tasks can be retried automatically

use super::message::ChannelMessage;
use super::outbound::OutboundRouter;
use super::telegram::TelegramChannel;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use zero_core::common::logging::RequestContext;
use zero_core::common::keywords::{detect_alias, keywords};
use zero_core::common::messages::messages;
use zero_core::common::{
    stream_keys, RedisStreamClient, RedisStreamConfig, TaskCreatedData, TaskEvent,
};

// ============================================================================
// Configuration
// ============================================================================

/// Task dispatcher configuration.
#[derive(Debug, Clone)]
pub struct TaskDispatcherConfig {
    /// Redis Streams client configuration.
    pub stream_config: RedisStreamConfig,
    /// Whether to use Redis Streams (true) or fallback to HTTP (false).
    pub enabled: bool,
    /// Immediate acknowledgement message template.
    pub ack_message: String,
    /// Default agent for tasks.
    pub default_agent: String,
}

impl Default for TaskDispatcherConfig {
    fn default() -> Self {
        Self {
            stream_config: RedisStreamConfig::default(),
            enabled: true,
            ack_message: messages().messages.task.acknowledged.clone(),
            default_agent: "build".to_string(),
        }
    }
}

// ============================================================================
// Task Request (Stream Payload)
// ============================================================================

/// Task request payload for the pending queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    /// Unique task identifier.
    pub task_id: String,
    /// User identifier.
    pub user_id: String,
    /// Channel type (telegram, discord, etc.).
    pub channel: String,
    /// Channel/chat ID.
    pub channel_id: String,
    /// User prompt.
    pub prompt: String,
    /// Agent to use.
    pub agent: String,
    /// Trace ID for distributed tracing.
    pub trace_id: String,
    /// Message history for context.
    #[serde(default)]
    pub chat_history: Vec<serde_json::Value>,
    /// Original message ID (for reply threading).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<String>,
    /// Creation timestamp (milliseconds since epoch).
    pub created_at: u64,
}

impl TaskRequest {
    /// Create a new task request from a channel message.
    pub fn from_channel_message(
        message: &ChannelMessage,
        agent: &str,
        trace_id: &str,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Extract text from MessageContent
        let prompt = match &message.content {
            crate::channels::message::MessageContent::Text { text } => text.clone(),
            crate::channels::message::MessageContent::Image { caption, .. } => {
                caption.clone().unwrap_or_default()
            }
            _ => String::new(),
        };

        Self {
            task_id: format!("task_{}", Uuid::new_v4()),
            user_id: message.user_id.clone(),
            channel: message.channel_type.as_str().to_string(),
            channel_id: message.channel_id.clone(),
            prompt,
            agent: agent.to_string(),
            trace_id: trace_id.to_string(),
            // Chat history is not available on ChannelMessage, use empty vec
            chat_history: Vec::new(),
            // Reply-to message ID from metadata if available
            reply_to_message_id: message.metadata.get("reply_to").cloned(),
            created_at: now,
        }
    }

    /// Convert to TaskCreatedData event.
    pub fn to_task_created_event(&self) -> TaskEvent {
        TaskEvent::TaskCreated(TaskCreatedData {
            task_id: self.task_id.clone(),
            user_id: self.user_id.clone(),
            channel: self.channel.clone(),
            channel_id: self.channel_id.clone(),
            prompt: self.prompt.clone(),
            agent: self.agent.clone(),
            trace_id: self.trace_id.clone(),
            chat_history: self.chat_history.clone(),
        })
    }
}

// ============================================================================
// Task Dispatcher
// ============================================================================

/// Dispatches tasks to Redis Streams for processing.
pub struct TaskDispatcher {
    config: TaskDispatcherConfig,
    client: Option<RedisStreamClient>,
    outbound: Arc<OutboundRouter>,
    telegram: Option<Arc<TelegramChannel>>,
}

impl TaskDispatcher {
    /// Create a new task dispatcher.
    pub fn new(config: TaskDispatcherConfig, outbound: Arc<OutboundRouter>) -> Self {
        Self {
            config,
            client: None,
            outbound,
            telegram: None,
        }
    }

    /// Add Telegram channel for acknowledgement messages.
    pub fn with_telegram(mut self, telegram: Arc<TelegramChannel>) -> Self {
        self.telegram = Some(telegram);
        self
    }

    /// Initialize the Redis connection.
    pub async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            tracing::info!("Task dispatcher disabled, using HTTP fallback");
            return Ok(());
        }

        let client = RedisStreamClient::new(self.config.stream_config.clone()).await?;

        // Ensure consumer group exists for the pending queue
        client
            .ensure_consumer_group(stream_keys::TASKS_PENDING)
            .await?;

        self.client = Some(client);
        tracing::info!("Task dispatcher initialized with Redis Streams");

        Ok(())
    }

    /// Check if the dispatcher is using Redis Streams.
    pub fn is_using_streams(&self) -> bool {
        self.client.is_some()
    }

    /// Dispatch a task to the queue.
    ///
    /// Returns the task ID and sends an immediate acknowledgement to the user.
    pub async fn dispatch(
        &self,
        message: &ChannelMessage,
        agent: &str,
        ctx: &RequestContext,
    ) -> Result<String> {
        let start = Instant::now();

        // Create task request
        let task_request = TaskRequest::from_channel_message(message, agent, &ctx.trace_id);
        let task_id = task_request.task_id.clone();

        // Add to Redis Stream
        if let Some(ref client) = self.client {
            // Add to pending queue
            let stream_id = client
                .xadd(stream_keys::TASKS_PENDING, &task_request)
                .await?;

            // Also publish the TaskCreated event to the task's event stream
            let event = zero_core::common::StreamEvent::new(0, task_request.to_task_created_event())
                .with_trace_context(ctx.trace_id.clone(), ctx.span_id.clone());

            client
                .xadd(&stream_keys::task_events(&task_id), &event)
                .await?;

            // Initialize task state
            let now = chrono::Utc::now().to_rfc3339();
            client
                .hset(
                    &stream_keys::task_state(&task_id),
                    &[
                        ("task_id", task_id.as_str()),
                        ("status", "pending"),
                        ("current_agent", agent),
                        ("progress_pct", "0"),
                        ("last_event_seq", "0"),
                        ("updated_at", &now),
                    ],
                )
                .await?;

            tracing::info!(
                task_id = %task_id,
                stream_id = %stream_id,
                agent = %agent,
                trace_id = %ctx.trace_id,
                duration_ms = %start.elapsed().as_millis(),
                "Task dispatched to queue"
            );
        } else {
            return Err(anyhow::anyhow!("Redis Streams not initialized"));
        }

        // Send immediate acknowledgement to user
        self.send_acknowledgement(message, &task_id).await?;

        Ok(task_id)
    }

    /// Send immediate acknowledgement to user.
    async fn send_acknowledgement(
        &self,
        message: &ChannelMessage,
        task_id: &str,
    ) -> Result<()> {
        let ack_text = format!("{}\n\n任务ID: `{}`", self.config.ack_message, task_id);

        // Use outbound router for sending acknowledgement
        let result = self.outbound
            .send_direct(
                message.channel_type,
                message.channel_id.clone(),
                crate::channels::message::OutgoingContent::Text { text: ack_text },
            )
            .await;

        if !result.success {
            tracing::warn!(
                task_id = %task_id,
                error = ?result.error,
                "Failed to send task acknowledgement"
            );
        }

        Ok(())
    }

    /// Check if the client is healthy.
    pub async fn is_healthy(&self) -> bool {
        match &self.client {
            Some(client) => client.is_healthy().await,
            None => false,
        }
    }

    /// Get queue depth (number of pending tasks).
    pub async fn queue_depth(&self) -> Result<u64> {
        match &self.client {
            Some(client) => Ok(client.xlen(stream_keys::TASKS_PENDING).await?),
            None => Err(anyhow::anyhow!("Redis Streams not initialized")),
        }
    }
}

// ============================================================================
// Agent Detection
// ============================================================================

use super::message::ChannelType;

/// Get the default agent for a channel type.
///
/// - IM channels (Telegram, Discord, Slack, etc.) default to "autonomous"
///   because IM messages are typically autonomous tasks that benefit from
///   independent decision-making and multi-step execution.
/// - CLI defaults to "build" for interactive development workflow.
pub fn default_agent_for_channel(channel_type: ChannelType) -> &'static str {
    match channel_type {
        ChannelType::Cli => "build",
        // All IM channels default to autonomous agent
        ChannelType::Telegram
        | ChannelType::Discord
        | ChannelType::Slack
        | ChannelType::Feishu
        | ChannelType::WeCom
        | ChannelType::DingTalk
        | ChannelType::WhatsApp
        | ChannelType::Matrix
        | ChannelType::IMessage
        | ChannelType::Email => "autonomous",
    }
}

/// Detect the agent to use based on message content.
///
/// Uses the keywords configuration for @mention alias detection.
/// Falls back to the default agent if no explicit mention is found.
pub fn detect_agent<'a>(message: &str, default_agent: &'a str) -> &'a str {
    let text = message.trim().to_lowercase();

    // Check for explicit @agent mentions using keywords config
    if text.starts_with('@') {
        let config = keywords();
        if let Some(agent) = detect_alias(message, config) {
            // SAFETY: We're returning a reference to a string that lives in config,
            // which is 'static (from the global KEYWORDS). We leak the string to
            // extend its lifetime to match the expected return type.
            return Box::leak(agent.to_string().into_boxed_str());
        }
    }

    // No implicit keyword matching - let autonomous agent decide which sub-agent to call
    // This avoids misrouting (e.g., "黄金市场" → picker instead of macro)
    // Users can still use explicit @agent mentions for direct routing

    default_agent
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::message::ChannelType;

    #[test]
    #[ignore = "Requires ~/.codecoder/keywords.json with agent aliases configured"]
    fn test_detect_agent_explicit_mention() {
        assert_eq!(detect_agent("@plan 帮我规划一下", "build"), "plan");
        assert_eq!(detect_agent("@code-review 看看这段代码", "build"), "code-reviewer");
        assert_eq!(detect_agent("@security 检查安全问题", "build"), "security-reviewer");
        assert_eq!(detect_agent("@macro 分析PMI数据", "build"), "macro");
    }

    #[test]
    fn test_detect_agent_default() {
        // Without explicit @mention, all messages go to default agent
        // Autonomous agent will decide which sub-agent to call
        assert_eq!(detect_agent("帮我写段代码", "build"), "build");
        assert_eq!(detect_agent("Hello world", "build"), "build");

        // These used to match domain keywords, now they go to default
        assert_eq!(detect_agent("黄金市场表现如何", "autonomous"), "autonomous");
        assert_eq!(detect_agent("帮我做个代码审查", "autonomous"), "autonomous");
        assert_eq!(detect_agent("分析一下宏观经济形势", "autonomous"), "autonomous");

        // Test with autonomous as default (IM channel behavior)
        assert_eq!(detect_agent("帮我写段代码", "autonomous"), "autonomous");
        assert_eq!(detect_agent("Hello world", "autonomous"), "autonomous");
    }

    #[test]
    fn test_default_agent_for_channel() {
        // CLI should default to "build" for interactive development
        assert_eq!(default_agent_for_channel(ChannelType::Cli), "build");

        // All IM channels should default to "autonomous"
        assert_eq!(default_agent_for_channel(ChannelType::Telegram), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::Discord), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::Slack), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::Feishu), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::WeCom), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::DingTalk), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::WhatsApp), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::Matrix), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::IMessage), "autonomous");
        assert_eq!(default_agent_for_channel(ChannelType::Email), "autonomous");
    }

    #[test]
    fn test_task_request_creation() {
        let message = ChannelMessage {
            id: "msg_123".to_string(),
            channel_id: "chat_456".to_string(),
            user_id: "user_789".to_string(),
            channel_type: ChannelType::Telegram,
            content: crate::channels::message::MessageContent::Text {
                text: "Test message".to_string(),
            },
            timestamp: 1234567890000,
            trace_id: "trace_abc".to_string(),
            span_id: "span_def".to_string(),
            parent_span_id: None,
            attachments: vec![],
            metadata: std::collections::HashMap::new(),
        };

        let request = TaskRequest::from_channel_message(&message, "build", "trace_xyz");

        assert!(request.task_id.starts_with("task_"));
        assert_eq!(request.user_id, "user_789");
        assert_eq!(request.channel, "telegram");
        assert_eq!(request.channel_id, "chat_456");
        assert_eq!(request.prompt, "Test message");
        assert_eq!(request.agent, "build");
        assert_eq!(request.trace_id, "trace_xyz");
    }
}
