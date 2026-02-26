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

use crate::message::{ChannelMessage, ChannelType, OutgoingContent};
use crate::outbound::OutboundRouter;
use crate::sse::{FinishData, ProgressData, TaskEvent, ToolUseData};
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
}

impl MessageTracker {
    fn new() -> Self {
        Self {
            progress_message_id: None,
            last_update: Instant::now(),
            current_text: String::new(),
            task_start: Instant::now(),
            tools_used: HashMap::new(),
            last_thought: String::new(),
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
        }
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

    /// Get or create a tracker for the given message.
    fn get_tracker(&self, msg_id: &str) -> dashmap::mapref::one::Ref<'_, String, Mutex<MessageTracker>> {
        self.trackers
            .entry(msg_id.to_string())
            .or_insert_with(|| Mutex::new(MessageTracker::new()))
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
            "Starting progress tracking"
        );

        // Initialize tracker with fresh state
        self.trackers.insert(msg.id.clone(), Mutex::new(MessageTracker::new()));

        // Send initial message and track it
        if let Ok(Some(progress_msg_id)) = self.send_new_message(msg, text).await {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.progress_message_id = Some(progress_msg_id);
            tracker.current_text = text.to_string();
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

        // Throttle output updates
        if tracker.last_update.elapsed() < self.throttle_interval {
            return Ok(());
        }

        tracker.last_update = Instant::now();

        // Truncate output for display (max 300 chars)
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        let truncated = if trimmed.len() > 300 {
            format!("{}...", &trimmed[..300])
        } else {
            trimmed.to_string()
        };

        let display_output = format!("📝 {}", truncated);

        // Try to edit existing progress message
        if msg.channel_type == ChannelType::Telegram {
            if let Some(progress_msg_id) = tracker.progress_message_id {
                let combined_text = format!("{}\n\n{}", tracker.current_text, display_output);
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
                OutgoingContent::Text { text: display_output },
            )
            .await;

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

    async fn on_finish(&self, msg: &ChannelMessage, event: &FinishData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            success = event.success,
            "Task finished"
        );

        // Get tracker data for summary before cleanup
        let summary = {
            let tracker_ref = self.get_tracker(&msg.id);
            let tracker = tracker_ref.lock().await;
            Self::generate_summary(&tracker)
        };

        // Clean up tracker
        self.remove_tracker(&msg.id);

        // Format final response
        let content = if event.success {
            if let Some(ref output) = event.output {
                // Append summary to the output
                let combined = format!("{}\n\n{}", output, summary);
                OutgoingContent::Markdown { text: combined }
            } else {
                OutgoingContent::Text {
                    text: format!("✅ 处理完成\n\n{}", summary),
                }
            }
        } else {
            let error_msg = event.error.as_deref().unwrap_or("Unknown error");
            OutgoingContent::Text {
                text: format!("❌ 处理失败: {}\n\n{}", error_msg, summary),
            }
        };

        // Use send_direct to avoid pending entry issues
        let result = self
            .router
            .send_direct(msg.channel_type.clone(), msg.channel_id.clone(), content)
            .await;

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
