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
}

impl MessageTracker {
    fn new() -> Self {
        Self {
            progress_message_id: None,
            last_update: Instant::now(),
            current_text: String::new(),
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
}

impl ImProgressHandler {
    /// Create a new IM progress handler.
    pub fn new(router: Arc<OutboundRouter>, telegram: Option<Arc<TelegramChannel>>) -> Self {
        Self {
            router,
            telegram,
            trackers: DashMap::new(),
            throttle_interval: Duration::from_millis(1000), // Default 1 second
        }
    }

    /// Set the throttle interval for progress updates.
    pub fn with_throttle(mut self, interval: Duration) -> Self {
        self.throttle_interval = interval;
        self
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

    async fn on_tool_use(&self, msg: &ChannelMessage, event: &ToolUseData) -> Result<()> {
        let tool_display = Self::format_tool_name(&event.tool);
        let text = format!("{} {}", tool_display, event.tool);

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

        // Clean up tracker
        self.remove_tracker(&msg.id);

        // Format final response
        let content = if event.success {
            if let Some(ref output) = event.output {
                // Send the actual output as markdown for proper formatting
                OutgoingContent::Markdown {
                    text: output.clone(),
                }
            } else {
                OutgoingContent::Text {
                    text: "✅ 处理完成".to_string(),
                }
            }
        } else {
            let error_msg = event.error.as_deref().unwrap_or("Unknown error");
            OutgoingContent::Text {
                text: format!("❌ 处理失败: {}", error_msg),
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
            TaskEvent::ToolUse(data) => {
                self.on_tool_use(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::Finish(data) => {
                self.on_finish(msg, &data).await?;
                Ok(true) // Finished
            }
            TaskEvent::Thought(_) => {
                // Thoughts are internal, don't send to user
                Ok(false)
            }
            TaskEvent::Output(_) => {
                // Outputs are part of the finish response
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
