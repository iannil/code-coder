/// ─── Dialogs & Overlays Component ────────────────────────────────────────────
///
/// 覆盖层渲染与按键处理：权限对话框、Plan 审批、AskUser 提问、
/// 帮助面板、模型切换器、斜杠命令补全、@文件补全弹出。

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::Frame;

use super::{TuiApp, Dialog};

/// ─── Render Overlays ────────────────────────────────────────────────────────
///
/// Render all active overlays on top of the message/input areas.
/// Each overlay is mutually exclusive and renders first with Clear.
pub fn render_overlays(frame: &mut Frame, area: Rect, input_area: Rect, app: &mut TuiApp) {
    // 1. Slash completion popup
    if app.slash_completion.active {
        render_slash_completion(frame, area, input_area, app);
        return;
    }

    // 2. Model picker popup
    if app.model_picker_active {
        render_model_picker(frame, area, input_area, app);
        return;
    }

    // 3. Help panel
    if app.help_active {
        render_help_panel(frame, area);
        return;
    }

    // 4. Permission/plan/question dialog
    if let Some(ref dialog) = app.dialog {
        render_dialog(frame, area, dialog);
        return;
    }

    // 5. File completion popup
    if app.completion.active && !app.completion.candidates.is_empty() {
        render_file_completion(frame, area, input_area, app);
    }
}

/// ─── Slash Completion Popup ─────────────────────────────────────────────────

fn render_slash_completion(frame: &mut Frame, area: Rect, input_area: Rect, app: &TuiApp) {
    // ADR 0002 §7: render only filtered commands. When filtered is empty
    // (shouldn't happen — refresh_slash_completion keeps it in sync), fall
    // back to showing all.
    let indices: Vec<usize> = if app.slash_completion.filtered.is_empty() {
        (0..app.slash_completion.commands.len()).collect()
    } else {
        app.slash_completion.filtered.clone()
    };
    let popup_width = area.width.min(50).max(30);
    let popup_height = (indices.len() as u16 + 2).min(14);
    let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
    let popup_y = input_area.y.saturating_sub(popup_height + 1);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let items: Vec<Line> = indices.iter().enumerate().map(|(pos, &i)| {
        let cmd = app.slash_completion.commands.get(i).copied().unwrap_or("");
        let desc = app.slash_completion.descriptions.get(i).copied().unwrap_or("");
        if pos == app.slash_completion.selected {
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
    }).collect();

    frame.render_widget(ratatui::widgets::Clear, popup_area);
    let popup_block = Paragraph::new(items)
        .block(Block::default()
            .title(" Commands ")
            .title_alignment(ratatui::layout::Alignment::Left));
    frame.render_widget(popup_block, popup_area);
}

/// ─── Model Picker Popup ─────────────────────────────────────────────────────

fn render_model_picker(frame: &mut Frame, area: Rect, input_area: Rect, app: &TuiApp) {
    let popup_width = area.width.min(40).max(20);
    let popup_height = (app.available_models.len() as u16 + 2).min(16);
    let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
    let popup_y = input_area.y.saturating_sub(popup_height + 1);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let items: Vec<Line> = app.available_models.iter().enumerate()
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
        .block(Block::default()
            .title(" Model ")
            .title_alignment(ratatui::layout::Alignment::Left));
    frame.render_widget(popup_block, popup_area);
}

/// ─── Help Panel ─────────────────────────────────────────────────────────────

fn render_help_panel(frame: &mut Frame, area: Rect) {
    let panel_width = area.width.min(72).max(50);
    // Panel needs ~50 lines for the full ADR 0001 binding list. Cap at
    // area.height - 2 so borders always fit, but never less than 20.
    let panel_height = (area.height.saturating_sub(2)).min(60).max(20);
    let panel_x = area.x + (area.width.saturating_sub(panel_width)) / 2;
    let panel_y = area.y + (area.height.saturating_sub(panel_height)) / 2;
    let panel_area = Rect::new(panel_x, panel_y, panel_width, panel_height);

    fn hdr(s: &str) -> Line {
        Line::styled(format!(" {s} "), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    }
    fn key(k: &str) -> Span {
        Span::styled(format!("{:<14}", k), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    }
    fn row<'a>(k: &'a str, d: &'a str) -> Line<'a> {
        Line::from(vec![Span::raw("  "), key(k), Span::raw(d)])
    }
    fn row_warn<'a>(k: &'a str, d: &'a str) -> Line<'a> {
        Line::from(vec![
            Span::raw("  "),
            key(k),
            Span::styled(format!("⚠ {d}"), Style::default().fg(Color::Yellow)),
        ])
    }

    let help_lines = vec![
        hdr("Editing"),
        Line::from(""),
        row("Enter",        "Submit message (empty: no-op)"),
        row("Shift+Enter",  "Insert newline (multi-line input)"),
        row("Alt+Enter",    "Force submit (overrides modifiers)"),
        row("Ctrl+Z / Y",   "Undo / redo input"),
        row("Ctrl+A / E",   "Cursor to line start / end"),
        row("Ctrl+W",       "Delete word backward"),
        row("Ctrl+U / K",   "Delete to start / end of line"),
        row("Tab",          "Accept completion / fold last message"),
        row("@",            "Trigger file completion"),
        Line::from(""),
        hdr("Navigation"),
        Line::from(""),
        row("Up / Down",    "Move cursor (or browse msgs when empty)"),
        row("Ctrl+Up/Dn",   "Walk input history"),
        row("Left / Right", "Move cursor one char"),
        row("Home / End",   "Cursor to line start / end"),
        row("g / G",        "Msg list: scroll to top / bottom (empty input)"),
        row("PgUp / PgDn",  "Msg list: scroll up / down by page"),
        row("End",          "Msg list: jump to bottom"),
        Line::from(""),
        hdr("Mode & Tools"),
        Line::from(""),
        row("Ctrl+F",       "Search messages"),
        row("Ctrl+R",       "Reverse search"),
        row("Ctrl+P",       "Switch model"),
        row("Ctrl+T",       "Toggle theme (partial — markdown only)"),
        row("Ctrl+H",       "Toggle this help panel"),
        row_warn("Ctrl+L",  "Clear messages — no confirm yet"),
        row("Ctrl+C",       "Busy: interrupt agent  ·  Idle: quit"),
        row_warn("Ctrl+Q",  "Quit"),
        row("Esc",          "Close current overlay (never quits)"),
        Line::from(""),
        hdr("Commands"),
        Line::from(""),
        row("/help",        "Show this panel"),
        row("/exit /quit",  "Quit"),
        row("/clear",       "Clear messages"),
        row("/reload",      "Reload context + skills"),
        row("/history",     "Show message count"),
        row("/tools",       "List available tools"),
        row("/skills",      "List loaded skills"),
        row("/memory",      "List memory entries"),
        row("/session",     "List saved sessions"),
        row("/resume [id]", "Resume a session"),
        row("/config",      "View / change settings"),
        row("/mcp",         "Manage MCP servers"),
        Line::from(""),
        Line::styled(" Esc to close ", Style::default().fg(Color::DarkGray)),
    ];

    frame.render_widget(ratatui::widgets::Clear, panel_area);
    let panel = Paragraph::new(help_lines)
        .block(Block::default()
            .title(" Help ")
            .title_alignment(ratatui::layout::Alignment::Left))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, panel_area);
}

/// ─── Dialog (Permission / Plan / Ask) ───────────────────────────────────────

fn render_dialog(frame: &mut Frame, area: Rect, dialog: &Dialog) {
    let (tool_name, tool_input_txt, risk) = match dialog {
        Dialog::ToolPermission { tool_name, tool_input, risk, .. } => (tool_name.as_str(), tool_input.as_str(), risk.as_str()),
        Dialog::PlanApproval { title: _, plan, .. } => ("plan", plan.as_str(), "plan approval"),
        Dialog::AskQuestion { question, .. } => ("ask_user", question.as_str(), "user question"),
    };

    let dialog_width = area.width.min(70).max(40);
    let info_str: &str = tool_input_txt;
    let input_lines = info_str.lines().count().min(6).max(1);
    let dialog_height = (5 + input_lines as u16).min(12).max(7);
    let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    let risk_style = if risk.contains("suspicious") || risk.contains("outside") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut content = vec![
        Line::styled(" [!] Tool Permission ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::styled(format!(" Tool: {}", tool_name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ];

    if !risk.is_empty() {
        content.push(Line::styled(format!(" Risk: {}", risk), risk_style));
    }

    let input_display: String = info_str.chars().take(400).collect();
    for line in input_display.lines().take(6) {
        content.push(Line::styled(format!("  {}", line), Style::default().fg(Color::DarkGray)));
    }
    if info_str.len() > 400 {
        content.push(Line::styled("  …(truncated)", Style::default().fg(Color::DarkGray)));
    }

    content.push(Line::from(""));
    match dialog {
        Dialog::AskQuestion { .. } => {
            content.push(Line::styled(
                " Type answer + Enter, or Esc to skip ",
                Style::default().fg(Color::Cyan),
            ));
        }
        Dialog::ToolPermission { .. } => {
            // ADR 0005: scope-aware approval keys.
            content.push(Line::styled(
                " Y=once  A=session  Shift+A=project  N=deny  Esc=cancel ",
                Style::default().fg(Color::Cyan),
            ));
        }
        Dialog::PlanApproval { .. } => {
            content.push(Line::styled(
                " Y=approve  N=reject  Esc=cancel ",
                Style::default().fg(Color::Cyan),
            ));
        }
    }

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    let dialog = Paragraph::new(content)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: false });
    frame.render_widget(dialog, dialog_area);
}

/// ─── File Completion Popup ──────────────────────────────────────────────────

fn render_file_completion(frame: &mut Frame, area: Rect, input_area: Rect, app: &TuiApp) {
    let popup_width = area.width.min(50).max(20);
    let popup_height = (app.completion.candidates.len() as u16 + 2).min(12);
    let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
    let popup_y = input_area.y.saturating_sub(popup_height + 1);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let items: Vec<Line> = app.completion.candidates.iter().enumerate()
        .map(|(i, c)| {
            if i == app.completion.selected {
                Line::styled(
                    format!(" ▸ {} ", c.display),
                    Style::default().fg(Color::Black).bg(Color::White),
                )
            } else {
                Line::styled(
                    format!("   {} ", c.display),
                    Style::default().fg(Color::White),
                )
            }
        })
        .collect();

    frame.render_widget(ratatui::widgets::Clear, popup_area);
    let popup_block = Paragraph::new(items)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .title(" Files ")
            .title_alignment(ratatui::layout::Alignment::Left));
    frame.render_widget(popup_block, popup_area);
}

/// ─── Key Handlers ───────────────────────────────────────────────────────────

/// Handle Y/N/A/Esc/Enter when a dialog is active.
///
/// ADR 0005: ToolPermission dialogs honor three scopes:
///   Y          → Once (re-prompt next time)
///   A          → AlwaysThisSession (no more prompts this session)
///   Shift+A    → AlwaysThisProject (persist to codecoder.json — Phase B)
/// Plan/Ask dialogs only have Y/N/Esc — scope is meaningless for them.
pub fn handle_dialog_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<crate::agent::AgentCommand>) {
    use crossterm::event::KeyCode;
    use crate::agent::AgentCommand;
    use crate::agent::PermScope;

    // Distinguish lowercase 'a' (Shift unset) from uppercase 'A' (Shift held).
    // crossterm reports Shift+via both KeyCode::Char('A') and modifiers, so
    // check modifiers first to be unambiguous.
    let shift = key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);

    match key.code {
        // ── Y: approve once ───────────────────────────────────────────────
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id,
                            allowed: true,
                            scope: PermScope::Once,
                        });
                    }
                    Dialog::PlanApproval { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "approved".into() });
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "yes".into() });
                    }
                }
            }
        }
        // ── A: session allow (Shift+A: project allow) ────────────────────
        // For Plan/Ask dialogs, 'A' has no scope meaning — fall through to
        // approve-once behavior (preserves prior "A approves" UX).
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { ref tool_name, request_id, .. } => {
                        let scope = if shift { PermScope::AlwaysThisProject } else { PermScope::AlwaysThisSession };
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id,
                            allowed: true,
                            scope,
                        });
                        // Accurate scope-aware confirmation. The old text
                        // ("will be allowed without prompting for this
                        // session") lied when project scope was chosen and
                        // when no persistence existed at all — both are now
                        // fixed (project persistence is Phase B but the
                        // scope field reaches the agent).
                        let scope_label = match scope {
                            PermScope::AlwaysThisSession => "this session",
                            PermScope::AlwaysThisProject => "this project (persists across sessions)",
                            PermScope::Once => unreachable!(),
                        };
                        app.messages.push(super::MessageItem::System {
                            text: format!("✓ {} auto-allowed for {}", tool_name, scope_label),
                        });
                    }
                    Dialog::PlanApproval { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "approved".into() });
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "yes".into() });
                    }
                }
            }
        }
        // ── N: deny ───────────────────────────────────────────────────────
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id,
                            allowed: false,
                            scope: PermScope::Once,
                        });
                    }
                    Dialog::PlanApproval { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "rejected".into() });
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "no".into() });
                    }
                }
            }
        }
        // ── Esc: cancel ───────────────────────────────────────────────────
        KeyCode::Esc => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } | Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse {
                            request_id,
                            allowed: false,
                            scope: PermScope::Once,
                        });
                    }
                    Dialog::PlanApproval { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "rejected".into() });
                    }
                }
            }
        }
        // ── Enter: AskQuestion submit, else no-op ─────────────────────────
        KeyCode::Enter => {
            if let Some(dialog) = app.dialog.take() {
                if let Dialog::AskQuestion { question, request_id } = dialog {
                    if !app.input.is_empty() {
                        let answer = app.input.trim().to_string();
                        app.messages.push(super::MessageItem::User { text: answer.clone() });
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer });
                        app.input.clear();
                        app.cursor_pos = 0;
                    } else {
                        // Empty answer → put question back
                        app.dialog = Some(Dialog::AskQuestion { question, request_id });
                    }
                } else {
                    app.dialog = Some(dialog);
                }
            }
        }
        _ => {}
    }
}

/// Handle Esc to close help panel.
pub fn handle_help_key(app: &mut TuiApp, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;
    if matches!(key.code, KeyCode::Esc) {
        app.help_active = false;
    }
}

/// Handle Up/Down/Enter/Esc for model picker.
pub fn handle_model_picker_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<crate::agent::AgentCommand>) {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Up => {
            if app.model_picker_selected > 0 {
                app.model_picker_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.model_picker_selected + 1 < app.available_models.len() {
                app.model_picker_selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(model) = app.available_models.get(app.model_picker_selected) {
                if model != &app.status.model {
                    let _ = cmd_tx.send(crate::agent::AgentCommand::SetModel { model: model.clone() });
                    app.status.model = model.clone();
                }
            }
            app.model_picker_active = false;
        }
        KeyCode::Esc => {
            app.model_picker_active = false;
        }
        _ => {}
    }
}

/// Handle Up/Down/Enter/Esc/Tab for slash command completion.
pub fn handle_slash_completion_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<crate::agent::AgentCommand>) {
    use crossterm::event::KeyCode;

    // ADR 0002 §7: navigation over the *filtered* list. Fall back to all
    // commands when filtered is empty (defensive — refresh should keep it
    // in sync, but a stale state shouldn't panic).
    let filtered: Vec<usize> = if app.slash_completion.filtered.is_empty() {
        (0..app.slash_completion.commands.len()).collect()
    } else {
        app.slash_completion.filtered.clone()
    };
    let max_pos = filtered.len().saturating_sub(1);

    match key.code {
        KeyCode::Up => {
            if app.slash_completion.selected > 0 {
                app.slash_completion.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.slash_completion.selected < max_pos {
                app.slash_completion.selected += 1;
            }
        }
        KeyCode::Enter => {
            // Resolve selected → actual command index → command string.
            if let Some(&cmd_idx) = filtered.get(app.slash_completion.selected) {
                if let Some(cmd) = app.slash_completion.commands.get(cmd_idx).copied() {
                    app.input = cmd.to_string();
                    app.cursor_pos = app.input.len();
                    crate::tui::input_area::send_message(app, cmd_tx);
                }
            }
            app.slash_completion.active = false;
        }
        KeyCode::Tab => {
            if !filtered.is_empty() {
                app.slash_completion.selected = (app.slash_completion.selected + 1) % filtered.len();
            }
        }
        KeyCode::Esc => {
            app.slash_completion.active = false;
        }
        _ => {}
    }
}

/// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{MessageItem, TuiApp, Dialog};
    use crate::agent::AgentCommand;
    use ratatui::backend::TestBackend;

    fn key(code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, modifiers)
    }

    // ── Dialog key: Y/N/A/Esc (ADR 0005 scope-aware) ─────────────────────

    #[test]
    fn test_dialog_y_permits_tool_once() {
        // ADR 0005: Y = Once scope.
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "test.txt".into(),
            request_id: 1,
            risk: "test".into(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_none(), "dialog should be consumed");
        let msg = rx.try_recv().unwrap();
        match msg {
            AgentCommand::PermissionResponse { allowed, scope, .. } => {
                assert!(allowed);
                assert_eq!(scope, crate::agent::PermScope::Once, "Y must send Once scope");
            }
            _ => panic!("expected PermissionResponse"),
        }
    }

    #[test]
    fn test_dialog_n_denies() {
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "test.txt".into(),
            request_id: 1,
            risk: "test".into(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('n'), crossterm::event::KeyModifiers::NONE), &tx);
        let msg = rx.try_recv().unwrap();
        match msg {
            AgentCommand::PermissionResponse { allowed, .. } => assert!(!allowed),
            _ => panic!("expected PermissionResponse"),
        }
    }

    #[test]
    fn test_dialog_esc_cancels() {
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "read_file".into(),
            tool_input: "secret".into(),
            request_id: 1,
            risk: "test".into(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_none());
        let msg = rx.try_recv().unwrap();
        match msg {
            AgentCommand::PermissionResponse { allowed, .. } => assert!(!allowed),
            _ => panic!("expected PermissionResponse"),
        }
    }

    // ── ADR 0005: Scope-aware approval ───────────────────────────────────

    #[test]
    fn adr0005_a_lowercase_sends_alwaystihssession() {
        // 'a' without Shift = AlwaysThisSession.
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "src/main.rs".into(),
            request_id: 7,
            risk: "moderate".into(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::NONE), &tx);
        match rx.try_recv().unwrap() {
            AgentCommand::PermissionResponse { allowed, scope, .. } => {
                assert!(allowed);
                assert_eq!(scope, crate::agent::PermScope::AlwaysThisSession);
            }
            _ => panic!("expected PermissionResponse"),
        }
        // Confirmation message should mention session scope.
        assert!(app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("session")
        )));
    }

    #[test]
    fn adr0005_shift_a_sends_alwaysthisproject() {
        // 'A' with Shift modifier = AlwaysThisProject.
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "src/lib.rs".into(),
            request_id: 9,
            risk: "moderate".into(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('A'), crossterm::event::KeyModifiers::SHIFT), &tx);
        match rx.try_recv().unwrap() {
            AgentCommand::PermissionResponse { allowed, scope, .. } => {
                assert!(allowed);
                assert_eq!(scope, crate::agent::PermScope::AlwaysThisProject);
            }
            _ => panic!("expected PermissionResponse"),
        }
        // Confirmation message should mention project scope.
        assert!(app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("project")
        )));
    }

    #[test]
    fn adr0005_dialog_consumed_after_a() {
        // A should consume the dialog like Y/N do.
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "read_file".into(),
            tool_input: "x".into(),
            request_id: 1,
            risk: "low".into(),
        });
        let (tx, _) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_none(), "A must consume the dialog");
    }

    #[test]
    fn adr0005_no_false_session_promise_for_y() {
        // Y must NOT push a "will be allowed without prompting" message —
        // only A does. This guards against the old lie where Y also claimed
        // session persistence.
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "x".into(),
            request_id: 1,
            risk: "low".into(),
        });
        let (tx, _) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        // No "auto-allowed" confirmation should be pushed for Y.
        assert!(!app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("auto-allowed")
        )), "Y must not push auto-allow confirmation");
    }

    // ── Model picker ──────────────────────────────────────────────────────

    #[test]
    fn test_model_picker_up_down() {
        let mut app = TuiApp::default();
        app.model_picker_active = true;
        app.model_picker_selected = 1;
        let (tx, _) = std::sync::mpsc::channel();
        handle_model_picker_key(&mut app, key(crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.model_picker_selected, 0);
        handle_model_picker_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.model_picker_selected, 1);
    }

    #[test]
    fn test_model_picker_esc_closes() {
        let mut app = TuiApp::default();
        app.model_picker_active = true;
        let (tx, _) = std::sync::mpsc::channel();
        handle_model_picker_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(!app.model_picker_active);
    }

    // ── Help panel ────────────────────────────────────────────────────────

    #[test]
    fn test_help_esc_closes() {
        let mut app = TuiApp::default();
        app.help_active = true;
        handle_help_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
        assert!(!app.help_active);
    }

    // ── Slash completion ──────────────────────────────────────────────────

    #[test]
    fn test_slash_completion_up_down() {
        let mut app = TuiApp::default();
        app.slash_completion.active = true;
        app.slash_completion.selected = 1;
        let (tx, _) = std::sync::mpsc::channel();
        handle_slash_completion_key(&mut app, key(crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.slash_completion.selected, 0);
        handle_slash_completion_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.slash_completion.selected, 1);
    }

    #[test]
    fn test_slash_completion_esc_closes() {
        let mut app = TuiApp::default();
        app.slash_completion.active = true;
        let (tx, _) = std::sync::mpsc::channel();
        handle_slash_completion_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(!app.slash_completion.active);
    }

    // ── Render overlay tests (TestBackend) ─────────────────────────────────

    #[test]
    fn test_render_permission_dialog() {
        let mut app = TuiApp::default();
        app.dialog = Some(Dialog::ToolPermission {
            tool_name: "write_file".into(),
            tool_input: "test.txt".into(),
            request_id: 1,
            risk: "moderate".into(),
        });
        app.status = crate::tui::StatusData::default();

        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("write_file"), "Should show permission: got {cell_text:.80}");
    }

    #[test]
    fn test_render_help_panel() {
        let mut app = TuiApp::default();
        app.help_active = true;
        app.status = crate::tui::StatusData::default();

        // Panel needs ~50 rows for the ADR 0001 binding list — give the
        // backend enough room for all four sections.
        let backend = TestBackend::new(80, 60);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("Editing"), "missing Editing section");
        assert!(cell_text.contains("Navigation"), "missing Navigation section");
        assert!(cell_text.contains("Ctrl+Q"), "missing Ctrl+Q quit binding");
        assert!(cell_text.contains("Ctrl+C"), "missing Ctrl+C binding");
        assert!(cell_text.contains("g / G"), "missing g/G scroll binding");
        assert!(cell_text.contains("Ctrl+Up/Dn"), "missing Ctrl+Up/Dn history binding");
    }

    #[test]
    fn test_render_model_picker() {
        let mut app = TuiApp::default();
        app.model_picker_active = true;
        app.status = crate::tui::StatusData::default();

        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("gpt-4o"), "Should show models: got {cell_text:.80}");
    }
}
