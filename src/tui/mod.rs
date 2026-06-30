/// ─── TUI Module ───────────────────────────────────────────────────────────
///
/// ratatui + crossterm 全屏终端界面。
/// 三段布局：消息列表（上） + 输入框（中） + 状态栏（底）
///
/// 组件拆分：
///   message_list — 消息渲染、虚拟滚动、搜索、缓存
///   input_area   — 输入框、光标、undo/redo、历史
///   dialogs      — 权限/计划/提问对话框、帮助、模型切换器
///   status_bar   — 底部状态栏
///   commands     — 斜杠命令处理、会话持久化

pub mod completion;
pub mod markdown;
pub mod message_list;
pub mod input_area;
pub mod dialogs;
pub mod status_bar;
#[allow(unused_imports)]
pub(crate) use status_bar::{compact_cwd, format_context_bar};
pub mod app;
pub mod commands;
pub use app::*;
pub use commands::*;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::layout::Flex;
use ratatui::Frame;
use std::time::Duration;

use crate::agent::{AgentCommand, AgentResponse};
use crate::event::SharedEventBus;
use std::sync::{Arc, Mutex};


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

    // 从 API 获取可用模型列表（失败则回退到硬编码列表）
    let cfg = config_store.get();
    let api_key = std::env::var("CODECODER_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .unwrap_or_default();
    let models = commands::fetch_available_models(&cfg.llm.api_base, &api_key);
    if !models.is_empty() {
        app.available_models = models;
    }

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

        // Panic-injection test: run with CODECODER_PANIC_TEST=1 to verify the
        // panic hook restores the terminal. Remove or gate behind a test cfg
        // once verified. Keep best-effort — must not interfere with normal runs.
        if std::env::var("CODECODER_PANIC_TEST").is_ok() {
            panic!("injected for terminal-restore test");
        }

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

        // 非阻塞事件轮询 + 16ms sleep 保底以实现 ≈60fps
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(&mut app, key, &cmd_tx);
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.auto_scroll = false;
                            app.scroll_offset = app.scroll_offset.saturating_add(3);
                        }
                        MouseEventKind::ScrollDown => {
                            if app.scroll_offset > 3 {
                                app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            } else {
                                app.scroll_offset = 0;
                                app.auto_scroll = true;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Paste(text) => {
                    // Bracketed paste: save snapshot then insert
                    input_area::save_undo_snapshot(&mut app);
                    app.input.insert_str(app.cursor_pos, &text);
                    app.cursor_pos += text.len();
                }
                _ => {}
            }
        }

        // 保底 sleep 实现 ≈60fps
        std::thread::sleep(Duration::from_millis(16));
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

/// ─── Render Dispatch ───────────────────────────────────────────────────────
///
/// 三段布局 + dispatch 到各组件 render。
/// 覆盖层（对话框/弹出列表）在消息区和输入区之上渲染。

fn render(frame: &mut Frame, app: &mut TuiApp, frame_count: u64) {
    let area = frame.area();

    // 三段 Flex 布局：消息区（flex_grow=1）+ 输入区（2行）+ 状态栏（1行）
    let [msg_area, input_area_rect, status_area] = ratatui::layout::Layout::new(
        ratatui::layout::Direction::Vertical,
        [
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(2),
            ratatui::layout::Constraint::Length(1),
        ],
    )
    .flex(Flex::Start)
    .areas(area);

    // 消息列表
    message_list::render(frame, msg_area, app, frame_count);

    // 输入区
    input_area::render(frame, input_area_rect, app, frame_count);

    // 覆盖层（对话框、弹出列表等）
    dialogs::render_overlays(frame, area, input_area_rect, app);

    // 状态栏
    status_bar::render(frame, status_area, &app.status, frame_count);
}

/// ─── Key Dispatch ──────────────────────────────────────────────────────────
///
/// 先检查覆盖层，再检查系统快捷键，最后 fallthrough 到输入区。

fn handle_key(
    app: &mut TuiApp,
    key: crossterm::event::KeyEvent,
    cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
) {
    // 1. 全局快捷键（覆盖层中也生效）
    match key.code {
        KeyCode::Char('q' | 'c' | 'd') if key.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true;
            return;
        }
        _ => {}
    }

    // 2. 覆盖层路由
    if app.dialog.is_some() {
        return dialogs::handle_dialog_key(app, key, cmd_tx);
    }
    if app.help_active {
        return dialogs::handle_help_key(app, key);
    }
    if app.model_picker_active {
        return dialogs::handle_model_picker_key(app, key, cmd_tx);
    }
    if app.slash_completion.active {
        return dialogs::handle_slash_completion_key(app, key, cmd_tx);
    }

    // 3. 系统快捷键（不在覆盖层中时）
    match key.code {
        KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
            app.search_active = !app.search_active;
            if !app.search_active {
                app.search_query.clear();
            }
            return;
        }
        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
            app.reverse_search_active = !app.reverse_search_active;
            if app.reverse_search_active {
                app.reverse_search_query.clear();
                app.reverse_search_results.clear();
                app.reverse_search_idx = 0;
            }
            return;
        }
        KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
            app.model_picker_active = !app.model_picker_active;
            app.model_picker_selected = app
                .available_models
                .iter()
                .position(|m| m == &app.status.model)
                .unwrap_or(0);
            return;
        }
        KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
            app.help_active = !app.help_active;
            return;
        }
        KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
            app.messages.clear();
            message_list::invalidate_cache();
            app.scroll_offset = 0;
            app.auto_scroll = true;
            return;
        }
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            app.dark_mode = !app.dark_mode;
            markdown::set_dark_mode(app.dark_mode);
            message_list::invalidate_cache();
            return;
        }
        KeyCode::Esc => {
            // Cascading close
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
            return;
        }
        KeyCode::PageUp => {
            app.auto_scroll = false;
            app.scroll_offset = app.scroll_offset.saturating_add(10);
            return;
        }
        KeyCode::PageDown => {
            if app.scroll_offset > 10 {
                app.scroll_offset = app.scroll_offset.saturating_sub(10);
            } else {
                app.scroll_offset = 0;
                app.auto_scroll = true;
            }
            return;
        }
        KeyCode::End => {
            app.scroll_offset = 0;
            app.auto_scroll = true;
            return;
        }
        KeyCode::Home => {
            app.auto_scroll = false;
            app.scroll_offset = 0;
            return;
        }
        _ => {}
    }

    // 4. 搜索/反向搜索模式
    if app.search_active || app.reverse_search_active {
        return input_area::handle_search_key(app, key);
    }

    // 5. 常规输入模式
    input_area::handle_input_key(app, key, cmd_tx);
}

/// ─── Agent Response Polling ───────────────────────────────────────────────

fn check_agent_responses(app: &mut TuiApp, resp_rx: &mut tokio::sync::mpsc::Receiver<AgentResponse>) {
    loop {
        match resp_rx.try_recv() {
            Ok(response) => {
                match response {
                    AgentResponse::Text { text, tokens_in, tokens_out, .. } => {
                        let took = app.thinking_start_time.take()
                            .map(|t| t.elapsed().as_secs_f32())
                            .unwrap_or(0.0);
                        if tokens_in > 0 || tokens_out > 0 {
                            app.status.token_count = (app.status.token_count as u32).saturating_add(tokens_in).saturating_add(tokens_out) as usize;
                        }
                        app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.starts_with("[write]") || text.starts_with("[think]")));
                        if !text.is_empty() && !app.status.streaming_complete {
                            app.messages.push(MessageItem::Assistant { text });
                        }
                        app.messages.push(MessageItem::System {
                            text: format!("[end] ({took:.1}s)"),
                        });
                        // 不移除 auto_scroll：如果用户在 agent 回复期间没有手动滚动，
                        // auto_scroll 应保持 true，继续自动跟随底部。
                        // 如果用户曾向上滚动，auto_scroll 已经是 false，应尊重用户选择。
                        app.status.agent_busy = false;
                        app.status.streaming_complete = false;
                        app.status.current_tool = None;
                        app.status.current_round = 0;
                        commands::auto_save_session(app);
                    }
                    AgentResponse::ToolCall { name, input, .. } => {
                        app.current_round += 1;
                        app.status.current_round = app.current_round;
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
                        message_list::invalidate_cache();
                        app.status.current_tool = None;
                    }
                    AgentResponse::StreamComplete => {
                        app.status.streaming_complete = true;
                    }
                    AgentResponse::Heartbeat { pending } => {
                        app.status.agent_busy = pending > 0;
                        app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text == "[send] Agent…" || text == "[send] Agent 处理中…"));
                    }
                    AgentResponse::LlmDelta { text } => {
                        if !app.status.agent_busy {
                            continue;
                        }
                        if app.current_round == 1 && app.messages.last().map_or(true, |m| !matches!(m, MessageItem::Assistant { .. })) {
                            app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.contains("LLM")));
                            app.messages.push(MessageItem::System {
                                text: "[write] LLM…".into(),
                            });
                        }
                        if let Some(last) = app.messages.last_mut() {
                            if let MessageItem::Assistant { text: t } = last {
                                t.push_str(&text);
                                message_list::invalidate_cache();
                            } else {
                                app.messages.push(MessageItem::Assistant { text });
                            }
                        } else {
                            app.messages.push(MessageItem::Assistant { text });
                        }
                    }
                    AgentResponse::ReasoningDelta { text } => {
                        if !app.status.agent_busy {
                            continue;
                        }
                        if let Some(MessageItem::Reasoning { text: t, .. }) = app.messages.last_mut() {
                            t.push_str(&text);
                            message_list::invalidate_cache();
                        } else {
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
                        app.status.streaming_complete = false;
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
                        app.dialog = Some(Dialog::AskQuestion { question, request_id });
                    }
                    AgentResponse::PlanRequest { title, plan, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("[plan] Plan: {}", title),
                        });
                        app.messages.push(MessageItem::Assistant { text: plan.clone() });
                        app.dialog = Some(Dialog::PlanApproval { title, plan, request_id });
                    }
                    AgentResponse::PermissionRequest { tool_name, tool_input, request_id, risk } => {
                        app.dialog = Some(Dialog::ToolPermission {
                            tool_name,
                            tool_input,
                            request_id,
                            risk,
                        });
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}
