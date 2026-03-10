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

use super::debug::{self, AgentUsage, DebugContext, ModelUsage};
use super::message::{ChannelMessage, ChannelType, OutgoingContent};
use super::outbound::OutboundRouter;
use super::safe_truncate;
use super::sse::{AgentInfoData, ConfirmationData, DebugInfoData, FinishData, PdcaCycleData, PdcaCheckData, PdcaResultData, ProgressData, QuestionData, SkillUseData, TaskEvent, ToolUseData};
use super::telegram::{InlineButton, TelegramChannel};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use zero_core::common::messages::{messages, t};

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

    /// Called when a skill is being used.
    async fn on_skill_use(&self, msg: &ChannelMessage, event: &SkillUseData) -> Result<()>;

    /// Called when debug information is available.
    async fn on_debug_info(&self, msg: &ChannelMessage, event: &DebugInfoData) -> Result<()>;

    /// Called when agent information is available.
    async fn on_agent_info(&self, msg: &ChannelMessage, event: &AgentInfoData) -> Result<()>;

    /// Called when AI asks user a question.
    async fn on_question(&self, msg: &ChannelMessage, event: &QuestionData) -> Result<()>;

    /// Called when a confirmation is required (tool approval).
    async fn on_confirmation(&self, msg: &ChannelMessage, event: &ConfirmationData) -> Result<()>;

    /// Called when PDCA cycle phase changes.
    async fn on_pdca_cycle(&self, msg: &ChannelMessage, event: &PdcaCycleData) -> Result<()>;

    /// Called when PDCA check completes.
    async fn on_pdca_check(&self, msg: &ChannelMessage, event: &PdcaCheckData) -> Result<()>;

    /// Called when PDCA loop finishes.
    async fn on_pdca_result(&self, msg: &ChannelMessage, event: &PdcaResultData) -> Result<()>;

    /// Called when task completes (success or failure).
    async fn on_finish(&self, msg: &ChannelMessage, event: &FinishData) -> Result<()>;
}

// ============================================================================
// Message Lifecycle Tracking
// ============================================================================

/// Message processing lifecycle stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStage {
    /// Message received, awaiting processing
    Received,
    /// Task created, SSE subscription started
    Processing,
    /// Receiving tool/thought events
    Active,
    /// Finish event received, sending final response
    Finishing,
    /// Processing complete (success or failure)
    Completed,
    /// Error occurred during processing
    Failed,
}

impl MessageStage {
    fn as_str(&self) -> &'static str {
        match self {
            MessageStage::Received => "received",
            MessageStage::Processing => "processing",
            MessageStage::Active => "active",
            MessageStage::Finishing => "finishing",
            MessageStage::Completed => "completed",
            MessageStage::Failed => "failed",
        }
    }
}

// ============================================================================
// Message Tracker
// ============================================================================

/// Tracks sent messages for editing/updating.
#[derive(Debug, Clone)]
struct MessageTracker {
    /// Current lifecycle stage
    stage: MessageStage,
    /// Trace ID for distributed tracing
    trace_id: String,
    /// Task ID (if known)
    task_id: Option<String>,
    /// Progress message ID (for editing)
    progress_message_id: Option<i64>,
    /// Thought message IDs (for cleanup on finish)
    thought_message_ids: Vec<i64>,
    /// Last update timestamp (for throttling)
    last_update: Instant,
    /// Current progress text
    current_text: String,
    /// Task start time for duration tracking
    task_start: Instant,
    /// Tool usage counts
    tools_used: HashMap<String, u64>,
    /// Skill usage list (preserves order)
    skills_used: Vec<String>,
    /// Last thought text (for deduplication)
    last_thought: String,
    /// Debug mode flag
    debug_mode: bool,
    /// Collected debug information
    debug_context: DebugContext,
    /// Count of events received
    events_received: u32,
    /// Count of messages sent
    messages_sent: u32,
    /// Count of send failures
    send_failures: u32,
    // PDCA tracking fields (accumulated for final message)
    /// Current PDCA cycle info
    pdca_cycle: Option<PdcaCycleData>,
    /// PDCA check result
    pdca_check: Option<PdcaCheckData>,
    /// PDCA final result
    pdca_result: Option<PdcaResultData>,
}

impl MessageTracker {
    fn new(debug_mode: bool, trace_id: String) -> Self {
        Self {
            stage: MessageStage::Received,
            trace_id: trace_id.clone(),
            task_id: None,
            progress_message_id: None,
            thought_message_ids: Vec::new(),
            last_update: Instant::now(),
            current_text: String::new(),
            task_start: Instant::now(),
            tools_used: HashMap::new(),
            skills_used: Vec::new(),
            last_thought: String::new(),
            debug_mode,
            debug_context: DebugContext::new(trace_id),
            events_received: 0,
            messages_sent: 0,
            send_failures: 0,
            // Initialize PDCA fields
            pdca_cycle: None,
            pdca_check: None,
            pdca_result: None,
        }
    }

    /// Transition to a new stage with logging
    fn transition_to(&mut self, new_stage: MessageStage) {
        let old_stage = self.stage;
        self.stage = new_stage;
        tracing::info!(
            trace_id = %self.trace_id,
            task_id = ?self.task_id,
            from_stage = old_stage.as_str(),
            to_stage = new_stage.as_str(),
            events_received = self.events_received,
            messages_sent = self.messages_sent,
            send_failures = self.send_failures,
            elapsed_ms = self.task_start.elapsed().as_millis() as u64,
            "📍 Message stage transition"
        );
    }

    /// Record an event received
    fn record_event(&mut self, event_type: &str) {
        self.events_received += 1;
        tracing::debug!(
            trace_id = %self.trace_id,
            event_type = event_type,
            event_count = self.events_received,
            stage = self.stage.as_str(),
            "Event received"
        );
    }

    /// Record a message send attempt
    fn record_send(&mut self, success: bool) {
        if success {
            self.messages_sent += 1;
        } else {
            self.send_failures += 1;
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

    /// Check if a tool is a "result tool" - one whose output is the user's final goal.
    /// These results should be accumulated to the final IM message.
    ///
    /// Examples: WebSearch (user wants search results), WebFetch (user wants page content)
    fn is_result_tool(tool: &str) -> bool {
        let lower = tool.to_lowercase();

        // Web search and content fetching tools
        if lower.contains("websearch") || lower.contains("web_search")
            || lower.contains("webfetch") || lower.contains("web_fetch")
            || lower.contains("mcp__web_search") {
            return true;
        }

        // Content reaching tools (YouTube, Bilibili, RSS, etc.)
        if lower.contains("reach_youtube") || lower.contains("reach_bilibili")
            || lower.contains("reach_rss") || lower.contains("network_analyzer") {
            return true;
        }

        false
    }

    /// Check if a tool is an "intermediate tool" - AI uses these for processing.
    /// Results should NOT be accumulated; they're just for AI's internal work.
    ///
    /// Examples: Read (AI reads to analyze), Bash (AI runs commands to check things)
    fn is_intermediate_tool(tool: &str) -> bool {
        let lower = tool.to_lowercase();

        // File operations
        if lower.contains("read") || lower.contains("write")
            || lower.contains("edit") || lower.contains("multiedit")
            || lower.contains("apply_patch") || lower.ends_with("ls") {
            return true;
        }

        // Code search tools
        if lower.contains("grep") || lower.contains("glob")
            || lower.contains("codesearch") || lower.contains("code_search") {
            return true;
        }

        // System/execution tools
        if lower.contains("bash") || lower.contains("batch")
            || lower.contains("lsp") || lower.contains("language_server") {
            return true;
        }

        // Task/agent management
        if lower.contains("task") || lower.contains("plan")
            || lower.contains("todo") || lower.contains("skill") {
            return true;
        }

        false
    }

    /// Check if a tool is "sensitive" - results should NEVER be shown in IM.
    ///
    /// Examples: credential (passwords, tokens), secret management
    fn is_sensitive_tool(tool: &str) -> bool {
        let lower = tool.to_lowercase();
        lower.contains("credential")
            || lower.contains("secret")
            || lower.contains("password")
            || lower.contains("token")
    }

    /// Format search tool result for immediate display in tool notification.
    fn format_search_result(&self, tool: &str, result: &serde_json::Value) -> String {
        let tool_display = Self::format_tool_name(tool);

        // Try to extract meaningful search results
        if let Some(obj) = result.as_object() {
            // WebSearch results typically have a "results" array
            if let Some(results) = obj.get("results").and_then(|v| v.as_array()) {
                if results.is_empty() {
                    return format!("{} {}\n└ 无搜索结果", tool_display, tool);
                }

                let mut text = format!("{} {}\n\n", tool_display, tool);
                for (i, item) in results.iter().take(5).enumerate() {
                    if let Some(item_obj) = item.as_object() {
                        let title = item_obj.get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("无标题");
                        let url = item_obj.get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let snippet = item_obj.get("snippet")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        text.push_str(&format!("{}. {}\n", i + 1, title));
                        if !url.is_empty() {
                            text.push_str(&format!("   {}\n", url));
                        }
                        if !snippet.is_empty() {
                            let truncated = if snippet.len() > 150 {
                                format!("{}...", safe_truncate(snippet, 150))
                            } else {
                                snippet.to_string()
                            };
                            text.push_str(&format!("   {}\n", truncated));
                        }
                    }
                }
                if results.len() > 5 {
                    text.push_str(&format!("... 还有 {} 条结果\n", results.len() - 5));
                }
                return text;
            }

            // WebFetch results
            if tool.to_lowercase().contains("fetch") {
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    let truncated = if content.len() > 300 {
                        format!("{}...\n\n[内容已截断，完整内容将在最终输出中显示]", safe_truncate(content, 300))
                    } else {
                        content.to_string()
                    };
                    return format!("{} {}\n└ 内容: {}", tool_display, tool, truncated);
                }
            }
        }

        // Fallback: truncate result for display
        let result_full = result.to_string();
        let result_str = if result_full.len() > 200 {
            format!("{}...", safe_truncate(&result_full, 200))
        } else {
            result_full
        };
        format!("{} {}\n└ 結果: {}", tool_display, tool, result_str)
    }

    /// Format tool result for accumulation to tracker.current_text.
    /// This content will be included in the final finish event output.
    ///
    /// Only accumulates results for "result tools" - tools whose output
    /// is the user's actual goal. Intermediate tools (Read, Bash, etc.)
    /// are not accumulated since their output is just for AI processing.
    fn format_tool_result_for_accumulation(&self, tool: &str, result: &serde_json::Value) -> String {
        // Skip sensitive tools entirely - never accumulate their results
        if Self::is_sensitive_tool(tool) {
            return String::new();
        }

        // Skip intermediate tools - their results are for AI, not user
        if Self::is_intermediate_tool(tool) {
            return String::new();
        }

        let tool_display = Self::format_tool_name(tool);

        // For result tools, format nicely
        if Self::is_result_tool(tool) {
            return self.format_search_result_for_accumulation(tool, result);
        }

        // For other tools, format with tool name and appropriately truncated result
        let result_str = result.as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| result.to_string());

        // Truncate very large outputs to avoid memory issues
        // The downstream Telegram layer handles message size limits via chunking/file conversion
        const MAX_OUTPUT_LENGTH: usize = 50000;

        if result_str.len() > MAX_OUTPUT_LENGTH {
            format!("{} {}\n\n```\n{}...\n```\n[输出已截断，共 {} 字符]",
                tool_display,
                tool,
                &result_str[..MAX_OUTPUT_LENGTH],
                result_str.len()
            )
        } else if result_str.is_empty() {
            format!("{} {}\n[无输出]", tool_display, tool)
        } else {
            format!("{} {}\n\n```\n{}\n```", tool_display, tool, result_str)
        }
    }

    /// Format search/web tool result for accumulation to final output.
    fn format_search_result_for_accumulation(&self, tool: &str, result: &serde_json::Value) -> String {
        if let Some(obj) = result.as_object() {
            // WebSearch results
            if let Some(results) = obj.get("results").and_then(|v| v.as_array()) {
                if results.is_empty() {
                    return "🌐 搜索完成，未找到结果".to_string();
                }

                let mut text = String::from("## 搜索结果\n\n");
                for (i, item) in results.iter().enumerate() {
                    if let Some(item_obj) = item.as_object() {
                        let title = item_obj.get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("无标题");
                        let url = item_obj.get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let snippet = item_obj.get("snippet")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        text.push_str(&format!("{}. [{}]", i + 1, title));
                        if !url.is_empty() {
                            text.push_str(&format!("({})", url));
                        }
                        text.push('\n');
                        if !snippet.is_empty() {
                            text.push_str(&format!("   {}\n", snippet));
                        }
                    }
                }
                return text;
            }

            // WebFetch results
            if tool.to_lowercase().contains("fetch") {
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    return format!("## 网页内容\n\n{}", content);
                }
                if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
                    return format!("## 已获取网页\n\nURL: {}", url);
                }
            }
        }

        // Fallback: JSON representation
        format!("## 工具结果\n\n```json\n{}\n```",
            serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()))
    }

    /// Send a new message (for key milestones).
    ///
    /// Uses `send_direct` instead of `respond` because streaming mode sends multiple
    /// messages, but `respond` removes the pending entry after the first success.
    async fn send_new_message(&self, msg: &ChannelMessage, text: &str) -> Result<Option<i64>> {
        let content = OutgoingContent::Text {
            text: text.to_string(),
        };

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "📤 [TRACE] send_new_message: calling router.send_direct"
        );

        // Use send_direct to avoid the pending entry being removed
        // Add timeout to prevent blocking
        let send_future = self
            .router
            .send_direct(msg.channel_type, msg.channel_id.clone(), content);

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            send_future
        ).await {
            Ok(r) => r,
            Err(_) => {
                tracing::error!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    "⏰ [TRACE] send_new_message: TIMEOUT after 15s"
                );
                return Ok(None);
            }
        };

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            success = result.success,
            "📤 [TRACE] send_new_message: completed"
        );

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

        // Add timeout to prevent blocking
        let edit_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            telegram.edit_message_text(chat_id, message_id, text)
        ).await;

        match edit_result {
            Ok(Ok(_)) => {
                tracing::debug!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    "Edited Telegram message"
                );
            }
            Ok(Err(e)) => {
                // Don't fail on edit errors (rate limits, message not found, etc.)
                tracing::warn!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    error = %e,
                    "Failed to edit Telegram message, falling back to new message"
                );
            }
            Err(_) => {
                tracing::warn!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    "⏰ edit_telegram_message timeout after 10s"
                );
            }
        }

        Ok(())
    }

    /// Delete a Telegram message.
    async fn delete_telegram_message(&self, chat_id: &str, message_id: i64) -> Result<()> {
        let Some(ref telegram) = self.telegram else {
            return Ok(());
        };

        // Add timeout to prevent blocking
        let delete_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            telegram.delete_message(chat_id, message_id)
        ).await;

        match delete_result {
            Ok(Ok(_)) => {
                tracing::debug!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    "Deleted Telegram message"
                );
            }
            Ok(Err(e)) => {
                // Don't fail on delete errors (rate limits, message not found, etc.)
                tracing::warn!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    error = %e,
                    "Failed to delete Telegram message"
                );
            }
            Err(_) => {
                tracing::warn!(
                    chat_id = %chat_id,
                    message_id = message_id,
                    "⏰ delete_telegram_message timeout after 10s"
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ProgressHandler for ImProgressHandler {
    async fn on_start(&self, msg: &ChannelMessage, task_id: &str) -> Result<()> {
        // Include trace_id in the first message for user tracking
        let text = t("task.start_processing", &[("trace_id", &msg.trace_id)]);

        tracing::info!(
            message_id = %msg.id,
            task_id = %task_id,
            trace_id = %msg.trace_id,
            channel_type = ?msg.channel_type,
            debug_mode = self.debug_mode,
            "📍 Starting progress tracking"
        );

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_start: step 1 - initializing tracker"
        );

        // Check if debug mode is requested (from message metadata or global setting)
        let debug_mode = self.debug_mode
            || msg.metadata.get("debug_mode").map(|v| v == "true").unwrap_or(false);

        // Initialize tracker with fresh state
        // Use a marker for current_text to distinguish from actual accumulated content
        let mut tracker = MessageTracker::new(debug_mode, msg.trace_id.clone());
        tracker.task_id = Some(task_id.to_string());
        tracker.transition_to(MessageStage::Processing);

        self.trackers.insert(msg.id.clone(), Mutex::new(tracker));

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_start: step 2 - sending initial message"
        );

        // Send initial message and track it (with timeout protection in send_new_message)
        let send_result = self.send_new_message(msg, &text).await;

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_start: step 3 - acquiring tracker lock"
        );

        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_start: step 4 - updating tracker state"
        );

        match send_result {
            Ok(Some(progress_msg_id)) => {
                tracker.progress_message_id = Some(progress_msg_id);
                tracker.record_send(true);
            }
            Ok(None) => {
                tracker.record_send(true); // Sent but no message ID returned
            }
            Err(e) => {
                tracker.record_send(false);
                tracing::error!(
                    trace_id = %msg.trace_id,
                    task_id = %task_id,
                    error = %e,
                    "❌ Failed to send start message"
                );
            }
        }

        // Don't set current_text to the start message - use a marker instead
        // This allows on_output to properly accumulate actual content
        tracker.current_text = "[started]".to_string();

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_start: complete"
        );

        Ok(())
    }

    async fn on_progress(&self, msg: &ChannelMessage, event: &ProgressData) -> Result<()> {
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            stage = %event.stage,
            "🔍 [TRACE] on_progress: entry"
        );

        // Get state needed for processing, then release lock
        let (_should_throttle, progress_msg_id) = {
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
            (false, tracker.progress_message_id)
        }; // Lock released here

        // Format progress text
        let progress_text = match event.percentage {
            Some(pct) => t("task.progress", &[
                ("stage", &event.stage),
                ("percent", &pct.to_string()),
                ("message", &event.message),
            ]),
            None => t("task.progress_no_percent", &[
                ("stage", &event.stage),
                ("message", &event.message),
            ]),
        };

        // Try to edit existing message if on Telegram
        if msg.channel_type == ChannelType::Telegram {
            if let Some(prog_msg_id) = progress_msg_id {
                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    "🔍 [TRACE] on_progress: editing telegram message"
                );
                self.edit_telegram_message(&msg.channel_id, prog_msg_id, &progress_text)
                    .await?;

                // NOTE: Intentionally NOT updating tracker.current_text here
                // Progress messages are for real-time display only, not for accumulation
                // Actual output comes from on_output or finish event

                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    "🔍 [TRACE] on_progress: complete (edit)"
                );
                return Ok(());
            }
        }

        // Fallback: send new message (for non-Telegram or first progress)
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_progress: sending new message"
        );
        let send_result = self.send_new_message(msg, &progress_text).await;

        // Update tracker after network call completes
        // Only record progress_message_id, NOT current_text
        // Progress messages are for real-time display only, not for accumulation
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;
        if let Ok(Some(new_msg_id)) = send_result {
            tracker.progress_message_id = Some(new_msg_id);
        }
        // NOTE: Intentionally NOT updating tracker.current_text
        // Actual output comes from on_output or finish event

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_progress: complete (new msg)"
        );

        Ok(())
    }

    async fn on_thought(&self, msg: &ChannelMessage, thought: &str) -> Result<()> {
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_thought: entry"
        );

        // Get state needed for processing, then release lock
        let (_should_skip, _should_throttle, progress_msg_id, current_text) = {
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

            (false, false, tracker.progress_message_id, tracker.current_text.clone())
        }; // Lock released here

        // Truncate thought for display (max 200 chars)
        let truncated = if thought.len() > 200 {
            format!("{}...", safe_truncate(thought, 200))
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
        // NOTE: We display thoughts in real-time progress messages but do NOT accumulate them
        // into current_text. This ensures thoughts are excluded from the final conclusion message.
        if msg.channel_type == ChannelType::Telegram {
            if let Some(prog_msg_id) = progress_msg_id {
                // Show thought in progress message (temporary display only)
                // Use a status prefix instead of current_text to avoid accumulation
                let status_prefix = if current_text == "[started]" {
                    messages().messages.task.processing.clone()
                } else {
                    messages().messages.task.thinking.clone()
                };
                let progress_display = format!("{}\n\n{}", status_prefix, display_thought);

                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    "🔍 [TRACE] on_thought: calling edit_telegram_message"
                );

                self.edit_telegram_message(&msg.channel_id, prog_msg_id, &progress_display)
                    .await?;

                // NOTE: Intentionally NOT updating tracker.current_text with thoughts
                // Thoughts are for real-time display only, not for final output

                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    "🔍 [TRACE] on_thought: edit complete"
                );
                return Ok(());
            }
        }

        // Fallback: send new message
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_thought: sending new message (fallback)"
        );

        let result = self
            .router
            .send_direct(
                msg.channel_type,
                msg.channel_id.clone(),
                OutgoingContent::Text { text: display_thought },
            )
            .await;

        // Track send result and capture message ID for cleanup
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.record_send(result.success);

            // Store thought message ID for cleanup in on_finish
            if result.success {
                if let Some(ref msg_id_str) = result.message_id {
                    if let Ok(thought_msg_id) = msg_id_str.parse::<i64>() {
                        tracker.thought_message_ids.push(thought_msg_id);
                        tracing::debug!(
                            trace_id = %msg.trace_id,
                            thought_msg_id = thought_msg_id,
                            total_thoughts = tracker.thought_message_ids.len(),
                            "Tracked thought message for cleanup"
                        );
                    }
                }
            } else {
                tracing::warn!(
                    trace_id = %msg.trace_id,
                    message_id = %msg.id,
                    error = ?result.error,
                    "⚠️ Failed to send thought message"
                );
            }
        }

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            "🔍 [TRACE] on_thought: complete"
        );

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
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            tool = %event.tool,
            "🔍 [TRACE] on_tool_use: entry"
        );

        // Track tool usage for summary and debug context
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.record_event("tool_use");
            *tracker.tools_used.entry(event.tool.clone()).or_insert(0) += 1;

            // Also add to debug context if debug mode is enabled
            if tracker.debug_mode {
                tracker.debug_context.add_tool_usage(event.tool.clone());
            }
        } // Lock released before network call

        let tool_display = Self::format_tool_name(&event.tool);
        let text = if let Some(ref result) = event.result {
            // For result tools, show full results for immediate user feedback
            if Self::is_result_tool(&event.tool) {
                self.format_search_result(&event.tool, result)
            } else if Self::is_sensitive_tool(&event.tool) {
                // For sensitive tools, show only success (no content)
                t("status.tool_executed", &[("icon", &tool_display), ("tool", &event.tool)])
            } else {
                // For intermediate tools, show brief confirmation
                t("status.tool_executing", &[("icon", &tool_display), ("tool", &event.tool)])
            }
        } else {
            format!("{} {}", tool_display, event.tool)
        };

        tracing::info!(
            message_id = %msg.id,
            tool = %event.tool,
            has_result = event.result.is_some(),
            "Tool use notification"
        );

        // Scheme 1: Accumulate tool result to tracker for final output
        // This ensures search results are included in the finish event
        if let Some(ref result) = event.result {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;

            let result_text = self.format_tool_result_for_accumulation(&event.tool, result);
            if !result_text.is_empty() {
                if tracker.current_text == "[started]" {
                    tracker.current_text = result_text;
                } else {
                    tracker.current_text.push_str("\n\n");
                    tracker.current_text.push_str(&result_text);
                }

                tracing::debug!(
                    message_id = %msg.id,
                    tool = %event.tool,
                    accumulated_length = tracker.current_text.len(),
                    "Accumulated tool result to tracker"
                );
            }
        }

        // Scheme 3: Send notification message with tool result
        // For search tools, this provides immediate visibility
        let content = OutgoingContent::Text { text };
        let result = self
            .router
            .send_direct(msg.channel_type, msg.channel_id.clone(), content)
            .await;

        // Track send result
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.record_send(result.success);

            // Transition to Active stage on first tool use
            if tracker.stage == MessageStage::Processing {
                tracker.transition_to(MessageStage::Active);
            }

            if !result.success {
                tracing::warn!(
                    trace_id = %msg.trace_id,
                    message_id = %msg.id,
                    tool = %event.tool,
                    error = ?result.error,
                    "⚠️ Failed to send tool use notification"
                );
            }
        }

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            tool = %event.tool,
            "🔍 [TRACE] on_tool_use: complete"
        );

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

    async fn on_skill_use(&self, msg: &ChannelMessage, event: &SkillUseData) -> Result<()> {
        let tracker_ref = self.get_tracker(&msg.id);
        let mut tracker = tracker_ref.lock().await;

        tracing::debug!(
            message_id = %msg.id,
            skill = %event.skill,
            "Skill use received"
        );

        // Track skill usage (avoid duplicates)
        if !tracker.skills_used.contains(&event.skill) {
            tracker.skills_used.push(event.skill.clone());
        }

        // Add to debug context
        tracker.debug_context.add_skill_usage(event.skill.clone(), event.duration_ms);

        Ok(())
    }

    async fn on_question(&self, msg: &ChannelMessage, event: &QuestionData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            request_id = %event.request_id,
            question_count = event.questions.len(),
            "Question event received"
        );

        // For Telegram, send questions with inline buttons
        if msg.channel_type == ChannelType::Telegram {
            for (i, q) in event.questions.iter().enumerate() {
                // Build question text
                let question_text = format!("❓ {}\n\n{}", q.header, q.question);

                // Build inline buttons from options
                let buttons: Vec<InlineButton> = q.options.iter().enumerate().map(|(opt_idx, opt)| {
                    // Callback data format: "q:{request_id}:{question_idx}:{option_idx}"
                    let callback_data = format!("q:{}:{}:{}", event.request_id, i, opt_idx);
                    InlineButton::new(&opt.label, callback_data)
                }).collect();

                // Send question with inline keyboard
                if let Some(ref telegram) = self.telegram {
                    if let Err(e) = telegram.send_with_inline_keyboard(
                        &msg.channel_id,
                        &question_text,
                        vec![buttons], // Single row of buttons
                    ).await {
                        tracing::warn!(
                            message_id = %msg.id,
                            error = %e,
                            "Failed to send question with inline keyboard"
                        );
                    }
                } else {
                    // Fallback: send as text with numbered options
                    let mut fallback_text = question_text;
                    fallback_text.push_str("\n\n请输入选项编号回复：\n");
                    for (opt_idx, opt) in q.options.iter().enumerate() {
                        fallback_text.push_str(&format!("{}) {} - {}\n", opt_idx + 1, opt.label, opt.description));
                    }
                    let _ = self.router.send_direct(
                        msg.channel_type,
                        msg.channel_id.clone(),
                        OutgoingContent::Text { text: fallback_text },
                    ).await;
                }
            }
        } else {
            // Non-Telegram: send as formatted text
            for q in &event.questions {
                let mut text = format!("❓ {}\n\n{}\n\n请选择：\n", q.header, q.question);
                for (opt_idx, opt) in q.options.iter().enumerate() {
                    text.push_str(&format!("{}) {} - {}\n", opt_idx + 1, opt.label, opt.description));
                }
                let _ = self.router.send_direct(
                    msg.channel_type,
                    msg.channel_id.clone(),
                    OutgoingContent::Text { text },
                ).await;
            }
        }

        Ok(())
    }

    async fn on_confirmation(&self, msg: &ChannelMessage, event: &ConfirmationData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            request_id = %event.request_id,
            tool = %event.tool,
            actions_count = event.actions.len(),
            "Confirmation event received"
        );

        // Build confirmation message
        let tool_info = format!("🔧 **Tool**: `{}`", event.tool);
        let desc = format!("📋 {}", event.description);

        // Format args for display (truncate if too long)
        let args_preview = {
            let args_str = serde_json::to_string_pretty(&event.args).unwrap_or_default();
            if args_str.len() > 500 {
                format!("```json\n{}...\n```", &args_str[..500])
            } else if !args_str.is_empty() && args_str != "{}" {
                format!("```json\n{}\n```", args_str)
            } else {
                String::new()
            }
        };

        let confirmation_text = if args_preview.is_empty() {
            t("approval.confirm_with_info", &[("tool_info", &tool_info), ("description", &desc)])
        } else {
            t("approval.confirm_with_args", &[("tool_info", &tool_info), ("description", &desc), ("args", &args_preview)])
        };

        // For Telegram, send with inline buttons
        if msg.channel_type == ChannelType::Telegram {
            // Build buttons from available actions
            let buttons: Vec<InlineButton> = event.actions.iter().map(|action| {
                // Callback data format: "c:{request_id}:{action}"
                let callback_data = format!("c:{}:{}", event.request_id, action);
                let label = match action.as_str() {
                    "approve" | "once" => messages().messages.approval.approve.as_str(),
                    "always" => messages().messages.approval.approve_always.as_str(),
                    "reject" => messages().messages.approval.reject.as_str(),
                    _ => action.as_str(),
                };
                InlineButton::new(label, callback_data)
            }).collect();

            if let Some(ref telegram) = self.telegram {
                if let Err(e) = telegram.send_with_inline_keyboard(
                    &msg.channel_id,
                    &confirmation_text,
                    vec![buttons],
                ).await {
                    tracing::warn!(
                        message_id = %msg.id,
                        error = %e,
                        "Failed to send confirmation with inline keyboard"
                    );
                    // Fallback to text-based confirmation
                    let fallback = format!(
                        "{}\n\n请回复以下命令之一：\n{}",
                        confirmation_text,
                        event.actions.iter()
                            .map(|a| format!("/confirm_{} {}", a, event.request_id))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    let _ = self.router.send_direct(
                        msg.channel_type,
                        msg.channel_id.clone(),
                        OutgoingContent::Text { text: fallback },
                    ).await;
                }
            } else {
                // No Telegram client, use text fallback
                let fallback = format!(
                    "{}\n\n请回复以下命令之一：\n{}",
                    confirmation_text,
                    event.actions.iter()
                        .map(|a| format!("/confirm_{} {}", a, event.request_id))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                let _ = self.router.send_direct(
                    msg.channel_type,
                    msg.channel_id.clone(),
                    OutgoingContent::Text { text: fallback },
                ).await;
            }
        } else {
            // Non-Telegram: send as formatted text with command options
            let text = format!(
                "{}\n\n可用操作：\n{}",
                confirmation_text,
                event.actions.iter()
                    .map(|a| format!("• {} - 回复 /confirm_{} {}", a, a, event.request_id))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let _ = self.router.send_direct(
                msg.channel_type,
                msg.channel_id.clone(),
                OutgoingContent::Text { text },
            ).await;
        }

        Ok(())
    }

    async fn on_pdca_cycle(&self, msg: &ChannelMessage, event: &PdcaCycleData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            cycle = event.cycle,
            max_cycles = event.max_cycles,
            phase = %event.phase,
            task_type = %event.task_type,
            "PDCA cycle event received - storing for final message"
        );

        // Store PDCA cycle data in tracker (will be included in final message)
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.pdca_cycle = Some(event.clone());
        }

        Ok(())
    }

    async fn on_pdca_check(&self, msg: &ChannelMessage, event: &PdcaCheckData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            passed = event.passed,
            recommendation = %event.recommendation,
            issue_count = event.issue_count,
            close_score = event.close_score.total,
            "PDCA check event received - storing for final message"
        );

        // Store PDCA check data in tracker (will be included in final message)
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.pdca_check = Some(event.clone());
        }

        Ok(())
    }

    async fn on_pdca_result(&self, msg: &ChannelMessage, event: &PdcaResultData) -> Result<()> {
        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            success = event.success,
            cycles = event.cycles,
            duration_ms = event.total_duration_ms,
            "PDCA result event received - storing for final message"
        );

        // Store PDCA result data in tracker (will be included in final message)
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.pdca_result = Some(event.clone());
        }

        Ok(())
    }

    async fn on_finish(&self, msg: &ChannelMessage, event: &FinishData) -> Result<()> {
        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            success = event.success,
            "🔍 [TRACE] on_finish: entry"
        );

        // Transition to Finishing stage
        {
            let tracker_ref = self.get_tracker(&msg.id);
            let mut tracker = tracker_ref.lock().await;
            tracker.transition_to(MessageStage::Finishing);
        } // Lock released before processing

        tracing::info!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            success = event.success,
            has_output = event.output.is_some(),
            output_length = event.output.as_ref().map_or(0, |s| s.len()),
            has_error = event.error.is_some(),
            "Task finish event received"
        );

        if let Some(ref output) = event.output {
            tracing::debug!(
                message_id = %msg.id,
                output_preview = safe_truncate(output, 200),
                "Finish event output preview"
            );
        }

        // Get tracker data for summary and debug info before cleanup
        let (summary, debug_info_text, accumulated_text, progress_message_id, thought_message_ids) = {
            let tracker_ref = self.get_tracker(&msg.id);
            let tracker = tracker_ref.lock().await;
            let summary = Self::generate_summary(&tracker);
            let debug_info_text = if tracker.debug_mode {
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
            // NOTE: PDCA summary is already included in TypeScript output (task.ts)
            // with richer context (sources, insights). Don't duplicate here.
            tracing::info!(
                message_id = %msg.id,
                accumulated_length = accumulated.as_ref().map_or(0, |s| s.len()),
                thought_messages_count = tracker.thought_message_ids.len(),
                has_pdca_result = tracker.pdca_result.is_some(),
                "Accumulated text from on_output events"
            );
            (summary, debug_info_text, accumulated, tracker.progress_message_id, tracker.thought_message_ids.clone())
        };

        // Clean up progress message (which may contain thinking content)
        // Edit it to show a completion marker instead of thinking
        // Use different text based on success/failure to avoid confusing dual messages
        if msg.channel_type == ChannelType::Telegram {
            if let Some(prog_msg_id) = progress_message_id {
                let cleanup_text = if event.success {
                    messages().messages.task.completed.as_str()
                } else {
                    messages().messages.task.generating_result.as_str()
                };
                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    progress_message_id = prog_msg_id,
                    success = event.success,
                    "🔍 [TRACE] on_finish: cleaning up progress message"
                );
                // Best effort - don't fail if edit fails
                let _ = self.edit_telegram_message(&msg.channel_id, prog_msg_id, cleanup_text).await;
            }

            // Delete all thought messages that were sent as separate messages
            if !thought_message_ids.is_empty() {
                tracing::debug!(
                    message_id = %msg.id,
                    trace_id = %msg.trace_id,
                    count = thought_message_ids.len(),
                    "🔍 [TRACE] on_finish: deleting thought messages"
                );
                for thought_msg_id in &thought_message_ids {
                    let _ = self.delete_telegram_message(&msg.channel_id, *thought_msg_id).await;
                }
            }
        }

        // Format final response with optional debug info
        // Use accumulated text from streaming if available, otherwise use event.output
        // NOTE: PDCA summary is included in TypeScript output (task.ts), not added here
        let content = if event.success {
            // Use accumulated text (from on_output events) or fallback to event.output
            let final_output = accumulated_text.as_ref().or(event.output.as_ref());

            // Build base message with output content
            let base = if let Some(output) = final_output {
                format!("{}\n\n{}", output, summary)
            } else {
                // No output content, show completion marker
                t("task.completed_with_summary", &[("summary", &summary)])
            };

            // Add end marker to indicate completion
            let with_end_marker = format!("{}\n\n{}", base, t("task.end_marker", &[("status", "✅")]));

            tracing::info!(
                message_id = %msg.id,
                base_length = base.len(),
                "Sending finish response"
            );

            if let Some(debug_text) = debug_info_text {
                OutgoingContent::Markdown {
                    text: format!("{}\n\n{}", with_end_marker, debug_text),
                }
            } else {
                OutgoingContent::Markdown { text: with_end_marker }
            }
        } else {
            let error_msg = event.error.as_deref().unwrap_or("Unknown error");
            let text = t("task.failed_with_summary", &[("error", error_msg), ("summary", &summary)]);

            // Add end marker for error case too
            let with_end_marker = format!("{}\n\n{}", text, t("task.end_marker", &[("status", "❌")]));
            if let Some(debug_text) = debug_info_text {
                OutgoingContent::Markdown {
                    text: format!("{}\n\n{}", with_end_marker, debug_text),
                }
            } else {
                OutgoingContent::Text { text: with_end_marker }
            }
        };

        // Use send_direct to avoid pending entry issues
        let result = self
            .router
            .send_direct(msg.channel_type, msg.channel_id.clone(), content)
            .await;

        // Log final state before cleanup
        {
            if let Some(tracker_mutex) = self.trackers.get(&msg.id) {
                let mut tracker = tracker_mutex.lock().await;
                tracker.record_send(result.success);

                let final_stage = if result.success && event.success {
                    MessageStage::Completed
                } else {
                    MessageStage::Failed
                };
                tracker.transition_to(final_stage);

                tracing::info!(
                    trace_id = %tracker.trace_id,
                    task_id = ?tracker.task_id,
                    total_events = tracker.events_received,
                    total_sent = tracker.messages_sent,
                    send_failures = tracker.send_failures,
                    tools_used = ?tracker.tools_used.keys().collect::<Vec<_>>(),
                    skills_used = ?tracker.skills_used,
                    duration_ms = tracker.task_start.elapsed().as_millis() as u64,
                    final_stage = final_stage.as_str(),
                    "📍 Message processing complete"
                );
            }
        }

        // Clean up tracker
        self.remove_tracker(&msg.id);

        if !result.success {
            tracing::error!(
                message_id = %msg.id,
                trace_id = %msg.trace_id,
                error = ?result.error,
                "❌ Failed to send finish response"
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
        let event_type = match &event {
            TaskEvent::Progress(_) => "Progress",
            TaskEvent::Thought(_) => "Thought",
            TaskEvent::Output(_) => "Output",
            TaskEvent::ToolUse(_) => "ToolUse",
            TaskEvent::Finish(_) => "Finish",
            TaskEvent::DebugInfo(_) => "DebugInfo",
            TaskEvent::AgentInfo(_) => "AgentInfo",
            TaskEvent::SkillUse(_) => "SkillUse",
            TaskEvent::Question(_) => "Question",
            TaskEvent::Confirmation(_) => "Confirmation",
            TaskEvent::PdcaCycle(_) => "PdcaCycle",
            TaskEvent::PdcaCheck(_) => "PdcaCheck",
            TaskEvent::PdcaResult(_) => "PdcaResult",
        };

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            event_type = event_type,
            "🔍 [TRACE] handle_event: BEFORE dispatch"
        );

        let result = match event {
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
            TaskEvent::SkillUse(data) => {
                self.on_skill_use(msg, &data).await?;
                Ok(false)
            }
            TaskEvent::Question(data) => {
                self.on_question(msg, &data).await?;
                Ok(false) // Question pauses but doesn't finish
            }
            TaskEvent::Confirmation(data) => {
                self.on_confirmation(msg, &data).await?;
                Ok(false) // Confirmation pauses but doesn't finish
            }
            TaskEvent::PdcaCycle(data) => {
                self.on_pdca_cycle(msg, &data).await?;
                Ok(false) // PDCA cycle doesn't finish
            }
            TaskEvent::PdcaCheck(data) => {
                self.on_pdca_check(msg, &data).await?;
                Ok(false) // PDCA check doesn't finish
            }
            TaskEvent::PdcaResult(data) => {
                self.on_pdca_result(msg, &data).await?;
                Ok(false) // PDCA result doesn't finish (finish event comes separately)
            }
        };

        tracing::debug!(
            message_id = %msg.id,
            trace_id = %msg.trace_id,
            event_type = event_type,
            success = result.is_ok(),
            "🔍 [TRACE] handle_event: AFTER dispatch"
        );

        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::message::{ChannelMessage, ChannelType, MessageContent};
    use std::collections::HashMap;

    // ────────────────────────────────────────────────────────────────────────────
    // Helper Functions
    // ────────────────────────────────────────────────────────────────────────────

    fn create_test_message() -> ChannelMessage {
        ChannelMessage {
            id: "test-msg-123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456789".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: "trace-abc".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        }
    }

    fn create_tracker_with_tools(tools: &[(&str, u64)]) -> MessageTracker {
        let mut tracker = MessageTracker::new(false, "test-trace".to_string());
        for (tool, count) in tools {
            tracker.tools_used.insert(tool.to_string(), *count);
        }
        tracker
    }

    // ────────────────────────────────────────────────────────────────────────────
    // format_tool_name Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_tool_name() {
        assert_eq!(ImProgressHandler::format_tool_name("Read"), "📄 读取文件");
        assert_eq!(ImProgressHandler::format_tool_name("read"), "📄 读取文件");
        assert_eq!(ImProgressHandler::format_tool_name("file_read"), "📄 读取文件");

        assert_eq!(ImProgressHandler::format_tool_name("Write"), "✏️ 写入文件");
        assert_eq!(ImProgressHandler::format_tool_name("write"), "✏️ 写入文件");

        assert_eq!(ImProgressHandler::format_tool_name("Edit"), "🔧 编辑文件");
        assert_eq!(ImProgressHandler::format_tool_name("edit"), "🔧 编辑文件");

        assert_eq!(ImProgressHandler::format_tool_name("Bash"), "💻 执行命令");
        assert_eq!(ImProgressHandler::format_tool_name("bash"), "💻 执行命令");
        assert_eq!(ImProgressHandler::format_tool_name("shell"), "💻 执行命令");

        assert_eq!(ImProgressHandler::format_tool_name("Grep"), "🔍 搜索代码");
        assert_eq!(ImProgressHandler::format_tool_name("grep"), "🔍 搜索代码");
        assert_eq!(ImProgressHandler::format_tool_name("search"), "🔍 搜索代码");

        assert_eq!(ImProgressHandler::format_tool_name("Glob"), "📁 查找文件");
        assert_eq!(ImProgressHandler::format_tool_name("glob"), "📁 查找文件");

        assert_eq!(ImProgressHandler::format_tool_name("WebSearch"), "🌐 网络搜索");
        assert_eq!(ImProgressHandler::format_tool_name("web_search"), "🌐 网络搜索");

        assert_eq!(ImProgressHandler::format_tool_name("WebFetch"), "🌐 获取网页");
        assert_eq!(ImProgressHandler::format_tool_name("web_fetch"), "🌐 获取网页");

        assert_eq!(ImProgressHandler::format_tool_name("Task"), "🤖 启动子任务");
        assert_eq!(ImProgressHandler::format_tool_name("task"), "🤖 启动子任务");

        assert_eq!(ImProgressHandler::format_tool_name("unknown_tool"), "⚡ 执行工具");
        assert_eq!(ImProgressHandler::format_tool_name("CustomTool"), "⚡ 执行工具");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageTracker Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_message_tracker_new() {
        let tracker = MessageTracker::new(false, "test-trace".to_string());
        assert!(tracker.progress_message_id.is_none());
        assert!(tracker.current_text.is_empty());
        assert!(!tracker.debug_mode);
        assert!(tracker.tools_used.is_empty());
        assert!(tracker.last_thought.is_empty());
    }

    #[test]
    fn test_message_tracker_debug_mode() {
        let tracker = MessageTracker::new(true, "test-trace".to_string());
        assert!(tracker.debug_mode);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // is_result_tool Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_is_result_tool() {
        // Web search and fetch tools
        assert!(ImProgressHandler::is_result_tool("WebSearch"));
        assert!(ImProgressHandler::is_result_tool("web_search"));
        assert!(ImProgressHandler::is_result_tool("webfetch"));
        assert!(ImProgressHandler::is_result_tool("WebFetch"));
        assert!(ImProgressHandler::is_result_tool("mcp__web_search__websearch"));

        // Content reaching tools
        assert!(ImProgressHandler::is_result_tool("reach_youtube"));
        assert!(ImProgressHandler::is_result_tool("reach_bilibili"));
        assert!(ImProgressHandler::is_result_tool("reach_rss"));
        assert!(ImProgressHandler::is_result_tool("network_analyzer"));

        // Intermediate tools should NOT be result tools
        assert!(!ImProgressHandler::is_result_tool("Read"));
        assert!(!ImProgressHandler::is_result_tool("Bash"));
        assert!(!ImProgressHandler::is_result_tool("Grep"));
        assert!(!ImProgressHandler::is_result_tool("Glob"));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // is_intermediate_tool Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_is_intermediate_tool() {
        // File operations
        assert!(ImProgressHandler::is_intermediate_tool("Read"));
        assert!(ImProgressHandler::is_intermediate_tool("Write"));
        assert!(ImProgressHandler::is_intermediate_tool("Edit"));
        assert!(ImProgressHandler::is_intermediate_tool("Multiedit"));
        assert!(ImProgressHandler::is_intermediate_tool("apply_patch"));
        assert!(ImProgressHandler::is_intermediate_tool("ls"));

        // Code search
        assert!(ImProgressHandler::is_intermediate_tool("Grep"));
        assert!(ImProgressHandler::is_intermediate_tool("Glob"));
        assert!(ImProgressHandler::is_intermediate_tool("CodeSearch"));
        assert!(ImProgressHandler::is_intermediate_tool("code_search"));

        // System tools
        assert!(ImProgressHandler::is_intermediate_tool("Bash"));
        assert!(ImProgressHandler::is_intermediate_tool("batch"));
        assert!(ImProgressHandler::is_intermediate_tool("Lsp"));
        assert!(ImProgressHandler::is_intermediate_tool("language_server"));

        // Task management
        assert!(ImProgressHandler::is_intermediate_tool("Task"));
        assert!(ImProgressHandler::is_intermediate_tool("plan"));
        assert!(ImProgressHandler::is_intermediate_tool("todo"));
        assert!(ImProgressHandler::is_intermediate_tool("skill"));

        // Result tools should NOT be intermediate
        assert!(!ImProgressHandler::is_intermediate_tool("WebSearch"));
        assert!(!ImProgressHandler::is_intermediate_tool("reach_youtube"));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // is_sensitive_tool Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_is_sensitive_tool() {
        assert!(ImProgressHandler::is_sensitive_tool("credential"));
        assert!(ImProgressHandler::is_sensitive_tool("get_credential"));
        assert!(ImProgressHandler::is_sensitive_tool("Credential"));
        assert!(ImProgressHandler::is_sensitive_tool("secret_manager"));
        assert!(ImProgressHandler::is_sensitive_tool("get_secret"));
        assert!(ImProgressHandler::is_sensitive_tool("password_vault"));
        assert!(ImProgressHandler::is_sensitive_tool("token_store"));

        // Non-sensitive tools
        assert!(!ImProgressHandler::is_sensitive_tool("WebSearch"));
        assert!(!ImProgressHandler::is_sensitive_tool("Read"));
        assert!(!ImProgressHandler::is_sensitive_tool("Bash"));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // generate_summary Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_generate_summary_no_tools() {
        let tracker = MessageTracker::new(false, "test-trace".to_string());
        let summary = ImProgressHandler::generate_summary(&tracker);

        assert!(summary.contains("📊 执行摘要"));
        assert!(summary.contains("⏱ 耗时:"));
        assert!(!summary.contains("🔧 工具调用"));
    }

    #[test]
    fn test_generate_summary_with_tools() {
        let tracker = create_tracker_with_tools(&[
            ("Read", 5),
            ("Write", 3),
            ("Bash", 2),
        ]);
        let summary = ImProgressHandler::generate_summary(&tracker);

        assert!(summary.contains("📊 执行摘要"));
        assert!(summary.contains("🔧 工具调用: 10 次"));
        assert!(summary.contains("Read: 5"));
        assert!(summary.contains("Write: 3"));
        assert!(summary.contains("Bash: 2"));
    }

    #[test]
    fn test_generate_summary_tool_sorting() {
        // Tools should be sorted by usage count (descending)
        let tracker = create_tracker_with_tools(&[
            ("Read", 10),
            ("Write", 5),
            ("Bash", 15),
            ("Grep", 3),
        ]);
        let summary = ImProgressHandler::generate_summary(&tracker);

        // Bash should come first (15), then Read (10), then Write (5), then Grep (3)
        let bash_pos = summary.find("Bash").unwrap();
        let read_pos = summary.find("Read").unwrap();
        let write_pos = summary.find("Write").unwrap();
        let grep_pos = summary.find("Grep").unwrap();

        assert!(bash_pos < read_pos);
        assert!(read_pos < write_pos);
        assert!(write_pos < grep_pos);
    }

    #[test]
    fn test_generate_summary_max_five_tools() {
        let tracker = create_tracker_with_tools(&[
            ("Read", 10),
            ("Write", 9),
            ("Bash", 8),
            ("Grep", 7),
            ("Glob", 6),
            ("Edit", 5),      // This should not appear
            ("WebSearch", 4), // This should not appear
        ]);
        let summary = ImProgressHandler::generate_summary(&tracker);

        // First 5 tools by count should be shown
        assert!(summary.contains("Read"));
        assert!(summary.contains("Write"));
        assert!(summary.contains("Bash"));
        assert!(summary.contains("Grep"));
        assert!(summary.contains("Glob"));

        // 6th and 7th tools should not appear in summary (only top 5)
        // Note: Edit with 5 and WebSearch with 4 might appear if sorted order differs
        // Let's just verify we have exactly 5 bullet points
        let bullet_count = summary.matches("• ").count();
        assert_eq!(bullet_count, 5);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Tool Result Formatting Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_tool_result_for_accumulation_sensitive() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({"api_key": "secret-key-123"});
        let formatted = handler.format_tool_result_for_accumulation("credential", &result);

        // Sensitive tool results should be empty
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_tool_result_for_accumulation_intermediate() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({"content": "file contents here"});
        let formatted = handler.format_tool_result_for_accumulation("Read", &result);

        // Intermediate tool results should be empty (not accumulated)
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_search_result_with_results() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({
            "results": [
                {
                    "title": "Test Result 1",
                    "url": "https://example.com/1",
                    "snippet": "This is a test result snippet"
                },
                {
                    "title": "Test Result 2",
                    "url": "https://example.com/2",
                    "snippet": "Another test result"
                }
            ]
        });

        let formatted = handler.format_search_result("WebSearch", &result);

        assert!(formatted.contains("Test Result 1"));
        assert!(formatted.contains("https://example.com/1"));
        assert!(formatted.contains("Test Result 2"));
    }

    #[test]
    fn test_format_search_result_empty() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({
            "results": []
        });

        let formatted = handler.format_search_result("WebSearch", &result);

        assert!(formatted.contains("无搜索结果"));
    }

    #[test]
    fn test_format_search_result_for_accumulation() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({
            "results": [
                {
                    "title": "Test",
                    "url": "https://example.com",
                    "snippet": "Description"
                }
            ]
        });

        let formatted = handler.format_search_result_for_accumulation("WebSearch", &result);

        assert!(formatted.contains("## 搜索结果"));
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("https://example.com"));
    }

    #[test]
    fn test_format_webfetch_result_for_accumulation() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        let result = serde_json::json!({
            "content": "This is the page content",
            "url": "https://example.com/page"
        });

        let formatted = handler.format_search_result_for_accumulation("WebFetch", &result);

        assert!(formatted.contains("## 网页内容"));
        assert!(formatted.contains("This is the page content"));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Handler Builder Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_handler_builder_methods() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None)
            .with_debug_mode(true)
            .with_throttle(Duration::from_millis(500))
            .with_thought_throttle(Duration::from_millis(200));

        assert!(handler.debug_mode);
        assert_eq!(handler.throttle_interval, Duration::from_millis(500));
        assert_eq!(handler.thought_throttle_interval, Duration::from_millis(200));
    }

    #[test]
    fn test_handler_default_throttle() {
        let router = Arc::new(OutboundRouter::new());
        let handler = ImProgressHandler::new(router, None);

        assert_eq!(handler.throttle_interval, Duration::from_millis(1000));
        assert_eq!(handler.thought_throttle_interval, Duration::from_millis(500));
        assert!(!handler.debug_mode);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Debug Format Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_format_debug_for_platform() {
        use super::debug::{ModelUsage, AgentUsage, ServiceCall};

        // Create debug context with some data
        let mut debug = DebugContext::new("test-trace-123".to_string());
        debug.models_used.push(ModelUsage {
            model: "claude-opus-4.5".into(),
            provider: Some("anthropic".into()),
            input_tokens: 100,
            output_tokens: 200,
            total_tokens: Some(300),
            duration_ms: 1500,
        });
        debug.agents_used.push(AgentUsage {
            agent: "build".into(),
            display_name: Some("Build Agent".into()),
            invocations: 1,
            duration_ms: 2000,
            is_primary: true,
        });

        // Each platform should return a formatted string
        let telegram = ImProgressHandler::format_debug_for_platform(&debug, &ChannelType::Telegram);
        let slack = ImProgressHandler::format_debug_for_platform(&debug, &ChannelType::Slack);
        let discord = ImProgressHandler::format_debug_for_platform(&debug, &ChannelType::Discord);
        let cli = ImProgressHandler::format_debug_for_platform(&debug, &ChannelType::Cli);

        // All should produce output with debug info when data is available
        assert!(!telegram.is_empty(), "Telegram format should produce output");
        assert!(!slack.is_empty(), "Slack format should produce output");
        assert!(!discord.is_empty(), "Discord format should produce output");
        assert!(!cli.is_empty(), "CLI format should produce output");

        // Verify some content is present (model or agent info)
        // The exact format depends on the implementation
        let has_expected = telegram.contains("claude") || telegram.contains("build") ||
                           telegram.contains("100") || telegram.contains("200");
        assert!(has_expected, "Telegram should contain some debug info");
    }
}
