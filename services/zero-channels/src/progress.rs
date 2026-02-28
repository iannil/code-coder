//! IM Progress Handler for real-time task feedback.
//!
//! This module implements the progress handling logic for streaming
//! task updates to IM channels (Telegram, Feishu, etc.).
//!
//! # Progress Feedback Strategy
//!
//! The handler uses a **hybrid mode** approach:
//! - **Key milestones** → Send new messages (start, tool use, finish)
//! - **Intermediate progress** → Edit existing progress message (throttled)
//!
//! This balances user visibility with API rate limits.

use crate::debug::{self, AgentUsage, DebugContext, ModelUsage};
use crate::message::{ChannelMessage, ChannelType, OutgoingContent};
use crate::outbound::OutboundRouter;
use crate::sse::{AgentInfoData, DebugInfoData, FinishData, ProgressData, TaskEvent, ToolUseData};
use crate::telegram::TelegramChannel;
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// ============================================================================
// Progress Handler Trait
// ============================================================================

/// Trait for handling task progress events.
#[async_trait]
pub trait ProgressHandler: Send + Sync {
    /// Called when task processing starts.
    async fn on_start(&self, msg: &ChannelMessage, task_id: &str) -> Result<()>;

    /// Called for progress updates during processing.
    async fn on_progress(&self, msg: &ChannelMessage, event: &ProgressData) -> Result<()>;

    /// Called when AI is thinking/reasoning.
    async fn on_thought(&self, msg: &ChannelMessage, thought: &str) -> Result<()>;

    /// Called when AI generates output text.
    async fn on_output(&self, msg: &ChannelMessage, output: &str) -> Result<()>;

    /// Called when a tool is being used.
    async fn on_tool_use(&self, msg: &ChannelMessage, event: &ToolUseData) -> Result<()>;

    /// Called when debug information is available.
    async fn on_debug_info(&self, msg: &ChannelMessage, event: &DebugInfoData) -> Result<()>;

    /// Called when agent information is available.
    async fn on_agent_info(&self, msg: &ChannelMessage, event: &AgentInfoData) -> Result<()>;

    /// Called when task completes (success or failure).
    async fn on_finish(&self, msg: &ChannelMessage, event: &FinishData) -> Result<()>;
}

// ============================================================================
// Message Tracker
// ============================================================================

/// Tracks sent messages for editing/updating.
#[derive(Debug, Clone)]
struct MessageTracker {
    /// Progress message ID (for editing)
    progress_message_id: Option<i64>,
    /// Last update timestamp (for throttling)
    last_update: Instant,
    /// Current progress text
    current_text: String,
    /// Task start time for duration tracking
    task_start: Instant,
    /// Tool usage counts
    tools_used: HashMap<String, u64>,
    /// Last thought text (for deduplication)
    last_thought: String,
    /// Debug mode flag
    debug_mode: bool,
    /// Collected debug information
    debug_context: DebugContext,
}

impl MessageTracker {
    fn new(debug_mode: bool, trace_id: String) -> Self {
        Self {
            progress_message_id: None,
            last_update: Instant::now(),
            current_text: String::new(),
            task_start: Instant::now(),
            tools_used: HashMap::new(),
            last_thought: String::new(),
            debug_mode,
            debug_context: DebugContext::new(trace_id),
        }
    }
}

// ============================================================================
// IM Progress Handler
// ============================================================================

/// Progress handler for IM channels.
///
/// Implements the hybrid progress feedback pattern:
/// - Sends new messages for key milestones
/// - Edits existing messages for intermediate progress (throttled)
///
/// Note: This handler uses `send_direct` instead of `respond` because streaming
/// mode sends multiple messages, but `respond` removes the pending entry after
/// the first successful send.
pub struct ImProgressHandler {
    /// Outbound router for sending messages
    router: Arc<OutboundRouter>,
    /// Telegram channel instance (for message editing)
    telegram: Option<Arc<TelegramChannel>>,
    /// Message trackers per original message ID
    trackers: DashMap<String, Mutex<MessageTracker>>,
    /// Throttle interval for progress updates
    throttle_interval: Duration,
    /// Thought throttle interval (thoughts can be frequent)
    thought_throttle_interval: Duration,
    /// Debug mode flag (global for all messages)
    debug_mode: bool,
}

impl ImProgressHandler {
    /// Create a new IM progress handler.
    pub fn new(router: Arc<OutboundRouter>, telegram: Option<Arc<TelegramChannel>>) -> Self {
        Self {
            router,
            telegram,
            trackers: DashMap::new(),
            throttle_interval: Duration::from_millis(1000), // Default 1 second
            thought_throttle_interval: Duration::from_millis(500), // 500ms for thoughts
            debug_mode: false,
        }
    }

    /// Set the debug mode flag.
    pub fn with_debug_mode(mut self, debug_mode: bool) -> Self {
        self.debug_mode = debug_mode;
        self
    }

    /// Set the throttle interval for progress updates.
    pub fn with_throttle(mut self, interval: Duration) -> Self {
        self.throttle_interval = interval;
        self
    }

    /// Set the throttle interval for thought updates.
    pub fn with_thought_throttle(mut self, interval: Duration) -> Self {
        self.thought_throttle_interval = interval;
        self
    }

    /// Generate execution summary for task completion.
    fn generate_summary(tracker: &MessageTracker) -> String {
        let duration = tracker.task_start.elapsed().as_secs_f64();
        let duration_str = if duration < 1.0 {
            format!("{:.0}ms", duration * 1000.0)
        } else if duration < 60.0 {
            format!("{:.1}s", duration)
        } else {
            let mins = (duration / 60.0).floor();
            let secs = duration % 60.0;
            format!("{}m {:.0}s", mins, secs)
        };

        let mut summary = format!("📊 执行摘要\n⏱ 耗时: {}", duration_str);

        if !tracker.tools_used.is_empty() {
            let total: u64 = tracker.tools_used.values().sum();
            summary.push_str(&format!("\n🔧 工具调用: {} 次", total));

            // Sort by count and show top tools
            let mut tools: Vec<_> = tracker.tools_used.iter().collect();
            tools.sort_by(|a, b| b.1.cmp(a.1));
            for (tool, count) in tools.iter().take(5) {
                summary.push_str(&format!("\n   • {}: {}", tool, count));
            }
        }

        summary
    }

    /// Format debug information based on the platform type.
    fn format_debug_for_platform(debug: &DebugContext, channel_type: &ChannelType) -> String {
        match channel_type {
            ChannelType::Telegram => debug::format_debug_html(debug),
            ChannelType::Slack => debug::format_debug_mrkdwn(debug),
            ChannelType::Discord => debug::format_debug_markdown(debug),
            _ => debug::format_debug_plain(debug),
        }
    }

    /// Get or create a tracker for the given message.
    fn get_tracker(&self, msg_id: &str) -> dashmap::mapref::one::Ref<'_, String, Mutex<MessageTracker>> {
        self.trackers
            .entry(msg_id.to_string())
            .or_insert_with(|| {
                // Note: This creates a tracker without proper initialization
                // The tracker should be initialized in on_start before this is called
                Mutex::new(MessageTracker::new(self.debug_mode, "unknown".to_string()))
            })
            .downgrade()
    }

    /// Remove tracker for a completed message.
    fn remove_tracker(&self, msg_id: &str) {
        self.trackers.remove(msg_id);
    }

    /// Format tool name for user-friendly display.
    fn format_tool_name(tool: &str) -> &'static str {
        match tool {
            "Read" | "read" | "file_read" => "📄 读取文件",
            "Write" | "write" | "file_write" => "✏️ 写入文件",
            "Edit" | "edit" => "🔧 编辑文件",
            "Bash" | "bash" | "shell" => "💻 执行命令",
            "Grep" | "grep" | "search" => "🔍 搜索代码",
            "Glob" | "glob" => "📁 查找文件",
            "WebSearch" | "web_search" => "🌐 网络搜索",
            "WebFetch" | "web_fetch" => "🌐 获取网页",
            "Task" | "task" => "🤖 启动子任务",
            _ => "⚡ 执行工具",
        }
    }

    /// Send a new message (for key milestones).
    ///
    /// Uses `send_direct` instead of `respond` because streaming mode sends multiple
    /// messages, but `respond` removes the pending entry after the first success.
    async fn send_new_message(&self, msg: &ChannelMessage, text: &str) -> Result<Option<i64>> {
        let content = OutgoingContent::Text {
            text: text.to_string(),
        };

        // Use send_direct to avoid the pending entry being removed
        let result = self
            .router
            .send_direct(msg.channel_type.clone(), msg.channel_id.clone(), content)
            .await;

        if !result.success {
            tracing::warn!(
                message_id = %msg.id,
                error = ?result.error,
                "Failed to send progress message"
            );
            return Ok(None);
        }

        // Try to parse message ID for Telegram
        if let Some(ref msg_id_str) = result.message_id {
            if let Ok(msg_id) = msg_id_str.parse::<i64>() {
                return Ok(Some(msg_id));
            }
        }

        Ok(None)
    }

    /// Edit an existing Telegram message.
    async fn edit_telegram_message(&self, chat_id: &str, message_id: i64, text: &str) -> Result<()> {
        let Some(ref telegram) = self.telegram else {
            return Ok(());
        };

        match telegram.edit_message_text(chat_id, message_id, text).await {
            Ok(_) => {
                tracing::debug!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    "Edited Telegram message"
                );
            }
            Err(e) => {
                // Don't fail on edit errors (rate limits, message not found, etc.)
                tracing::warn!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    error = %e,
                    "Failed to edit Telegram message, falling back to new message"
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ProgressHandler for ImProgressHandler {
    async fn on_start(&self, msg: &ChannelMessage, task_id: &str) -> Result<()> {
        let text = "🚀 开始处理...";

        tracing::info!(
            message_id = %msg.id,
            task_id = %task_id,
            channel_type = ?msg.channel_type,
            debug_mode = self.debug_mode,
            "Starting progress tracking"
        );

        // Check if debug mode is requested (from message metadata or global setting)
        let debug_mode = self.debug_mode
            || msg.metadata.get("debug_mode").map(|v| v == "true").unwrap_or(false);

        // Initialize tracker with fresh state
        // Use a marker for current_text to distinguish from actual accumulated content
        self.trackers.insert(
            msg.id.clone(),
            Mutex::new(MessageTracker::new(debug_mode, msg.trace_id.clone())),
        );

        // Send initial message and track it
        if let Ok(Some(progress_msg_id)) = self.send_new_message(msg, text).await {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.progress_message_id = Some(progress_msg_id);
            // Don't set current_text to the start message - use a marker instead
            // This allows on_output to properly accumulate actual content
            tracker.current_text = "[started]".to_string();
        }

        Ok(())
    }

    async fn on_progress(&self, msg: &ChannelMessage, event: &ProgressData) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        // Throttle progress updates
        if tracker.last_update.elapsed() < self.throttle_interval {
            tracing::debug!(
                message_id = %msg.id,
                stage = %event.stage,
                "Throttling progress update"
            );
            return Ok(());
        }

        tracker.last_update = Instant::now();

        // Format progress text
        let progress_text = match event.percentage {
            Some(pct) => format!("⏳ {} ({}%)\n{}", event.stage, pct, event.message),
            None => format!("⏳ {}\n{}", event.stage, event.message),
        };

        // Try to edit existing message if on Telegram
        if msg.channel_type == ChannelType::Telegram {
            if let Some(progress_msg_id) = tracker.progress_message_id {
                self.edit_telegram_message(&msg.channel_id, progress_msg_id, &progress_text)
                    .await?;
                tracker.current_text = progress_text;
                return Ok(());
            }
        }

        // Fallback: send new message (for non-Telegram or first progress)
        if let Ok(Some(new_msg_id)) = self.send_new_message(msg, &progress_text).await {
            tracker.progress_message_id = Some(new_msg_id);
        }
        tracker.current_text = progress_text;

        Ok(())
    }

    async fn on_thought(&self, msg: &ChannelMessage, thought: &str) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        // Deduplicate: skip if same as last thought
        if tracker.last_thought == thought {
            return Ok(());
        }

        // Throttle thought updates
        if tracker.last_update.elapsed() < self.thought_throttle_interval {
            return Ok(());
        }

        tracker.last_update = Instant::now();
        tracker.last_thought = thought.to_string();

        // Truncate thought for display (max 200 chars)
        let truncated = if thought.len() > 200 {
            format!("{}...", &thought[..200])
        } else {
            thought.to_string()
        };

        // Format thought with 💭 prefix, filter out start/end markers
        let display_thought = if truncated.contains("[思考开始]") {
            "💭 开始思考...".to_string()
        } else {
            let trimmed = truncated.trim();
            if trimmed.is_empty() {
                return Ok(());
            }
            format!("💭 {}", trimmed)
        };

        // Try to edit existing progress message
        if msg.channel_type == ChannelType::Telegram {
            if let Some(progress_msg_id) = tracker.progress_message_id {
                let combined_text = format!("{}\n\n{}", tracker.current_text, display_thought);
                self.edit_telegram_message(&msg.channel_id, progress_msg_id, &combined_text)
                    .await?;
                tracker.current_text = combined_text;
                return Ok(());
            }
        }

        // Fallback: send new message
        let _ = self
            .router
            .send_direct(
                msg.channel_type.clone(),
                msg.channel_id.clone(),
                OutgoingContent::Text { text: display_thought },
            )
            .await;

        Ok(())
    }

    async fn on_output(&self, msg: &ChannelMessage, output: &str) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        // Silently accumulate output text for final display
        // Don't send individual output chunks to avoid spam
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        // Initialize accumulation if this is the first output
        if tracker.current_text == "[started]" {
            tracker.current_text = trimmed.to_string();
            tracing::debug!(
                message_id = %msg.id,
                length = trimmed.len(),
                "Initialized accumulated text"
            );
            return Ok(());
        }

        // Deduplication: Skip if output is already contained in current text
        let current_lower = tracker.current_text.to_lowercase();
        let output_lower = trimmed.to_lowercase();

        // Skip if output is a substring of current text
        if current_lower.contains(&output_lower) {
            tracing::debug!(
                message_id = %msg.id,
                output_length = trimmed.len(),
                "Skipping duplicate output (substring match)"
            );
            return Ok(());
        }

        // Skip if output starts with significant content that matches end of current text
        // Use char_indices() for safe UTF-8 slicing
        if trimmed.chars().count() > 20 && current_lower.chars().count() > 50 {
            // Get first 50 characters (not bytes) safely
            let output_start: String = output_lower.chars().take(50).collect();
            // Get last 50 characters of current text safely
            let current_end: String = current_lower.chars().rev().take(50).collect::<Vec<_>>().into_iter().rev().collect();
            if current_end.contains(&output_start) {
                tracing::debug!(
                    message_id = %msg.id,
                    output_length = trimmed.len(),
                    "Skipping duplicate output (prefix overlap)"
                );
                return Ok(());
            }
        }

        // Append output text
        tracker.current_text.push_str("\n\n");
        tracker.current_text.push_str(trimmed);

        tracing::debug!(
            message_id = %msg.id,
            accumulated_length = tracker.current_text.len(),
            "Accumulated output text"
        );

        Ok(())
    }

    async fn on_tool_use(&self, msg: &ChannelMessage, event: &ToolUseData) -> Result<()> {
        // Track tool usage for summary
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            *tracker.tools_used.entry(event.tool.clone()).or_insert(0) += 1;
        }

        let tool_display = Self::format_tool_name(&event.tool);
        let text = if let Some(ref result) = event.result {
            // Truncate result for display
            let result_str = if result.to_string().len() > 100 {
                format!("{}...", &result.to_string()[..100])
            } else {
                result.to_string()
            };
            format!("{} {}\n└ 結果: {}", tool_display, event.tool, result_str)
        } else {
            format!("{} {}", tool_display, event.tool)
        };

        tracing::info!(
            message_id = %msg.id,
            tool = %event.tool,
            "Tool use notification"
        );

        // Tool use is a key milestone - send a new message
        // (Don't track this message, it's informational only)
        let content = OutgoingContent::Text { text };
        let _ = self
            .router
            .send_direct(msg.channel_type.clone(), msg.channel_id.clone(), content)
            .await;

        Ok(())
    }

    async fn on_debug_info(&self, msg: &ChannelMessage, event: &DebugInfoData) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        // Only collect debug info if debug mode is enabled
        if !tracker.debug_mode {
            return Ok(());
        }

        tracing::debug!(
            message_id = %msg.id,
            model = ?event.model,
            input_tokens = ?event.input_tokens,
            output_tokens = ?event.output_tokens,
            "Debug info received"
        );

        // Add model usage to debug context
        if let Some(model) = &event.model {
            let usage = ModelUsage {
                model: model.clone(),
                provider: event.provider.clone(),
                input_tokens: event.input_tokens.unwrap_or(0),
                output_tokens: event.output_tokens.unwrap_or(0),
                total_tokens: event.total_tokens,
                duration_ms: event.duration_ms.unwrap_or(0),
            };
            tracker.debug_context.add_model_usage(usage);
        }

        // Update data flow metrics if available
        if let Some(req_bytes) = event.request_bytes {
            tracker.debug_context.data_flow.request_bytes = req_bytes;
        }
        if let Some(resp_bytes) = event.response_bytes {
            tracker.debug_context.data_flow.response_bytes = resp_bytes;
        }

        Ok(())
    }

    async fn on_agent_info(&self, msg: &ChannelMessage, event: &AgentInfoData) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        // Only collect agent info if debug mode is enabled
        if !tracker.debug_mode {
            return Ok(());
        }

        tracing::debug!(
            message_id = %msg.id,
            agent = %event.agent,
            is_primary = event.is_primary,
            "Agent info received"
        );

        // Add agent usage to debug context
        let usage = AgentUsage {
            agent: event.agent.clone(),
            display_name: event.display_name.clone(),
            invocations: 1, // Each event represents one invocation
            duration_ms: event.duration_ms.unwrap_or(0),
            is_primary: event.is_primary.unwrap_or(false),
        };
        tracker.debug_context.add_agent_usage(usage);

        Ok(())
    }

    async fn on_finish(&self, msg: &ChannelMessage, event: &FinishData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            success = event.success,
            has_output = event.output.is_some(),
            output_length = event.output.as_ref().map_or(0, |s| s.len()),
            has_error = event.error.is_some(),
            "Task finish event received"
        );

        if let Some(ref output) = event.output {
            tracing::debug!(
                message_id = %msg.id,
                output_preview = &output[..output.len().min(200)],
                "Finish event output preview"
            );
        }

        // Get tracker data for summary and debug info before cleanup
        let (summary, debug_info_text, accumulated_text) = {
            let tracker_ref = self.get_tracker(&msg.id);
            let tracker = tracker_ref.lock().await;
            let summary = Self::generate_summary(&tracker);
            let debug_info_text = if tracker.debug_mode && !tracker.debug_context.is_empty() {
                Some(Self::format_debug_for_platform(&tracker.debug_context, &msg.channel_type))
            } else {
                None
            };
            // Get accumulated text from streaming output events
            // Exclude the "[started]" marker used for initialization
            let accumulated = if !tracker.current_text.is_empty() && tracker.current_text != "[started]" {
                Some(tracker.current_text.clone())
            } else {
                None
            };
            tracing::info!(
                message_id = %msg.id,
                accumulated_length = accumulated.as_ref().map_or(0, |s| s.len()),
                "Accumulated text from on_output events"
            );
            (summary, debug_info_text, accumulated)
        };

        // Clean up tracker
        self.remove_tracker(&msg.id);

        // Format final response with optional debug info
        // Use accumulated text from streaming if available, otherwise use event.output
        let content = if event.success {
            // Use accumulated text (from on_output events) or fallback to event.output
            let final_output = accumulated_text.as_ref().or(event.output.as_ref());

            // Always send a message - either the output or a completion marker
            let base = if let Some(output) = final_output {
                format!("{}\n\n{}", output, summary)
            } else {
                // No output content, just show summary
                format!("✅ 处理完成\n\n{}", summary)
            };

            tracing::info!(
                message_id = %msg.id,
                base_length = base.len(),
                "Sending finish response"
            );

            if let Some(debug_text) = debug_info_text {
                OutgoingContent::Markdown {
                    text: format!("{}\n\n{}", base, debug_text),
                }
            } else {
                OutgoingContent::Markdown { text: base }
            }
        } else {
            let error_msg = event.error.as_deref().unwrap_or("Unknown error");
            let text = format!("❌ 处理失败: {}\n\n{}", error_msg, summary);
            if let Some(debug_text) = debug_info_text {
                OutgoingContent::Markdown {
                    text: format!("{}\n\n{}", text, debug_text),
                }
            } else {
                OutgoingContent::Text { text }
            }
        };

        // Use send_direct to avoid pending entry issues
        let result = self
            .router
            .send_direct(msg.channel_type.clone(), msg.channel_id.clone(), content)
            .await;

        tracing::info!(
            message_id = %msg.id,
            success = result.success,
            error = ?result.error,
            "Finish response send result"
        );

        if !result.success {
            tracing::error!(
                message_id = %msg.id,
                error = ?result.error,
                "Failed to send finish response"
            );
        }

        Ok(())
    }
}

// ============================================================================
// Progress Event Processing
// ============================================================================

impl ImProgressHandler {
    /// Process a task event and dispatch to appropriate handler.
    pub async fn handle_event(&self, msg: &ChannelMessage, event: TaskEvent) -> Result<bool> {
        match event {
            TaskEvent::Progress(data) => {
                self.on_progress(msg, &data).await?;
                Ok(false) // Not finished
            }
            TaskEvent::Thought(data) => {
                self.on_thought(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::Output(data) => {
                self.on_output(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::ToolUse(data) => {
                self.on_tool_use(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::Finish(data) => {
                self.on_finish(msg, &data).await?;
                Ok(true) // Finished
            }
            TaskEvent::DebugInfo(data) => {
                self.on_debug_info(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::AgentInfo(data) => {
                self.on_agent_info(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::Confirmation(_) => {
                // TODO: Implement confirmation handling via inline buttons
                tracing::warn!(
                    message_id = %msg.id,
                    "Confirmation requests not yet implemented in streaming mode"
                );
                Ok(false)
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_name() {
        assert_eq!(ImProgressHandler::format_tool_name("Read"), "📄 读取文件");
        assert_eq!(ImProgressHandler::format_tool_name("Bash"), "💻 执行命令");
        assert_eq!(ImProgressHandler::format_tool_name("WebSearch"), "🌐 网络搜索");
        assert_eq!(ImProgressHandler::format_tool_name("unknown_tool"), "⚡ 执行工具");
    }

    #[test]
    fn test_message_tracker_new() {
        let tracker = MessageTracker::new();
        assert!(tracker.progress_message_id.is_none());
        assert!(tracker.current_text.is_empty());
    }
}
