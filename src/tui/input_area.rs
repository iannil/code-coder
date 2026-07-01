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

/// ─── Multiline Wrap Helpers (V2) ───────────────────────────────────────────
///
/// Self-contained wrap algorithm. Single source of truth: same `Vec<WrapLine>`
/// feeds height calculation, content rendering, and cursor positioning — so
/// they can never drift apart.

/// One visual line after wrapping. Byte ranges map back into the original
/// input (no character splitting, no word-boundary surprises).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapLine {
    /// The wrapped text (no `\n`).
    pub text: String,
    /// Byte offset in original input where this line starts (inclusive).
    pub start_byte: usize,
    /// Byte offset where this line ends (exclusive).
    pub end_byte: usize,
    /// Display width in terminal columns (emoji/CJK = 2).
    pub display_width: usize,
}

/// Wrap `input` to `width` columns (reserving 2 for `> ` prefix on line 1).
/// Returns one `WrapLine` per visual line. Empty input yields a single empty
/// `WrapLine`.
pub fn compute_input_lines(input: &str, width: u16) -> Vec<WrapLine> {
    let content_width = width.saturating_sub(2).max(1) as usize;
    let mut out = Vec::new();

    let mut line_start_byte = 0usize;
    let mut line_text = String::new();
    let mut line_width = 0usize;

    for (byte_idx, ch) in input.char_indices() {
        if ch == '\n' {
            // Close current line at the \n boundary
            out.push(WrapLine {
                text: std::mem::take(&mut line_text),
                start_byte: line_start_byte,
                end_byte: byte_idx,
                display_width: line_width,
            });
            line_start_byte = byte_idx + 1; // skip past \n
            line_width = 0;
            continue;
        }
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if line_width + w > content_width && !line_text.is_empty() {
            // Wrap: flush current line, start new at this char
            out.push(WrapLine {
                text: std::mem::take(&mut line_text),
                start_byte: line_start_byte,
                end_byte: byte_idx,
                display_width: line_width,
            });
            line_start_byte = byte_idx;
            line_width = 0;
        }
        line_text.push(ch);
        line_width += w;
    }
    // Flush final line (even if empty — preserves the "input has n+1 lines"
    // invariant for n newlines).
    out.push(WrapLine {
        text: line_text,
        start_byte: line_start_byte,
        end_byte: input.len(),
        display_width: line_width,
    });
    out
}

/// Dynamic input-area height: `lines + 2` (border + padding), capped at
/// `term_height / 2`, minimum 3. Task 3 fills this in.
pub fn compute_input_height(_input: &str, _term_height: u16, _width: u16) -> u16 {
    3
}

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

    // Kill-ring accumulation: consecutive kills (Ctrl+K/U/W) concatenate. Any
    // other key breaks the run, so reset the flag here and let the kill arms
    // re-set it. `was_kill` tells those arms whether to extend or replace.
    let was_kill = app.kill_accumulating;
    app.kill_accumulating = false;

    match key.code {
        // ── Ctrl combos for editing ──────────────────────────────────────
        // ADR 0001: Ctrl+A/E are line-edge (readline), matching Home/End —
        // current line, not whole buffer (matters for multi-line input).
        KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = line_start(&app.input, app.cursor_pos);
        }
        KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
            app.cursor_pos = line_end(&app.input, app.cursor_pos);
        }
        // Ctrl+W — kill word before cursor → kill-ring (prepend).
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            if app.cursor_pos > 0 {
                save_undo_snapshot(app);
                let before = &app.input[..app.cursor_pos];
                let trimmed = before.trim_end();
                let word_start = trimmed
                    .rfind(|c: char| c.is_whitespace())
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let killed: String = app.input.drain(word_start..app.cursor_pos).collect();
                app.cursor_pos = word_start;
                push_kill(app, &killed, was_kill, KillSide::Prepend);
            }
        }
        // Ctrl+U — kill to start of current line → kill-ring (prepend).
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            let start = line_start(&app.input, app.cursor_pos);
            if app.cursor_pos > start {
                save_undo_snapshot(app);
                let killed: String = app.input.drain(start..app.cursor_pos).collect();
                app.cursor_pos = start;
                push_kill(app, &killed, was_kill, KillSide::Prepend);
            }
        }
        // Ctrl+K — kill to end of current line → kill-ring (append).
        KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
            let end = line_end(&app.input, app.cursor_pos);
            if end > app.cursor_pos {
                save_undo_snapshot(app);
                let killed: String = app.input.drain(app.cursor_pos..end).collect();
                push_kill(app, &killed, was_kill, KillSide::Append);
            }
        }
        // Ctrl+Shift+Z — redo (relocated from Ctrl+Y, which is now yank).
        KeyCode::Char('z') | KeyCode::Char('Z')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if let Some(next) = app.redo_stack.pop() {
                save_undo_snapshot(app);
                app.input = next;
                app.cursor_pos = app.input.len();
            }
        }
        // Ctrl+Z — undo.
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
        // Ctrl+Y — yank (paste the kill-ring at the cursor), Claude-consistent.
        KeyCode::Char('y') if key.modifiers == KeyModifiers::CONTROL => {
            if !app.kill_ring.is_empty() {
                save_undo_snapshot(app);
                let text = app.kill_ring.clone();
                app.input.insert_str(app.cursor_pos, &text);
                app.cursor_pos += text.len();
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
                app.input.insert(app.cursor_pos, '\n');
                app.cursor_pos += 1;
            } else if app.input[..app.cursor_pos].ends_with('\\') {
                // Trailing-backslash line continuation: turn the `\` directly
                // before the cursor into a newline instead of submitting
                // (mirrors the original's multi-line continuation).
                let bs = app.cursor_pos - 1;
                save_undo_snapshot(app);
                app.input.remove(bs);
                app.input.insert(bs, '\n');
                app.cursor_pos = bs + 1;
            } else {
                send_message(app, cmd_tx);
            }
        }

        // ── Word-wise cursor (Ctrl/Alt + Left/Right) ────────────────────
        // Ctrl+Up/Down own history, but Ctrl/Alt+Left/Right are free — bind
        // them to prev/next word, matching the original's readline word nav.
        KeyCode::Left
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.cursor_pos = prev_word(&app.input, app.cursor_pos);
        }
        KeyCode::Right
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.cursor_pos = next_word(&app.input, app.cursor_pos);
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
            app.cursor_pos = line_start(&app.input, app.cursor_pos);
        }
        KeyCode::End => {
            app.cursor_pos = line_end(&app.input, app.cursor_pos);
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
    let prefix_tail = prefix_lower.trim_start_matches('/');
    app.slash_completion.active = true;
    // Subsequence (fuzzy) match: keep commands whose name contains the typed
    // chars in order — so "cfg" matches "config". Prefix matches rank first.
    let mut matches: Vec<(bool, usize)> = app
        .slash_completion
        .commands
        .iter()
        .enumerate()
        .filter_map(|(i, cmd)| {
            let cmd_lower = cmd.to_lowercase();
            let cmd_tail = cmd_lower.trim_start_matches('/');
            if cmd_tail.starts_with(prefix_tail) {
                Some((true, i))
            } else if is_subsequence(prefix_tail, cmd_tail) {
                Some((false, i))
            } else {
                None
            }
        })
        .collect();
    // Stable sort keeps enumerate order within each group; prefix (true) first.
    matches.sort_by_key(|(is_prefix, _)| !is_prefix);
    app.slash_completion.filtered = matches.into_iter().map(|(_, i)| i).collect();
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

/// Which end of the kill-ring a freshly-killed span attaches to.
pub(crate) enum KillSide {
    /// Killed text is forward of the cursor (Ctrl+K) → append.
    Append,
    /// Killed text is behind the cursor (Ctrl+U / Ctrl+W) → prepend.
    Prepend,
}

/// Add `killed` to the kill-ring. When the previous key was also a kill
/// (`was_kill`), the span concatenates onto the existing ring (Claude-style
/// accumulation); otherwise it replaces the ring.
pub(crate) fn push_kill(app: &mut TuiApp, killed: &str, was_kill: bool, side: KillSide) {
    if !was_kill {
        app.kill_ring.clear();
    }
    match side {
        KillSide::Append => app.kill_ring.push_str(killed),
        KillSide::Prepend => {
            let mut s = killed.to_string();
            s.push_str(&app.kill_ring);
            app.kill_ring = s;
        }
    }
    app.kill_accumulating = true;
}

/// True if every char of `needle` appears in `haystack` in order (a
/// subsequence / fuzzy match). Empty needle matches everything.
pub(crate) fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut hay = haystack.chars();
    'outer: for nc in needle.chars() {
        for hc in hay.by_ref() {
            if hc == nc {
                continue 'outer;
            }
        }
        return false; // ran out of haystack before matching nc
    }
    true
}

/// Byte offset of the start of the line containing `pos` (after the previous
/// '\n', or 0). Used by Home and Ctrl+A.
pub(crate) fn line_start(input: &str, pos: usize) -> usize {
    input[..pos.min(input.len())].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Byte offset of the end of the line containing `pos` (just before the next
/// '\n', or input length). Used by End and Ctrl+E.
pub(crate) fn line_end(input: &str, pos: usize) -> usize {
    let pos = pos.min(input.len());
    match input[pos..].find('\n') {
        Some(i) => pos + i,
        None => input.len(),
    }
}

/// Byte offset one word to the left of `pos`: skip trailing whitespace, then
/// the word characters before it. Boundary-safe (slices land on char edges
/// because whitespace/non-whitespace splits never bisect a char).
pub(crate) fn prev_word(input: &str, pos: usize) -> usize {
    let before = &input[..pos.min(input.len())];
    let no_ws = before.trim_end_matches(|c: char| c.is_whitespace());
    no_ws.trim_end_matches(|c: char| !c.is_whitespace()).len()
}

/// Byte offset one word to the right of `pos`: skip leading whitespace, then
/// the word characters after it (lands at the end of the next word).
pub(crate) fn next_word(input: &str, pos: usize) -> usize {
    let pos = pos.min(input.len());
    let after = &input[pos..];
    let after_ws = after.trim_start_matches(|c: char| c.is_whitespace());
    let after_word = after_ws.trim_start_matches(|c: char| !c.is_whitespace());
    input.len() - after_word.len()
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
    fn test_key_ctrl_shift_z_redo_input() {
        // Redo relocated to Ctrl+Shift+Z (Ctrl+Y is now yank).
        let mut app = TuiApp::default();
        app.input = "first".into();
        app.redo_stack.push("second".into());
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('z'),
            crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::SHIFT, &tx);
        assert_eq!(app.input, "second");
    }

    // ── Kill-ring (Claude-consistent) ─────────────────────────────────────

    #[test]
    fn killring_ctrl_y_yanks_killed_text() {
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        app.cursor_pos = 11;
        let (tx, _) = std::sync::mpsc::channel();
        // Ctrl+W kills "world" into the ring.
        press(&mut app, crossterm::event::KeyCode::Char('w'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello ");
        assert_eq!(app.kill_ring, "world");
        // Ctrl+Y yanks it back at the cursor.
        press(&mut app, crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.input, "hello world");
        assert_eq!(app.cursor_pos, 11);
    }

    #[test]
    fn killring_consecutive_kills_accumulate() {
        // Two consecutive Ctrl+W kills should concatenate (prepend) in the ring.
        let mut app = TuiApp::default();
        app.input = "foo bar baz".into();
        app.cursor_pos = 11;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('w'), crossterm::event::KeyModifiers::CONTROL, &tx); // kill "baz"
        press(&mut app, crossterm::event::KeyCode::Char('w'), crossterm::event::KeyModifiers::CONTROL, &tx); // kill "bar "
        assert_eq!(app.kill_ring, "bar baz", "consecutive kills accumulate in order");
    }

    #[test]
    fn killring_non_kill_key_breaks_accumulation() {
        let mut app = TuiApp::default();
        app.input = "foo bar".into();
        app.cursor_pos = 7;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('w'), crossterm::event::KeyModifiers::CONTROL, &tx); // kill "bar"
        assert_eq!(app.kill_ring, "bar");
        // A normal keystroke breaks the kill run.
        press(&mut app, crossterm::event::KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE, &tx);
        assert!(!app.kill_accumulating, "typing breaks the kill run");
        // Next kill replaces, not extends.
        app.cursor_pos = line_start(&app.input, app.cursor_pos).max(0);
        app.cursor_pos = app.input.len();
        press(&mut app, crossterm::event::KeyCode::Char('u'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert!(!app.kill_ring.contains("bar"), "ring replaced after the run broke: {:?}", app.kill_ring);
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

    // ── Slash completion fuzzy match ──────────────────────────────────────

    #[test]
    fn is_subsequence_matches_in_order() {
        assert!(is_subsequence("cfg", "config"));
        assert!(is_subsequence("hist", "history"));
        assert!(is_subsequence("", "anything"));
        assert!(!is_subsequence("cgf", "config"), "out-of-order must fail");
        assert!(!is_subsequence("xyz", "config"));
    }

    #[test]
    fn slash_filter_fuzzy_and_prefix_ranking() {
        let mut app = TuiApp::default();
        // "/cfg" should fuzzy-match "/config" even though it's not a prefix.
        app.input = "/cfg".into();
        refresh_slash_completion(&mut app);
        let names: Vec<&str> = app.slash_completion.filtered.iter()
            .map(|&i| app.slash_completion.commands[i])
            .collect();
        assert!(names.contains(&"/config"), "fuzzy '/cfg' should match /config: {names:?}");

        // Prefix matches rank ahead of pure subsequence matches.
        app.input = "/re".into();
        refresh_slash_completion(&mut app);
        let ranked: Vec<&str> = app.slash_completion.filtered.iter()
            .map(|&i| app.slash_completion.commands[i])
            .collect();
        let first_prefix = ranked.iter().position(|c| c.trim_start_matches('/').starts_with("re"));
        let first_fuzzy = ranked.iter().position(|c| !c.trim_start_matches('/').starts_with("re"));
        if let (Some(p), Some(f)) = (first_prefix, first_fuzzy) {
            assert!(p < f, "prefix matches must come before fuzzy ones: {ranked:?}");
        }
    }

    // ── Cursor helpers ────────────────────────────────────────────────────

    #[test]
    fn line_start_end_on_multiline() {
        let s = "ab\ncde\nf";
        // cursor inside "cde" (pos 5)
        assert_eq!(line_start(s, 5), 3, "line_start → after first '\\n'");
        assert_eq!(line_end(s, 5), 6, "line_end → before second '\\n'");
        // first line
        assert_eq!(line_start(s, 1), 0);
        assert_eq!(line_end(s, 1), 2);
        // last line (no trailing '\n')
        assert_eq!(line_end(s, 7), 8);
    }

    #[test]
    fn prev_next_word_basic() {
        let s = "foo bar baz";
        assert_eq!(prev_word(s, 11), 8, "prev_word from end → start of 'baz'");
        assert_eq!(prev_word(s, 8), 4, "prev_word skips the space → start of 'bar'");
        assert_eq!(next_word(s, 0), 3, "next_word from start → end of 'foo'");
        assert_eq!(next_word(s, 3), 7, "next_word skips space → end of 'bar'");
    }

    #[test]
    fn ctrl_left_right_move_by_word() {
        let mut app = TuiApp::default();
        app.input = "foo bar baz".into();
        app.cursor_pos = 11;
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Left, crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 8, "Ctrl+Left → start of 'baz'");
        press(&mut app, crossterm::event::KeyCode::Right, crossterm::event::KeyModifiers::ALT, &tx);
        assert_eq!(app.cursor_pos, 11, "Alt+Right → end of 'baz'");
    }

    #[test]
    fn ctrl_a_e_are_line_edge_in_multiline() {
        let mut app = TuiApp::default();
        app.input = "ab\ncde\nf".into();
        app.cursor_pos = 5; // inside "cde"
        let (tx, _) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 3, "Ctrl+A → current line start, not buffer start");
        press(&mut app, crossterm::event::KeyCode::Char('e'), crossterm::event::KeyModifiers::CONTROL, &tx);
        assert_eq!(app.cursor_pos, 6, "Ctrl+E → current line end, not buffer end");
    }

    #[test]
    fn trailing_backslash_enter_continues_line() {
        let mut app = TuiApp::default();
        app.input = "line one\\".into();
        app.cursor_pos = app.input.len();
        let (tx, _rx) = std::sync::mpsc::channel();
        press(&mut app, crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE, &tx);
        assert_eq!(app.input, "line one\n", "trailing '\\' + Enter → newline, not submit");
        assert!(!app.messages.iter().any(|m| matches!(m, MessageItem::User { .. })), "must not submit");
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

    // ── compute_input_lines (V2 Task 2) ───────────────────────────────────────

    #[test]
    fn test_compute_lines_empty() {
        let lines = compute_input_lines("", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 0);
        assert_eq!(lines[0].display_width, 0);
    }

    #[test]
    fn test_compute_lines_single_short() {
        let lines = compute_input_lines("hello", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 5);
        assert_eq!(lines[0].display_width, 5);
    }

    #[test]
    fn test_compute_lines_newline() {
        let lines = compute_input_lines("a\nb", 80);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "a");
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines[0].end_byte, 1);
        assert_eq!(lines[1].text, "b");
        assert_eq!(lines[1].start_byte, 2);
        assert_eq!(lines[1].end_byte, 3);
    }

    #[test]
    fn test_compute_lines_long_wrap() {
        // 50 chars, content width 30 (width 32 minus 2 for `> `).
        let input = "a".repeat(50);
        let lines = compute_input_lines(&input, 32);
        assert!(lines.len() >= 2, "got {} lines", lines.len());
        // Byte ranges should be contiguous and non-overlapping.
        for i in 1..lines.len() {
            assert_eq!(lines[i].start_byte, lines[i - 1].end_byte,
                "gap/overlap at index {i}");
        }
        assert_eq!(lines[0].start_byte, 0);
        assert_eq!(lines.last().unwrap().end_byte, 50);
        // Each non-final line should be at most 30 display width.
        for l in &lines {
            assert!(l.display_width <= 30, "line {:?} exceeds content width", l.text);
        }
    }

    #[test]
    fn test_compute_lines_wide_chars() {
        // Each 🚀 is display-width 2. content width 10 → 5 per line.
        let input = "🚀".repeat(10);
        let lines = compute_input_lines(&input, 12); // width 12 → content 10
        assert_eq!(lines.len(), 2, "got {} lines", lines.len());
        assert_eq!(lines[0].display_width, 10);
        assert_eq!(lines[1].display_width, 10);
    }

    #[test]
    fn test_compute_lines_narrow_width() {
        // Width < 10 should not panic; content_width = max(width-2, 1) = 1.
        let lines = compute_input_lines("abc", 3);
        // Each character on its own line (content width 1).
        assert_eq!(lines.len(), 3);
        for l in &lines {
            assert!(l.display_width <= 1);
        }
    }
}
