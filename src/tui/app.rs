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
    /// 上一次 session save 错误（去重用：相同错误不重复 push 到消息列表）
    pub last_save_error: Option<String>,
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
}

/// 斜杠命令补全状态
#[derive(Debug, Clone)]
pub struct SlashCompletionState {
    pub active: bool,
    pub selected: usize,
    pub commands: Vec<&'static str>,
    pub descriptions: Vec<&'static str>,
    /// ADR 0002 §7: indices into `commands`/`descriptions` matching the
    /// current input prefix. When non-empty, popup + navigation show only
    /// these. When input has a space (args started), filtered resets to
    /// all indices and the popup stays open as a hint until submission.
    pub filtered: Vec<usize>,
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
            filtered: Vec::new(),
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
            last_save_error: None,
            config_store: None,
            mcp_registry: None,
        }
    }
}

// ─── Message Bounds & Compaction ────────────────────────────────────────────
//
// Two-layer strategy to keep the in-memory message list and the persisted
// session file bounded:
//
// 1. In-memory bounds (`enforce_message_bounds`): caps total message count
//    and approximate byte size. When exceeded, evicts in priority order:
//      Phase 1 — tombstone `ToolCall.output` (preserve name + input)
//      Phase 2 — drop all `Reasoning` messages entirely
//      Phase 3 — FIFO drain remaining messages
//    Always keeps at least 1 message.
//
// 2. Save-time compaction (`compact_messages_for_save`): truncates any
//    single text field larger than `COMPACT_FIELD_THRESHOLD` down to
//    `COMPACT_FIELD_TARGET` with a `[trimmed N chars]` marker. Runs on
//    every session save, so the persisted file is independently bounded.

/// Maximum number of messages kept in memory. FIFO eviction beyond this.
const MAX_MESSAGES: usize = 500;

/// Approximate byte budget for the in-memory message list (text fields only).
const MAX_MESSAGE_BYTES: usize = 50 * 1024 * 1024;

/// Per-field threshold at which save-time compaction kicks in.
const COMPACT_FIELD_THRESHOLD: usize = 10 * 1024;

/// Length to which oversized fields are truncated at save time.
const COMPACT_FIELD_TARGET: usize = 1024;

/// Approximate byte size of a single message (text fields only).
fn message_size(m: &MessageItem) -> usize {
    match m {
        MessageItem::User { text }
        | MessageItem::Assistant { text }
        | MessageItem::System { text } => text.len(),
        MessageItem::ToolCall { name, input, output, .. } => {
            name.len() + input.len() + output.len()
        }
        MessageItem::Reasoning { text, .. } => text.len(),
    }
}

/// Approximate total byte size of the message list.
fn messages_bytes(messages: &[MessageItem]) -> usize {
    messages.iter().map(message_size).sum()
}

/// True when the message list exceeds either bound. The bytes condition also
/// requires `len > 1` so we never empty the list to satisfy a byte budget.
fn exceeds_bounds(messages: &[MessageItem]) -> bool {
    messages.len() > MAX_MESSAGES
        || (messages.len() > 1 && messages_bytes(messages) > MAX_MESSAGE_BYTES)
}

/// Enforce in-memory message bounds (see module docs for the eviction order).
pub fn enforce_message_bounds(app: &mut TuiApp) {
    // Fast path: small lists skip the byte scan entirely.
    if app.messages.len() <= MAX_MESSAGES && app.messages.len() < 50 {
        return;
    }

    // Phase 1 — tombstone tool outputs.
    if exceeds_bounds(&app.messages) {
        for m in &mut app.messages {
            if let MessageItem::ToolCall { output, .. } = m {
                if !output.starts_with("[trimmed:") {
                    let n = output.len();
                    *output = format!("[trimmed: was {n} chars]");
                }
            }
        }
    }

    // Phase 2 — drop all reasoning.
    if exceeds_bounds(&app.messages) {
        app.messages.retain(|m| !matches!(m, MessageItem::Reasoning { .. }));
    }

    // Phase 3a — drain by count.
    if app.messages.len() > MAX_MESSAGES {
        let excess = app.messages.len() - MAX_MESSAGES;
        app.messages.drain(..excess);
    }

    // Phase 3b — drain by bytes (one at a time, since size per message varies).
    while app.messages.len() > 1 && messages_bytes(&app.messages) > MAX_MESSAGE_BYTES {
        app.messages.remove(0);
    }
}

/// Compact a single message's long text fields in place (save-time).
pub fn compact_message_fields(m: &mut MessageItem) {
    let trim = |s: &mut String| {
        if s.len() > COMPACT_FIELD_THRESHOLD {
            let original = s.len();
            s.truncate(COMPACT_FIELD_TARGET);
            s.push_str(&format!("[trimmed {} chars]", original - COMPACT_FIELD_TARGET));
        }
    };
    match m {
        MessageItem::User { text }
        | MessageItem::Assistant { text }
        | MessageItem::System { text }
        | MessageItem::Reasoning { text, .. } => trim(text),
        MessageItem::ToolCall { input, output, .. } => {
            trim(input);
            trim(output);
        }
    }
}

/// Apply save-time per-field compaction to a message list (in place).
pub fn compact_messages_for_save(messages: &mut [MessageItem]) {
    for m in messages.iter_mut() {
        compact_message_fields(m);
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    fn user(text: &str) -> MessageItem {
        MessageItem::User { text: text.into() }
    }
    fn assistant(text: &str) -> MessageItem {
        MessageItem::Assistant { text: text.into() }
    }
    fn tool_call(name: &str, input: &str, output: &str) -> MessageItem {
        MessageItem::ToolCall {
            name: name.into(),
            input: input.into(),
            output: output.into(),
            expanded: false,
            show_full: false,
        }
    }
    fn reasoning(text: &str) -> MessageItem {
        MessageItem::Reasoning { text: text.into(), expanded: false }
    }

    // ── enforce_message_bounds ───────────────────────────────────────────────

    #[test]
    fn bounds_noop_under_limits() {
        let mut app = TuiApp::default();
        for i in 0..10 {
            app.messages.push(user(&format!("msg {i}")));
        }
        enforce_message_bounds(&mut app);
        assert_eq!(app.messages.len(), 10, "small list should be untouched");
    }

    #[test]
    fn bounds_count_triggers_fifo() {
        let mut app = TuiApp::default();
        for i in 0..(MAX_MESSAGES + 50) {
            app.messages.push(user(&format!("msg {i}")));
        }
        enforce_message_bounds(&mut app);
        assert_eq!(app.messages.len(), MAX_MESSAGES);
        // FIFO: first 50 should be gone, last MAX_MESSAGES kept
        assert!(matches!(&app.messages[0], MessageItem::User { text } if text == "msg 50"));
    }

    #[test]
    fn bounds_tombstone_tool_output_phase_1() {
        let mut app = TuiApp::default();
        // 60 tool calls each with large output → exceeds 50-message fast-path
        // but stays under MAX_MESSAGES count. Bytes will exceed budget.
        let big = "x".repeat(1024 * 1024); // 1 MB
        for _ in 0..60 {
            app.messages.push(tool_call("read_file", "foo.rs", &big));
        }
        enforce_message_bounds(&mut app);
        // All tool outputs should now be tombstones
        for m in &app.messages {
            if let MessageItem::ToolCall { output, .. } = m {
                assert!(output.starts_with("[trimmed:"), "output should be tombstoned, got: {output}");
            }
        }
    }

    #[test]
    fn bounds_drop_reasoning_phase_2() {
        let mut app = TuiApp::default();
        let big = "x".repeat(1024 * 1024);
        // 60 reasoning messages: phase 1 (tool outputs) is a no-op,
        // phase 2 should drop them all.
        for _ in 0..60 {
            app.messages.push(reasoning(&big));
        }
        enforce_message_bounds(&mut app);
        // Reasoning should be dropped if it was the only way to get under byte bound
        let remaining_reasoning = app.messages.iter()
            .filter(|m| matches!(m, MessageItem::Reasoning { .. }))
            .count();
        assert_eq!(remaining_reasoning, 0, "reasoning should be dropped in phase 2");
    }

    #[test]
    fn bounds_always_keeps_at_least_one() {
        let mut app = TuiApp::default();
        // Single enormous message that exceeds byte budget alone
        let huge = "x".repeat(100 * 1024 * 1024); // 100 MB
        app.messages.push(assistant(&huge));
        enforce_message_bounds(&mut app);
        assert_eq!(app.messages.len(), 1, "must keep at least 1 message");
    }

    // ── compact_message_fields / compact_messages_for_save ───────────────────

    #[test]
    fn compact_noop_under_threshold() {
        let mut m = user("hello");
        compact_message_fields(&mut m);
        assert!(matches!(m, MessageItem::User { text } if text == "hello"));
    }

    #[test]
    fn compact_truncates_long_text() {
        let original = "a".repeat(COMPACT_FIELD_THRESHOLD + 5000);
        let mut m = assistant(&original);
        compact_message_fields(&mut m);
        if let MessageItem::Assistant { text } = m {
            assert!(text.len() < original.len(), "should be shorter");
            assert!(text.contains("[trimmed"), "should have trimmed marker");
            assert!(text.starts_with('a'), "should keep prefix");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn compact_tool_call_truncates_input_and_output() {
        let big = "b".repeat(COMPACT_FIELD_THRESHOLD + 1000);
        let mut m = tool_call("read_file", &big, &big);
        compact_message_fields(&mut m);
        if let MessageItem::ToolCall { input, output, .. } = m {
            assert!(input.contains("[trimmed"), "input should be trimmed");
            assert!(output.contains("[trimmed"), "output should be trimmed");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn compact_messages_for_save_applies_to_all() {
        let big = "c".repeat(COMPACT_FIELD_THRESHOLD + 100);
        let mut messages = vec![user(&big), assistant(&big), tool_call("t", &big, &big)];
        compact_messages_for_save(&mut messages);
        // All entries should be smaller than original
        let total: usize = messages.iter().map(message_size).sum();
        assert!(total < big.len() * 5, "total should shrink significantly");
    }

    // ── message_size / messages_bytes ────────────────────────────────────────

    #[test]
    fn size_user_message() {
        assert_eq!(message_size(&user("hello")), 5);
    }

    #[test]
    fn size_tool_call_sums_fields() {
        let m = tool_call("read", "in", "out");
        assert_eq!(message_size(&m), 4 + 2 + 3); // name + input + output
    }
}
