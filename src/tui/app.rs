/// ─── App State ────────────────────────────────────────────────────────────
///
/// Extracted from mod.rs to reduce the 5000+ line file.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::completion;

pub struct TuiApp {
    /// 消息历史（纯文本，后续升级为富文本）
    pub messages: Vec<MessageItem>,

    /// 当前输入缓冲区
    pub input: String,

    /// 光标位置（字符索引）
    pub cursor_pos: usize,

    /// 输入历史（↑↓ 导航）
    pub input_history: Vec<String>,
    pub history_pos: usize,

    /// 撤销/重做栈
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,

    /// 消息列表滚动偏移（0 = 顶部，正数 = 向下滚动行数）
    pub scroll_offset: usize,

    /// 是否自动跟随底部（新消息时保持在最下方）
    pub auto_scroll: bool,

    /// @ 文件补全状态
    pub completion: CompletionState,

    /// Ctrl+F 搜索状态
    pub search_active: bool,
    pub search_query: String,
    pub search_match_count: usize,
    pub search_current_match: usize,

    /// Ctrl+R 反向搜索状态
    pub reverse_search_active: bool,
    pub reverse_search_query: String,
    pub reverse_search_results: Vec<usize>,
    pub reverse_search_idx: usize,

    /// 斜杠命令补全状态
    pub slash_completion: SlashCompletionState,

    /// 帮助面板状态
    pub help_active: bool,

    /// 模型切换器状态
    pub model_picker_active: bool,
    pub model_picker_selected: usize,
    pub available_models: Vec<String>,

    /// 覆盖层对话框（权限/计划/提问）
    pub dialog: Option<Dialog>,

    /// 主题切换（暗/亮模式）
    pub dark_mode: bool,

    /// 消息选择模式 —选中消息的索引
    pub selected_msg: Option<usize>,



    /// 状态栏数据
    pub status: StatusData,

    /// 消息发送时间戳（用于显示耗时）
    pub thinking_start_time: Option<Instant>,

    /// 当前工具调用轮次
    pub current_round: usize,

    /// 是否需要退出
    pub should_quit: bool,

    /// 会话持久化存储
    pub session_store: Option<crate::session::SessionStore>,
    /// 当前会话 ID（None = 新会话）
    pub current_session_id: Option<String>,
    /// 配置存储
    pub config_store: Option<crate::config::ConfigStore>,
    /// MCP 注册表
    pub mcp_registry: Option<Arc<Mutex<crate::mcp::McpRegistry>>>,
}

/// 消息列表中的一条
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageItem {
    User { text: String },
    Assistant { text: String },
    ToolCall { name: String, input: String, output: String, expanded: bool, show_full: bool },
    System { text: String },
    /// Collapsible reasoning/thinking block (CoT)
    Reasoning { text: String, expanded: bool },
}

/// 覆盖层对话框枚举（替代 PendingPermission）
#[derive(Debug, Clone)]
pub enum Dialog {
    ToolPermission {
        tool_name: String,
        tool_input: String,
        request_id: u64,
        risk: String,
    },
    PlanApproval {
        title: String,
        plan: String,
        request_id: u64,
    },
    AskQuestion {
        question: String,
        request_id: u64,
    },
    PlanReview {
        title: String,
        plan: String,
        request_id: u64,
    },
}

/// 斜杠命令补全状态
#[derive(Debug, Clone)]
pub struct SlashCompletionState {
    pub active: bool,
    pub selected: usize,
    pub commands: Vec<&'static str>,
    pub descriptions: Vec<&'static str>,
}

impl Default for SlashCompletionState {
    fn default() -> Self {
        Self {
            active: false,
            selected: 0,
            commands: vec![
                "/help", "/exit", "/quit", "/reload", "/clear", "/history",
                "/session", "/resume", "/config", "/mcp", "/tools", "/skills", "/memory",
            ],
            descriptions: vec![
                "Show help and shortcuts", "Exit the application", "Exit the application",
                "Reload context and skills", "Clear conversation history", "Show message count",
                "List saved sessions", "Resume a previous session",
                "View or change settings (model, api_base, etc.)",
                "Manage MCP servers (list, start, stop)", "List available tools",
                "List loaded skills", "List memory entries",
            ],
        }
    }
}

/// @ 文件补全状态
#[derive(Debug, Clone)]
pub struct CompletionState {
    pub active: bool,
    pub query: String,
    pub candidates: Vec<completion::CompletionCandidate>,
    pub selected: usize,
    pub at_pos: usize,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self {
            active: false,
            query: String::new(),
            candidates: Vec::new(),
            selected: 0,
            at_pos: 0,
        }
    }
}

/// 状态栏数据
#[derive(Debug, Clone)]
pub struct StatusData {
    pub model: String,
    pub cwd: String,
    pub context_pct: f32,
    pub token_count: usize,
    pub api_key_set: bool,
    pub agent_busy: bool,
    pub current_tool: Option<String>,
    pub connection_type: String,
    pub elapsed_secs: u64,
    pub current_round: usize,
    pub streaming_complete: bool,
}

impl Default for StatusData {
    fn default() -> Self {
        Self {
            model: "gpt-4o".into(),
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            context_pct: 0.0,
            token_count: 0,
            api_key_set: std::env::var("CODECODER_API_KEY").is_ok(),
            agent_busy: false,
            current_tool: None,
            connection_type: if std::env::var("CODECODER_API_KEY").is_ok() {
                "OpenAI".into()
            } else {
                "Stub".into()
            },
            elapsed_secs: 0,
            current_round: 0,
            streaming_complete: false,
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            input_history: Vec::new(),
            history_pos: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            completion: CompletionState::default(),
            search_active: false,
            search_query: String::new(),
            search_match_count: 0,
            search_current_match: 0,
            reverse_search_active: false,
            reverse_search_query: String::new(),
            reverse_search_results: Vec::new(),
            reverse_search_idx: 0,
            model_picker_active: false,
            model_picker_selected: 0,
            available_models: vec![
                "gpt-4o".into(), "gpt-4o-mini".into(), "gpt-4.1".into(),
                "gpt-4.1-mini".into(), "gpt-4.1-nano".into(), "o3".into(),
                "o4-mini".into(), "claude-sonnet-4-20250514".into(),
                "claude-haiku-3-5".into(), "deepseek-chat".into(),
                "llama3.2".into(), "gemini-2.5-flash".into(),
            ],
            dialog: None,
            dark_mode: true,
            selected_msg: None,
            slash_completion: SlashCompletionState::default(),
            help_active: false,
            status: StatusData::default(),
            thinking_start_time: None,
            current_round: 0,
            should_quit: false,
            session_store: None,
            current_session_id: None,
            config_store: None,
            mcp_registry: None,
        }
    }
}
