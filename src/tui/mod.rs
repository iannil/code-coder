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
pub mod diff;
pub mod markdown;
pub mod message_list;
pub mod input_area;
pub mod dialogs;
pub mod status_bar;
pub mod theme;
#[allow(unused_imports)]
pub(crate) use status_bar::{compact_cwd, format_context_bar};
pub mod app;
pub mod commands;
pub use app::*;
pub use commands::*;
pub use theme::Theme;

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
    let session_store_for_thread = crate::session::SessionStore::open(
        &std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    // ADR 0004: spawn background save thread. The TUI sends snapshots via
    // save_tx; the thread serializes + writes off the main loop.
    let save_tx = commands::spawn_save_thread(session_store_for_thread);
    app.session_store = Some(session_store);
    app.current_session_id = None;
    app.config_store = Some(config_store);
    app.mcp_registry = Some(mcp_registry);
    app.save_tx = Some(save_tx);

    // 帧计数器（用于 spinner 动画）
    let mut frame_count: u64 = 0;

    // 主事件循环
    while !app.should_quit {
        frame_count = frame_count.wrapping_add(1);

        // Enforce message bounds at frame start. Cheap fast-path when small;
        // when over budget, evicts tool outputs → reasoning → FIFO. Also
        // covers /resume loading a previously-unbounded session.
        app::enforce_message_bounds(&mut app);

        // ADR 0004: flush any pending dirty mark to the background save
        // thread (debounced ~5s). No-op when nothing's dirty or the
        // window hasn't elapsed.
        commands::flush_pending_save(&mut app);

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

        // 事件循环：先 drain 已积压的事件（非阻塞），再阻塞等待下一个事件
        // 或 16ms 超时（≈60fps 用于 spinner / 状态栏计时器刷新）。
        // 比 poll(0)+sleep 的忙等节省 CPU，事件密集时也不丢。
        while event::poll(Duration::from_millis(0))? {
            process_event(&mut app, event::read()?, &cmd_tx);
        }
        if event::poll(Duration::from_millis(16))? {
            process_event(&mut app, event::read()?, &cmd_tx);
        }
    }

    // 发送 shutdown
    let _ = cmd_tx.send(AgentCommand::Shutdown);

    // ADR 0004: flush final dirty mark + drop sender so the background
    // save thread drains and exits. The thread is detached; we trust the
    // OS to keep it alive long enough to finish. If a sync save happens
    // (no thread wired), errors surface via the message list.
    commands::flush_on_exit(&mut app);

    // Capture any save error that surfaced into the message list (legacy
    // sync path only — background path logs to codecoder.log directly).
    let exit_save_error: Option<String> = if let Some(ref err) = app.last_save_error {
        Some(format!("session save failed: {err}"))
    } else {
        None
    };

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

    // Raw mode 已退出，stderr 可写
    if let Some(msg) = exit_save_error {
        eprintln!("[codecoder] {msg}");
    }
    Ok(())
}

/// ─── Render Dispatch ───────────────────────────────────────────────────────
///
/// 三段布局 + dispatch 到各组件 render。
/// 覆盖层（对话框/弹出列表）在消息区和输入区之上渲染。

fn render(frame: &mut Frame, app: &mut TuiApp, frame_count: u64) {
    let area = frame.area();

    // V2: input area height is dynamic — grows with content, capped at
    // half the terminal height. compute_input_height is pure (no I/O).
    let input_height = crate::tui::input_area::compute_input_height(
        &app.input,
        area.height,
        area.width,
    );

    let [msg_area, input_area_rect, status_area] = ratatui::layout::Layout::new(
        ratatui::layout::Direction::Vertical,
        [
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(input_height),
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
    status_bar::render(frame, status_area, &app.status, frame_count, &app.theme);
}

/// ─── Key Dispatch ──────────────────────────────────────────────────────────
///
/// 先检查覆盖层，再检查系统快捷键，最后 fallthrough 到输入区。

fn handle_key(
    app: &mut TuiApp,
    key: crossterm::event::KeyEvent,
    cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
) {
    // ADR 0001 §1 — Global keys (active even with overlays):
    //   Ctrl+Q              → quit (the only exit shortcut)
    //   Ctrl+C, agent busy  → send Interrupt (Phase B: real mid-call cancel)
    //   Ctrl+C, agent idle  → quit (same as Ctrl+Q)
    // Esc is NOT here — it never quits (see §3 below).
    match key.code {
        KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
            if app.status.agent_busy {
                let _ = cmd_tx.send(AgentCommand::Interrupt);
                // ADR 0001 Phase B: the agent's handle_message checks the
                // cancel flag at each round / LLM delta / tool call, so it
                // returns promptly as "[interrupted by user]". agent_busy
                // stays true until that Text response arrives here and the
                // response handler clears it.
            } else {
                app.should_quit = true;
            }
            return;
        }
        _ => {}
    }

    // 2. Overlay routing (mutually exclusive)
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

    // 3. System shortcuts (only when no overlay is open)
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
        // ADR 0006: Ctrl+L is destructive — route through Dialog::Confirm.
        KeyCode::Char('l') if key.modifiers == KeyModifiers::CONTROL => {
            app.dialog = Some(Dialog::Confirm {
                message: "Clear all messages from this session?".into(),
                action: crate::tui::ConfirmAction::ClearMessages,
            });
            return;
        }
        KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
            // ADR 0003: swap the entire Theme struct. Every render reads
            // from app.theme.<role>, so this single assignment updates
            // all components consistently.
            app.theme = if app.theme.is_dark() {
                crate::tui::Theme::light()
            } else {
                crate::tui::Theme::dark()
            };
            markdown::set_dark_mode(app.theme.is_dark());
            message_list::invalidate_cache();
            return;
        }
        // ADR 0001: Esc cascading close — never quits. Order matters:
        // innermost mode first.
        KeyCode::Esc => {
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
            if app.completion.active {
                app.completion.active = false;
                return;
            }
            // Nothing to close: no-op. User must press Ctrl+Q to quit.
            return;
        }
        // ADR 0001 §14: vim-style g/G for top/bottom scroll. Guarded by
        // empty-input check so plain `g` in the middle of typing still
        // inserts the character — these only fire when the user is browsing.
        KeyCode::Char('g') if key.modifiers == KeyModifiers::NONE && app.input.is_empty() => {
            app.auto_scroll = false;
            app.scroll_offset = 0;
            return;
        }
        KeyCode::Char('G')
            if (key.modifiers == KeyModifiers::SHIFT
                || key.modifiers == KeyModifiers::NONE)
                && app.input.is_empty() =>
        {
            app.auto_scroll = true;
            app.scroll_offset = 0;
            return;
        }
        KeyCode::PageUp => {
            app.auto_scroll = false;
            app.scroll_offset = app.scroll_offset.saturating_add(PAGE_SCROLL_LINES);
            return;
        }
        KeyCode::PageDown => {
            if app.scroll_offset > PAGE_SCROLL_LINES {
                app.scroll_offset = app.scroll_offset.saturating_sub(PAGE_SCROLL_LINES);
            } else {
                app.scroll_offset = 0;
                app.auto_scroll = true;
            }
            return;
        }
        // ADR 0001: Home/End are cursor-to-line-edge keys, NOT message-list
        // scroll. Scrolling the list to top/bottom is g/G (with PageUp/PageDown
        // for paging). We deliberately do NOT bind Home/End here so they fall
        // through to input_area::handle_input_key for line-edge cursor motion.
        _ => {}
    }

    // 4. Search/reverse-search input modes
    if app.search_active || app.reverse_search_active {
        return input_area::handle_search_key(app, key);
    }

    // 5. Regular input mode
    input_area::handle_input_key(app, key, cmd_tx);
}

/// Lines scrolled per PageUp/PageDown press. ADR 0001 §11: extracted from
/// the old magic number 10 so the value is documented and adjustable.
const PAGE_SCROLL_LINES: usize = 10;

/// Dispatch a single terminal event (key / mouse / paste). Extracted so the
/// event loop can share one code path between the drain phase and the
/// blocking-wait phase.
fn process_event(app: &mut TuiApp, ev: Event, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
    match ev {
        Event::Key(key) => {
            if key.kind == KeyEventKind::Press {
                handle_key(app, key, cmd_tx);
            }
        }
        Event::Mouse(mouse) => match mouse.kind {
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
        },
        Event::Paste(text) => {
            // Bracketed paste: save snapshot then insert at cursor.
            input_area::save_undo_snapshot(app);
            app.input.insert_str(app.cursor_pos, &text);
            app.cursor_pos += text.len();
            // ADR 0002 §7: pasting "/foo" should also activate the popup.
            input_area::refresh_slash_completion(app);
        }
        _ => {}
    }
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
                            app.status.token_count = app.status.token_count
                                .saturating_add(tokens_in as usize)
                                .saturating_add(tokens_out as usize);
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
                    AgentResponse::AskUser { question, options, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("[ask] Agent asks: {}", question),
                        });
                        app.dialog = Some(Dialog::AskQuestion { question, options, selected: 0, request_id });
                    }
                    AgentResponse::PlanRequest { title, plan, request_id } => {
                        app.messages.push(MessageItem::System {
                            text: format!("[plan] Plan: {}", title),
                        });
                        app.messages.push(MessageItem::Assistant { text: plan.clone() });
                        // Default to "manually approve" (index 1) — the safe
                        // choice; auto-accept pre-grants edit tools.
                        app.dialog = Some(Dialog::PlanApproval { title, plan, selected: 1, request_id });
                    }
                    AgentResponse::PermissionRequest { tool_name, tool_input, request_id, risk } => {
                        app.dialog = Some(Dialog::ToolPermission {
                            tool_name,
                            tool_input,
                            request_id,
                            risk,
                        });
                    }
                    AgentResponse::PersistPermission { tool_name } => {
                        // ADR 0005 Phase B: append to codecoder.json's
                        // permissions.allowlist. Dedup so re-granting the
                        // same tool doesn't bloat the file. Errors are
                        // surfaced to the message list once per error
                        // string (same pattern as session auto-save).
                        if let Some(ref mut store) = app.config_store {
                            let allowlist = &mut store.get_mut().permissions.allowlist;
                            if !allowlist.iter().any(|t| t == &tool_name) {
                                allowlist.push(tool_name.clone());
                                match store.save() {
                                    Ok(()) => {
                                        app.messages.push(MessageItem::System {
                                            text: format!("✓ '{}' added to project allowlist (codecoder.json)", tool_name),
                                        });
                                    }
                                    Err(e) => {
                                        let err = format!("Failed to persist permission: {e}");
                                        crate::log(&format!("[error] {err}"));
                                        app.messages.push(MessageItem::System { text: err });
                                    }
                                }
                            }
                        } else {
                            app.messages.push(MessageItem::System {
                                text: format!(
                                    "⚠ '{}' granted for project but no config store wired — grant will not persist.",
                                    tool_name
                                ),
                            });
                        }
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

// ─── ADR 0001 Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod adr0001_tests {
    use super::*;
    use crate::agent::AgentCommand;
    use crate::tui::TuiApp;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, modifiers)
    }

    fn press(app: &mut TuiApp, code: KeyCode, modifiers: KeyModifiers, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
        handle_key(app, key(code, modifiers), cmd_tx);
    }

    #[test]
    fn ctrl_q_quits() {
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('q'), KeyModifiers::CONTROL, &tx);
        assert!(app.should_quit, "Ctrl+Q must quit");
    }

    #[test]
    fn ctrl_c_when_idle_quits() {
        let mut app = TuiApp::default();
        app.status.agent_busy = false;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL, &tx);
        assert!(app.should_quit, "Ctrl+C idle must quit (same as Ctrl+Q)");
    }

    #[test]
    fn ctrl_c_when_busy_sends_interrupt_and_does_not_quit() {
        let mut app = TuiApp::default();
        app.status.agent_busy = true;
        let (tx, rx) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL, &tx);
        assert!(!app.should_quit, "Ctrl+C busy must NOT quit");
        let cmd = rx.try_recv().expect("should send an AgentCommand");
        match cmd {
            AgentCommand::Interrupt => {}
            other => panic!("expected Interrupt, got {other:?}"),
        }
    }

    #[test]
    fn ctrl_d_no_longer_quits() {
        // ADR 0001: only Ctrl+Q quits. Ctrl+D is no longer a quit shortcut.
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL, &tx);
        assert!(!app.should_quit, "Ctrl+D must not quit");
    }

    #[test]
    fn esc_with_no_overlays_is_noop() {
        // Esc with nothing to close: no-op (no quit, no state change).
        let mut app = TuiApp::default();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.should_quit, "Esc with nothing open must not quit");
    }

    #[test]
    fn esc_closes_search_then_is_noop() {
        // First Esc closes search; second Esc (nothing open) is no-op.
        let mut app = TuiApp::default();
        app.search_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.search_active, "first Esc should close search");
        assert!(!app.should_quit, "closing search must not quit");
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.should_quit, "second Esc with nothing open must be no-op");
    }

    #[test]
    fn esc_cascade_priority_reverse_search_first() {
        // Both reverse_search and search active: Esc closes reverse first.
        let mut app = TuiApp::default();
        app.reverse_search_active = true;
        app.search_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.reverse_search_active, "Esc should close reverse_search first");
        assert!(app.search_active, "search should remain active");
    }

    #[test]
    fn g_on_empty_input_scrolls_to_top() {
        // ADR 0001: g (lowercase) on empty input = scroll msg list to top.
        let mut app = TuiApp::default();
        app.input = String::new();
        app.auto_scroll = true;
        app.scroll_offset = 50;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('g'), KeyModifiers::NONE, &tx);
        assert!(!app.auto_scroll, "g should disable auto_scroll");
        assert_eq!(app.scroll_offset, 0, "g should reset scroll_offset to 0 (top)");
    }

    #[test]
    fn g_with_text_in_input_inserts_g() {
        // ADR 0001 guard: g only acts as scroll shortcut when input is empty.
        // With text in input, g falls through to input handler and inserts.
        let mut app = TuiApp::default();
        app.input = "abc".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('g'), KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "abcg", "g should insert as text when input non-empty");
    }

    #[test]
    fn shift_g_scrolls_to_bottom() {
        // G (Shift+G) jumps to bottom. Like lowercase g, requires empty
        // input — Shift+G in the middle of typing would otherwise surprise
        // the user by jumping the message list.
        let mut app = TuiApp::default();
        app.input = String::new();
        app.auto_scroll = false;
        app.scroll_offset = 100;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT, &tx);
        assert!(app.auto_scroll, "G should re-enable auto_scroll");
        assert_eq!(app.scroll_offset, 0, "G should reset offset (auto-scroll takes over)");
    }

    #[test]
    fn esc_closes_completion_at_top_level() {
        // ADR 0001: Esc cascade (including completion close) lives in
        // handle_key at mod.rs level, not in handle_input_key. Verify the
        // cascade closes completion without quitting.
        let mut app = TuiApp::default();
        app.completion.active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE, &tx);
        assert!(!app.completion.active, "Esc should close completion via cascade");
        assert!(!app.should_quit, "Esc must not quit");
    }

    // ── ADR 0003 — Theme switching ───────────────────────────────────────

    #[test]
    fn adr0003_default_theme_is_dark() {
        let app = TuiApp::default();
        assert!(app.theme.is_dark(), "TuiApp default theme must be dark");
    }

    #[test]
    fn adr0003_ctrl_t_swaps_theme() {
        // Ctrl+T must swap the entire theme struct — every render reads
        // from app.theme.<role>, so this single toggle affects all
        // components consistently.
        let mut app = TuiApp::default();
        let original = app.theme;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, KeyCode::Char('t'), KeyModifiers::CONTROL, &tx);
        assert_ne!(app.theme, original, "Ctrl+T must change the theme");
        assert!(!app.theme.is_dark(), "after one toggle from dark, theme should be light");
        // Toggle back.
        press(&mut app, KeyCode::Char('t'), KeyModifiers::CONTROL, &tx);
        assert_eq!(app.theme, original, "second Ctrl+T returns to original");
    }
}
