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
use super::{MessageItem, TuiApp};
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

    // ADR 0003: read colors from app.theme.
    let separator_line = Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(1) as usize),
        Style::default().fg(app.theme.secondary_text),
    ));
    frame.render_widget(
        Paragraph::new(separator_line),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // Input content on line 2
    let input_content_y = area.y + 1;
    let cursor_pos = app.cursor_pos.min(app.input.len());
    let prefix_span = Span::styled("> ", Style::default().fg(app.theme.accent_text));
    let input_display = if app.input.is_empty() {
        Line::from(prefix_span)
    } else {
        let mut line = Line::from(vec![prefix_span]);
        line.spans.push(Span::raw(&app.input));
        line
    };
    let input_content_area = Rect::new(area.x, input_content_y, area.width, 1);
    let input_paragraph = Paragraph::new(input_display)
        .style(Style::default().fg(app.theme.primary_text));
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
        // ADR 0001: Enter always means "submit the current submittable thing."
        // - Alt/Ctrl+Enter: force submit even when modifiers would otherwise
        //   be interpreted differently.
        // - Shift+Enter: insert a literal newline (multi-line input).
        // - Plain Enter on AskQuestion dialog: handled here as a submission
        //   of the typed answer (mirrors dialog handler; kept here so the
        //   input-area path doesn't drop the input silently).
        // - Plain Enter otherwise: submit. Empty input is a no-op (does NOT
        //   fold reasoning, accept completion, or scroll — those moved to Tab
        //   per ADR 0001).
        KeyCode::Enter
            if key.modifiers == KeyModifiers::ALT
                || key.modifiers == KeyModifiers::CONTROL =>
        {
            send_message(app, cmd_tx);
        }
        KeyCode::Enter => {
            if app.model_picker_active || app.slash_completion.active {
                return; // handled by dialogs
            }
            // H2 fix: Enter accepts the highlighted @ file-completion candidate
            // (mirrors the original) instead of submitting a half-typed @query.
            if app.completion.active && !app.completion.candidates.is_empty() {
                accept_file_completion(app);
                return;
            }
            // When any dialog is open, key routing in mod.rs sends Enter to the
            // dialog handler — this input-area path is never reached with a
            // dialog active. The guard stays as defense-in-depth.
            if app.dialog.is_some() {
                return;
            }
            if app.input.is_empty() {
                return; // ADR 0001: no-op. No more fold/accept/scroll magic.
            }
            if key.modifiers == KeyModifiers::SHIFT {
                app.input.push('\n');
                app.cursor_pos = app.input.len();
            } else {
                send_message(app, cmd_tx);
            }
        }

        // ── Cursor navigation (multi-line aware per ADR 0001) ───────────
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
        // ADR 0001: Up/Down move cursor within multi-line input. When input
        // is empty (no cursor movement possible), Up/Down enter browse mode
        // and walk the message list. History is on Ctrl+Up/Ctrl+Down below.
        KeyCode::Up if key.modifiers == KeyModifiers::CONTROL => {
            navigate_history(app, Direction::Prev);
        }
        KeyCode::Down if key.modifiers == KeyModifiers::CONTROL => {
            navigate_history(app, Direction::Next);
        }
        // H2 fix: while the @ file-completion popup is open, Up/Down move the
        // popup selection instead of the cursor — otherwise `completion.selected`
        // is stuck at 0 and Tab/Enter can only ever pick the first candidate.
        KeyCode::Up if app.completion.active && !app.completion.candidates.is_empty() => {
            if app.completion.selected > 0 {
                app.completion.selected -= 1;
            }
        }
        KeyCode::Down if app.completion.active && !app.completion.candidates.is_empty() => {
            let max = app.completion.candidates.len().saturating_sub(1);
            if app.completion.selected < max {
                app.completion.selected += 1;
            }
        }
        KeyCode::Up => {
            if try_move_cursor_vertical(app, -1) {
                // cursor moved within input — done
            } else if app.input.is_empty() && !app.messages.is_empty() {
                let max_idx = app.messages.len().saturating_sub(1);
                app.selected_msg = Some(match app.selected_msg {
                    Some(i) if i > 0 => i - 1,
                    _ => max_idx,
                });
            }
        }
        KeyCode::Down => {
            if try_move_cursor_vertical(app, 1) {
                // cursor moved within input — done
            } else if app.input.is_empty() && app.selected_msg.is_some() {
                let max_idx = app.messages.len().saturating_sub(1);
                app.selected_msg = Some(match app.selected_msg {
                    Some(i) if i < max_idx => i + 1,
                    _ => 0,
                });
            } else if app.input.is_empty() && !app.messages.is_empty() {
                app.selected_msg = Some(0);
            }
        }
        // ADR 0001: Home/End move to the edge of the CURRENT line (readline
        // convention), not the whole buffer — matters for multi-line input.
        KeyCode::Home => {
            let before = &app.input[..app.cursor_pos];
            app.cursor_pos = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        }
        KeyCode::End => {
            let after = &app.input[app.cursor_pos..];
            app.cursor_pos = match after.find('\n') {
                Some(i) => app.cursor_pos + i,
                None => app.input.len(),
            };
        }

        // ── Character input ──────────────────────────────────────────────
        KeyCode::Char(c) => {
            save_undo_snapshot(app);
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += c.len_utf8();

            // ADR 0002 §7: slash completion is input-driven. Refresh on
            // every keystroke — activates when input starts with '/' and
            // has no whitespace, filters by prefix, deactivates otherwise.
            refresh_slash_completion(app);

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
                if app.completion.active && app.cursor_pos - 1 == app.completion.at_pos {
                    app.completion.active = false;
                }
                let mut new_pos = app.cursor_pos.saturating_sub(1);
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.input.remove(new_pos);
                app.cursor_pos = new_pos;
                // ADR 0002 §7: slash completion refresh covers both
                // activation (re-pressing '/') and deactivation.
                refresh_slash_completion(app);
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
        // ADR 0001 priority: completion accept > slash-cycle > fold/expand
        // the last collapsible message. The old "insert 2 spaces" fallback
        // is removed — Tab is exclusively for accepting/cycling/folding.
        KeyCode::Tab if app.completion.active => {
            accept_file_completion(app);
        }
        KeyCode::Tab => {
            if app.slash_completion.active {
                app.slash_completion.selected = (app.slash_completion.selected + 1)
                    % app.slash_completion.commands.len();
                return;
            }
            // No completion: fold/expand the last collapsible message.
            // Reasoning toggles `expanded`. ToolCall only toggles `show_full`
            // when already expanded (collapsed ToolCalls expand via selected_msg
            // browse, not Tab — keep Tab scoped to "more detail" semantics).
            if let Some(MessageItem::Reasoning { expanded, .. }) = app.messages.last_mut() {
                *expanded = !*expanded;
                return;
            }
            if let Some(MessageItem::ToolCall { expanded, show_full, .. }) = app.messages.last_mut() {
                if *expanded {
                    *show_full = !*show_full;
                    return;
                }
            }
            // Nothing to fold: no-op (ADR 0001 reserves Tab for fold/expand,
            // not for indenting input).
        }

        // ── Esc in input mode ────────────────────────────────────────────
        // ADR 0001: Esc never quits. The cascading close (search → selected_msg
        // → completion) is handled by mod.rs's system-level Esc before we get
        // here. If we do reach here, it's a no-op.
        KeyCode::Esc => {}

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

/// Direction for history navigation (Ctrl+Up / Ctrl+Down).
pub(crate) enum Direction {
    Prev,
    Next,
}

/// ADR 0002 §7: input-driven slash completion filter.
///
/// Activates the slash popup when `app.input` starts with `/` and contains
/// no whitespace; populates `filtered` with command indices whose name starts
/// with the typed prefix; clamps `selected` to the filtered range. Deactivates
/// the popup otherwise. Call after every Char / Backspace / Paste that
/// touches the input buffer.
pub(crate) fn refresh_slash_completion(app: &mut TuiApp) {
    // Only activate when input begins with '/' and has no whitespace —
    // once args start, the user is typing parameters, not the command name.
    let prefix = if app.input.starts_with('/') && !app.input[1..].contains(char::is_whitespace) {
        &app.input[1..]
    } else if app.input.starts_with('/') {
        // Has whitespace: keep popup open with all commands visible as a
        // reference, but no filtering.
        ""
    } else {
        app.slash_completion.active = false;
        app.slash_completion.filtered.clear();
        app.slash_completion.selected = 0;
        return;
    };

    let prefix_lower = prefix.to_lowercase();
    app.slash_completion.active = true;
    app.slash_completion.filtered = app
        .slash_completion
        .commands
        .iter()
        .enumerate()
        .filter_map(|(i, cmd)| {
            let cmd_lower = cmd.to_lowercase();
            // Strip leading '/' for prefix match, then compare.
            let cmd_tail = cmd_lower.trim_start_matches('/');
            let prefix_tail = prefix_lower.trim_start_matches('/');
            if cmd_tail.starts_with(prefix_tail) { Some(i) } else { None }
        })
        .collect();
    // Clamp selected to filtered range; reset to top when filter shrinks.
    let max = app.slash_completion.filtered.len().saturating_sub(1);
    if app.slash_completion.selected > max {
        app.slash_completion.selected = 0;
    }
}

/// Accept the currently-highlighted @ file-completion candidate: replace the
/// `@query` span (from the stored `at_pos` up to the cursor) with the
/// candidate's display path, move the cursor to the end of the insertion, and
/// close the popup. Shared by Tab and Enter (H2 fix).
pub(crate) fn accept_file_completion(app: &mut TuiApp) {
    if let Some(candidate) = app.completion.candidates.get(app.completion.selected) {
        let at_pos = app.completion.at_pos;
        let end_of_word = app.cursor_pos.min(app.input.len());
        if at_pos <= end_of_word && app.input.is_char_boundary(at_pos) {
            let display = candidate.display.clone();
            app.input.replace_range(at_pos..end_of_word, &display);
            app.cursor_pos = at_pos + display.len();
        }
    }
    app.completion.active = false;
}

/// Walk input history (Ctrl+Up / Ctrl+Down per ADR 0001).
/// Independent of input contents — overrides whatever is in the buffer,
/// matching readline behavior. Caller can undo to recover.
pub(crate) fn navigate_history(app: &mut TuiApp, dir: Direction) {
    if app.input_history.is_empty() {
        return;
    }
    match dir {
        Direction::Prev => {
            if app.history_pos > 0 {
                app.history_pos -= 1;
                app.input = app.input_history[app.history_pos].clone();
                app.cursor_pos = app.input.len();
            }
        }
        Direction::Next => {
            if app.history_pos < app.input_history.len() {
                app.history_pos += 1;
                if app.history_pos == app.input_history.len() {
                    app.input.clear();
                    app.cursor_pos = 0;
                } else {
                    app.input = app.input_history[app.history_pos].clone();
                    app.cursor_pos = app.input.len();
                }
            }
        }
    }
}

/// Try to move the cursor one display row up (-1) or down (+1) inside the
/// multi-line input buffer. Returns true if the cursor moved (i.e. input
/// had multiple lines and the cursor was not at the boundary already),
/// false otherwise — callers fall back to browse-mode behavior when false.
pub(crate) fn try_move_cursor_vertical(app: &mut TuiApp, delta: i32) -> bool {
    if !app.input.contains('\n') {
        return false; // single-line input: nothing to move vertically
    }
    let cursor = app.cursor_pos.min(app.input.len());
    // Find current row/col
    let before = &app.input[..cursor];
    let row = before.matches('\n').count();
    let last_nl = before.rfind('\n');
    let col = match last_nl {
        Some(idx) => cursor - idx - 1,
        None => cursor,
    };

    let lines: Vec<&str> = app.input.split('\n').collect();
    let target_row = row as i32 + delta;
    if target_row < 0 || target_row as usize >= lines.len() {
        return false; // at top/bottom edge: no movement
    }
    let target_row = target_row as usize;
    let target_col = col.min(lines[target_row].len());
    // Recompute byte offset: sum of preceding lines + target_col, accounting for '\n' separators
    let mut new_cursor = 0usize;
    for (i, l) in lines.iter().enumerate() {
        if i == target_row {
            // Advance to char boundary at or before target_col bytes
            let mut boundary = target_col;
            while boundary > 0 && !lines[target_row].is_char_boundary(boundary) {
                boundary -= 1;
            }
            new_cursor += boundary;
            break;
        }
        new_cursor += l.len() + 1; // +1 for '\n'
    }
    app.cursor_pos = new_cursor;
    true
}

pub fn send_message(app: &mut TuiApp, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) {
    let input = app.input.trim().to_string();
    if input.is_empty() {
        return;
    }

    // ADR 0002: every slash-prefixed input is intercepted by the local
    // dispatcher. Nothing starting with '/' ever reaches the agent.
    if input.starts_with('/') {
        super::commands::dispatch_slash_command(app, &input, cmd_tx);
        app.input.clear();
        app.cursor_pos = 0;
        return;
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

    // ── ADR 0001 — Keybinding & Mode Semantics ────────────────────────────

    #[test]
    fn adr0001_esc_in_input_does_not_quit() {
        // Esc with empty input + no overlays must be a no-op (not quit).
        let mut app = TuiApp::default();
        app.input = String::new();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE, &tx);
        assert!(!app.should_quit, "Esc must not set should_quit");
    }

    #[test]
    fn adr0001_esc_in_input_handler_is_noop_when_completion_active() {
        // ADR 0001: at the input_area layer, Esc is a no-op. The completion-
        // closing cascade lives at mod.rs handle_key (tested there). When
        // handle_input_key is reached with completion still active (which
        // shouldn't happen in normal flow but is the contract), Esc must
        // neither quit nor mutate state.
        let mut app = TuiApp::default();
        app.completion.active = true;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE, &tx);
        assert!(!app.should_quit, "Esc at input layer must not quit");
        // Note: completion is NOT closed here — that's the cascade's job.
    }

    #[test]
    fn adr0001_enter_on_empty_input_is_noop() {
        // Empty input + plain Enter must NOT trigger any state change.
        // Previously this could fold reasoning, accept completion, or scroll.
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Reasoning { text: "CoT".into(), expanded: false });
        app.input = String::new();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE, &tx);
        // Reasoning must remain un-folded (Enter no longer toggles).
        match &app.messages[0] {
            MessageItem::Reasoning { expanded, .. } => assert!(!*expanded, "Enter must not fold reasoning"),
            _ => panic!("expected Reasoning"),
        }
    }

    #[test]
    fn adr0001_tab_folds_last_reasoning_message() {
        // Tab is now the exclusive fold/expand key for Reasoning.
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Reasoning { text: "CoT".into(), expanded: false });
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Tab, crossterm::event::KeyModifiers::NONE, &tx);
        match &app.messages[0] {
            MessageItem::Reasoning { expanded, .. } => assert!(*expanded, "Tab should expand reasoning"),
            _ => panic!("expected Reasoning"),
        }
    }

    #[test]
    fn adr0001_tab_toggles_show_full_on_expanded_toolcall() {
        // Tab on an already-expanded ToolCall toggles show_full.
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::ToolCall {
            name: "read".into(), input: "x".into(), output: "y".into(),
            expanded: true, show_full: false,
        });
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Tab, crossterm::event::KeyModifiers::NONE, &tx);
        match &app.messages[0] {
            MessageItem::ToolCall { show_full, .. } => assert!(*show_full, "Tab should toggle show_full"),
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn adr0001_tab_does_not_insert_spaces_when_no_target() {
        // Old behavior: Tab when nothing to fold inserted 2 spaces. ADR 0001
        // removes that — Tab with no completion and nothing to fold is no-op.
        let mut app = TuiApp::default();
        app.input = "abc".into();
        app.cursor_pos = 3;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Tab, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "abc", "Tab must not insert spaces");
    }

    #[test]
    fn adr0001_ctrl_up_walks_history_backward() {
        // Ctrl+Up is the new history-backward key (plain Up no longer is).
        let mut app = TuiApp::default();
        app.input_history = vec!["first".into(), "second".into(), "third".into()];
        app.history_pos = 3; // past the end — current (empty) input
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "third");
        assert_eq!(app.history_pos, 2);
    }

    #[test]
    fn adr0001_ctrl_down_walks_history_forward() {
        let mut app = TuiApp::default();
        app.input_history = vec!["first".into(), "second".into()];
        app.history_pos = 0;
        // Override input to confirm Ctrl+Down supersedes buffer contents.
        app.input = "garbage".into();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "second");
        assert_eq!(app.history_pos, 1);
    }

    #[test]
    fn adr0001_plain_up_on_empty_input_enters_browse() {
        // Plain Up with empty input enters browse mode (selects last msg).
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "m1".into() });
        app.messages.push(MessageItem::User { text: "m2".into() });
        app.input = String::new();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.selected_msg, Some(1), "Up on empty should select last message");
    }

    #[test]
    fn adr0001_plain_up_in_multiline_moves_cursor() {
        // Plain Up inside multi-line input moves cursor up a row, does not
        // enter browse mode (input not empty).
        let mut app = TuiApp::default();
        app.input = "line1\nline2\nline3".into();
        // Cursor at end of last line
        app.cursor_pos = app.input.len();
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE, &tx);
        // Cursor should have moved to start of "line3" → end of "line2"
        // i.e. just before the second '\n'
        assert!(app.cursor_pos < app.input.len(), "cursor must move up");
        assert_eq!(&app.input[app.cursor_pos..app.cursor_pos+1], "\n", "cursor should sit at row boundary");
    }

    // ── H1 — Home/End are line-edge cursor keys (not list scroll) ─────────

    #[test]
    fn h1_home_moves_to_current_line_start() {
        let mut app = TuiApp::default();
        app.input = "line1\nline2\nline3".into();
        app.cursor_pos = 14; // inside "line3" (after "li")
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Home, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 12, "Home goes to start of current line, not buffer start");
    }

    #[test]
    fn h1_end_moves_to_current_line_end() {
        let mut app = TuiApp::default();
        app.input = "line1\nline2\nline3".into();
        app.cursor_pos = 6; // start of "line2"
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::End, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.cursor_pos, 11, "End goes to end of current line (before the '\\n'), not buffer end");
    }

    // ── H2 — @ file completion is navigable and acceptable ────────────────

    fn cand(s: &str) -> completion::CompletionCandidate {
        completion::CompletionCandidate { display: s.into(), path: s.into() }
    }

    #[test]
    fn h2_down_up_move_completion_selection() {
        let mut app = TuiApp::default();
        app.completion.active = true;
        app.completion.candidates = vec![cand("a.rs"), cand("b.rs"), cand("c.rs")];
        app.completion.selected = 0;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.completion.selected, 1, "Down should advance completion selection");
        press(&mut app, crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.completion.selected, 2);
        press(&mut app, crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.completion.selected, 2, "Down clamps at last candidate");
        press(&mut app, crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.completion.selected, 1, "Up should move selection back");
    }

    #[test]
    fn h2_enter_accepts_selected_candidate() {
        let mut app = TuiApp::default();
        app.input = "see @b".into();
        app.cursor_pos = app.input.len();
        app.completion.active = true;
        app.completion.at_pos = 4; // the '@'
        app.completion.candidates = vec![cand("a.rs"), cand("bbb.rs")];
        app.completion.selected = 1;
        let (tx, _rx) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "see bbb.rs", "Enter inserts the highlighted candidate, not the first");
        assert!(!app.completion.active, "accepting closes the popup");
        // Must NOT have submitted a message.
        assert!(!app.messages.iter().any(|m| matches!(m, MessageItem::User { .. })));
    }

    #[test]
    fn h2_tab_accepts_selected_candidate() {
        let mut app = TuiApp::default();
        app.input = "@a".into();
        app.cursor_pos = 2;
        app.completion.active = true;
        app.completion.at_pos = 0;
        app.completion.candidates = vec![cand("alpha.rs"), cand("beta.rs")];
        app.completion.selected = 1;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Tab, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "beta.rs", "Tab inserts the highlighted (2nd) candidate");
        assert!(!app.completion.active);
    }

    #[test]
    fn adr0001_ctrl_enter_submits_even_with_modifiers() {
        // Ctrl+Enter / Alt+Enter force submit (override).
        let mut app = TuiApp::default();
        app.input = "hello".into();
        // Keep rx alive so cmd_tx.send succeeds — otherwise send_message
        // takes its early-return path and never clears the input.
        let (tx, _rx) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::CONTROL, &tx);
        // send_message pushes User msg + clears input
        assert!(app.input.is_empty(), "Ctrl+Enter should submit and clear input");
        assert!(app.messages.iter().any(|m| matches!(m, MessageItem::User { text } if text == "hello")));
    }
}
