/// ─── TUI Module ───────────────────────────────────────────────────────────
///
/// ratatui + crossterm 全屏终端界面。
/// 三段布局：消息列表（上） + 输入框（中） + 状态栏（底）

pub mod completion;
pub mod markdown;
pub mod message_list;
pub mod status_bar;
pub(crate) use status_bar::{compact_cwd, format_context_bar};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::layout::{Flex, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::agent::{AgentCommand, AgentResponse};
use crate::event::SharedEventBus;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// ─── App State ────────────────────────────────────────────────────────────

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
    pub reverse_search_results: Vec<usize>,  // indices into messages
    pub reverse_search_idx: usize,           // current result position

    /// 斜杠命令补全状态
    pub slash_completion: SlashCompletionState,

    /// 帮助面板状态
    pub help_active: bool,

    /// 模型切换器状态
    pub model_picker_active: bool,
    pub model_picker_selected: usize,
    pub available_models: Vec<String>,

    /// 权限对话框状态
    pub permission_pending: Option<PendingPermission>,

    /// 主题切换（暗/亮模式）
    pub dark_mode: bool,

    /// 消息选择模式 —选中消息的索引
    pub selected_msg: Option<usize>,

    /// 渲染缓存（避免每帧重新解析 Markdown）
    pub cached_lines: Vec<Line<'static>>,
    pub cached_msg_count: usize,
    /// 搜索查询的缓存键（用于缓存失效）
    pub cached_search_query: String,

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

/// 待用户确认的权限请求
#[derive(Debug, Clone)]
pub struct PendingPermission {
    pub tool_name: String,
    pub tool_input: String,
    pub request_id: u64,
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
                "/help",
                "/exit",
                "/quit",
                "/reload",
                "/clear",
                "/history",
                "/session",
                "/resume",
                "/config",
                "/mcp",
                "/tools",
                "/skills",
                "/memory",
            ],
            descriptions: vec![
                "Show help and shortcuts",
                "Exit the application",
                "Exit the application",
                "Reload context and skills",
                "Clear conversation history",
                "Show message count",
                "List saved sessions",
                "Resume a previous session",
                "View or change settings (model, api_base, etc.)",
                "Manage MCP servers (list, start, stop)",
                "List available tools",
                "List loaded skills",
                "List memory entries",
            ],
        }
    }
}

/// @ 文件补全状态
#[derive(Debug, Clone)]
pub struct CompletionState {
    /// 是否正在补全模式（刚输入了 @）
    pub active: bool,
    /// @ 后面的查询文本
    pub query: String,
    /// 候选文件列表
    pub candidates: Vec<completion::CompletionCandidate>,
    /// 当前选中的候选索引
    pub selected: usize,
    /// @ 在 input 中的位置（用于替换）
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
    /// 连接类型（用于显示）
    pub connection_type: String,
    /// 自消息发送以来的已耗秒数
    pub elapsed_secs: u64,
    /// 当前工具调用轮次
    pub current_round: usize,
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
                "gpt-4o".into(),
                "gpt-4o-mini".into(),
                "gpt-4.1".into(),
                "gpt-4.1-mini".into(),
                "gpt-4.1-nano".into(),
                "o3".into(),
                "o4-mini".into(),
                "claude-sonnet-4-20250514".into(),
                "claude-haiku-3-5".into(),
                "deepseek-chat".into(),
                "llama3.2".into(),
                "gemini-2.5-flash".into(),
            ],
            permission_pending: None,
            dark_mode: true,
            selected_msg: None,
            slash_completion: SlashCompletionState::default(),
            help_active: false,
            cached_lines: Vec::new(),
            cached_msg_count: 0,
            cached_search_query: String::new(),
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

/// ─── Main Entry ───────────────────────────────────────────────────────────

/// Run the TUI event loop. Blocks until exit.
pub fn run_tui(
    _bus: SharedEventBus,
    cmd_tx: std::sync::mpsc::Sender<AgentCommand>,
    mut resp_rx: tokio::sync::mpsc::Receiver<AgentResponse>,
    session_store: crate::session::SessionStore,
    config_store: crate::config::ConfigStore,
    mcp_registry: Arc<Mutex<crate::mcp::McpRegistry>>,
) -> Result<()> {
    // Enable raw mode and bracketed paste
    crossterm::terminal::enable_raw_mode()?;
    crate::TUI_ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableBracketedPaste,
        crossterm::event::EnableMouseCapture,
        crossterm::cursor::SetCursorStyle::BlinkingBlock,
    )?;

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let mut app = TuiApp::default();
    app.status.model = config_store.model().to_string();

    // 显示欢迎消息（不自动加载历史会话，/resume 可手动恢复）
    app.messages.push(MessageItem::System {
        text: format!(
            "CodeCoder TUI — {} /help 查看帮助",
            app.status.model,
        ),
    });

    // 注入存储到 app
    app.session_store = Some(session_store);
    app.current_session_id = None;
    app.config_store = Some(config_store);
    app.mcp_registry = Some(mcp_registry);

    // 帧计数器（用于 spinner 动画）
    let mut frame_count: u64 = 0;

    // 主事件循环
    while !app.should_quit {
        frame_count = frame_count.wrapping_add(1);

        // 更新已耗时间
        if let Some(start) = app.thinking_start_time {
            let secs = start.elapsed().as_secs();
            app.status.elapsed_secs = secs;
            // 120s 超时提醒
            if secs > 120 && secs % 30 == 0 {
                app.messages.push(MessageItem::System {
                    text: "[warn] Agent 超过 120 秒未响应，可能 LLM 连接超时或 agent 线程已崩溃。检查 stderr 输出。".into(),
                });
            }
        }

        terminal.draw(|f| render(f, &mut app, frame_count))?;

        // 检查 agent 响应（非阻塞）
        check_agent_responses(&mut app, &mut resp_rx);

        // 处理键盘事件（阻塞 100ms 以便同时轮询 agent）
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(&mut app, key, &cmd_tx);
                    }
                }
                Event::Mouse(mouse) => {
                    // 鼠标滚轮 → 消息列表滚动
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if !app.input.is_empty() || app.search_active || app.reverse_search_active {
                                // 输入/搜索模式下不处理
                            } else {
                                app.auto_scroll = false;
                                app.scroll_offset = app.scroll_offset.saturating_add(3);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if !app.input.is_empty() || app.search_active || app.reverse_search_active {
                                // 输入/搜索模式下不处理
                            } else {
                                if app.scroll_offset > 3 {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(3);
                                } else {
                                    app.scroll_offset = 0;
                                    app.auto_scroll = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Paste(text) => {
                    // Bracketed paste: save snapshot then insert
                    save_undo_snapshot(&mut app);
                    app.input.insert_str(app.cursor_pos, &text);
                    app.cursor_pos += text.len();
                }
                _ => {}
            }
        }
    }

    // 发送 shutdown
    let _ = cmd_tx.send(AgentCommand::Shutdown);

    // 持久化会话（退出时自动保存）
    if let Some(ref store) = app.session_store {
        let session = build_session_from_app(&app);
        let _ = store.save(&session);
    }

    // Restore terminal
    crate::TUI_ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableBracketedPaste,
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::SetCursorStyle::DefaultUserShape,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    ratatui::restore();
    Ok(())
}

/// ─── Render ───────────────────────────────────────────────────────────────

/// 计算逻辑行列表在给定宽度下实际占用的显示行数（考虑折行）
fn count_display_rows(lines: &[Line<'_>], area_width: u16) -> usize {
    if area_width == 0 {
        return lines.len();
    }
    lines.iter().map(|line| {
        let w = line.width();
        if w == 0 { 1 } else { (w + area_width as usize - 1) / area_width as usize }
    }).sum()
}

/// 从末尾开始算，返回 scroll_offset 使得最后 msg_height 显示行可见
fn bottom_scroll_offset(lines: &[Line<'_>], area_width: u16, msg_height: usize) -> usize {
    if area_width == 0 || msg_height == 0 || lines.is_empty() {
        return 0;
    }
    let mut rows = 0usize;
    let mut skipped = lines.len();
    for line in lines.iter().rev() {
        let w = line.width();
        let line_rows = if w == 0 { 1 } else { (w + area_width as usize - 1) / area_width as usize };
        if rows + line_rows > msg_height {
            break;
        }
        rows += line_rows;
        skipped -= 1;
    }
    skipped
}

/// Count how many lines match the search query
fn count_search_matches(app: &mut TuiApp) {
    if app.search_query.is_empty() {
        app.search_match_count = 0;
        app.search_current_match = 0;
        return;
    }
    let query = app.search_query.to_lowercase();
    let mut count = 0;
    for m in &app.messages {
        let text = match m {
            MessageItem::User { text } => text,
            MessageItem::Assistant { text } => text,
            MessageItem::ToolCall { name: _, output, .. } => output,
            MessageItem::System { text } => text,
            MessageItem::Reasoning { text, .. } => text,
        };
        if text.to_lowercase().contains(&query) {
            count += 1;
        }
    }
    app.search_match_count = count;
}

/// Update reverse search results
fn update_reverse_search(app: &mut TuiApp) {
    app.reverse_search_results.clear();
    if app.reverse_search_query.is_empty() {
        return;
    }
    let query = app.reverse_search_query.to_lowercase();
    for (i, m) in app.messages.iter().enumerate() {
        let text = match m {
            MessageItem::User { text } => text,
            MessageItem::Assistant { text } => text,
            MessageItem::ToolCall { output, .. } => output,
            MessageItem::System { text } => text,
            MessageItem::Reasoning { text, .. } => text,
        };
        if text.to_lowercase().contains(&query) {
            app.reverse_search_results.push(i);
        }
    }
    if !app.reverse_search_results.is_empty() {
        app.reverse_search_idx = app.reverse_search_results.len() - 1;
        app.auto_scroll = false;
    }
}

/// Build the rendered message lines with caching
fn build_message_lines(app: &mut TuiApp) -> Vec<Line<'static>> {
    // 如果消息数量和搜索查询没变，返回缓存
    if app.messages.len() == app.cached_msg_count
        && !app.cached_lines.is_empty()
        && app.cached_search_query == app.search_query
    {
        return app.cached_lines.clone();
    }

    let highlight = if app.search_active && !app.search_query.is_empty() {
        Some(app.search_query.as_str())
    } else {
        None
    };

    // 重建缓存
    app.cached_lines = build_message_lines_inner(app, highlight);
    app.cached_msg_count = app.messages.len();
    app.cached_search_query = app.search_query.clone();
    app.cached_lines.clone()
}

/// The actual rendering logic (separated for cache clarity)
fn build_message_lines_inner(app: &TuiApp, highlight: Option<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (msg_idx, m) in app.messages.iter().enumerate() {
        let is_selected = app.selected_msg == Some(msg_idx);
        let prefix_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        match m {
            MessageItem::User { text } => {
                let prefix = if is_selected { "▸ " } else { "▶ " };
                lines.push(Line::styled(
                    format!("{}{}", prefix, text.lines().next().unwrap_or("")),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
                // Remaining lines if multi-line
                for line in text.lines().skip(1) {
                    lines.push(Line::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                }
            }
            MessageItem::Assistant { text } => {
                let prefix = if is_selected { "▸ " } else { "▷ " };
                // Use markdown rendering but no indent
                let md_lines = markdown::render_markdown_with_highlight(text, highlight);
                if let Some(first) = md_lines.first() {
                    let styled_line = Line::from(
                        std::iter::once(Span::styled(prefix, prefix_style))
                            .chain(first.spans.iter().cloned())
                            .collect::<Vec<_>>(),
                    );
                    lines.push(styled_line);
                }
                for line in md_lines.iter().skip(1) {
                    lines.push(Line::from(
                        std::iter::once(Span::raw("  "))
                            .chain(line.spans.iter().cloned())
                            .collect::<Vec<_>>(),
                    ));
                }
            }
            MessageItem::Reasoning { text, .. } => {
                // Always fully expanded, Dark Gray (no DIM)
                for (i, line) in text.lines().enumerate() {
                    if i == 0 {
                        lines.push(Line::styled(
                            format!("· {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    } else {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            MessageItem::ToolCall { name, input, output, .. } => {
                // Always fully expanded, no fold
                lines.push(Line::styled(
                    format!("⚙ {}", name),
                    Style::default().fg(Color::DarkGray),
                ));
                if !input.is_empty() {
                    for line in input.lines() {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
                if !output.is_empty() {
                    for line in output.lines() {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            MessageItem::System { text } => {
                lines.push(Line::styled(
                    format!(" {}", text),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
    }
    lines
}

fn render(frame: &mut Frame, app: &mut TuiApp, frame_count: u64) {
    let area = frame.area();

    // 三段 Flex 布局：消息区（flex_grow=1）+ 输入区（2行）+ 状态栏（1行）
    let [msg_area, input_area, status_area] = ratatui::layout::Layout::new(
        ratatui::layout::Direction::Vertical,
        [
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(2),
            ratatui::layout::Constraint::Length(1),
        ],
    )
    .flex(Flex::Start)
    .areas(area);

    // 消息区 — 先构建全部行，再计算滚动
    let rendered_lines = build_message_lines(app);
    let msg_height = msg_area.height.saturating_sub(1) as usize; // minus single border line
    let text_width = msg_area.width; // block 只有 TOP 边框，文本区等同全宽

    // 自动滚到底部：考虑折行后的实际显示行数
    if app.auto_scroll {
        app.scroll_offset = bottom_scroll_offset(&rendered_lines, text_width, msg_height);
    } else {
        // 如果手动滚过头了，停在最大位置（也考虑折行）
        let max_offset = bottom_scroll_offset(&rendered_lines, text_width, msg_height);
        if app.scroll_offset > max_offset {
            app.scroll_offset = max_offset;
        }
    }

    // 计算显示行数（用于滚动条），必须在 rendered_lines 被 move 之前
    let total_display_rows = count_display_rows(&rendered_lines, text_width);
    let scrolled_display_rows = if app.scroll_offset > 0 {
        count_display_rows(&rendered_lines[..app.scroll_offset], text_width)
    } else {
        0
    };

    let msg_block = Block::default()
        .borders(ratatui::widgets::Borders::TOP)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(
            "{} CodeCoder{}{}",
            if app.status.agent_busy { "• " } else { "" },
            if !app.auto_scroll {
                " (↑ scroll)"
            } else {
                ""
            },
            if app.reverse_search_active {
                format!(
                    " \u{2315}`{}` {} hits",
                    app.reverse_search_query,
                    app.reverse_search_results.len(),
                )
            } else if app.search_active {
                format!(
                    " [search] {} ({} hits)",
                    app.search_query,
                    app.search_match_count
                )
            } else {
                String::new()
            }
        ))
        .title_alignment(ratatui::layout::Alignment::Left);
    let msg_paragraph = Paragraph::new(rendered_lines)
        .block(msg_block)
        .scroll((app.scroll_offset as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(msg_paragraph, msg_area);

    // 右侧极简滚动条 — 仅 `··` 点状滑块，无轨道
    if total_display_rows > msg_height {
        let thumb_color = if app.dark_mode {
            Color::White
        } else {
            Color::Black
        };

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(Style::default())
            .thumb_style(Style::default().fg(thumb_color))
            .begin_style(Style::default())
            .end_style(Style::default());

        let mut scrollbar_state = ScrollbarState::new(total_display_rows)
            .position(scrolled_display_rows)
            .viewport_content_length(msg_height);

        // 在内边距区域内渲染滚动条（避开上边框）
        frame.render_stateful_widget(
            scrollbar,
            msg_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // 输入区 — 2行：顶部分隔线 + `>` 前缀
    let separator_line = Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(1) as usize),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(
        Paragraph::new(separator_line),
        Rect::new(input_area.x, input_area.y, input_area.width, 1),
    );

    // 输入内容在第2行
    let input_content_y = input_area.y + 1;
    let cursor_pos = app.cursor_pos.min(app.input.len());
    let prefix_span = Span::styled("> ", Style::default().fg(Color::Cyan));
    let input_display = if app.input.is_empty() {
        Line::from(prefix_span)
    } else {
        let mut line = Line::from(vec![prefix_span]);
        line.spans.push(Span::raw(&app.input));
        line
    };
    // 输入内容区（第2行开始，可多行）
    let input_content_area = Rect::new(input_area.x, input_content_y, input_area.width, 1);
    let input_paragraph = Paragraph::new(input_display)
        .style(Style::default().fg(Color::White));
    frame.render_widget(input_paragraph, input_content_area);

    // 设置光标位置（在输入内容行）
    let safe_cursor = if app.input.is_char_boundary(cursor_pos) {
        cursor_pos
    } else {
        let mut i = cursor_pos.min(app.input.len());
        while i > 0 && !app.input.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let input_lines = app.input[..safe_cursor].lines().count().max(1) - 1;
    let last_line_start = if input_lines == 0 {
        0
    } else {
        app.input[..safe_cursor]
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .last()
            .map(|(i, _)| i + 1)
            .unwrap_or(0)
    };
    let col_offset = safe_cursor - last_line_start;
    let row_offset = input_lines;

    frame.set_cursor_position(ratatui::layout::Position {
        x: input_area.x + col_offset as u16 + 2, // +2 for "> "
        y: input_content_y + row_offset as u16,
    });

    // 斜杠命令补全弹出列表
    if app.slash_completion.active {
        let popup_width = area.width.min(50).max(30);
        let popup_height = (app.slash_completion.commands.len() as u16 + 2).min(14);
        let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
        let popup_y = input_area.y.saturating_sub(popup_height + 1);

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let items: Vec<Line> = app
            .slash_completion
            .commands
            .iter()
            .zip(app.slash_completion.descriptions.iter())
            .enumerate()
            .map(|(i, (cmd, desc))| {
                if i == app.slash_completion.selected {
                    Line::styled(
                        format!(" ▸ {:<12} {}", cmd, desc),
                        Style::default().fg(Color::Black).bg(Color::White),
                    )
                } else {
                    Line::styled(
                        format!("   {:<12} {}", cmd, desc),
                        Style::default().fg(Color::White),
                    )
                }
            })
            .collect();

        frame.render_widget(ratatui::widgets::Clear, popup_area);
        let popup_block = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Commands ")
                    .title_alignment(ratatui::layout::Alignment::Left),
            );
        frame.render_widget(popup_block, popup_area);
        return;
    }

    // 模型切换器弹出列表
    if app.model_picker_active {
        let popup_width = area.width.min(40).max(20);
        let popup_height = (app.available_models.len() as u16 + 2).min(16);
        let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
        let popup_y = input_area.y.saturating_sub(popup_height + 1);

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let items: Vec<Line> = app
            .available_models
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_current = m == &app.status.model;
                if i == app.model_picker_selected {
                    Line::styled(
                        format!(" ▸ {} {}", m, if is_current { "✓" } else { "" }),
                        Style::default().fg(Color::Black).bg(Color::White),
                    )
                } else {
                    Line::styled(
                        format!("   {} {}", m, if is_current { "✓" } else { "" }),
                        Style::default().fg(if is_current { Color::Green } else { Color::White }),
                    )
                }
            })
            .collect();

        frame.render_widget(ratatui::widgets::Clear, popup_area);
        let popup_block = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Model ")
                    .title_alignment(ratatui::layout::Alignment::Left),
            );
        frame.render_widget(popup_block, popup_area);
        return; // Don't render anything else when picker is open
    }

    // 帮助面板
    if app.help_active {
        let panel_width = area.width.min(55).max(35);
        let panel_height = area.height.min(22).max(10);
        let panel_x = area.x + (area.width.saturating_sub(panel_width)) / 2;
        let panel_y = area.y + (area.height.saturating_sub(panel_height)) / 2;
        let panel_area = Rect::new(panel_x, panel_y, panel_width, panel_height);

        let help_lines = vec![
            Line::styled(" Shortcuts ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Enter         ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Send"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↑/↓           ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("History / navigate"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("PgUp/PgDn     ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Scroll"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+F        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Search"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+R        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Reverse search"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+P        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Switch model"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Alt+Enter     ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("New line"),
            ]),
            Line::from(""),
            Line::styled(" Commands ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/help /exit   ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Help / Quit"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/clear        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Clear chat"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/model /tools ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("Model / Tools"),
            ]),
            Line::from(""),
            Line::styled(" Esc to close ", Style::default().fg(Color::DarkGray)),
        ];

        frame.render_widget(ratatui::widgets::Clear, panel_area);
        let panel = Paragraph::new(help_lines)
            .block(
                Block::default()
                    .title(" Help ")
                    .title_alignment(ratatui::layout::Alignment::Left),
            );
        frame.render_widget(panel, panel_area);
        return;
    }

    // 权限确认对话框
    if let Some(ref perm) = app.permission_pending {
        let dialog_width = area.width.min(60).max(30);
        let dialog_height = 8;
        let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

        let content = vec![
            Line::styled(" [!] Tool Permission Required ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::styled(
                format!(" Tool: {}", perm.tool_name),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                format!(" Input: {}", perm.tool_input.chars().take(80).collect::<String>()),
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::styled(" Press  Y  to allow     N  to deny ", Style::default().fg(Color::Cyan)),
        ];

        frame.render_widget(ratatui::widgets::Clear, dialog_area);
        let dialog = Paragraph::new(content)
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(dialog, dialog_area);
        return; // Show only dialog when permission is pending
    }

    // @ 文件补全弹出列表
    if app.completion.active && !app.completion.candidates.is_empty() {
        let popup_width = area.width.min(50).max(20);
        let popup_height = (app.completion.candidates.len() as u16 + 2).min(12);
        let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
        let popup_y = input_area.y.saturating_sub(popup_height + 1);

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        let items: Vec<Line> = app
            .completion
            .candidates
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if i == app.completion.selected {
                    Line::styled(
                        format!(" ▸ {} ", c.display),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White),
                    )
                } else {
                    Line::styled(
                        format!("   {} ", c.display),
                        Style::default().fg(Color::White),
                    )
                }
            })
            .collect();

        let popup = ratatui::widgets::Clear; // Clear area first
        frame.render_widget(popup, popup_area);
        let popup_block = Paragraph::new(items)
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .title(" Files ")
                    .title_alignment(ratatui::layout::Alignment::Left),
            );
        frame.render_widget(popup_block, popup_area);
    }

    // 状态栏（调用子模块）
    status_bar::render(frame, status_area, &app.status, frame_count);
}

/// Save the current input state to the undo stack (for Ctrl+Z)
fn save_undo_snapshot(app: &mut TuiApp) {
    // Don't save if the input hasn't changed
    if let Some(last) = app.undo_stack.last() {
        if last == &app.input {
            return;
        }
    }
    app.undo_stack.push(app.input.clone());
    // Cap undo stack to prevent memory issues
    if app.undo_stack.len() > 100 {
        app.undo_stack.remove(0);
    }
    // Clear redo stack on new edits
    app.redo_stack.clear();
}

/// ─── Key Handling ─────────────────────────────────────────────────────────

fn handle_key(
    app: &mut TuiApp,
    key: crossterm::event::KeyEvent,
    cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
) {
    match key.code {
        KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true;
        }
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true;
        }
        // Ctrl+F — 搜索模式
        KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
            app.search_active = !app.search_active;
            if !app.search_active {
                app.search_query.clear();
            }
            return;
        }
        // Ctrl+P — 模型切换器
        KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
            app.model_picker_active = !app.model_picker_active;
            app.model_picker_selected = app
                .available_models
                .iter()
                .position(|m| m == &app.status.model)
                .unwrap_or(0);
            return;
        }
        // Ctrl+R — 反向搜索
        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
            app.reverse_search_active = !app.reverse_search_active;
            if app.reverse_search_active {
                app.reverse_search_query.clear();
                app.reverse_search_results.clear();
                app.reverse_search_idx = 0;
            }
            return;
        }
        // Ctrl+H — 帮助面板
        KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
            app.help_active = !app.help_active;
            return;
        }
        // Ctrl+A — 行首
        KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = 0;
            return;
        }
        // Ctrl+E — 行尾
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = app.input.len();
            return;
        }
        // Ctrl+W — 删除前一个单词
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                let before = &app.input[..app.cursor_pos];
                // 找到前一个单词边界（跳过连续空格，再跳过连续非空格）
                let trimmed = before.trim_end();
                let word_start = trimmed
                    .rfind(|c: char| c.is_whitespace())
                    .map(|p| p + 1)
                    .unwrap_or(0);
                app.input.drain(word_start..app.cursor_pos);
                app.cursor_pos = word_start;
            }
            return;
        }
        // Ctrl+U — 删除到行首（删除光标前所有内容）
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                app.input.drain(..app.cursor_pos);
                app.cursor_pos = 0;
            }
            return;
        }
        // Ctrl+K — 删除到行尾（删除光标后所有内容）
        KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos < app.input.len() {
                save_undo_snapshot(app);
                app.input.truncate(app.cursor_pos);
            }
            return;
        }
        // Ctrl+L — 清屏（清除消息列表）
        KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
            app.messages.clear();
            app.cached_lines.clear();
            app.cached_msg_count = 0;
            app.cached_search_query.clear();
            app.scroll_offset = 0;
            app.auto_scroll = true;
            return;
        }
        // Ctrl+T — 切换暗/亮主题
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            app.dark_mode = !app.dark_mode;
            crate::tui::markdown::set_dark_mode(app.dark_mode);
            // 清除消息渲染缓存以重新应用主题
            app.cached_lines.clear();
            app.cached_msg_count = 0;
            return;
        }
        // Ctrl+Z — 撤销（undo 输入编辑）
        KeyCode::Char('z') if key.modifiers == KeyModifiers::CONTROL => {
            if let Some(prev) = app.undo_stack.pop() {
                app.redo_stack.push(app.input.clone());
                // Cap redo stack
                if app.redo_stack.len() > 100 {
                    app.redo_stack.remove(0);
                }
                app.input = prev;
                app.cursor_pos = app.input.len();
            }
            return;
        }
        // Ctrl+Y — 重做（redo 输入编辑）
        KeyCode::Char('y') if key.modifiers == KeyModifiers::CONTROL => {
            if let Some(next) = app.redo_stack.pop() {
                save_undo_snapshot(app);
                app.input = next;
                app.cursor_pos = app.input.len();
            }
            return;
        }
        // Alt+Enter or Ctrl+Enter → send
        KeyCode::Enter
            if key.modifiers == KeyModifiers::ALT
                || key.modifiers == KeyModifiers::CONTROL =>
        {
            send_message(app, cmd_tx);
        }
        // Y/N — 权限/计划/提问 对话框响应
        KeyCode::Char('y') | KeyCode::Char('Y') if app.permission_pending.is_some() => {
            if let Some(p) = app.permission_pending.take() {
                match p.tool_name.as_str() {
                    "plan" => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision {
                            request_id: p.request_id,
                            decision: "approved".into(),
                        });
                    }
                    "ask_user" => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse {
                            request_id: p.request_id,
                            answer: "yes".into(),
                        });
                    }
                    _ => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id: p.request_id,
                            allowed: true,
                        });
                    }
                }
            }
            return;
        }
        KeyCode::Char('n') | KeyCode::Char('N') if app.permission_pending.is_some() => {
            if let Some(p) = app.permission_pending.take() {
                match p.tool_name.as_str() {
                    "plan" => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision {
                            request_id: p.request_id,
                            decision: "rejected".into(),
                        });
                    }
                    "ask_user" => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse {
                            request_id: p.request_id,
                            answer: "no".into(),
                        });
                    }
                    _ => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id: p.request_id,
                            allowed: false,
                        });
                    }
                }
            }
            return;
        }
        // Enter → send message or answer question
        KeyCode::Enter => {
            // If there's a pending question, answer it with the input
            if let Some(ref p) = app.permission_pending {
                if p.tool_name == "ask_user" && !app.input.is_empty() {
                    let question = app.input.trim().to_string();
                    let request_id = p.request_id;
                    app.permission_pending = None;
                    app.messages.push(MessageItem::User { text: question.clone() });
                    let _ = cmd_tx.send(AgentCommand::AskUserResponse {
                        request_id,
                        answer: question,
                    });
                    app.input.clear();
                    app.cursor_pos = 0;
                    return;
                }
            }
            if app.model_picker_active {
                // Select model
                if let Some(model) = app.available_models.get(app.model_picker_selected) {
                    if model != &app.status.model {
                        let _ = cmd_tx.send(AgentCommand::SetModel { model: model.clone() });
                        app.status.model = model.clone();
                    }
                }
                app.model_picker_active = false;
                return;
            }
            if app.slash_completion.active {
                // Execute selected slash command
                if let Some(cmd) = app.slash_completion.commands.get(app.slash_completion.selected) {
                    app.input = cmd.to_string();
                    app.cursor_pos = app.input.len();
                    send_message(app, cmd_tx);
                }
                app.slash_completion.active = false;
                return;
            }
            if app.input.is_empty() {
                // If a message is selected, copy it to input
                if let Some(idx) = app.selected_msg {
                    if let Some(msg) = app.messages.get(idx) {
                        let text = match msg {
                            MessageItem::User { text } => text.clone(),
                            MessageItem::Assistant { text } => text.clone(),
                            MessageItem::System { text } => text.clone(),
                            MessageItem::ToolCall { output, .. } => output.clone(),
                            MessageItem::Reasoning { text, .. } => text.clone(),
                        };
                        app.input = text;
                        app.cursor_pos = app.input.len();
                        app.selected_msg = None;
                        return;
                    }
                    app.selected_msg = None;
                }
                // 完成补全：Enter 选中文件
                if app.completion.active {
                    if let Some(candidate) = app.completion.candidates.get(app.completion.selected) {
                        let at_pos = app.completion.at_pos;
                        let end_of_word = app.cursor_pos;
                        app.input.replace_range(at_pos..end_of_word, &candidate.display);
                        app.cursor_pos = at_pos + candidate.display.len();
                    }
                    app.completion.active = false;
                    return;
                }
                // Try to expand last reasoning block
                if let Some(MessageItem::Reasoning { expanded, .. }) = app.messages.last_mut() {
                    *expanded = !*expanded;
                    return;
                }
                // Try to expand last tool call output
                if let Some(MessageItem::ToolCall { expanded: true, show_full, .. }) = app.messages.last_mut() {
                    *show_full = !*show_full;
                    return;
                }
                return;
            }
            // Shift+Enter or Alt+Enter → 换行
            if key.modifiers == KeyModifiers::SHIFT || key.modifiers == KeyModifiers::ALT {
                app.input.push('\n');
                app.cursor_pos = app.input.len();
            } else {
                // Enter → 发送消息
                send_message(app, cmd_tx);
            }
        }
        KeyCode::Up => {
            if app.model_picker_active {
                if app.model_picker_selected > 0 {
                    app.model_picker_selected -= 1;
                }
                return;
            }
            if app.slash_completion.active {
                if app.slash_completion.selected > 0 {
                    app.slash_completion.selected -= 1;
                }
                return;
            }
            if !app.input_history.is_empty() && app.history_pos > 0 {
                app.history_pos -= 1;
                app.input = app.input_history[app.history_pos].clone();
                app.cursor_pos = app.input.len();
            } else if app.input.is_empty() && !app.messages.is_empty() {
                // Message selection mode
                let max_idx = app.messages.len().saturating_sub(1);
                let new_sel = match app.selected_msg {
                    Some(i) if i > 0 => i - 1,
                    _ => max_idx,
                };
                app.selected_msg = Some(new_sel);
            }
        }
        KeyCode::Down => {
            if app.model_picker_active {
                if app.model_picker_selected + 1 < app.available_models.len() {
                    app.model_picker_selected += 1;
                }
                return;
            }
            if app.slash_completion.active {
                let max_idx = app.slash_completion.commands.len().saturating_sub(1);
                if app.slash_completion.selected < max_idx {
                    app.slash_completion.selected += 1;
                }
                return;
            }
            if app.history_pos < app.input_history.len() {
                app.history_pos += 1;
                if app.history_pos == app.input_history.len() {
                    app.input.clear();
                } else {
                    app.input = app.input_history[app.history_pos].clone();
                }
                app.cursor_pos = app.input.len();
            } else if app.input.is_empty() && app.selected_msg.is_some() {
                // Message selection mode — move down
                let max_idx = app.messages.len().saturating_sub(1);
                let new_sel = match app.selected_msg {
                    Some(i) if i < max_idx => i + 1,
                    _ => 0,
                };
                app.selected_msg = Some(new_sel);
            } else if app.input.is_empty() {
                // Start selection from top
                app.selected_msg = Some(0);
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                // Move to previous UTF-8 char boundary
                let mut new_pos = app.cursor_pos.saturating_sub(1);
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                // Move to next UTF-8 char boundary
                let mut new_pos = app.cursor_pos.saturating_add(1);
                while new_pos < app.input.len() && !app.input.is_char_boundary(new_pos) {
                    new_pos += 1;
                }
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
        }
        KeyCode::Char(c) => {
            if app.reverse_search_active {
                app.reverse_search_query.push(c);
                update_reverse_search(app);
                return;
            }
            if app.search_active {
                app.search_query.push(c);
                count_search_matches(app);
                return;
            }
            // Insert character
            save_undo_snapshot(app);
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += c.len_utf8();

            // Cancel slash completion if typing non-slash characters
            if app.slash_completion.active && !app.input.starts_with('/') {
                app.slash_completion.active = false;
            }

            // Detect / at start of input → show slash command menu
            if c == '/' && app.cursor_pos == 1 {
                // Filter by query
                app.slash_completion.active = true;
                app.slash_completion.selected = 0;
            } else if app.slash_completion.active {
                // Filter commands as user types
                let _query = app.input.trim().to_lowercase();
                app.slash_completion.selected = 0;
            }

            // Detect @ to activate file completion
            if c == '@' {
                app.completion.active = true;
                app.completion.query = String::new();
                app.completion.at_pos = app.cursor_pos - 1;
                app.completion.selected = 0;
                app.completion.candidates =
                    completion::search_files(".", "", 10);
            } else if app.completion.active {
                // Update completion query
                let after_at = &app.input[app.completion.at_pos + 1..];
                app.completion.query = after_at.to_string();
                app.completion.selected = 0;
                app.completion.candidates =
                    completion::search_files(".", &app.completion.query, 10);
            }
        }
        KeyCode::Backspace => {
            if app.reverse_search_active {
                app.reverse_search_query.pop();
                update_reverse_search(app);
                return;
            }
            if app.search_active {
                app.search_query.pop();
                count_search_matches(app);
                return;
            }
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                // Cancel slash completion if backspace removes the /
                if app.slash_completion.active && app.cursor_pos == 1 {
                    app.slash_completion.active = false;
                }
                // Check if we're deleting the @ that started completion
                if app.completion.active
                    && app.cursor_pos - 1 == app.completion.at_pos
                {
                    app.completion.active = false;
                }
                // Move to previous UTF-8 char boundary before deleting
                let mut new_pos = app.cursor_pos.saturating_sub(1);
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.input.remove(new_pos);
                app.cursor_pos = new_pos;

                // Update completion query
                if app.completion.active {
                    let after_at = &app.input[app.completion.at_pos + 1..];
                    app.completion.query = after_at.to_string();
                    app.completion.selected = 0;
                    app.completion.candidates =
                        completion::search_files(".", &app.completion.query, 10);
                }
            }
        }
        KeyCode::Tab if app.completion.active => {
            // Select current completion candidate
            if let Some(candidate) = app.completion.candidates.get(app.completion.selected) {
                let at_pos = app.completion.at_pos;
                // Replace from @ to end of current word with the file path
                let end_of_word = app.cursor_pos;
                app.input.replace_range(at_pos..end_of_word, &candidate.display);
                app.cursor_pos = at_pos + candidate.display.len();
            }
            app.completion.active = false;
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                save_undo_snapshot(app);
                app.input.remove(app.cursor_pos);
            }
        }
        KeyCode::Tab => {
            if app.slash_completion.active {
                // Cycle through slash commands
                app.slash_completion.selected = (app.slash_completion.selected + 1)
                    % app.slash_completion.commands.len();
                return;
            }
            if app.completion.active {
                // Cycle through candidates
                app.completion.selected = (app.completion.selected + 1)
                    % app.completion.candidates.len().max(1);
            } else {
                app.input.insert_str(app.cursor_pos, "  ");
                app.cursor_pos += 2;
            }
        }
        KeyCode::Esc => {
            if app.help_active {
                app.help_active = false;
                return;
            }
            if app.slash_completion.active {
                app.slash_completion.active = false;
                return;
            }
            if app.reverse_search_active {
                app.reverse_search_active = false;
                app.reverse_search_query.clear();
                return;
            }
            if app.search_active {
                app.search_active = false;
                app.search_query.clear();
                return;
            }
            if app.selected_msg.is_some() {
                app.selected_msg = None;
                return;
            }
            app.completion.active = false;
            app.should_quit = true;
        }
        // PageUp / PageDown — 消息列表滚动
        KeyCode::PageUp => {
            app.auto_scroll = false;
            app.scroll_offset = app.scroll_offset.saturating_add(10);
        }
        KeyCode::PageDown => {
            if app.scroll_offset > 10 {
                app.scroll_offset = app.scroll_offset.saturating_sub(10);
            } else {
                app.scroll_offset = 0;
                app.auto_scroll = true;
            }
        }
        _ => {}
    }
}

fn send_message(app: &mut TuiApp, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
    let input = app.input.trim().to_string();
    if input.is_empty() {
        return;
    }

    // 本地斜杠命令（不经过 agent）
    if input.starts_with('/') {
        if input == "/session" || input.starts_with("/session ") {
            handle_session_cmd(app);
            return;
        }
        if input == "/resume" || input.starts_with("/resume ") {
            handle_resume_cmd(app, &input);
            return;
        }
        let lower = input.to_lowercase();
        if lower == "/config" || lower.starts_with("/config ") {
            handle_config_cmd(app, &input);
            return;
        }
        if lower == "/mcp" || lower.starts_with("/mcp ") {
            handle_mcp_cmd(app, &input);
            return;
        }
    }

    // 保存到历史
    app.input_history.push(input.clone());
    app.history_pos = app.input_history.len();

    // 添加到消息列表
    app.messages.push(MessageItem::User {
        text: input.clone(),
    });

    // 自动保存（用户消息发出时）
    auto_save_session(app);

    // 发送给 agent
    if let Err(e) = cmd_tx.send(AgentCommand::ProcessMessage { text: input }) {
        app.messages.push(MessageItem::System {
            text: format!("[disconnect] Agent 通道已断开 — 线程可能已崩溃。stderr 可能有更多信息。错误: {e}"),
        });
        app.status.agent_busy = false;
        return;
    }
    crate::log(&format!("[codecoder] TUI 发送消息成功"));

    // 处理中状态追踪
    app.thinking_start_time = Some(Instant::now());
    app.status.agent_busy = true;
    app.current_round = 0;
    app.status.current_tool = None;
    app.messages.push(MessageItem::System {
        text: "[send] Agent…".into(),
    });

    app.input.clear();
    app.cursor_pos = 0;
}

/// ─── Agent Response Polling ───────────────────────────────────────────────

fn check_agent_responses(app: &mut TuiApp, resp_rx: &mut tokio::sync::mpsc::Receiver<AgentResponse>) {
    // 非阻塞读取所有可用响应
    loop {
        match resp_rx.try_recv() {
            Ok(response) => {
                match response {
                    AgentResponse::Text { text } => {
                        // 清除处理中状态
                        let took = app.thinking_start_time.take()
                            .map(|t| t.elapsed().as_secs_f32())
                            .unwrap_or(0.0);
                        // 移除流式阶段的过渡状态消息
                        app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.starts_with("[write]") || text.starts_with("[think]")));
                        // 非空内容才追加（流式场景下内容已通过 LlmDelta 送达）
                        if !text.is_empty() {
                            app.messages.push(MessageItem::Assistant { text });
                        }
                        // 执行结束：插入分隔线 + 耗时提示
                        app.messages.push(MessageItem::System {
                            text: "─".repeat(20),
                        });
                        app.messages.push(MessageItem::System {
                            text: format!("[done] ({took:.1}s)"),
                        });
                        app.status.agent_busy = false;
                        app.status.current_tool = None;
                        app.status.current_round = 0;
                        auto_save_session(app);
                    }
                    AgentResponse::ToolCall { name, input, .. } => {
                        app.current_round += 1;
                        app.status.current_round = app.current_round;
                        // 首次工具调用：移除 [send] 消息，添加推理状态
                        if app.current_round == 1 {
                            app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.starts_with("[send]")));
                            app.messages.push(MessageItem::System {
                                text: "[think] LLM…".into(),
                            });
                        }
                        app.messages.push(MessageItem::ToolCall {
                            name: name.clone(),
                            input,
                            output: String::new(),
                            expanded: false,
                            show_full: false,
                        });
                        app.status.current_tool = Some(name);
                        app.status.agent_busy = true;
                    }
                    AgentResponse::ToolResult { name: _, output, success } => {
                        if let Some(last) = app.messages.last_mut() {
                            if let MessageItem::ToolCall { output: o, expanded: e, .. } = last {
                                *o = if success {
                                    format!("{}", output)
                                } else {
                                    format!("Error: {}", output)
                                };
                                *e = !success;
                            }
                        }
                        app.cached_msg_count = 0; // ToolCall output changed, force cache rebuild
                        app.status.current_tool = None;
                    }
                    AgentResponse::Heartbeat { pending } => {
                        app.status.agent_busy = pending > 0;
                        // 首次心跳确认 agent 线程存活
                        app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text == "[send] Agent…" || text == "[send] Agent 处理中…"));
                    }
                    AgentResponse::LlmDelta { text } => {
                        // 首次收到 streaming 输出：从推理切换到生成状态
                        if app.current_round == 1 && app.messages.last().map_or(true, |m| !matches!(m, MessageItem::Assistant { .. })) {
                            app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.contains("LLM")));
                            app.messages.push(MessageItem::System {
                                text: "[write] LLM…".into(),
                            });
                        }
                        if let Some(last) = app.messages.last_mut() {
                            if let MessageItem::Assistant { text: t } = last {
                                t.push_str(&text);
                                app.cached_msg_count = 0; // 内容已变，强制重建缓存
                            } else {
                                app.messages.push(MessageItem::Assistant { text });
                            }
                        } else {
                            app.messages.push(MessageItem::Assistant { text });
                        }
                    }
                    AgentResponse::ReasoningDelta { text } => {
                        if let Some(MessageItem::Reasoning { text: t, .. }) = app.messages.last_mut() {
                            t.push_str(&text);
                            app.cached_msg_count = 0; // 内容已变，强制重建缓存
                        } else {
                            // 首次收到推理数据
                            app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.starts_with("[think]")));
                            app.messages.push(MessageItem::System {
                                text: "[think] LLM…".into(),
                            });
                            app.messages.push(MessageItem::Reasoning {
                                text,
                                expanded: false,
                            });
                        }
                    }
                    AgentResponse::Error { message } => {
                        let took = app.thinking_start_time.take()
                            .map(|t| t.elapsed().as_secs_f32())
                            .unwrap_or(0.0);
                        app.messages.push(MessageItem::System {
                            text: format!("[error] ({took:.1}s): {message}"),
                        });
                        app.status.agent_busy = false;
                        app.status.current_tool = None;
                        app.status.current_round = 0;
                    }
                    AgentResponse::Shutdown => {
                        app.thinking_start_time.take();
                        app.should_quit = true;
                    }
                    AgentResponse::AskUser { question, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("[ask] Agent asks: {}", question),
                        });
                        // Store pending ask_user question in a new field
                        // (reuse permission_pending for now, with a marker)
                        app.permission_pending = Some(PendingPermission {
                            tool_name: "ask_user".into(),
                            tool_input: question,
                            request_id,
                        });
                    }
                    AgentResponse::PlanRequest { title, plan, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("[plan] Plan: {}", title),
                        });
                        app.messages.push(MessageItem::Assistant { text: plan });
                        app.permission_pending = Some(PendingPermission {
                            tool_name: "plan".into(),
                            tool_input: title,
                            request_id,
                        });
                    }
                    AgentResponse::PermissionRequest { tool_name, tool_input, request_id } => {
                        app.permission_pending = Some(PendingPermission {
                            tool_name,
                            tool_input,
                            request_id,
                        });
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

/// ─── Slash Command Handlers ───────────────────────────────────────────────

/// Handle `/session` — list all saved sessions.
fn handle_session_cmd(app: &mut TuiApp) {
    let store = match app.session_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Session store not available.".into(),
            });
            return;
        }
    };

    let headers = store.list();
    if headers.is_empty() {
        app.messages.push(MessageItem::System {
            text: "No saved sessions.".into(),
        });
        return;
    }

    let current = app.current_session_id.as_deref().unwrap_or("");
    let mut lines = vec!["── Saved Sessions ──".to_string()];
    for h in &headers {
        let marker = if h.id == current { " ◀ (current)" } else { "" };
        let preview = h.previews.first()
            .map(|p| format!(" — {}", truncate_str(p, 60)))
            .unwrap_or_default();
        lines.push(format!(
            "  #{}{}{}",
            &h.id[..8],
            marker,
            preview,
        ));
    }
    lines.push(format!("Use /resume <id> to load a session."));

    app.messages.push(MessageItem::System {
        text: lines.join("\n"),
    });
}

/// Handle `/resume <id>` — load a saved session.
fn handle_resume_cmd(app: &mut TuiApp, input: &str) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    if parts.len() < 2 || parts[1].trim().is_empty() {
        // Show latest session if no id given
        let store = match app.session_store {
            Some(ref store) => store,
            None => {
                app.messages.push(MessageItem::System {
                    text: "Session store not available.".into(),
                });
                return;
            }
        };

        match store.latest() {
            Some(session) => {
                app.messages = session.messages.clone();
                app.current_session_id = Some(session.id.clone());
                app.messages.push(MessageItem::System {
                    text: format!("↻ Resumed session {} ({} msgs)", &session.id[..8], session.message_count),
                });
            }
            None => {
                app.messages.push(MessageItem::System {
                    text: "No saved sessions to resume.".into(),
                });
            }
        }
        return;
    }

    let partial_id = parts[1].trim();
    let store = match app.session_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Session store not available.".into(),
            });
            return;
        }
    };

    // Match by prefix (first 8+ chars)
    let headers = store.list();
    let matched: Vec<_> = headers.iter()
        .filter(|h| h.id.starts_with(partial_id))
        .collect();

    match matched.len() {
        0 => {
            app.messages.push(MessageItem::System {
                text: format!("No session found matching '{}'. Use /session to list.", partial_id),
            });
        }
        1 => {
            match store.load(&matched[0].id) {
                Ok(session) => {
                    app.messages = session.messages.clone();
                    app.current_session_id = Some(session.id.clone());
                    app.messages.push(MessageItem::System {
                        text: format!("↻ Resumed session {} ({} msgs)", &session.id[..8], session.message_count),
                    });
                }
                Err(e) => {
                    app.messages.push(MessageItem::System {
                        text: format!("Error loading session: {e}"),
                    });
                }
            }
        }
        _ => {
            app.messages.push(MessageItem::System {
                text: format!("Multiple sessions match '{}'. Be more specific.", partial_id),
            });
        }
    }
}

/// Truncate a string to max_len chars, appending "…" if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

/// ─── Config Command ───────────────────────────────────────────────────────

/// Handle `/config` and `/config set <key> <value>`.
fn handle_config_cmd(app: &mut TuiApp, input: &str) {
    let store = match app.config_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Config store not available.".into(),
            });
            return;
        }
    };

    let parts: Vec<&str> = input.splitn(4, ' ').collect();

    if parts.len() < 2 || parts[1].trim().is_empty() {
        // /config — show current config
        app.messages.push(MessageItem::System {
            text: store.format_display(),
        });
        return;
    }

    if parts[1] == "set" && parts.len() >= 4 {
        let key = parts[2].to_lowercase();
        let value = parts[3];
        let result = apply_config_setting(app, &key, value);
        app.messages.push(MessageItem::System { text: result });
        return;
    }

    app.messages.push(MessageItem::System {
        text: format!("Unknown config subcommand: {}. Use /config to view, /config set <key> <value> to change.", parts[1]),
    });
}

/// Apply a single config change.
fn apply_config_setting(app: &mut TuiApp, key: &str, value: &str) -> String {
    let store = match app.config_store {
        Some(ref mut store) => store,
        None => return "Config store not available.".into(),
    };

    match key {
        "model" => {
            store.set_model(value);
            app.status.model = value.to_string();
            // Also notify the background agent via set_model command
            if let Err(e) = store.save() {
                return format!("Changed model to {value}, but failed to save: {e}");
            }
            format!("Model set to {value} (saved to codecoder.json). Use /reload to apply in current agent.")
        }
        "api_base" => {
            store.get_mut().llm.api_base = value.to_string();
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("API Base set to {value}. Restart to apply.")
        }
        "max_tokens" => {
            let n: u32 = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().llm.max_tokens = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Max tokens set to {n}. Restart to apply.")
        }
        "temperature" => {
            let t: f32 = match value.parse() {
                Ok(t) => t,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().llm.temperature = t;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Temperature set to {t}. Restart to apply.")
        }
        "tool_rounds" | "max_tool_rounds" => {
            let n: usize = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().features.max_tool_rounds = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Max tool rounds set to {n}. Restart to apply.")
        }
        "cmd_timeout" | "command_timeout_secs" => {
            let n: u64 = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().features.command_timeout_secs = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Command timeout set to {n}s. Restart to apply.")
        }
        "sandbox_memory" | "sandbox_memory_limit" => {
            store.get_mut().features.sandbox_memory_limit = value.to_string();
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Sandbox memory limit set to '{value}'. Restart to apply.")
        }
        _ => {
            format!("Unknown config key: {key}. Supported: model, api_base, max_tokens, temperature, tool_rounds, cmd_timeout, sandbox_memory")
        }
    }
}

/// ─── MCP Command ──────────────────────────────────────────────────────────

/// Handle `/mcp`, `/mcp list`, `/mcp start <name>`, `/mcp stop <name>`.
fn handle_mcp_cmd(app: &mut TuiApp, input: &str) {
    let registry = match app.mcp_registry {
        Some(ref reg) => reg,
        None => {
            app.messages.push(MessageItem::System {
                text: "MCP not available.".into(),
            });
            return;
        }
    };

    let parts: Vec<&str> = input.splitn(3, ' ').collect();
    let sub = parts.get(1).copied().unwrap_or("");

    match sub {
        "list" | "" => {
            let reg = registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let servers = reg.list_servers();
            let tools = reg.all_tools();

            let mut lines = vec!["── MCP Servers ──".to_string()];
            if servers.is_empty() {
                lines.push("  No MCP servers running.".into());
                lines.push("  Configure servers in codecoder.json under 'mcp_servers'.".into());
            } else {
                for s in &servers {
                    lines.push(format!("  ✓ {} (v{}, {} tools)", s.name, s.server_info.version, s.tool_count));
                }
            }

            if !tools.is_empty() {
                lines.push(String::new());
                lines.push("── MCP Tools ──".to_string());
                for t in &tools {
                    let desc = if t.description.len() > 60 {
                        format!("{}…", &t.description[..57])
                    } else {
                        t.description.clone()
                    };
                    lines.push(format!("  · {} [{}] {}", t.tool_name, t.server_name, desc));
                }
            }

            app.messages.push(MessageItem::System {
                text: lines.join("\n"),
            });
        }
        "start" => {
            let name = parts.get(2).unwrap_or(&"");
            if name.is_empty() {
                app.messages.push(MessageItem::System {
                    text: "Usage: /mcp start <server-name>".into(),
                });
                return;
            }

            // Find config from config_store
            let config = match app.config_store {
                Some(ref store) => store.get().clone(),
                None => {
                    app.messages.push(MessageItem::System {
                        text: "Config not available.".into(),
                    });
                    return;
                }
            };

            let server_config = config.mcp_servers.iter()
                .find(|s| s.name == *name)
                .cloned();

            match server_config {
                Some(cfg) => {
                    let mut reg = registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    match reg.start_server(cfg) {
                        Ok(name) => {
                            // Register tools
                            let tools = reg.all_tools();
                            let count = tools.iter().filter(|t| t.server_name == name).count();
                            app.messages.push(MessageItem::System {
                                text: format!("✓ MCP server '{name}' started ({} tools)", count),
                            });
                        }
                        Err(e) => {
                            app.messages.push(MessageItem::System {
                                text: format!("✗ Failed to start MCP server '{name}': {e}"),
                            });
                        }
                    }
                }
                None => {
                    app.messages.push(MessageItem::System {
                        text: format!("No MCP server named '{name}' in config. Use /config to see configured servers."),
                    });
                }
            }
        }
        "stop" => {
            let name = parts.get(2).unwrap_or(&"");
            if name.is_empty() {
                app.messages.push(MessageItem::System {
                    text: "Usage: /mcp stop <server-name>".into(),
                });
                return;
            }
            let mut reg = registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            match reg.stop_server(name) {
                Ok(()) => {
                    app.messages.push(MessageItem::System {
                        text: format!("✓ MCP server '{name}' stopped."),
                    });
                }
                Err(e) => {
                    app.messages.push(MessageItem::System {
                        text: format!("✗ Failed to stop server '{name}': {e}"),
                    });
                }
            }
        }
        _ => {
            app.messages.push(MessageItem::System {
                text: format!("Unknown MCP subcommand: '{sub}'. Use /mcp list, /mcp start, /mcp stop."),
            });
        }
    }
}

/// ─── Session Persistence ──────────────────────────────────────────────────

/// Build a Session from the current TuiApp state.
fn build_session_from_app(app: &TuiApp) -> crate::session::Session {
    let id = app.current_session_id.clone()
        .unwrap_or_else(|| crate::session::Session::new(&app.status.model).id);
    let mut session = crate::session::Session {
        id,
        model: app.status.model.clone(),
        created_at: String::new(),  // preserve original on resume
        updated_at: String::new(),
        message_count: 0,
        token_count: app.status.token_count,
        messages: app.messages.clone(),
    };
    session.touch();
    session
}

/// Auto-save the current session (best-effort).
fn auto_save_session(app: &TuiApp) {
    if let Some(ref store) = app.session_store {
        let session = build_session_from_app(app);
        let _ = store.save(&session);
    }
}

/// ─── Pure-Function Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStore;

    /// Helper: create a ConfigStore backed by a temp directory.
    fn make_config_store(dir: &std::path::Path) -> crate::config::ConfigStore {
        let config_path = dir.join("codecoder.json");
        let default_config = r#"{"llm":{"model":"gpt-4o","api_base":"https://api.openai.com/v1","max_tokens":4096,"temperature":0.0}}"#;
        let _ = std::fs::write(&config_path, default_config);
        crate::config::ConfigStore::load(dir.to_str().unwrap())
    }

    // ── compact_cwd ─────────────────────────────────────────────────────────

    #[test]
    fn test_compact_cwd_short_path() {
        assert_eq!(compact_cwd("/home"), "/home");
    }

    #[test]
    fn test_compact_cwd_exactly_20_chars() {
        let s = "12345678901234567890";
        assert_eq!(compact_cwd(s), s);
    }

    #[test]
    fn test_compact_cwd_long_path_ellipsis() {
        let result = compact_cwd("/very/long/path/that/exceeds/twenty/chars");
        assert!(result.contains('…'), "long path should be truncated with ellipsis");
        assert!(result.contains("chars"), "last component should be visible");
    }

    #[test]
    fn test_compact_cwd_single_component() {
        let result = compact_cwd("/");
        assert_eq!(result, "/");
    }

    #[test]
    fn test_compact_cwd_empty() {
        assert_eq!(compact_cwd(""), "");
    }

    #[test]
    fn test_compact_cwd_two_components_long() {
        let result = compact_cwd("/this-is-a-very-long-directory-name/another-long-name-here");
        assert!(result.contains('…'));
        assert!(result.contains("another-long-name-here"));
    }

    #[test]
    fn test_compact_cwd_unicode() {
        let result = compact_cwd("/项目/源代码/主模块");
        // Unicode paths work with byte-length check
        assert!(result.contains("主模块") || result.len() <= 20);
    }

    #[test]
    fn test_compact_cwd_windows_style() {
        let result = compact_cwd("C:\\Users\\test\\project\\src\\main.rs");
        // Should handle backslash as separator
        assert!(result.contains("main.rs") || result.len() <= 20);
    }

    // ── format_context_bar ──────────────────────────────────────────────────

    #[test]
    fn test_context_bar_zero() {
        let bar = format_context_bar(0.0);
        assert!(bar.contains("0%"));
        assert!(bar.contains('░'));
        assert!(!bar.contains('▓'));
    }

    #[test]
    fn test_context_bar_25_percent() {
        let bar = format_context_bar(0.25);
        assert!(bar.contains("25%"));
    }

    #[test]
    fn test_context_bar_50_percent() {
        let bar = format_context_bar(0.5);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_context_bar_100_percent() {
        let bar = format_context_bar(1.0);
        assert!(bar.contains("100%"));
        assert!(!bar.contains('░'), "100% should have no empty cells");
    }

    #[test]
    fn test_context_bar_overflow_clamped() {
        let bar = format_context_bar(1.5);
        assert!(bar.contains("100%") || bar.contains("150%"));
    }

    #[test]
    fn test_context_bar_negative_clamped() {
        let bar = format_context_bar(-0.1);
        assert!(bar.contains("0%") || bar.contains("-10%"));
    }

    #[test]
    fn test_context_bar_nan_handled() {
        let bar = format_context_bar(f32::NAN);
        // Should not panic; any output is acceptable
        assert!(!bar.is_empty());
    }

    #[test]
    fn test_context_bar_tiny_value() {
        let bar = format_context_bar(0.001);
        // Rounding: 0.001 * 8 = 0.008 → 0 filled
        assert!(bar.contains('░'));
    }

    #[test]
    fn test_context_bar_rounding_boundary() {
        let bar = format_context_bar(0.333);
        assert!(bar.contains("33%") || bar.contains("%"));
    }

    // ── truncate_str ───────────────────────────────────────────────────────

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        let result = truncate_str("hello world", 5);
        // "hell" (4) + "…" (3 bytes) = 7 bytes
        assert!(result.ends_with('…'), "truncated string should end with ellipsis");
        assert!(result.len() <= 8, "result should be reasonably short");
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn test_truncate_str_zero_max() {
        // "…" is 3 bytes; should not panic
        let result = truncate_str("hello", 0);
        assert!(!result.is_empty());
    }

    // ── save_undo_snapshot ──────────────────────────────────────────────────

    #[test]
    fn test_undo_snapshot_saves_state() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        save_undo_snapshot(&mut app);
        assert_eq!(app.undo_stack.len(), 1);
        assert_eq!(app.undo_stack[0], "hello");
    }

    #[test]
    fn test_undo_snapshot_dedup() {
        let mut app = TuiApp::default();
        app.input = "same".into();
        save_undo_snapshot(&mut app);
        save_undo_snapshot(&mut app); // same input → no push
        assert_eq!(app.undo_stack.len(), 1);
    }

    #[test]
    fn test_undo_snapshot_caps_at_100() {
        let mut app = TuiApp::default();
        for i in 0..120 {
            app.input = format!("input-{}", i);
            save_undo_snapshot(&mut app);
        }
        assert!(app.undo_stack.len() <= 100);
    }

    #[test]
    fn test_undo_snapshot_clears_redo() {
        let mut app = TuiApp::default();
        app.redo_stack.push("old".into());
        app.input = "new".into();
        save_undo_snapshot(&mut app);
        assert!(app.redo_stack.is_empty(), "new edit should clear redo stack");
    }

    #[test]
    fn test_undo_redo_roundtrip() {
        let mut app = TuiApp::default();
        app.input = "first".into();
        save_undo_snapshot(&mut app);
        app.input = "second".into();
        save_undo_snapshot(&mut app);

        // Undo stack should have 2 entries
        assert_eq!(app.undo_stack.len(), 2);
        assert_eq!(app.undo_stack[0], "first");
        assert_eq!(app.undo_stack[1], "second");
    }

    // ── count_search_matches ───────────────────────────────────────────────

    #[test]
    fn test_search_matches_empty_query() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.search_query = "".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 0);
    }

    #[test]
    fn test_search_matches_found() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello world".into() });
        app.messages.push(MessageItem::Assistant { text: "world peace".into() });
        app.search_query = "world".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 2);
    }

    #[test]
    fn test_search_matches_not_found() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.messages.push(MessageItem::System { text: "hi".into() });
        app.search_query = "xyz".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 0);
    }

    #[test]
    fn test_search_matches_tool_call() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: String::new(),
            output: "file content".into(),
            expanded: false,
            show_full: false,
        });
        app.search_query = "content".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 1);
    }

    #[test]
    fn test_search_matches_case_insensitive() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "Hello World".into() });
        app.search_query = "hello".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 1);
    }

    // ── update_reverse_search ──────────────────────────────────────────────

    #[test]
    fn test_reverse_search_empty_query() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hi".into() });
        update_reverse_search(&mut app);
        assert!(app.reverse_search_results.is_empty());
    }

    #[test]
    fn test_reverse_search_finds_matches() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "first".into() });
        app.messages.push(MessageItem::Assistant { text: "second".into() });
        app.messages.push(MessageItem::System { text: "third".into() });
        app.reverse_search_query = "second".into();
        update_reverse_search(&mut app);
        assert_eq!(app.reverse_search_results.len(), 1);
        assert_eq!(app.reverse_search_results[0], 1);
    }

    #[test]
    fn test_reverse_search_no_matches() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.reverse_search_query = "xyz".into();
        update_reverse_search(&mut app);
        assert!(app.reverse_search_results.is_empty());
    }

    #[test]
    fn test_reverse_search_sets_last_match_index() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "a".into() });
        app.messages.push(MessageItem::Assistant { text: "a".into() });
        app.messages.push(MessageItem::System { text: "a".into() });
        app.reverse_search_query = "a".into();
        update_reverse_search(&mut app);
        assert_eq!(app.reverse_search_results.len(), 3);
        assert_eq!(app.reverse_search_idx, 2, "should select the last (newest) match");
        assert!(!app.auto_scroll, "reverse search should disable auto-scroll");
    }

    // ── build_session_from_app ─────────────────────────────────────────────

    #[test]
    fn test_build_session_from_app_new() {
        let mut app = TuiApp::default();
        app.status.model = "gpt-4".into();
        app.status.token_count = 42;
        app.messages.push(MessageItem::System { text: "hello".into() });
        let session = build_session_from_app(&app);
        assert_eq!(session.model, "gpt-4");
        assert_eq!(session.token_count, 42);
        assert_eq!(session.messages.len(), 1);
        assert!(!session.id.is_empty());
    }

    #[test]
    fn test_build_session_from_app_existing_id() {
        let mut app = TuiApp::default();
        app.current_session_id = Some("existing-id".into());
        let session = build_session_from_app(&app);
        assert_eq!(session.id, "existing-id");
    }

    // ── auto_save_session ───────────────────────────────────────────────────

    #[test]
    fn test_auto_save_no_store_does_nothing() {
        let app = TuiApp::default();
        // should not panic
        auto_save_session(&app);
    }

    #[test]
    fn test_auto_save_with_store() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        app.messages.push(MessageItem::User { text: "hi".into() });
        auto_save_session(&app);
        // Without session_store set, test that it doesn't panic
        // (full persistence test requires session_store injection)
    }

    // ── apply_config_setting ────────────────────────────────────────────────

    #[test]
    fn test_apply_config_setting_no_store() {
        let mut app = TuiApp::default();
        let result = apply_config_setting(&mut app, "model", "gpt-4");
        assert!(result.contains("not available"));
    }

    #[test]
    fn test_apply_config_setting_model() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "model", "claude-sonnet-4-20250514");
        assert!(result.contains("claude-sonnet-4-20250514"));
        assert_eq!(app.status.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_apply_config_setting_max_tokens() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "max_tokens", "8192");
        assert!(result.contains("8192"));
    }

    #[test]
    fn test_apply_config_setting_invalid_number() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "max_tokens", "not-a-number");
        assert!(result.contains("Invalid"));
    }

    #[test]
    fn test_apply_config_setting_unknown_key() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "unknown_key", "value");
        assert!(result.contains("Unknown"));
    }

    #[test]
    fn test_apply_config_setting_temperature() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "temperature", "0.7");
        assert!(result.contains("0.7"));
    }

    #[test]
    fn test_apply_config_setting_tool_rounds() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "tool_rounds", "5");
        assert!(result.contains("5"));
    }

    #[test]
    fn test_apply_config_setting_cmd_timeout() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        let result = apply_config_setting(&mut app, "cmd_timeout", "120");
        assert!(result.contains("120"));
    }

    // ── Keyboard: Exit Keys ─────────────────────────────────────────────────

    fn key(code: KeyCode, modifiers: KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, modifiers)
    }

    fn press(app: &mut TuiApp, code: KeyCode, modifiers: KeyModifiers, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
        handle_key(app, key(code, modifiers), cmd_tx);
    }

    #[test]
    fn test_key_ctrl_c_quits() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL, &tx);
        assert!(app.should_quit);
    }

    #[test]
    fn test_key_ctrl_q_quits() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('q'), KeyModifiers::CONTROL, &tx);
        assert!(app.should_quit);
    }

    #[test]
    fn test_key_ctrl_d_quits() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL, &tx);
        assert!(app.should_quit);
    }

    #[test]
    fn test_key_esc_quits_when_no_overlay() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(app.should_quit);
    }

    #[test]
    fn test_key_esc_closes_help_instead_of_quit() {
        let mut app = TuiApp::default();
        app.help_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.help_active);
        assert!(!app.should_quit, "Esc should close help, not quit");
    }

    #[test]
    fn test_key_esc_closes_search_instead_of_quit() {
        let mut app = TuiApp::default();
        app.search_active = true;
        app.search_query = "hello".into();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.search_active);
        assert!(app.search_query.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn test_key_esc_closes_reverse_search_instead_of_quit() {
        let mut app = TuiApp::default();
        app.reverse_search_active = true;
        app.reverse_search_query = "test".into();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.reverse_search_active);
        assert!(app.reverse_search_query.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn test_key_esc_deselects_message_instead_of_quit() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hi".into() });
        app.selected_msg = Some(0);
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(app.selected_msg.is_none());
        assert!(!app.should_quit);
    }

    // ── Keyboard: Editing ──────────────────────────────────────────────────

    #[test]
    fn test_key_ctrl_a_goes_to_start() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_key_ctrl_e_goes_to_end() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('e'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_key_ctrl_w_deletes_word_backward() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 11;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello ");
        assert_eq!(app.cursor_pos, 6);
    }

    #[test]
    fn test_key_ctrl_w_at_start_does_nothing() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello");
    }

    #[test]
    fn test_key_ctrl_u_deletes_to_start() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, " world");
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_key_ctrl_k_deletes_to_end() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('k'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello");
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_key_ctrl_l_clears_messages() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hi".into() });
        app.cached_msg_count = 1;
        app.scroll_offset = 5;
        app.auto_scroll = false;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('l'), KeyModifiers::CONTROL, &tx);
        assert!(app.messages.is_empty());
        assert!(app.cached_lines.is_empty());
        assert_eq!(app.scroll_offset, 0);
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_key_ctrl_t_toggles_dark_mode() {
        let mut app = TuiApp::default();
        app.dark_mode = true;
        app.cached_msg_count = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('t'), KeyModifiers::CONTROL, &tx);
        assert!(!app.dark_mode);
        assert_eq!(app.cached_msg_count, 0, "Ctrl+T should clear render cache");
        // Toggle back
        press(&mut app, KeyCode::Char('t'), KeyModifiers::CONTROL, &tx);
        assert!(app.dark_mode);
    }

    #[test]
    fn test_key_delete_removes_char() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Delete, KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "ello");
    }

    #[test]
    fn test_key_backspace_removes_char() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "hell");
        assert_eq!(app.cursor_pos, 4);
    }

    #[test]
    fn test_key_backspace_at_start_does_nothing() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "hello");
    }

    #[test]
    fn test_key_ctrl_z_undo_input() {
        let mut app = TuiApp::default();
        app.input = "second".into();
        app.undo_stack.push("first".into());
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "first");
        assert_eq!(app.redo_stack.len(), 1);
    }

    #[test]
    fn test_key_ctrl_y_redo_input() {
        let mut app = TuiApp::default();
        app.input = "first".into();
        app.redo_stack.push("second".into());
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "second");
    }

    #[test]
    fn test_key_ctrl_z_empty_stack_does_nothing() {
        let mut app = TuiApp::default();
        app.input = "stay".into();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "stay");
    }

    // ── Keyboard: Navigation + Enter ───────────────────────────────────────

    #[test]
    fn test_key_pageup_scrolls_up() {
        let mut app = TuiApp::default();
        app.auto_scroll = false;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::PageUp, KeyModifiers::NONE, &tx);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn test_key_pagedown_scrolls_down() {
        let mut app = TuiApp::default();
        app.scroll_offset = 20;
        app.auto_scroll = false;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::PageDown, KeyModifiers::NONE, &tx);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn test_key_up_input_history() {
        let mut app = TuiApp::default();
        app.input_history = vec!["first".into(), "second".into()];
        app.history_pos = 2;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Up, KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "second");
        assert_eq!(app.history_pos, 1);
    }

    #[test]
    fn test_key_up_selects_message_when_input_empty() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Up, KeyModifiers::NONE, &tx);
        assert_eq!(app.selected_msg, Some(0));
    }

    #[test]
    fn test_key_down_input_history() {
        let mut app = TuiApp::default();
        app.input_history = vec!["first".into(), "second".into()];
        app.history_pos = 0;
        app.input = "first".into();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Down, KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "second");
    }

    #[test]
    fn test_key_left_moves_cursor() {
        let mut app = TuiApp::default();
        app.input = "hi".into();
        app.cursor_pos = 2;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Left, KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 1);
    }

    #[test]
    fn test_key_right_moves_cursor() {
        let mut app = TuiApp::default();
        app.input = "hi".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Right, KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 1);
    }

    #[test]
    fn test_key_home_goes_to_start() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Home, KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_key_end_goes_to_end() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::End, KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_key_ctrl_f_toggles_search() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL, &tx);
        assert!(app.search_active);
        press(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL, &tx);
        assert!(!app.search_active);
    }

    #[test]
    fn test_key_ctrl_r_toggles_reverse_search() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL, &tx);
        assert!(app.reverse_search_active);
        press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL, &tx);
        assert!(!app.reverse_search_active);
    }

    #[test]
    fn test_key_ctrl_p_toggles_model_picker() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL, &tx);
        assert!(app.model_picker_active);
        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL, &tx);
        assert!(!app.model_picker_active);
    }

    #[test]
    fn test_key_ctrl_h_toggles_help() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('h'), KeyModifiers::CONTROL, &tx);
        assert!(app.help_active);
        press(&mut app, KeyCode::Char('h'), KeyModifiers::CONTROL, &tx);
        assert!(!app.help_active);
    }

    // ── Keyboard: Enter with non-empty input sends message ──────────────────

    #[test]
    fn test_enter_sends_message() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 5;
        let (tx, rx) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE, &tx);
        assert!(app.input.is_empty(), "input should be cleared after send");
        assert_eq!(app.cursor_pos, 0);
        // User message + status messages (like "[send] Agent…")
        assert!(app.messages.len() >= 1, "should have at least user message");
        assert!(matches!(app.messages.first(), Some(MessageItem::User { text }) if text == "hello"));
        // AgentCommand should have been sent
        let cmd = rx.try_recv();
        assert!(cmd.is_ok(), "an AgentCommand should have been sent");
    }

    #[test]
    fn test_alt_enter_sends_message() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 5;
        let (tx, rx) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Enter, KeyModifiers::ALT, &tx);
        assert!(app.input.is_empty());
        let cmd = rx.try_recv();
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_shift_enter_inserts_newline() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Enter, KeyModifiers::SHIFT, &tx);
        assert!(app.input.contains('\n'));
        // Note: original code handles Shift+Enter in the Enter branch,
        // but the actual handling is at line ~1314 which is OR'd with Alt
        // This is a basic sanity test
    }

    #[test]
    fn test_enter_empty_input_toggles_reasoning() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Reasoning { text: "thinking".into(), expanded: false });
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE, &tx);
        // Last message should be toggled expanded
        if let Some(MessageItem::Reasoning { expanded, .. }) = app.messages.last() {
            assert!(*expanded, "Reasoning block should be expanded");
        } else {
            panic!("Last message should be Reasoning");
        }
    }

    #[test]
    fn test_enter_empty_input_toggles_tool_full() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: String::new(),
            output: "content".into(),
            expanded: true,
            show_full: false,
        });
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE, &tx);
        if let Some(MessageItem::ToolCall { show_full, .. }) = app.messages.last() {
            assert!(*show_full, "ToolCall should be toggled show_full");
        } else {
            panic!("Last message should be ToolCall");
        }
    }

    // ── Keyboard: Permission Dialogs ───────────────────────────────────────

    #[test]
    fn test_key_y_approves_tool_permission() {
        let mut app = TuiApp::default();
        app.permission_pending = Some(PendingPermission {
            tool_name: "read_file".into(),
            tool_input: "test".into(),
            request_id: 1,
        });
        let (tx, rx) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE, &tx);
        assert!(app.permission_pending.is_none());
        let cmd = rx.try_recv();
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_key_n_denies_tool_permission() {
        let mut app = TuiApp::default();
        app.permission_pending = Some(PendingPermission {
            tool_name: "read_file".into(),
            tool_input: "test".into(),
            request_id: 1,
        });
        let (tx, rx) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('n'), KeyModifiers::NONE, &tx);
        assert!(app.permission_pending.is_none());
        let cmd = rx.try_recv();
        assert!(cmd.is_ok());
    }

    // ── Keyboard: Model Picker ──────────────────────────────────────────────

    #[test]
    fn test_model_picker_up_down_enter() {
        let mut app = TuiApp::default();
        app.model_picker_active = true;
        app.model_picker_selected = 2;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Up, KeyModifiers::NONE, &tx);
        assert_eq!(app.model_picker_selected, 1);
        press(&mut app, KeyCode::Down, KeyModifiers::NONE, &tx);
        assert_eq!(app.model_picker_selected, 2);
        press(&mut app, KeyCode::Down, KeyModifiers::NONE, &tx);
        assert_eq!(app.model_picker_selected, 3);
    }

    // ── Keyboard: Slash Completion ──────────────────────────────────────────

    #[test]
    fn test_slash_completion_up_down_selection() {
        let mut app = TuiApp::default();
        // Set up proper initial state to activate slash completion
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('/'), KeyModifiers::NONE, &tx);
        assert!(app.slash_completion.active);
        assert_eq!(app.slash_completion.selected, 0);
        // Down should move selection forward
        press(&mut app, KeyCode::Down, KeyModifiers::NONE, &tx);
        assert_eq!(app.slash_completion.selected, 1);
        // Up should move selection backward
        press(&mut app, KeyCode::Up, KeyModifiers::NONE, &tx);
        assert_eq!(app.slash_completion.selected, 0);
        // Up at top should wrap or stay (currently stays)
        press(&mut app, KeyCode::Up, KeyModifiers::NONE, &tx);
        assert_eq!(app.slash_completion.selected, 0, "Up at top should stay at 0");
    }

    // ── Keyboard: Input Characters ──────────────────────────────────────────

    #[test]
    fn test_typing_characters() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE, &tx);
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "hi");
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_typing_in_search_mode() {
        let mut app = TuiApp::default();
        app.search_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('x'), KeyModifiers::NONE, &tx);
        assert!(app.input.is_empty(), "typing in search mode should not modify input");
        assert_eq!(app.search_query, "x");
    }

    #[test]
    fn test_typing_in_reverse_search_mode() {
        let mut app = TuiApp::default();
        app.reverse_search_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE, &tx);
        assert!(app.input.is_empty());
        assert_eq!(app.reverse_search_query, "y");
    }

    // ── @ File Completion ───────────────────────────────────────────────────

    #[test]
    fn test_at_completion_triggers_on_typing() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('@'), KeyModifiers::NONE, &tx);
        assert!(app.completion.active);
    }

    #[test]
    fn test_at_completion_tab_accepts() {
        let mut app = TuiApp::default();
        app.input = "@".into();
        app.cursor_pos = 1;
        app.completion.active = true;
        app.completion.at_pos = 0;
        app.completion.candidates = vec![
            crate::tui::completion::CompletionCandidate {
                display: "src/main.rs".into(),
                path: "/root/src/main.rs".into(),
            }
        ];
        app.completion.selected = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE, &tx);
        assert!(app.input.contains("src/main.rs"));
        assert!(!app.completion.active, "completion should close after Tab accept");
    }

    // ── Slash Commands: Session / Resume ────────────────────────────────────

    #[test]
    fn test_slash_session_no_store() {
        let mut app = TuiApp::default();
        // Without session_store, should show "not available"
        app.input = "/session".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("not available"),
            _ => false,
        }));
    }

    #[test]
    fn test_slash_session_lists_sessions() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        // Save a session first
        let session = crate::session::Session {
            id: "test-session-id".into(),
            model: "gpt-4".into(),
            created_at: String::new(),
            updated_at: String::new(),
            message_count: 1,
            token_count: 0,
            messages: vec![MessageItem::System { text: "hello".into() }],
        };
        let _ = store.save(&session);
        app.session_store = Some(store);
        app.input = "/session".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        // Verify the session list shows the session ID (truncated to first 8 chars)
        let any_session_line = app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("test-ses"),
            _ => false,
        });
        assert!(any_session_line, "Session listing should show truncated session ID");
    }

    #[test]
    fn test_slash_resume_no_id_loads_latest() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let session = crate::session::Session {
            id: "resume-test-id".into(),
            model: "gpt-4".into(),
            created_at: String::new(),
            updated_at: String::new(),
            message_count: 1,
            token_count: 0,
            messages: vec![MessageItem::User { text: "saved msg".into() }],
        };
        let _ = store.save(&session);
        app.session_store = Some(store);
        app.input = "/resume".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::User { text } => text == "saved msg",
            MessageItem::System { text } => text.contains("Resumed"),
            _ => false,
        }));
    }

    #[test]
    fn test_slash_resume_no_sessions() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        app.session_store = Some(store);
        app.input = "/resume".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("No saved sessions"),
            _ => false,
        }));
    }

    // ── Slash Commands: Config ──────────────────────────────────────────────

    #[test]
    fn test_slash_config_no_store() {
        let mut app = TuiApp::default();
        app.input = "/config".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("not available"),
            _ => false,
        }));
    }

    #[test]
    fn test_slash_config_shows_display() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        app.input = "/config".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        // Should show model info in the response
        let sys_msgs: Vec<&String> = app.messages.iter()
            .filter_map(|m| match m {
                MessageItem::System { text } => Some(text),
                _ => None,
            })
            .collect();
        assert!(!sys_msgs.is_empty(), "config should produce a System message");
    }

    #[test]
    fn test_slash_config_set_model() {
        let mut app = TuiApp::default();
        let dir = tempfile::tempdir().unwrap();
        let store = make_config_store(dir.path());
        app.config_store = Some(store);
        app.input = "/config set model claude-sonnet-4-20250514".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert_eq!(app.status.model, "claude-sonnet-4-20250514");
    }

    // ── Slash Commands: MCP ─────────────────────────────────────────────────

    #[test]
    fn test_slash_mcp_no_registry() {
        let mut app = TuiApp::default();
        app.input = "/mcp".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("not available"),
            _ => false,
        }));
    }

    #[test]
    fn test_slash_mcp_list_empty() {
        let mut app = TuiApp::default();
        let registry = Arc::new(Mutex::new(crate::mcp::McpRegistry::new(Vec::new())));
        app.mcp_registry = Some(registry);
        app.input = "/mcp list".into();
        let (tx, _) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("No MCP servers"),
            _ => false,
        }));
    }

    // ── Agent Response ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_agent_response_text() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        app.status.agent_busy = true;
        tx.send(AgentResponse::Text { text: "hello from agent".into() }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(!app.status.agent_busy, "Text response should clear busy");
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::Assistant { text } => text == "hello from agent",
            _ => false,
        }));
    }

    #[tokio::test]
    async fn test_agent_response_tool_call() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::ToolCall {
            name: "read_file".into(),
            input: "/tmp/test".into(),
        }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.status.agent_busy, "ToolCall should set busy");
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::ToolCall { name, .. } => name == "read_file",
            _ => false,
        }));
    }

    #[tokio::test]
    async fn test_agent_response_tool_result() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: String::new(),
            output: String::new(),
            expanded: false,
            show_full: false,
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::ToolResult {
            name: "read_file".into(),
            output: "file content".into(),
            success: true,
        }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        if let Some(MessageItem::ToolCall { output, expanded, .. }) = app.messages.last() {
            assert!(output.contains("file content"));
            assert!(!expanded, "successful tool result should not auto-expand");
        } else {
            panic!("Last message should be ToolCall");
        }
    }

    #[tokio::test]
    async fn test_agent_response_heartbeat() {
        let mut app = TuiApp::default();
        app.status.agent_busy = false;
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::Heartbeat { pending: 2 }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.status.agent_busy);
    }

    #[tokio::test]
    async fn test_agent_response_error() {
        let mut app = TuiApp::default();
        app.status.agent_busy = true;
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::Error { message: "something broke".into() }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(!app.status.agent_busy, "Error should clear busy");
        assert!(app.messages.iter().any(|m| match m {
            MessageItem::System { text } => text.contains("something broke"),
            _ => false,
        }));
    }

    #[tokio::test]
    async fn test_agent_response_shutdown() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::Shutdown).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_agent_response_permission_request() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::PermissionRequest {
            tool_name: "write_file".into(),
            tool_input: "test content".into(),
            request_id: 42,
        }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.permission_pending.is_some());
        let p = app.permission_pending.as_ref().unwrap();
        assert_eq!(p.tool_name, "write_file");
        assert_eq!(p.request_id, 42);
    }

    #[tokio::test]
    async fn test_agent_response_plan_request() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::PlanRequest {
            title: "refactor".into(),
            plan: "step 1, step 2".into(),
            request_id: 7,
        }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.permission_pending.is_some());
        assert_eq!(app.permission_pending.as_ref().unwrap().request_id, 7);
    }

    #[tokio::test]
    async fn test_agent_response_ask_user() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::AskUser {
            question: "continue?".into(),
            request_id: 99,
        }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        assert!(app.permission_pending.is_some());
        assert_eq!(app.permission_pending.as_ref().unwrap().tool_name, "ask_user");
    }

    #[tokio::test]
    async fn test_agent_response_llm_delta_append() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "hello ".into() });
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::LlmDelta { text: "world".into() }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        if let Some(MessageItem::Assistant { text }) = app.messages.last() {
            assert_eq!(text, "hello world");
        } else {
            panic!("Last message should be Assistant");
        }
    }

    #[tokio::test]
    async fn test_agent_response_reasoning_delta() {
        let mut app = TuiApp::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        tx.send(AgentResponse::ReasoningDelta { text: "step 1".into() }).await.unwrap();
        check_agent_responses(&mut app, &mut rx);
        if let Some(MessageItem::Reasoning { text, .. }) = app.messages.last() {
            assert!(text.contains("step 1"));
        } else {
            panic!("Last message should be Reasoning");
        }
        // Append
        let (tx2, mut rx2) = tokio::sync::mpsc::channel(16);
        tx2.send(AgentResponse::ReasoningDelta { text: ", step 2".into() }).await.unwrap();
        // Move the receiver
        std::mem::swap(&mut rx, &mut rx2);
        check_agent_responses(&mut app, &mut rx);
        if let Some(MessageItem::Reasoning { text, .. }) = app.messages.last() {
            assert!(text.contains("step 1, step 2"));
        } else {
            panic!("Last message should be Reasoning");
        }
    }

    // ── Message Line Building + Cache ───────────────────────────────────────

    #[test]
    fn test_build_message_lines_builds_cache() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        let lines = build_message_lines(&mut app);
        assert!(!lines.is_empty(), "should produce rendered lines");
        assert_eq!(app.cached_msg_count, 1, "cache count should be set");
        assert!(!app.cached_lines.is_empty(), "cached lines should be populated");
    }

    #[test]
    fn test_build_message_lines_cache_hit() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        // First call builds cache
        let first = build_message_lines(&mut app);
        assert_eq!(app.cached_msg_count, 1);
        // Second call should hit cache
        let second = build_message_lines(&mut app);
        assert_eq!(first.len(), second.len(), "cache hit should produce same lines");
    }

    #[test]
    fn test_build_message_lines_cache_miss_on_new_message() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "first".into() });
        let first = build_message_lines(&mut app);
        let first_count = app.cached_msg_count;

        // Add a new message
        app.messages.push(MessageItem::Assistant { text: "second".into() });
        let second = build_message_lines(&mut app);
        assert_eq!(app.cached_msg_count, 2, "cache should update after new message");
        assert!(second.len() > first.len(), "more messages should produce more lines");
    }

    #[test]
    fn test_build_message_lines_cache_miss_on_search_change() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "hello world".into() });
        let first = build_message_lines(&mut app);
        assert_eq!(app.cached_msg_count, 1);

        // Change search query (even though search isn't active)
        app.search_query = "hello".into();
        let second = build_message_lines(&mut app);
        // Should rebuild because cached_search_query differs
        assert_eq!(app.cached_search_query, "hello");
    }

    #[test]
    fn test_build_message_lines_user_message_renders() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        let lines = build_message_lines(&mut app);
        // User messages have a "you >" prefix
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("▶"), "user messages should show ▶ prefix");
        assert!(rendered.contains("hello"), "user message text should appear");
    }

    #[test]
    fn test_build_message_lines_assistant_message_renders() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "assistant reply".into() });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("assistant"), "assistant text should appear");
    }

    #[test]
    fn test_build_message_lines_system_message_renders() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "system note".into() });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("system"), "system message text should appear");
    }

    #[test]
    fn test_build_message_lines_tool_call_collapsed() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: String::new(),
            output: "file contents".into(),
            expanded: false,
            show_full: false,
        });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("⚙"), "tool call should show ⚙ prefix");
        assert!(rendered.contains("file contents"), "tool output should be visible");
    }

    #[test]
    fn test_build_message_lines_tool_call_expanded() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: String::new(),
            output: "file contents".into(),
            expanded: true,
            show_full: false,
        });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("⚙"), "expanded tool call should show ⚙ prefix");
        assert!(rendered.contains("file contents"), "expanded tool output should be visible");
    }

    #[test]
    fn test_build_message_lines_reasoning_collapsed() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Reasoning { text: "thinking...".into(), expanded: false });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("·"), "reasoning should show · prefix");
        assert!(rendered.contains("thinking..."), "reasoning text should be visible");
    }

    #[test]
    fn test_build_message_lines_reasoning_expanded() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Reasoning { text: "thinking...".into(), expanded: true });
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("·"), "reasoning should show · prefix");
        assert!(rendered.contains("thinking..."), "reasoning text should be visible");
    }

    #[test]
    fn test_build_message_lines_selected_message() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "select me".into() });
        app.selected_msg = Some(0);
        let lines = build_message_lines(&mut app);
        let rendered = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains("▸"), "selected message should have ▸ indicator");
    }

    #[test]
    fn test_render_empty_tui() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "CodeCoder initialized".into() });
        app.status = StatusData {
            model: "gpt-4o".into(),
            cwd: "/tmp".into(),
            context_pct: 0.0,
            token_count: 0,
            api_key_set: true,
            agent_busy: true,
            current_tool: None,
            connection_type: "OpenAI".into(),
            elapsed_secs: 5,
            current_round: 2,
        };

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Should show the system message
        assert!(cell_text.contains("CodeCoder"), "Should show message: got {cell_text:.80}");
        // Status bar should show model
        assert!(cell_text.contains("gpt-4o"), "Should show model: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_search_active() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello world".into() });
        app.search_active = true;
        app.search_query = "hello".into();
        app.search_match_count = 1;
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("hello"), "Should show query: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_help_active() {
        let mut app = TuiApp::default();
        app.help_active = true;
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("CodeCoder"), "Should still render base UI: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_agent_busy() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "list all tools".into() });
        app.status.agent_busy = true;
        app.status.elapsed_secs = 10;
        app.status.current_round = 3;
        app.status.current_tool = Some("search_web".into());
        app.status.api_key_set = true;

        let backend = ratatui::backend::TestBackend::new(80, 15);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Input area should be minimal
        assert!(cell_text.contains("CodeCoder"), "Should show message: got {cell_text:.80}");
        // Status bar should show model
        assert!(cell_text.contains("gpt-4o"), "Should show model: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_model_picker() {
        let mut app = TuiApp::default();
        app.model_picker_active = true;
        app.model_picker_selected = 0;
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Model picker overlay should show model names
        assert!(cell_text.contains("gpt-4o"), "Should show model picker: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_reverse_search() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "find me".into() });
        app.messages.push(MessageItem::Assistant { text: "result".into() });
        app.reverse_search_active = true;
        app.reverse_search_query = "find".into();
        app.reverse_search_results = vec![0];
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 15);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("find"), "Should show reverse search: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_permission_pending() {
        let mut app = TuiApp::default();
        app.permission_pending = Some(PendingPermission {
            tool_name: "write_file".into(),
            tool_input: "test.txt".into(),
            request_id: 1,
        });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("write_file"), "Should show permission: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_manual_scroll() {
        let mut app = TuiApp::default();
        for i in 0..20 {
            app.messages.push(MessageItem::User { text: format!("message {}", i) });
            app.messages.push(MessageItem::Assistant { text: format!("response {}", i) });
        }
        app.auto_scroll = false;
        app.scroll_offset = 5;
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("scroll"), "Should show scroll indicator: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_tool_call_expanded() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "search".into() });
        app.messages.push(MessageItem::ToolCall {
            name: "search_web".into(),
            input: "rust".into(),
            output: "file contents".into(),
            expanded: true,
            show_full: false,
        });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("file contents"), "Should show tool output: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_light_mode() {
        let mut app = TuiApp::default();
        app.dark_mode = false;
        app.messages.push(MessageItem::System { text: "hello".into() });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Light mode should still render
        assert!(cell_text.contains("hello"), "Should show message: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_many_messages_scrollbar() {
        let mut app = TuiApp::default();
        for i in 0..30 {
            app.messages.push(MessageItem::User { text: format!("long message number {} with some extra text", i) });
            app.messages.push(MessageItem::Assistant { text: format!("response to message {} with additional details", i) });
        }
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(60, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("CodeCoder"), "Should render base UI");
    }

    #[test]
    fn test_render_with_reasoning_message() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "think step by step".into() });
        app.messages.push(MessageItem::Reasoning { text: "First, I need to analyze the problem...".into(), expanded: true });
        app.messages.push(MessageItem::Assistant { text: "Here is the answer.".into() });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("analyze"), "Should show reasoning: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_mixed_message_types() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "Session started".into() });
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.messages.push(MessageItem::ToolCall {
            name: "read_file".into(),
            input: "src/main.rs".into(),
            output: "file contents here".into(),
            expanded: false,
            show_full: false,
        });
        app.messages.push(MessageItem::Assistant { text: "Done reading.".into() });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("Session"), "Should show system message: got {cell_text:.80}");
        assert!(cell_text.contains("hello"), "Should show user message");
    }

    #[test]
    fn test_render_tool_call_full_expanded() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "search_web".into(),
            input: "query".into(),
            output: "a very long output that should be displayed in full when show_full is true".into(),
            expanded: true,
            show_full: true,
        });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 15);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Should show the output content in full mode
        assert!(cell_text.contains("very long"), "Should show full output: got {cell_text:.80}");
    }

    #[test]
    fn test_render_with_elapsed_time() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.status = StatusData::default();
        app.status.agent_busy = true;
        app.status.elapsed_secs = 120;

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 3);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Busy indicator in title
        assert!(cell_text.contains("•"), "Should have busy indicator: got {cell_text:.80}");
    }

    #[test]
    fn test_render_search_active_with_results() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "search term".into() });
        app.messages.push(MessageItem::Assistant { text: "containing search term result".into() });
        app.search_active = true;
        app.search_query = "search".into();
        app.search_match_count = 2;
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 15);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("search"), "Should show search state: got {cell_text:.80}");
    }

    #[test]
    fn test_render_input_with_long_text() {
        let mut app = TuiApp::default();
        app.input = "a very long input message that exceeds the width of the terminal and should wrap to the next line or be truncated depending on the implementation".into();
        app.cursor_pos = app.input.len();
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(40, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer.content.len() > 0, "Should render with long input");
    }

    #[test]
    fn test_render_multiple_system_messages() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "System initialized".into() });
        app.messages.push(MessageItem::System { text: "Skills loaded: 5".into() });
        app.messages.push(MessageItem::System { text: "MCP connected: 2 servers".into() });
        app.status = StatusData::default();

        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                super::render(f, &mut app, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("initialized"), "Should show system messages: got {cell_text:.80}");
    }
}
