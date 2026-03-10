//! Internationalized messages configuration.
//!
//! This module loads message templates from a configuration file
//! (`~/.codecoder/messages.json`) and provides functions for retrieving
//! translated messages with parameter interpolation.
//!
//! # Configuration File
//!
//! ```json
//! {
//!   "locale": "zh-CN",
//!   "messages": {
//!     "task": {
//!       "acknowledged": "🚀 收到，正在处理...",
//!       "failed": "❌ 处理失败: {error}"
//!     },
//!     "approval": {
//!       "approve": "✅ 批准",
//!       "reject": "❌ 拒绝"
//!     }
//!   }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use zero_core::common::messages::{messages, t};
//!
//! // Simple message
//! let msg = messages().task.acknowledged.clone();
//!
//! // Message with parameters
//! let error_msg = t("task.failed", &[("error", "Network timeout")]);
//! // => "❌ 处理失败: Network timeout"
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use super::config::config_dir;

// ============================================================================
// Types
// ============================================================================

/// Task lifecycle messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessages {
    /// Sent when task is received and queued
    #[serde(default = "default_task_acknowledged")]
    pub acknowledged: String,

    /// Sent when task processing starts
    #[serde(default = "default_task_start_processing")]
    pub start_processing: String,

    /// Generic processing indicator
    #[serde(default = "default_task_processing")]
    pub processing: String,

    /// Shown when AI is reasoning/thinking
    #[serde(default = "default_task_thinking")]
    pub thinking: String,

    /// Progress update with percentage
    #[serde(default = "default_task_progress")]
    pub progress: String,

    /// Progress update without percentage
    #[serde(default = "default_task_progress_no_percent")]
    pub progress_no_percent: String,

    /// Sent when task completes successfully
    #[serde(default = "default_task_completed")]
    pub completed: String,

    /// Completion with summary
    #[serde(default = "default_task_completed_with_summary")]
    pub completed_with_summary: String,

    /// Sent when task fails
    #[serde(default = "default_task_failed")]
    pub failed: String,

    /// Failure with summary
    #[serde(default = "default_task_failed_with_summary")]
    pub failed_with_summary: String,

    /// Shown while generating final result
    #[serde(default = "default_task_generating_result")]
    pub generating_result: String,

    /// End marker with status
    #[serde(default = "default_task_end_marker")]
    pub end_marker: String,

    /// Task ID suffix
    #[serde(default = "default_task_id_suffix")]
    pub task_id_suffix: String,
}

impl Default for TaskMessages {
    fn default() -> Self {
        Self {
            acknowledged: default_task_acknowledged(),
            start_processing: default_task_start_processing(),
            processing: default_task_processing(),
            thinking: default_task_thinking(),
            progress: default_task_progress(),
            progress_no_percent: default_task_progress_no_percent(),
            completed: default_task_completed(),
            completed_with_summary: default_task_completed_with_summary(),
            failed: default_task_failed(),
            failed_with_summary: default_task_failed_with_summary(),
            generating_result: default_task_generating_result(),
            end_marker: default_task_end_marker(),
            task_id_suffix: default_task_id_suffix(),
        }
    }
}

/// Approval/HitL messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalMessages {
    /// Title for authorization requests
    #[serde(default = "default_approval_title")]
    pub title: String,

    /// Header for confirmation dialogs
    #[serde(default = "default_approval_confirm_action")]
    pub confirm_action: String,

    /// Confirmation with tool info
    #[serde(default = "default_approval_confirm_with_info")]
    pub confirm_with_info: String,

    /// Confirmation with tool info and args
    #[serde(default = "default_approval_confirm_with_args")]
    pub confirm_with_args: String,

    /// Approve button text
    #[serde(default = "default_approval_approve")]
    pub approve: String,

    /// Always approve button text
    #[serde(default = "default_approval_approve_always")]
    pub approve_always: String,

    /// Reject button text
    #[serde(default = "default_approval_reject")]
    pub reject: String,

    /// Status: approved
    #[serde(default = "default_approval_approved")]
    pub approved: String,

    /// Approved with approver name
    #[serde(default = "default_approval_approved_by")]
    pub approved_by: String,

    /// Status: rejected
    #[serde(default = "default_approval_rejected")]
    pub rejected: String,

    /// Rejected with approver name
    #[serde(default = "default_approval_rejected_by")]
    pub rejected_by: String,

    /// Rejected with full details
    #[serde(default = "default_approval_rejected_with_reason")]
    pub rejected_with_reason: String,

    /// Status: pending approval
    #[serde(default = "default_approval_pending")]
    pub pending: String,

    /// Waiting for approval status
    #[serde(default = "default_approval_waiting")]
    pub waiting: String,

    /// Approval queue title
    #[serde(default = "default_approval_queue_title")]
    pub queue_title: String,

    /// Empty approval queue message
    #[serde(default = "default_approval_queue_empty")]
    pub queue_empty: String,

    /// Selection prompt text
    #[serde(default = "default_approval_select_prompt")]
    pub select_prompt: String,
}

impl Default for ApprovalMessages {
    fn default() -> Self {
        Self {
            title: default_approval_title(),
            confirm_action: default_approval_confirm_action(),
            confirm_with_info: default_approval_confirm_with_info(),
            confirm_with_args: default_approval_confirm_with_args(),
            approve: default_approval_approve(),
            approve_always: default_approval_approve_always(),
            reject: default_approval_reject(),
            approved: default_approval_approved(),
            approved_by: default_approval_approved_by(),
            rejected: default_approval_rejected(),
            rejected_by: default_approval_rejected_by(),
            rejected_with_reason: default_approval_rejected_with_reason(),
            pending: default_approval_pending(),
            waiting: default_approval_waiting(),
            queue_title: default_approval_queue_title(),
            queue_empty: default_approval_queue_empty(),
            select_prompt: default_approval_select_prompt(),
        }
    }
}

/// Status indicator messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessages {
    /// Auto-approval decision
    #[serde(default = "default_status_auto_approve")]
    pub auto_approve: String,

    /// Pending approval decision
    #[serde(default = "default_status_pending_approval")]
    pub pending_approval: String,

    /// Denied decision
    #[serde(default = "default_status_denied")]
    pub denied: String,

    /// Tool executed (result hidden)
    #[serde(default = "default_status_tool_executed")]
    pub tool_executed: String,

    /// Tool executing
    #[serde(default = "default_status_tool_executing")]
    pub tool_executing: String,

    /// User answer received
    #[serde(default = "default_status_answer_received")]
    pub answer_received: String,

    /// User answer failed
    #[serde(default = "default_status_answer_failed")]
    pub answer_failed: String,

    /// Option selected
    #[serde(default = "default_status_option_selected")]
    pub option_selected: String,

    /// Status: pass
    #[serde(default = "default_status_pass")]
    pub pass: String,

    /// Status: needs improvement
    #[serde(default = "default_status_needs_improvement")]
    pub needs_improvement: String,

    /// Section header for existing capabilities
    #[serde(default = "default_status_existing_capabilities")]
    pub existing_capabilities: String,

    /// Section header for risk warnings
    #[serde(default = "default_status_risk_warning")]
    pub risk_warning: String,
}

impl Default for StatusMessages {
    fn default() -> Self {
        Self {
            auto_approve: default_status_auto_approve(),
            pending_approval: default_status_pending_approval(),
            denied: default_status_denied(),
            tool_executed: default_status_tool_executed(),
            tool_executing: default_status_tool_executing(),
            answer_received: default_status_answer_received(),
            answer_failed: default_status_answer_failed(),
            option_selected: default_status_option_selected(),
            pass: default_status_pass(),
            needs_improvement: default_status_needs_improvement(),
            existing_capabilities: default_status_existing_capabilities(),
            risk_warning: default_status_risk_warning(),
        }
    }
}

/// Error messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessages {
    /// Generic load failure
    #[serde(default = "default_error_load_failed")]
    pub load_failed: String,

    /// Approval action failed
    #[serde(default = "default_error_approve_failed")]
    pub approve_failed: String,

    /// Rejection action failed
    #[serde(default = "default_error_reject_failed")]
    pub reject_failed: String,

    /// Generic operation failure
    #[serde(default = "default_error_operation_failed")]
    pub operation_failed: String,

    /// Operation failed with retry hint
    #[serde(default = "default_error_operation_failed_retry")]
    pub operation_failed_retry: String,

    /// Config save failure
    #[serde(default = "default_error_config_save_failed")]
    pub config_save_failed: String,

    /// Config load failure
    #[serde(default = "default_error_config_load_failed")]
    pub config_load_failed: String,

    /// Connection lost during processing
    #[serde(default = "default_error_connection_lost")]
    pub connection_lost: String,

    /// Telegram not configured error
    #[serde(default = "default_error_telegram_not_configured")]
    pub telegram_not_configured: String,

    /// Verification failure
    #[serde(default = "default_error_verification_failed")]
    pub verification_failed: String,

    /// Error message prefix
    #[serde(default = "default_error_prefix")]
    pub error_prefix: String,
}

impl Default for ErrorMessages {
    fn default() -> Self {
        Self {
            load_failed: default_error_load_failed(),
            approve_failed: default_error_approve_failed(),
            reject_failed: default_error_reject_failed(),
            operation_failed: default_error_operation_failed(),
            operation_failed_retry: default_error_operation_failed_retry(),
            config_save_failed: default_error_config_save_failed(),
            config_load_failed: default_error_config_load_failed(),
            connection_lost: default_error_connection_lost(),
            telegram_not_configured: default_error_telegram_not_configured(),
            verification_failed: default_error_verification_failed(),
            error_prefix: default_error_prefix(),
        }
    }
}

/// Authorization/binding messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMessages {
    /// Trading notification binding success
    #[serde(default = "default_auth_binding_success")]
    pub binding_success: String,
}

impl Default for AuthMessages {
    fn default() -> Self {
        Self {
            binding_success: default_auth_binding_success(),
        }
    }
}

/// Search-related messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMessages {
    /// Search completed with no results
    #[serde(default = "default_search_no_results")]
    pub no_results: String,
}

impl Default for SearchMessages {
    fn default() -> Self {
        Self {
            no_results: default_search_no_results(),
        }
    }
}

/// Autonomous mode messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousMessages {
    /// Autonomous mode task completed
    #[serde(default = "default_autonomous_task_completed")]
    pub task_completed: String,

    /// Autonomous mode task incomplete
    #[serde(default = "default_autonomous_task_incomplete")]
    pub task_incomplete: String,

    /// Status: problem solved
    #[serde(default = "default_autonomous_status_solved")]
    pub status_solved: String,

    /// Status: could not auto-resolve
    #[serde(default = "default_autonomous_status_not_solved")]
    pub status_not_solved: String,

    /// Build success
    #[serde(default = "default_autonomous_build_success")]
    pub build_success: String,

    /// Build failed
    #[serde(default = "default_autonomous_build_failed")]
    pub build_failed: String,

    /// Autonomous decision paused
    #[serde(default = "default_autonomous_decision_paused")]
    pub decision_paused: String,
}

impl Default for AutonomousMessages {
    fn default() -> Self {
        Self {
            task_completed: default_autonomous_task_completed(),
            task_incomplete: default_autonomous_task_incomplete(),
            status_solved: default_autonomous_status_solved(),
            status_not_solved: default_autonomous_status_not_solved(),
            build_success: default_autonomous_build_success(),
            build_failed: default_autonomous_build_failed(),
            decision_paused: default_autonomous_decision_paused(),
        }
    }
}

/// Context management messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessages {
    /// Context clear failed
    #[serde(default = "default_context_clear_failed")]
    pub clear_failed: String,

    /// Context compact failed
    #[serde(default = "default_context_compact_failed")]
    pub compact_failed: String,

    /// Context clear error with retry hint
    #[serde(default = "default_context_clear_error_retry")]
    pub clear_error_retry: String,
}

impl Default for ContextMessages {
    fn default() -> Self {
        Self {
            clear_failed: default_context_clear_failed(),
            compact_failed: default_context_compact_failed(),
            clear_error_retry: default_context_clear_error_retry(),
        }
    }
}

/// All message categories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AllMessages {
    #[serde(default)]
    pub task: TaskMessages,
    #[serde(default)]
    pub approval: ApprovalMessages,
    #[serde(default)]
    pub status: StatusMessages,
    #[serde(default)]
    pub error: ErrorMessages,
    #[serde(default)]
    pub auth: AuthMessages,
    #[serde(default)]
    pub search: SearchMessages,
    #[serde(default)]
    pub autonomous: AutonomousMessages,
    #[serde(default)]
    pub context: ContextMessages,
}

/// Root messages configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesConfig {
    /// Locale identifier
    #[serde(default = "default_locale")]
    pub locale: String,

    /// Configuration version
    #[serde(default = "default_version")]
    pub version: String,

    /// All message categories
    #[serde(default)]
    pub messages: AllMessages,
}

impl Default for MessagesConfig {
    fn default() -> Self {
        Self {
            locale: default_locale(),
            version: default_version(),
            messages: AllMessages::default(),
        }
    }
}

// ============================================================================
// Default Value Functions - Task Messages
// ============================================================================

fn default_task_acknowledged() -> String {
    "🚀 收到，正在处理...".to_string()
}

fn default_task_start_processing() -> String {
    "🚀 开始处理...\n📍 Trace: {trace_id}".to_string()
}

fn default_task_processing() -> String {
    "🚀 处理中...".to_string()
}

fn default_task_thinking() -> String {
    "⏳ 思考中...".to_string()
}

fn default_task_progress() -> String {
    "⏳ {stage} ({percent}%)\n{message}".to_string()
}

fn default_task_progress_no_percent() -> String {
    "⏳ {stage}\n{message}".to_string()
}

fn default_task_completed() -> String {
    "✅ 处理完成".to_string()
}

fn default_task_completed_with_summary() -> String {
    "✅ 处理完成\n\n{summary}".to_string()
}

fn default_task_failed() -> String {
    "❌ 处理失败: {error}".to_string()
}

fn default_task_failed_with_summary() -> String {
    "❌ 处理失败: {error}\n\n{summary}".to_string()
}

fn default_task_generating_result() -> String {
    "⏳ 正在生成结果...".to_string()
}

fn default_task_end_marker() -> String {
    "─────────────────\n{status} 回复结束".to_string()
}

fn default_task_id_suffix() -> String {
    "\n\n任务ID: `{task_id}`".to_string()
}

// ============================================================================
// Default Value Functions - Approval Messages
// ============================================================================

fn default_approval_title() -> String {
    "🔐 CodeCoder 授权请求".to_string()
}

fn default_approval_confirm_action() -> String {
    "⚠️ **确认执行操作**".to_string()
}

fn default_approval_confirm_with_info() -> String {
    "⚠️ **确认执行操作**\n\n{tool_info}\n{description}".to_string()
}

fn default_approval_confirm_with_args() -> String {
    "⚠️ **确认执行操作**\n\n{tool_info}\n{description}\n\n{args}".to_string()
}

fn default_approval_approve() -> String {
    "✅ 批准".to_string()
}

fn default_approval_approve_always() -> String {
    "✅ 始终批准".to_string()
}

fn default_approval_reject() -> String {
    "❌ 拒绝".to_string()
}

fn default_approval_approved() -> String {
    "✅ 已批准".to_string()
}

fn default_approval_approved_by() -> String {
    "✅ 已批准 by {approver}".to_string()
}

fn default_approval_rejected() -> String {
    "❌ 已拒绝".to_string()
}

fn default_approval_rejected_by() -> String {
    "❌ 已拒绝 by {approver}".to_string()
}

fn default_approval_rejected_with_reason() -> String {
    "❌ 已拒绝\n审批人: {approver}\n原因: {reason}\n时间: {time}".to_string()
}

fn default_approval_pending() -> String {
    "⏳ 待审批".to_string()
}

fn default_approval_waiting() -> String {
    "⏳ 等待审批".to_string()
}

fn default_approval_queue_title() -> String {
    "📋 审批队列 ({count} 待处理)".to_string()
}

fn default_approval_queue_empty() -> String {
    "没有待处理的审批请求".to_string()
}

fn default_approval_select_prompt() -> String {
    "请选择批准或拒绝此操作：".to_string()
}

// ============================================================================
// Default Value Functions - Status Messages
// ============================================================================

fn default_status_auto_approve() -> String {
    "自动批准".to_string()
}

fn default_status_pending_approval() -> String {
    "等待审批".to_string()
}

fn default_status_denied() -> String {
    "拒绝".to_string()
}

fn default_status_tool_executed() -> String {
    "{icon} {tool}\n✅ 已执行 (结果已隐藏)".to_string()
}

fn default_status_tool_executing() -> String {
    "{icon} {tool}\n✅ 执行中...".to_string()
}

fn default_status_answer_received() -> String {
    "✅ 已收到回答".to_string()
}

fn default_status_answer_failed() -> String {
    "❌ 回答失败".to_string()
}

fn default_status_option_selected() -> String {
    "✅ 已选择选项 {index}".to_string()
}

fn default_status_pass() -> String {
    "✅ 通过".to_string()
}

fn default_status_needs_improvement() -> String {
    "⚠️ 需改进".to_string()
}

fn default_status_existing_capabilities() -> String {
    "✅ **现有能力**".to_string()
}

fn default_status_risk_warning() -> String {
    "⚠️ **风险提示**".to_string()
}

// ============================================================================
// Default Value Functions - Error Messages
// ============================================================================

fn default_error_load_failed() -> String {
    "加载失败: {error}".to_string()
}

fn default_error_approve_failed() -> String {
    "批准失败".to_string()
}

fn default_error_reject_failed() -> String {
    "拒绝失败".to_string()
}

fn default_error_operation_failed() -> String {
    "❌ 操作失败: {error}".to_string()
}

fn default_error_operation_failed_retry() -> String {
    "❌ 操作失败，请稍后重试: {error}".to_string()
}

fn default_error_config_save_failed() -> String {
    "❌ 配置保存失败: {error}".to_string()
}

fn default_error_config_load_failed() -> String {
    "❌ 配置加载失败: {error}".to_string()
}

fn default_error_connection_lost() -> String {
    "处理异常中断 (收到 {count} 个事件后连接断开)。请重试。".to_string()
}

fn default_error_telegram_not_configured() -> String {
    "❌ Telegram 未配置，无法绑定交易通知".to_string()
}

fn default_error_verification_failed() -> String {
    "验证失败。".to_string()
}

fn default_error_prefix() -> String {
    "❌ 错误: {error}".to_string()
}

// ============================================================================
// Default Value Functions - Auth Messages
// ============================================================================

fn default_auth_binding_success() -> String {
    "✅ *交易通知绑定成功*\n\n已绑定到当前 Telegram 会话。\n所有交易信号将推送至此处。".to_string()
}

// ============================================================================
// Default Value Functions - Search Messages
// ============================================================================

fn default_search_no_results() -> String {
    "🌐 搜索完成，未找到结果".to_string()
}

// ============================================================================
// Default Value Functions - Autonomous Messages
// ============================================================================

fn default_autonomous_task_completed() -> String {
    "🤖 **[自主模式] 任务完成**".to_string()
}

fn default_autonomous_task_incomplete() -> String {
    "🤖 **[自主模式] 任务未完成**".to_string()
}

fn default_autonomous_status_solved() -> String {
    "✅ **状态**: 问题已解决".to_string()
}

fn default_autonomous_status_not_solved() -> String {
    "⚠️ **状态**: 未能自动解决".to_string()
}

fn default_autonomous_build_success() -> String {
    "✅ **成功构建**: {type} - {identifier}".to_string()
}

fn default_autonomous_build_failed() -> String {
    "❌ **构建失败**: {summary}".to_string()
}

fn default_autonomous_decision_paused() -> String {
    "⚠️ 自主决策暂停\n\n**CLOSE 评估分数**: {score}/10".to_string()
}

// ============================================================================
// Default Value Functions - Context Messages
// ============================================================================

fn default_context_clear_failed() -> String {
    "❌ 清空上下文失败: {error}".to_string()
}

fn default_context_compact_failed() -> String {
    "❌ 压缩上下文失败: {error}".to_string()
}

fn default_context_clear_error_retry() -> String {
    "⚠️ 清空上下文时出现错误，请重试。".to_string()
}

// ============================================================================
// Default Value Functions - Root
// ============================================================================

fn default_locale() -> String {
    "zh-CN".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

// ============================================================================
// Loading Functions
// ============================================================================

/// Path to the messages configuration file.
pub fn messages_path() -> PathBuf {
    config_dir().join("messages.json")
}

/// Load messages configuration from file.
///
/// Returns config if successful, default if file doesn't exist.
pub fn load_messages_from_file() -> MessagesConfig {
    let path = messages_path();

    if !path.exists() {
        tracing::debug!("Messages config not found, using defaults");
        return MessagesConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<MessagesConfig>(&content) {
            Ok(config) => {
                tracing::info!(
                    path = %path.display(),
                    locale = %config.locale,
                    "Loaded messages configuration"
                );
                config
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to parse messages config, using defaults"
                );
                MessagesConfig::default()
            }
        },
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "Failed to read messages config, using defaults"
            );
            MessagesConfig::default()
        }
    }
}

/// Global messages configuration (lazy loaded).
static MESSAGES: OnceLock<MessagesConfig> = OnceLock::new();

/// Get the global messages configuration.
pub fn messages() -> &'static MessagesConfig {
    MESSAGES.get_or_init(load_messages_from_file)
}

/// Load messages configuration (for API compatibility).
pub fn load_messages() -> &'static MessagesConfig {
    messages()
}

// ============================================================================
// Translation Function
// ============================================================================

/// Get a translated message with parameter interpolation.
///
/// # Arguments
///
/// * `key` - Dot-notation key path (e.g., "task.failed", "approval.approve")
/// * `params` - Optional parameters to interpolate as `&[("key", "value")]`
///
/// # Returns
///
/// The translated message with parameters interpolated.
///
/// # Examples
///
/// ```rust,ignore
/// use zero_core::common::messages::t;
///
/// // Simple message
/// let msg = t("task.acknowledged", &[]);
/// // => "🚀 收到，正在处理..."
///
/// // Message with parameters
/// let error = t("task.failed", &[("error", "Network timeout")]);
/// // => "❌ 处理失败: Network timeout"
///
/// // Multiple parameters
/// let rejected = t("approval.rejected_with_reason", &[
///     ("approver", "admin"),
///     ("reason", "Too risky"),
///     ("time", "2024-01-01 12:00"),
/// ]);
/// ```
pub fn t(key: &str, params: &[(&str, &str)]) -> String {
    let config = messages();
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() != 2 {
        tracing::warn!(key = %key, "Invalid message key format");
        return key.to_string();
    }

    let category = parts[0];
    let message_key = parts[1];

    let template = get_template(config, category, message_key);

    match template {
        Some(t) => interpolate(&t, params),
        None => {
            tracing::warn!(key = %key, "Unknown message key");
            key.to_string()
        }
    }
}

/// Get a template string by category and key.
fn get_template(config: &MessagesConfig, category: &str, key: &str) -> Option<String> {
    match category {
        "task" => match key {
            "acknowledged" => Some(config.messages.task.acknowledged.clone()),
            "start_processing" => Some(config.messages.task.start_processing.clone()),
            "processing" => Some(config.messages.task.processing.clone()),
            "thinking" => Some(config.messages.task.thinking.clone()),
            "progress" => Some(config.messages.task.progress.clone()),
            "progress_no_percent" => Some(config.messages.task.progress_no_percent.clone()),
            "completed" => Some(config.messages.task.completed.clone()),
            "completed_with_summary" => Some(config.messages.task.completed_with_summary.clone()),
            "failed" => Some(config.messages.task.failed.clone()),
            "failed_with_summary" => Some(config.messages.task.failed_with_summary.clone()),
            "generating_result" => Some(config.messages.task.generating_result.clone()),
            "end_marker" => Some(config.messages.task.end_marker.clone()),
            "task_id_suffix" => Some(config.messages.task.task_id_suffix.clone()),
            _ => None,
        },
        "approval" => match key {
            "title" => Some(config.messages.approval.title.clone()),
            "confirm_action" => Some(config.messages.approval.confirm_action.clone()),
            "confirm_with_info" => Some(config.messages.approval.confirm_with_info.clone()),
            "confirm_with_args" => Some(config.messages.approval.confirm_with_args.clone()),
            "approve" => Some(config.messages.approval.approve.clone()),
            "approve_always" => Some(config.messages.approval.approve_always.clone()),
            "reject" => Some(config.messages.approval.reject.clone()),
            "approved" => Some(config.messages.approval.approved.clone()),
            "approved_by" => Some(config.messages.approval.approved_by.clone()),
            "rejected" => Some(config.messages.approval.rejected.clone()),
            "rejected_by" => Some(config.messages.approval.rejected_by.clone()),
            "rejected_with_reason" => Some(config.messages.approval.rejected_with_reason.clone()),
            "pending" => Some(config.messages.approval.pending.clone()),
            "waiting" => Some(config.messages.approval.waiting.clone()),
            "queue_title" => Some(config.messages.approval.queue_title.clone()),
            "queue_empty" => Some(config.messages.approval.queue_empty.clone()),
            "select_prompt" => Some(config.messages.approval.select_prompt.clone()),
            _ => None,
        },
        "status" => match key {
            "auto_approve" => Some(config.messages.status.auto_approve.clone()),
            "pending_approval" => Some(config.messages.status.pending_approval.clone()),
            "denied" => Some(config.messages.status.denied.clone()),
            "tool_executed" => Some(config.messages.status.tool_executed.clone()),
            "tool_executing" => Some(config.messages.status.tool_executing.clone()),
            "answer_received" => Some(config.messages.status.answer_received.clone()),
            "answer_failed" => Some(config.messages.status.answer_failed.clone()),
            "option_selected" => Some(config.messages.status.option_selected.clone()),
            "pass" => Some(config.messages.status.pass.clone()),
            "needs_improvement" => Some(config.messages.status.needs_improvement.clone()),
            "existing_capabilities" => Some(config.messages.status.existing_capabilities.clone()),
            "risk_warning" => Some(config.messages.status.risk_warning.clone()),
            _ => None,
        },
        "error" => match key {
            "load_failed" => Some(config.messages.error.load_failed.clone()),
            "approve_failed" => Some(config.messages.error.approve_failed.clone()),
            "reject_failed" => Some(config.messages.error.reject_failed.clone()),
            "operation_failed" => Some(config.messages.error.operation_failed.clone()),
            "operation_failed_retry" => Some(config.messages.error.operation_failed_retry.clone()),
            "config_save_failed" => Some(config.messages.error.config_save_failed.clone()),
            "config_load_failed" => Some(config.messages.error.config_load_failed.clone()),
            "connection_lost" => Some(config.messages.error.connection_lost.clone()),
            "telegram_not_configured" => Some(config.messages.error.telegram_not_configured.clone()),
            "verification_failed" => Some(config.messages.error.verification_failed.clone()),
            "error_prefix" => Some(config.messages.error.error_prefix.clone()),
            _ => None,
        },
        "auth" => match key {
            "binding_success" => Some(config.messages.auth.binding_success.clone()),
            _ => None,
        },
        "search" => match key {
            "no_results" => Some(config.messages.search.no_results.clone()),
            _ => None,
        },
        "autonomous" => match key {
            "task_completed" => Some(config.messages.autonomous.task_completed.clone()),
            "task_incomplete" => Some(config.messages.autonomous.task_incomplete.clone()),
            "status_solved" => Some(config.messages.autonomous.status_solved.clone()),
            "status_not_solved" => Some(config.messages.autonomous.status_not_solved.clone()),
            "build_success" => Some(config.messages.autonomous.build_success.clone()),
            "build_failed" => Some(config.messages.autonomous.build_failed.clone()),
            "decision_paused" => Some(config.messages.autonomous.decision_paused.clone()),
            _ => None,
        },
        "context" => match key {
            "clear_failed" => Some(config.messages.context.clear_failed.clone()),
            "compact_failed" => Some(config.messages.context.compact_failed.clone()),
            "clear_error_retry" => Some(config.messages.context.clear_error_retry.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Interpolate parameters into a template string.
///
/// Replaces `{key}` with the corresponding value from params.
fn interpolate(template: &str, params: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_messages() {
        let config = MessagesConfig::default();
        assert_eq!(config.locale, "zh-CN");
        assert_eq!(config.messages.task.acknowledged, "🚀 收到，正在处理...");
        assert_eq!(config.messages.approval.approve, "✅ 批准");
    }

    #[test]
    fn test_t_simple() {
        // Uses defaults since no config file
        let msg = t("task.acknowledged", &[]);
        assert_eq!(msg, "🚀 收到，正在处理...");
    }

    #[test]
    fn test_t_with_params() {
        let msg = t("task.failed", &[("error", "Network timeout")]);
        assert_eq!(msg, "❌ 处理失败: Network timeout");
    }

    #[test]
    fn test_t_multiple_params() {
        let msg = t("approval.rejected_with_reason", &[
            ("approver", "admin"),
            ("reason", "Too risky"),
            ("time", "2024-01-01 12:00"),
        ]);
        assert!(msg.contains("admin"));
        assert!(msg.contains("Too risky"));
        assert!(msg.contains("2024-01-01 12:00"));
    }

    #[test]
    fn test_t_unknown_key() {
        let msg = t("unknown.key", &[]);
        assert_eq!(msg, "unknown.key");
    }

    #[test]
    fn test_t_invalid_format() {
        let msg = t("invalidkey", &[]);
        assert_eq!(msg, "invalidkey");
    }

    #[test]
    fn test_interpolate() {
        let template = "Hello {name}, you have {count} messages";
        let result = interpolate(template, &[("name", "Alice"), ("count", "5")]);
        assert_eq!(result, "Hello Alice, you have 5 messages");
    }

    #[test]
    fn test_interpolate_missing_param() {
        let template = "Hello {name}, you have {count} messages";
        let result = interpolate(template, &[("name", "Alice")]);
        assert_eq!(result, "Hello Alice, you have {count} messages");
    }
}
