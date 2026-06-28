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
    let popup_width = area.width.min(50).max(30);
    let popup_height = (app.slash_completion.commands.len() as u16 + 2).min(14);
    let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
    let popup_y = input_area.y.saturating_sub(popup_height + 1);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let items: Vec<Line> = app.slash_completion.commands.iter()
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
    let panel_width = area.width.min(55).max(35);
    let panel_height = area.height.min(22).max(10);
    let panel_x = area.x + (area.width.saturating_sub(panel_width)) / 2;
    let panel_y = area.y + (area.height.saturating_sub(panel_height)) / 2;
    let panel_area = Rect::new(panel_x, panel_y, panel_width, panel_height);

    let help_lines = vec![
        Line::styled(" Shortcuts ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("Enter         ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Send")]),
        Line::from(vec![Span::raw("  "), Span::styled("↑/↓           ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("History / navigate")]),
        Line::from(vec![Span::raw("  "), Span::styled("PgUp/PgDn     ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Scroll")]),
        Line::from(vec![Span::raw("  "), Span::styled("Ctrl+F        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Search")]),
        Line::from(vec![Span::raw("  "), Span::styled("Ctrl+R        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Reverse search")]),
        Line::from(vec![Span::raw("  "), Span::styled("Ctrl+P        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Switch model")]),
        Line::from(vec![Span::raw("  "), Span::styled("Alt+Enter     ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("New line")]),
        Line::from(""),
        Line::styled(" Commands ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("/help /exit   ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Help / Quit")]),
        Line::from(vec![Span::raw("  "), Span::styled("/clear        ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Clear chat")]),
        Line::from(vec![Span::raw("  "), Span::styled("/model /tools ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::raw("Model / Tools")]),
        Line::from(""),
        Line::styled(" Esc to close ", Style::default().fg(Color::DarkGray)),
    ];

    frame.render_widget(ratatui::widgets::Clear, panel_area);
    let panel = Paragraph::new(help_lines)
        .block(Block::default()
            .title(" Help ")
            .title_alignment(ratatui::layout::Alignment::Left));
    frame.render_widget(panel, panel_area);
}

/// ─── Dialog (Permission / Plan / Ask) ───────────────────────────────────────

fn render_dialog(frame: &mut Frame, area: Rect, dialog: &Dialog) {
    let (tool_name, tool_input_txt, risk) = match dialog {
        Dialog::ToolPermission { tool_name, tool_input, risk, .. } => (tool_name.as_str(), tool_input.as_str(), risk.as_str()),
        Dialog::PlanApproval { title: _, plan, .. } => ("plan", plan.as_str(), "plan approval"),
        Dialog::AskQuestion { question, .. } => ("ask_user", question.as_str(), "user question"),
        Dialog::PlanReview { title: _, plan, .. } => ("plan_review", plan.as_str(), "plan review"),
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
        _ => {
            content.push(Line::styled(
                " Y=allow  N=deny  A=always-allow  Esc=cancel ",
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
pub fn handle_dialog_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<crate::agent::AgentCommand>) {
    use crossterm::event::KeyCode;
    use crate::agent::AgentCommand;

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse { request_id, allowed: true });
                    }
                    Dialog::PlanApproval { request_id, .. } | Dialog::PlanReview { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "approved".into() });
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "yes".into() });
                    }
                }
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if let Some(dialog) = app.dialog.take() {
                let tool_name = match dialog {
                    Dialog::ToolPermission { ref tool_name, request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse { request_id, allowed: true });
                        tool_name.clone()
                    }
                    Dialog::PlanApproval { request_id, .. } | Dialog::PlanReview { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "approved".into() });
                        "plan".into()
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "yes".into() });
                        "ask_user".into()
                    }
                };
                app.messages.push(super::MessageItem::System {
                    text: format!("✓ {} will be allowed without prompting for this session", tool_name),
                });
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse { request_id, allowed: false });
                    }
                    Dialog::PlanApproval { request_id, .. } | Dialog::PlanReview { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "rejected".into() });
                    }
                    Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "no".into() });
                    }
                }
            }
        }
        KeyCode::Esc => {
            if let Some(dialog) = app.dialog.take() {
                match dialog {
                    Dialog::ToolPermission { request_id, .. } | Dialog::AskQuestion { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PermissionResponse { request_id, allowed: false });
                    }
                    Dialog::PlanApproval { request_id, .. } | Dialog::PlanReview { request_id, .. } => {
                        let _ = cmd_tx.send(AgentCommand::PlanDecision { request_id, decision: "rejected".into() });
                    }
                }
            }
        }
        KeyCode::Enter => {
            // AskQuestion: submit typed answer
            if let Some(dialog) = app.dialog.take() {
                if let Dialog::AskQuestion { question, request_id } = dialog {
                    if !app.input.is_empty() {
                        let answer = app.input.trim().to_string();
                        app.messages.push(super::MessageItem::User { text: answer.clone() });
                        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer });
                        app.input.clear();
                        app.cursor_pos = 0;
                    } else {
                        // Empty answer → put question in dialog back
                        app.dialog = Some(Dialog::AskQuestion { question, request_id });
                    }
                } else {
                    // Non-ask dialog: put it back
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

    match key.code {
        KeyCode::Up => {
            if app.slash_completion.selected > 0 {
                app.slash_completion.selected -= 1;
            }
        }
        KeyCode::Down => {
            let max_idx = app.slash_completion.commands.len().saturating_sub(1);
            if app.slash_completion.selected < max_idx {
                app.slash_completion.selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(cmd) = app.slash_completion.commands.get(app.slash_completion.selected) {
                app.input = cmd.to_string();
                app.cursor_pos = app.input.len();
                crate::tui::input_area::send_message(app, cmd_tx);
            }
            app.slash_completion.active = false;
        }
        KeyCode::Tab => {
            app.slash_completion.selected = (app.slash_completion.selected + 1)
                % app.slash_completion.commands.len();
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

    // ── Dialog key: Y/N/A/Esc ─────────────────────────────────────────────

    #[test]
    fn test_dialog_y_permits_tool() {
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
            AgentCommand::PermissionResponse { allowed, .. } => assert!(allowed),
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

        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("Shortcuts"), "Should show shortcuts: got {cell_text:.80}");
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
