/// ─── TUI Module ───────────────────────────────────────────────────────────
///
/// ratatui + crossterm 全屏终端界面。
/// 三段布局：消息列表（上） + 输入框（中） + 状态栏（底）

pub mod completion;
pub mod markdown;
pub mod message_list;
pub mod status_bar;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Flex, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Duration;

use crate::agent::{AgentCommand, AgentResponse};
use crate::event::SharedEventBus;
use std::sync::{Arc, Mutex};

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
    mut config_store: crate::config::ConfigStore,
    mcp_registry: Arc<Mutex<crate::mcp::McpRegistry>>,
) -> Result<()> {
    // Enable raw mode and bracketed paste
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableBracketedPaste,
        crossterm::cursor::SetCursorStyle::BlinkingBlock,
    )?;

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let mut app = TuiApp::default();
    app.status.model = config_store.model().to_string();

    // ── 加载最新持久化会话 ───────────────────────────────────────────────
    let mut current_session_id: Option<String> = None;
    if let Some(session) = session_store.latest() {
        // Only resume if the session has actual messages
        if !session.messages.is_empty() {
            app.messages = session.messages.clone();
            current_session_id = Some(session.id.clone());
            // Restore model from session
            if !session.model.is_empty() {
                app.status.model = session.model;
            }
            // Show resume indicator
            app.messages.push(MessageItem::System {
                text: format!("↻ Resumed session {} ({})", &session.id[..8], session.message_count),
            });
        }
    }

    // 如果没恢复，显示欢迎消息
    if current_session_id.is_none() {
        app.messages.push(MessageItem::System {
            text: format!(
                "CodeCoder TUI — {} model",
                app.status.model,
            ),
        });
    }

    // 注入存储到 app
    app.session_store = Some(session_store);
    app.current_session_id = current_session_id;
    app.config_store = Some(config_store);
    app.mcp_registry = Some(mcp_registry);

    // 帧计数器（用于 spinner 动画）
    let mut frame_count: u64 = 0;

    // 主事件循环
    while !app.should_quit {
        frame_count = frame_count.wrapping_add(1);
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
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::SetCursorStyle::DefaultUserShape,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    ratatui::restore();
    Ok(())
}

/// ─── Render ───────────────────────────────────────────────────────────────

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
        match m {
            MessageItem::User { text } => {
                if is_selected {
                    lines.push(Line::styled(
                        " ▸ you >",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    lines.push(Line::styled(
                        "  you >",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::DIM),
                    ));
                }
                lines.extend(
                    markdown::render_markdown_with_highlight(text, highlight)
                        .into_iter()
                        .map(|l| {
                            let mut line = l;
                            line.spans.insert(0, Span::raw("    "));
                            line
                        }),
                );
                lines.push(Line::from(""));
            }
            MessageItem::Assistant { text } => {
                lines.extend(markdown::render_markdown_with_highlight(text, highlight));
                lines.push(Line::from(""));
            }
            MessageItem::Reasoning { text, expanded } => {
                if *expanded {
                    lines.push(Line::styled(
                        " ▼ 🧠 Thinking:".to_string(),
                        Style::default().fg(Color::Magenta),
                    ));
                    for line in text.lines() {
                        lines.push(Line::styled(
                            format!(" │ {}", line),
                            Style::default().fg(Color::Magenta).add_modifier(Modifier::DIM),
                        ));
                    }
                    lines.push(Line::styled(
                        " └─",
                        Style::default().fg(Color::Magenta),
                    ));
                } else {
                    lines.push(Line::styled(
                        format!(" ▶ 🧠 Thinking ({} chars)", text.len()),
                        Style::default().fg(Color::Magenta),
                    ));
                }
                lines.push(Line::from(""));
            }
            MessageItem::ToolCall {
                name,
                output,
                expanded,
                show_full,
                ..
            } => {
                if *expanded {
                    lines.push(Line::styled(
                        format!(" {} {}:", "▼".yellow(), name),
                        Style::default().fg(Color::Yellow),
                    ));
                    let output_lines: Vec<&str> = output.lines().collect();
                    let max_preview = 20;
                    let total = output_lines.len();

                    let visible_lines: Vec<&&str> = if *show_full || total <= max_preview {
                        output_lines.iter().collect()
                    } else {
                        output_lines[..max_preview].iter().collect()
                    };

                    for line in visible_lines {
                        lines.push(Line::styled(
                            format!("   {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }

                    if total > max_preview && !*show_full {
                        lines.push(Line::styled(
                            format!("   ... ({} more lines — press Enter to expand)", total - max_preview),
                            Style::default().fg(Color::Blue).add_modifier(Modifier::ITALIC),
                        ));
                    } else if total > max_preview {
                        lines.push(Line::styled(
                            format!("   ... (showing all {} lines)", total),
                            Style::default().fg(Color::Blue).add_modifier(Modifier::ITALIC),
                        ));
                    }
                } else {
                    lines.push(Line::styled(
                        format!(" {} {}", "▶".yellow(), name),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                lines.push(Line::from(""));
            }
            MessageItem::System { text } => {
                lines.push(Line::styled(
                    format!(" {}", text),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                ));
                lines.push(Line::from(""));
            }
        }
    }
    lines
}

fn render(frame: &mut Frame, app: &mut TuiApp, frame_count: u64) {
    let area = frame.area();

    // 三段 Flex 布局：消息区（flex_grow=1）+ 输入区（3行）+ 状态栏（1行）
    let [msg_area, input_area, status_area] = ratatui::layout::Layout::new(
        ratatui::layout::Direction::Vertical,
        [
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(1),
        ],
    )
    .flex(Flex::Start)
    .areas(area);

    // 消息区 — 先构建全部行，再计算滚动
    let rendered_lines = build_message_lines(app);
    let total_lines = rendered_lines.len();
    let msg_height = msg_area.height.saturating_sub(1) as usize; // minus single border line

    // 自动滚到底部：如果在 auto_scroll 模式，始终显示最新消息
    if app.auto_scroll {
        app.scroll_offset = total_lines.saturating_sub(msg_height);
    } else if app.scroll_offset > total_lines.saturating_sub(msg_height) {
        // 如果手动滚过头了，停在最大位置
        app.scroll_offset = total_lines.saturating_sub(msg_height);
    }

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
                    " \u{1f50d} {} ({} hits)",
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

    // 输入区（带光标显示）— 使用细分割线风格
    let cursor_pos = app.cursor_pos.min(app.input.len());
    let input_display = if app.input.is_empty() {
        if app.status.agent_busy {
            " Agent is thinking...".to_string()
        } else {
            " Send a message... (Alt+Enter)".to_string()
        }
    } else {
        app.input.clone()
    };
    let input_paragraph = Paragraph::new(input_display)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::TOP)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(if app.input.is_empty() {
            Color::DarkGray
        } else {
            Color::White
        }))
        .wrap(Wrap { trim: false });
    frame.render_widget(input_paragraph, input_area);

    // 设置光标位置
    // 计算光标在 input 区域内的行内偏移
    let input_lines = app.input[..cursor_pos].lines().count().max(1) - 1;
    let last_line_start = if input_lines == 0 {
        0
    } else {
        app.input[..cursor_pos]
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .last()
            .map(|(i, _)| i + 1)
            .unwrap_or(0)
    };
    let col_offset = cursor_pos - last_line_start;
    let row_offset = input_lines;

    // ratatui 自动处理光标的 set_cursor
    frame.set_cursor_position(ratatui::layout::Position {
        x: input_area.x + col_offset as u16 + 1, // +1 for border left
        y: input_area.y + row_offset as u16 + 1, // +1 for border top
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
                Block::bordered()
                    .border_type(BorderType::Plain)
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
                Block::bordered()
                    .border_type(BorderType::Plain)
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
            Line::styled(" Keyboard Shortcuts ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Enter           ", Style::default().fg(Color::Green)),
                Span::raw("Send message"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("S+Enter         ", Style::default().fg(Color::Green)),
                Span::raw("New line in input"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↑/↓             ", Style::default().fg(Color::Green)),
                Span::raw("Input history / navigate lists"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("←/→/Home/End    ", Style::default().fg(Color::Green)),
                Span::raw("Cursor movement"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("PgUp/PgDn       ", Style::default().fg(Color::Green)),
                Span::raw("Scroll messages"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Tab             ", Style::default().fg(Color::Green)),
                Span::raw("Cycle completions"),
            ]),
            Line::from(""),
            Line::styled(" Shortcuts ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+F          ", Style::default().fg(Color::Green)),
                Span::raw("Search messages"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+R          ", Style::default().fg(Color::Green)),
                Span::raw("Reverse search"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+P          ", Style::default().fg(Color::Green)),
                Span::raw("Switch model"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+H          ", Style::default().fg(Color::Green)),
                Span::raw("This help panel"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+A/E        ", Style::default().fg(Color::Green)),
                Span::raw("Line start/end"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+W          ", Style::default().fg(Color::Green)),
                Span::raw("Delete word backward"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+U          ", Style::default().fg(Color::Green)),
                Span::raw("Delete to line start"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+K          ", Style::default().fg(Color::Green)),
                Span::raw("Delete to line end"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+L          ", Style::default().fg(Color::Green)),
                Span::raw("Clear screen"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+Z/Y        ", Style::default().fg(Color::Green)),
                Span::raw("Undo / Redo"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+T          ", Style::default().fg(Color::Green)),
                Span::raw("Toggle dark/light theme"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Ctrl+C/Q/D/Esc  ", Style::default().fg(Color::Green)),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::styled(" Commands ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/help           ", Style::default().fg(Color::Green)),
                Span::raw("Show this help"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/exit /quit     ", Style::default().fg(Color::Green)),
                Span::raw("Exit application"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/reload         ", Style::default().fg(Color::Green)),
                Span::raw("Reload skills & context"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/clear          ", Style::default().fg(Color::Green)),
                Span::raw("Clear conversation"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/tools /skills  ", Style::default().fg(Color::Green)),
                Span::raw("List tools/skills"),
            ]),
            Line::from(""),
            Line::styled(" Press Esc to close ", Style::default().fg(Color::DarkGray)),
        ];

        frame.render_widget(ratatui::widgets::Clear, panel_area);
        let panel = Paragraph::new(help_lines)
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(Color::Cyan)),
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
            Line::styled(" ⚠  Tool Permission Required ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                app.cursor_pos += 1;
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
            app.cursor_pos += 1;

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
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);

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
    let _ = cmd_tx.send(AgentCommand::ProcessMessage { text: input });

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
                        app.messages.push(MessageItem::Assistant { text });
                        app.status.agent_busy = false;
                        auto_save_session(app);
                    }
                    AgentResponse::ToolCall { name, input, .. } => {
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
                        app.status.current_tool = None;
                    }
                    AgentResponse::Heartbeat { pending } => {
                        app.status.agent_busy = pending > 0;
                    }
                    AgentResponse::LlmDelta { text } => {
                        if let Some(last) = app.messages.last_mut() {
                            if let MessageItem::Assistant { text: t } = last {
                                t.push_str(&text);
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
                        } else {
                            app.messages.push(MessageItem::Reasoning {
                                text,
                                expanded: false,
                            });
                        }
                    }
                    AgentResponse::Error { message } => {
                        app.messages.push(MessageItem::System {
                            text: format!("Error: {}", message),
                        });
                        app.status.agent_busy = false;
                    }
                    AgentResponse::Shutdown => {
                        app.should_quit = true;
                    }
                    AgentResponse::AskUser { question, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("🔍 Agent asks: {}", question),
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
                            text: format!("📋 Plan: {}", title),
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
