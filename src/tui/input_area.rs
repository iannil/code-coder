/// ─── Input Area Component ────────────────────────────────────────────────────
///
/// 输入框渲染、光标管理、undo/redo、历史导航、搜索/反向搜索按键处理。

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Instant;

use super::completion;
use super::{Dialog, MessageItem, TuiApp};
use crate::agent::AgentCommand;

/// ─── Undo Snapshot ──────────────────────────────────────────────────────────

/// Save the current input state to the undo stack (for Ctrl+Z)
pub fn save_undo_snapshot(app: &mut TuiApp) {
    if let Some(last) = app.undo_stack.last() {
        if last == &app.input {
            return;
        }
    }
    app.undo_stack.push(app.input.clone());
    if app.undo_stack.len() > 100 {
        app.undo_stack.remove(0);
    }
    app.redo_stack.clear();
}

/// ─── Render ─────────────────────────────────────────────────────────────────

/// Render the input area (separator line + input line + cursor).
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp, frame_count: u64) {
    let _ = frame_count; // reserved for future cursor blink animation

    // Separator line
    let separator_line = Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(1) as usize),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(
        Paragraph::new(separator_line),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // Input content on line 2
    let input_content_y = area.y + 1;
    let cursor_pos = app.cursor_pos.min(app.input.len());
    let prefix_span = Span::styled("> ", Style::default().fg(Color::Cyan));
    let input_display = if app.input.is_empty() {
        Line::from(prefix_span)
    } else {
        let mut line = Line::from(vec![prefix_span]);
        line.spans.push(Span::raw(&app.input));
        line
    };
    let input_content_area = Rect::new(area.x, input_content_y, area.width, 1);
    let input_paragraph = Paragraph::new(input_display)
        .style(Style::default().fg(Color::White));
    frame.render_widget(input_paragraph, input_content_area);

    // Cursor position
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
        x: area.x + col_offset as u16 + 2,
        y: input_content_y + row_offset as u16,
    });
}

/// ─── Input Key Handling ──────────────────────────────────────────────────────

/// Handle keys when in regular input mode (no overlay active, no search mode).
pub fn handle_input_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
    use crossterm::event::{KeyCode, KeyModifiers};

    match key.code {
        // ── Ctrl combos for editing ──────────────────────────────────────
        KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = 0;
        }
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = app.input.len();
        }
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                let before = &app.input[..app.cursor_pos];
                let trimmed = before.trim_end();
                let word_start = trimmed
                    .rfind(|c: char| c.is_whitespace())
                    .map(|p| p + 1)
                    .unwrap_or(0);
                app.input.drain(word_start..app.cursor_pos);
                app.cursor_pos = word_start;
            }
        }
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                app.input.drain(..app.cursor_pos);
                app.cursor_pos = 0;
            }
        }
        KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos < app.input.len() {
                save_undo_snapshot(app);
                app.input.truncate(app.cursor_pos);
            }
        }
        // Ctrl+Z — undo
        KeyCode::Char('z') if key.modifiers == KeyModifiers::CONTROL => {
            if let Some(prev) = app.undo_stack.pop() {
                app.redo_stack.push(app.input.clone());
                if app.redo_stack.len() > 100 {
                    app.redo_stack.remove(0);
                }
                app.input = prev;
                app.cursor_pos = app.input.len();
            }
        }
        // Ctrl+Y — redo
        KeyCode::Char('y') if key.modifiers == KeyModifiers::CONTROL => {
            if let Some(next) = app.redo_stack.pop() {
                save_undo_snapshot(app);
                app.input = next;
                app.cursor_pos = app.input.len();
            }
        }

        // ── Enter ────────────────────────────────────────────────────────
        KeyCode::Enter
            if key.modifiers == KeyModifiers::ALT
                || key.modifiers == KeyModifiers::CONTROL =>
        {
            send_message(app, cmd_tx);
        }
        KeyCode::Enter => {
            if app.model_picker_active {
                return; // handled by dialogs
            }
            if app.slash_completion.active {
                return; // handled by dialogs
            }
            if let Some(ref dialog) = app.dialog {
                if let Dialog::AskQuestion { question: _, request_id } = dialog {
                    if !app.input.is_empty() {
                        let answer = app.input.trim().to_string();
                        let rid = *request_id;
                        app.dialog = None;
                        app.messages.push(MessageItem::User { text: answer.clone() });
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse {
                            request_id: rid,
                            answer,
                        });
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.auto_scroll = true;
                        app.scroll_offset = 0;
                        return;
                    }
                }
            }
            if app.input.is_empty() {
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
                if let Some(MessageItem::Reasoning { expanded, .. }) = app.messages.last_mut() {
                    *expanded = !*expanded;
                    return;
                }
                if let Some(MessageItem::ToolCall { expanded: true, show_full, .. }) = app.messages.last_mut() {
                    *show_full = !*show_full;
                    return;
                }
                app.auto_scroll = true;
                app.scroll_offset = 0;
                return;
            }
            if key.modifiers == KeyModifiers::SHIFT {
                app.input.push('\n');
                app.cursor_pos = app.input.len();
            } else {
                send_message(app, cmd_tx);
            }
        }

        // ── Cursor navigation ────────────────────────────────────────────
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                let mut new_pos = app.cursor_pos.saturating_sub(1);
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                let mut new_pos = app.cursor_pos.saturating_add(1);
                while new_pos < app.input.len() && !app.input.is_char_boundary(new_pos) {
                    new_pos += 1;
                }
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Up => {
            if !app.input_history.is_empty() && app.history_pos > 0 {
                app.history_pos -= 1;
                app.input = app.input_history[app.history_pos].clone();
                app.cursor_pos = app.input.len();
            } else if app.input.is_empty() && !app.messages.is_empty() {
                let max_idx = app.messages.len().saturating_sub(1);
                app.selected_msg = Some(match app.selected_msg {
                    Some(i) if i > 0 => i - 1,
                    _ => max_idx,
                });
            }
        }
        KeyCode::Down => {
            if app.history_pos < app.input_history.len() {
                app.history_pos += 1;
                if app.history_pos == app.input_history.len() {
                    app.input.clear();
                } else {
                    app.input = app.input_history[app.history_pos].clone();
                }
                app.cursor_pos = app.input.len();
            } else if app.input.is_empty() && app.selected_msg.is_some() {
                let max_idx = app.messages.len().saturating_sub(1);
                app.selected_msg = Some(match app.selected_msg {
                    Some(i) if i < max_idx => i + 1,
                    _ => 0,
                });
            } else if app.input.is_empty() {
                app.selected_msg = Some(0);
            }
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
        }

        // ── Character input ──────────────────────────────────────────────
        KeyCode::Char(c) => {
            save_undo_snapshot(app);
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += c.len_utf8();

            // Cancel slash completion if typing non-slash characters
            if app.slash_completion.active && !app.input.starts_with('/') {
                app.slash_completion.active = false;
            }
            // Detect / at start → slash command menu
            if c == '/' && app.cursor_pos == 1 {
                app.slash_completion.active = true;
                app.slash_completion.selected = 0;
            } else if app.slash_completion.active {
                app.slash_completion.selected = 0;
            }
            // Detect @ → file completion
            if c == '@' {
                app.completion.active = true;
                app.completion.query = String::new();
                app.completion.at_pos = app.cursor_pos - 1;
                app.completion.selected = 0;
                app.completion.candidates = completion::search_files(".", "", 10);
            } else if app.completion.active {
                let after_at = &app.input[app.completion.at_pos + 1..];
                app.completion.query = after_at.to_string();
                app.completion.selected = 0;
                app.completion.candidates = completion::search_files(".", &app.completion.query, 10);
            }
        }

        // ── Backspace ────────────────────────────────────────────────────
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                if app.slash_completion.active && app.cursor_pos == 1 {
                    app.slash_completion.active = false;
                }
                if app.completion.active && app.cursor_pos - 1 == app.completion.at_pos {
                    app.completion.active = false;
                }
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
                    app.completion.candidates = completion::search_files(".", &app.completion.query, 10);
                }
            }
        }

        // ── Delete ───────────────────────────────────────────────────────
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                save_undo_snapshot(app);
                app.input.remove(app.cursor_pos);
            }
        }

        // ── Tab ──────────────────────────────────────────────────────────
        KeyCode::Tab if app.completion.active => {
            if let Some(candidate) = app.completion.candidates.get(app.completion.selected) {
                let at_pos = app.completion.at_pos;
                let end_of_word = app.cursor_pos;
                app.input.replace_range(at_pos..end_of_word, &candidate.display);
                app.cursor_pos = at_pos + candidate.display.len();
            }
            app.completion.active = false;
        }
        KeyCode::Tab => {
            if app.slash_completion.active {
                app.slash_completion.selected = (app.slash_completion.selected + 1)
                    % app.slash_completion.commands.len();
                return;
            }
            if app.completion.active {
                app.completion.selected = (app.completion.selected + 1)
                    % app.completion.candidates.len().max(1);
            } else {
                app.input.insert_str(app.cursor_pos, "  ");
                app.cursor_pos += 2;
            }
        }

        // ── Esc in input mode ────────────────────────────────────────────
        KeyCode::Esc => {
            if app.selected_msg.is_some() {
                app.selected_msg = None;
                return;
            }
            app.completion.active = false;
            app.should_quit = true;
        }

        _ => {}
    }
}

/// Handle keys when in search or reverse-search mode.
pub fn handle_search_key(app: &mut TuiApp, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Char(c) => {
            if app.reverse_search_active {
                app.reverse_search_query.push(c);
                crate::tui::message_list::update_reverse_search(app);
            } else if app.search_active {
                app.search_query.push(c);
                crate::tui::message_list::count_search_matches(app);
            }
        }
        KeyCode::Backspace => {
            if app.reverse_search_active {
                app.reverse_search_query.pop();
                crate::tui::message_list::update_reverse_search(app);
            } else if app.search_active {
                app.search_query.pop();
                crate::tui::message_list::count_search_matches(app);
            }
        }
        KeyCode::Esc | KeyCode::Enter => {
            // Esc/Enter exit search mode (handled by mod.rs routing, but
            // here we guard against stray keys reaching here)
        }
        _ => {}
    }
}

/// ─── Send Message ───────────────────────────────────────────────────────────
/// (kept here because it's closely tied to the input area)

pub fn send_message(app: &mut TuiApp, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
    let input = app.input.trim().to_string();
    if input.is_empty() {
        return;
    }

    // Local slash commands (handled elsewhere)
    if input.starts_with('/') {
        let lower = input.to_lowercase();
        if input == "/session" || input.starts_with("/session ")
            || input == "/resume" || input.starts_with("/resume ")
            || lower == "/config" || lower.starts_with("/config ")
            || lower == "/mcp" || lower.starts_with("/mcp ")
        {
            // These are handled by dedicated functions in commands.rs;
            // dispatch through main loop to avoid double-handling.
            // For now, pass through to agent.
        }
    }

    app.input_history.push(input.clone());
    app.history_pos = app.input_history.len();
    app.messages.retain(|m| !matches!(m, MessageItem::System { text } if text.starts_with("[end]")));
    app.messages.push(MessageItem::User { text: input.clone() });
    super::commands::auto_save_session(app);
    app.auto_scroll = true;
    app.scroll_offset = 0;

    if let Err(e) = cmd_tx.send(AgentCommand::ProcessMessage { text: input }) {
        app.messages.push(MessageItem::System {
            text: format!("[disconnect] Agent 通道已断开 — 线程可能已崩溃。stderr 可能有更多信息。错误: {e}"),
        });
        app.status.agent_busy = false;
        return;
    }
    crate::log(&format!("[codecoder] TUI 发送消息成功"));

    app.thinking_start_time = Some(Instant::now());
    app.status.agent_busy = true;
    app.current_round = 0;
    app.status.current_tool = None;
    app.status.streaming_complete = false;
    app.messages.push(MessageItem::System { text: "[send] Agent…".into() });

    app.input.clear();
    app.cursor_pos = 0;
}

/// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentCommand;
    use crate::tui::TuiApp;

    fn key(code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, modifiers)
    }

    fn press(app: &mut TuiApp, code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
        handle_input_key(app, key(code, modifiers), cmd_tx);
    }

    // ── save_undo_snapshot ─────────────────────────────────────────────────

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
        save_undo_snapshot(&mut app);
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
        assert!(app.redo_stack.is_empty());
    }

    // ── Input editing ─────────────────────────────────────────────────────

    #[test]
    fn test_key_ctrl_a_goes_to_start() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_key_ctrl_e_goes_to_end() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('e'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_key_ctrl_w_deletes_word_backward() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 11;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('w'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello ");
        assert_eq!(app.cursor_pos, 6);
    }

    #[test]
    fn test_key_ctrl_u_deletes_to_start() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('u'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, " world");
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_key_ctrl_k_deletes_to_end() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('k'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello");
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_key_delete_removes_char() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Delete, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "ello");
    }

    #[test]
    fn test_key_backspace_removes_char() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 5;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Backspace, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "hell");
        assert_eq!(app.cursor_pos, 4);
    }

    #[test]
    fn test_key_left_moves_cursor() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Left, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_key_right_moves_cursor() {
        let mut app = TuiApp::default();
        app.input = "hello".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Right, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 4);
    }

    #[test]
    fn test_key_ctrl_z_undo_input() {
        let mut app = TuiApp::default();
        app.input = "second".into();
        app.undo_stack.push("first".into());
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('z'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "first");
        assert_eq!(app.redo_stack.len(), 1);
    }

    #[test]
    fn test_key_ctrl_y_redo_input() {
        let mut app = TuiApp::default();
        app.input = "first".into();
        app.redo_stack.push("second".into());
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "second");
    }
}
