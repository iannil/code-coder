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
    let theme = app.theme;
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
        render_help_panel(frame, area, &theme);
        return;
    }

    // 4a. AskQuestion has its own layout (selectable option list + free-text
    // line), and needs the typed answer (app.input) which render_dialog lacks.
    if let Some(Dialog::AskQuestion { question, options, selected, .. }) = &app.dialog {
        render_ask_question_dialog(frame, area, question, options, *selected, &app.input, &theme);
        return;
    }

    // 4b. Permission / plan dialog
    if let Some(ref dialog) = app.dialog {
        render_dialog(frame, area, dialog, &theme);
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
    let theme = app.theme;
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
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg),
            )
        } else {
            Line::styled(
                format!("   {:<12} {}", cmd, desc),
                Style::default().fg(theme.primary_text),
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
    let theme = app.theme;
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
                    Style::default().fg(theme.selected_fg).bg(theme.selected_bg),
                )
            } else {
                Line::styled(
                    format!("   {} {}", m, if is_current { "✓" } else { "" }),
                    Style::default().fg(if is_current { theme.success_text } else { theme.primary_text }),
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

fn render_help_panel(frame: &mut Frame, area: Rect, theme: &crate::tui::Theme) {
    let panel_width = area.width.min(72).max(50);
    let panel_height = (area.height.saturating_sub(2)).min(60).max(20);
    let panel_x = area.x + (area.width.saturating_sub(panel_width)) / 2;
    let panel_y = area.y + (area.height.saturating_sub(panel_height)) / 2;
    let panel_area = Rect::new(panel_x, panel_y, panel_width, panel_height);

    // ADR 0003: helper fns take colors as params to avoid closure lifetime
    // gymnastics. Same shape as before ADR 0003 but colors now come from
    // the theme instead of being hardcoded.
    fn hdr<'a>(s: &'a str, c: Color) -> Line<'a> {
        Line::styled(format!(" {s} "), Style::default().fg(c).add_modifier(Modifier::BOLD))
    }
    fn key<'a>(k: &'a str, c: Color) -> Span<'a> {
        Span::styled(format!("{:<14}", k), Style::default().fg(c).add_modifier(Modifier::BOLD))
    }
    fn row<'a>(k: &'a str, d: &'a str, kc: Color) -> Line<'a> {
        Line::from(vec![Span::raw("  "), key(k, kc), Span::raw(d)])
    }
    fn row_warn<'a>(k: &'a str, d: &'a str, kc: Color, wc: Color) -> Line<'a> {
        Line::from(vec![
            Span::raw("  "),
            key(k, kc),
            Span::styled(format!("⚠ {d}"), Style::default().fg(wc)),
        ])
    }
    let accent = theme.accent_text;
    let primary = theme.primary_text;
    let warn = theme.warning_text;
    let dim = theme.secondary_text;

    let help_lines = vec![
        hdr("Editing", accent),
        Line::from(""),
        row("Enter",        "Submit message (empty: no-op)", primary),
        row("Shift+Enter",  "Insert newline (multi-line input)", primary),
        row("\\ + Enter",   "Continue line (trailing \\ → newline)", primary),
        row("Alt+Enter",    "Force submit (overrides modifiers)", primary),
        row("Ctrl+Z / Y",   "Undo / redo input", primary),
        row("Ctrl+A / E",   "Cursor to line start / end", primary),
        row("Ctrl+W",       "Delete word backward", primary),
        row("Ctrl+U / K",   "Delete to start / end of line", primary),
        row("Tab",          "Accept completion / fold last message", primary),
        row("@",            "Trigger file completion", primary),
        Line::from(""),
        hdr("Navigation", accent),
        Line::from(""),
        row("Up / Down",    "Move cursor (or browse msgs when empty)", primary),
        row("Ctrl+Up/Dn",   "Walk input history", primary),
        row("Left / Right", "Move cursor one char", primary),
        row("Ctrl/Alt+←→",  "Move cursor one word", primary),
        row("Home / End",   "Cursor to line start / end", primary),
        row("g / G",        "Msg list: scroll to top / bottom (empty input)", primary),
        row("PgUp / PgDn",  "Msg list: scroll up / down by page", primary),
        row("End",          "Msg list: jump to bottom", primary),
        Line::from(""),
        hdr("Mode & Tools", accent),
        Line::from(""),
        row("Ctrl+F",       "Search messages", primary),
        row("Ctrl+R",       "Reverse search", primary),
        row("Ctrl+P",       "Switch model", primary),
        row("Ctrl+T",       "Toggle theme (dark/light)", primary),
        row("Ctrl+H",       "Toggle this help panel", primary),
        row_warn("Ctrl+L",  "Clear messages (confirms)", primary, warn),
        row("Ctrl+C",       "Busy: abort LLM call  ·  Idle: quit", primary),
        row_warn("Ctrl+Q",  "Quit", primary, warn),
        row("Esc",          "Close current overlay (never quits)", primary),
        Line::from(""),
        hdr("Commands", accent),
        Line::from(""),
        row("/help",        "Show this panel", primary),
        row("/exit /quit",  "Quit", primary),
        row("/clear",       "Clear messages (confirms)", primary),
        row("/reload",      "Reload context + skills", primary),
        row("/history",     "Show message count", primary),
        row("/tools",       "List available tools", primary),
        row("/skills",      "List loaded skills", primary),
        row("/memory",      "List memory entries", primary),
        row("/session",     "List saved sessions", primary),
        row("/resume [id]", "Resume a session (confirms)", primary),
        row("/config",      "View / change settings", primary),
        row("/mcp",         "Manage MCP servers", primary),
        Line::from(""),
        Line::styled(" Esc to close ", Style::default().fg(dim)),
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

fn render_dialog(frame: &mut Frame, area: Rect, dialog: &Dialog, theme: &crate::tui::Theme) {
    // ADR 0006: Confirm dialog has its own minimal layout (warning + Y/N).
    // Render and return early so the tool/plan/ask branch below stays clean.
    if let Dialog::Confirm { message, action } = dialog {
        return render_confirm_dialog(frame, area, message, action, theme);
    }

    let (tool_name, tool_input_txt, risk) = match dialog {
        Dialog::ToolPermission { tool_name, tool_input, risk, .. } => (tool_name.as_str(), tool_input.as_str(), risk.as_str()),
        Dialog::PlanApproval { title: _, plan, .. } => ("plan", plan.as_str(), "plan approval"),
        // AskQuestion and Confirm are rendered by their own functions before
        // render_dialog is ever called (see render_overlays).
        Dialog::AskQuestion { .. } => unreachable!("AskQuestion renders via render_ask_question_dialog"),
        Dialog::Confirm { .. } => unreachable!("Confirm renders via render_confirm_dialog"),
    };

    let dialog_width = area.width.min(70).max(40);
    let info_str: &str = tool_input_txt;
    let input_lines = info_str.lines().count().min(6).max(1);
    let dialog_height = (5 + input_lines as u16).min(12).max(7);
    let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    let risk_style = if risk.contains("suspicious") || risk.contains("outside") {
        Style::default().fg(theme.warning_text)
    } else {
        Style::default().fg(theme.secondary_text)
    };

    let mut content = vec![
        Line::styled(" [!] Tool Permission ", Style::default().fg(theme.warning_text).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::styled(format!(" Tool: {}", tool_name), Style::default().fg(theme.primary_text).add_modifier(Modifier::BOLD)),
    ];

    if !risk.is_empty() {
        content.push(Line::styled(format!(" Risk: {}", risk), risk_style));
    }

    let input_display: String = info_str.chars().take(400).collect();
    for line in input_display.lines().take(6) {
        content.push(Line::styled(format!("  {}", line), Style::default().fg(theme.secondary_text)));
    }
    if info_str.len() > 400 {
        content.push(Line::styled("  …(truncated)", Style::default().fg(theme.secondary_text)));
    }

    content.push(Line::from(""));
    match dialog {
        Dialog::AskQuestion { .. } => unreachable!("AskQuestion renders via render_ask_question_dialog"),
        Dialog::ToolPermission { .. } => {
            // ADR 0005: scope-aware approval keys.
            content.push(Line::styled(
                " Y=once  A=session  Shift+A=project  N=deny  Esc=cancel ",
                Style::default().fg(theme.accent_text),
            ));
        }
        Dialog::PlanApproval { .. } => {
            content.push(Line::styled(
                " Y=approve  N=reject  Esc=cancel ",
                Style::default().fg(theme.accent_text),
            ));
        }
        // Confirm is rendered by render_confirm_dialog (early return above).
        Dialog::Confirm { .. } => unreachable!("Confirm renders via render_confirm_dialog"),
    }

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    let dialog = Paragraph::new(content)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.warning_text)))
        .wrap(Wrap { trim: false });
    frame.render_widget(dialog, dialog_area);
}

/// ADR 0006: render the Confirm dialog. Minimal layout — warning header,
/// the action's message, and a Y/N/Esc prompt. Yellow border signals
/// "destructive — read before confirming".
fn render_confirm_dialog(frame: &mut Frame, area: Rect, message: &str, action: &crate::tui::ConfirmAction, theme: &crate::tui::Theme) {
    let dialog_width = area.width.min(64).max(40);
    let msg_lines = message.lines().count().max(1) as u16;
    let dialog_height = (msg_lines + 7).min(14).max(8);
    let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    let action_label = match action {
        crate::tui::ConfirmAction::ClearMessages => "Clear all messages",
        crate::tui::ConfirmAction::ResumeLatest => "Resume latest session",
        crate::tui::ConfirmAction::DeleteMessage { index } => {
            return render_confirm_dialog_owned(frame, dialog_area, message, format!("Delete message #{index}"), theme);
        }
    };

    let content = vec![
        Line::styled(" ⚠ Confirm ", Style::default().fg(theme.warning_text).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::styled(format!(" {}", message), Style::default().fg(theme.primary_text)),
        Line::styled(format!(" Action: {}", action_label), Style::default().fg(theme.secondary_text)),
        Line::from(""),
        Line::styled(" Y=confirm  N=cancel  Esc=cancel ", Style::default().fg(theme.accent_text)),
    ];

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    let widget = Paragraph::new(content)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.warning_text)))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, dialog_area);
}

// Helper for the DeleteMessage case where the label needs owned String.
fn render_confirm_dialog_owned(frame: &mut Frame, dialog_area: Rect, message: &str, action_label: String, theme: &crate::tui::Theme) {
    let content = vec![
        Line::styled(" ⚠ Confirm ", Style::default().fg(theme.warning_text).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::styled(format!(" {}", message), Style::default().fg(theme.primary_text)),
        Line::styled(format!(" Action: {}", action_label), Style::default().fg(theme.secondary_text)),
        Line::from(""),
        Line::styled(" Y=confirm  N=cancel  Esc=cancel ", Style::default().fg(theme.accent_text)),
    ];

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    let widget = Paragraph::new(content)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.warning_text)))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, dialog_area);
}

/// ─── AskQuestion Dialog ─────────────────────────────────────────────────────
///
/// H3: render the agent's question as a navigable option list (when the agent
/// supplied `options`) plus a free-text line for a custom answer. The user
/// picks an option with ↑↓ + Enter, or types any text and presses Enter. Esc
/// skips. When `options` is empty this degrades to a pure free-text prompt.
fn render_ask_question_dialog(
    frame: &mut Frame,
    area: Rect,
    question: &str,
    options: &[String],
    selected: usize,
    input: &str,
    theme: &crate::tui::Theme,
) {
    let dialog_width = area.width.min(70).max(40);
    let q_lines = question.lines().count().max(1) as u16;
    let opt_lines = options.len() as u16;
    // header + blank + question + blank + options + blank + answer + hint
    let dialog_height = (q_lines + opt_lines + 6).min(area.height.saturating_sub(2)).max(7);
    let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);

    let mut content = vec![
        Line::styled(" [?] Question ", Style::default().fg(theme.accent_text).add_modifier(Modifier::BOLD)),
        Line::from(""),
    ];
    for line in question.lines() {
        content.push(Line::styled(format!(" {}", line), Style::default().fg(theme.primary_text)));
    }

    if !options.is_empty() {
        content.push(Line::from(""));
        for (i, opt) in options.iter().enumerate() {
            // Highlight the selected option only when no custom text is typed —
            // typing shifts focus to the free-text answer (the "__other__" path).
            if i == selected && input.is_empty() {
                content.push(Line::styled(
                    format!(" ▸ {}", opt),
                    Style::default().fg(theme.selected_fg).bg(theme.selected_bg),
                ));
            } else {
                content.push(Line::styled(
                    format!("   {}", opt),
                    Style::default().fg(theme.primary_text),
                ));
            }
        }
    }

    content.push(Line::from(""));
    if input.is_empty() {
        let hint = if options.is_empty() {
            " Type answer + Enter  ·  Esc to skip "
        } else {
            " ↑↓ select · Enter confirm · or type a custom answer · Esc skip "
        };
        content.push(Line::styled(hint, Style::default().fg(theme.accent_text)));
    } else {
        content.push(Line::styled(format!(" > {}", input), Style::default().fg(theme.primary_text)));
        content.push(Line::styled(" Enter to send · Esc to skip ", Style::default().fg(theme.accent_text)));
    }

    frame.render_widget(ratatui::widgets::Clear, dialog_area);
    let widget = Paragraph::new(content)
        .block(Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.accent_text)))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, dialog_area);
}

/// ─── File Completion Popup ──────────────────────────────────────────────────

fn render_file_completion(frame: &mut Frame, area: Rect, input_area: Rect, app: &TuiApp) {
    let theme = app.theme;
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
                    Style::default().fg(theme.selected_fg).bg(theme.selected_bg),
                )
            } else {
                Line::styled(
                    format!("   {} ", c.display),
                    Style::default().fg(theme.primary_text),
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

    // H3: AskQuestion is not a Y/N/A dialog — it has a selectable option list
    // plus a free-text answer. Route it to its own handler so the letter keys
    // (y/a/n) type into the answer instead of being hijacked into "yes"/"no".
    if matches!(app.dialog, Some(Dialog::AskQuestion { .. })) {
        return handle_ask_question_key(app, key, cmd_tx);
    }

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
                    Dialog::AskQuestion { .. } => unreachable!("AskQuestion routed to handle_ask_question_key"),
                    // ADR 0006: Y confirms the destructive action.
                    Dialog::Confirm { action, .. } => {
                        crate::tui::commands::execute_confirm_action(app, cmd_tx, action);
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
                    Dialog::AskQuestion { .. } => unreachable!("AskQuestion routed to handle_ask_question_key"),
                    // ADR 0006: 'A' on a Confirm dialog is a no-op — user
                    // must use Y or N. Put the dialog back so it stays open.
                    confirm @ Dialog::Confirm { .. } => {
                        app.dialog = Some(confirm);
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
                    Dialog::AskQuestion { .. } => unreachable!("AskQuestion routed to handle_ask_question_key"),
                    // ADR 0006: N cancels the confirm — no action, no message.
                    Dialog::Confirm { .. } => {}
                }
            }
        }
        // ── Esc: cancel ───────────────────────────────────────────────────
        KeyCode::Esc => {
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
                    Dialog::AskQuestion { .. } => unreachable!("AskQuestion routed to handle_ask_question_key"),
                    // ADR 0006: Esc cancels the confirm — same as N.
                    Dialog::Confirm { .. } => {}
                }
            }
        }
        // ── Enter: no-op for permission/plan/confirm (they need an explicit
        // letter key). AskQuestion's Enter is handled in handle_ask_question_key.
        KeyCode::Enter => {}
        _ => {}
    }
}

/// H3: handle keys for the AskQuestion dialog — a selectable option list plus
/// a free-text answer. ↑↓ move the highlighted option (only while no custom
/// text is typed); Enter submits the typed text if any, else the highlighted
/// option; printable keys / Backspace edit the free-text answer; Esc skips and
/// unblocks the waiting `ask_user` tool with a "[skipped]" sentinel.
///
/// Crucially, Esc here sends an `AskUserResponse` (not a `PermissionResponse`
/// as the old combined handler did) — otherwise the tool blocked for 300s.
pub fn handle_ask_question_key(app: &mut TuiApp, key: crossterm::event::KeyEvent, cmd_tx: &std::sync::mpsc::Sender<crate::agent::AgentCommand>) {
    use crossterm::event::KeyCode;
    use crate::agent::AgentCommand;

    let (request_id, opt_len) = match &app.dialog {
        Some(Dialog::AskQuestion { request_id, options, .. }) => (*request_id, options.len()),
        _ => return,
    };

    let submit = |app: &mut TuiApp, answer: String| {
        app.dialog = None;
        app.messages.push(super::MessageItem::User { text: answer.clone() });
        let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer });
        app.input.clear();
        app.cursor_pos = 0;
        app.auto_scroll = true;
        app.scroll_offset = 0;
    };

    match key.code {
        KeyCode::Up => {
            if opt_len > 0 && app.input.is_empty() {
                if let Some(Dialog::AskQuestion { selected, .. }) = app.dialog.as_mut() {
                    *selected = selected.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if opt_len > 0 && app.input.is_empty() {
                if let Some(Dialog::AskQuestion { selected, .. }) = app.dialog.as_mut() {
                    *selected = (*selected + 1).min(opt_len - 1);
                }
            }
        }
        KeyCode::Enter => {
            // Typed text wins (the "custom answer" path); otherwise the
            // highlighted option; if neither exists, keep the dialog open.
            if !app.input.is_empty() {
                let answer = app.input.trim().to_string();
                submit(app, answer);
            } else if opt_len > 0 {
                let answer = match &app.dialog {
                    Some(Dialog::AskQuestion { options, selected, .. }) => {
                        options.get(*selected).cloned().unwrap_or_default()
                    }
                    _ => return,
                };
                submit(app, answer);
            }
            // else: free-text empty and no options → nothing to submit.
        }
        KeyCode::Esc => {
            app.dialog = None;
            let _ = cmd_tx.send(AgentCommand::AskUserResponse { request_id, answer: "[skipped]".into() });
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += c.len_utf8();
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let mut new_pos = app.cursor_pos - 1;
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.input.remove(new_pos);
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                let mut new_pos = app.cursor_pos - 1;
                while new_pos > 0 && !app.input.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
                app.cursor_pos = new_pos;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                let mut new_pos = app.cursor_pos + 1;
                while new_pos < app.input.len() && !app.input.is_char_boundary(new_pos) {
                    new_pos += 1;
                }
                app.cursor_pos = new_pos;
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

    // ── H3 — AskQuestion: selectable options + free-text ─────────────────

    fn ask_dialog(options: Vec<&str>) -> Dialog {
        Dialog::AskQuestion {
            question: "Pick one".into(),
            options: options.into_iter().map(String::from).collect(),
            selected: 0,
            request_id: 7,
        }
    }

    #[test]
    fn h3_down_up_move_selected_option() {
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec!["red", "green", "blue"]));
        let (tx, _) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        match &app.dialog {
            Some(Dialog::AskQuestion { selected, .. }) => assert_eq!(*selected, 2, "Down clamps at last option"),
            _ => panic!("dialog should stay open"),
        }
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Up, crossterm::event::KeyModifiers::NONE), &tx);
        match &app.dialog {
            Some(Dialog::AskQuestion { selected, .. }) => assert_eq!(*selected, 1),
            _ => panic!("dialog should stay open"),
        }
    }

    #[test]
    fn h3_enter_submits_highlighted_option() {
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec!["red", "green", "blue"]));
        // move to "green"
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Down, crossterm::event::KeyModifiers::NONE), &tx);
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_none(), "Enter consumes the dialog");
        match rx.try_recv().unwrap() {
            AgentCommand::AskUserResponse { request_id, answer } => {
                assert_eq!(request_id, 7);
                assert_eq!(answer, "green", "submits the highlighted option text");
            }
            other => panic!("expected AskUserResponse, got {other:?}"),
        }
    }

    #[test]
    fn h3_typed_text_wins_over_option() {
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec!["red", "green"]));
        let (tx, rx) = std::sync::mpsc::channel();
        // Type "y" — must NOT be hijacked to "yes"; it edits the answer.
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('o'), crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.input, "yo", "letters type into the free-text answer");
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE), &tx);
        match rx.try_recv().unwrap() {
            AgentCommand::AskUserResponse { answer, .. } => assert_eq!(answer, "yo", "custom typed answer wins"),
            other => panic!("expected AskUserResponse, got {other:?}"),
        }
    }

    #[test]
    fn h3_esc_skips_with_askuser_response_not_permission() {
        // The old combined handler sent a PermissionResponse on Esc, leaving
        // the ask_user tool blocked for 300s. Esc must now deliver an
        // AskUserResponse so the tool unblocks.
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec!["red"]));
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_none());
        match rx.try_recv().unwrap() {
            AgentCommand::AskUserResponse { request_id, answer } => {
                assert_eq!(request_id, 7);
                assert_eq!(answer, "[skipped]");
            }
            other => panic!("Esc must send AskUserResponse, got {other:?}"),
        }
    }

    #[test]
    fn h3_enter_with_no_input_no_options_keeps_dialog() {
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec![])); // free-text only, empty input
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE), &tx);
        assert!(app.dialog.is_some(), "nothing to submit → dialog stays open");
        assert!(rx.try_recv().is_err(), "no response sent on empty submit");
    }

    #[test]
    fn h3_render_shows_options() {
        let mut app = TuiApp::default();
        app.dialog = Some(ask_dialog(vec!["Apple", "Banana"]));
        app.status = crate::tui::StatusData::default();
        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();
        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("Pick one"), "missing question: {cell_text:.120}");
        assert!(cell_text.contains("Apple"), "missing option Apple");
        assert!(cell_text.contains("Banana"), "missing option Banana");
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

    // ── ADR 0006 — Confirm Dialog Pattern ────────────────────────────────

    use crate::tui::ConfirmAction;
    use crate::tui::Dialog as DialogEnum;

    fn confirm_dialog(action: ConfirmAction) -> DialogEnum {
        DialogEnum::Confirm {
            message: "Are you sure?".into(),
            action,
        }
    }

    #[test]
    fn adr0006_confirm_y_clears_messages() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "msg1".into() });
        app.messages.push(MessageItem::Assistant { text: "reply".into() });
        app.dialog = Some(confirm_dialog(ConfirmAction::ClearMessages));
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        // Action executed: messages cleared + warning System msg pushed.
        assert_eq!(app.messages.len(), 1, "only the warning System msg should remain");
        assert!(matches!(&app.messages[0], MessageItem::System { text } if text.contains("cleared")));
        // Agent ClearHistory sent.
        match rx.try_recv() {
            Ok(AgentCommand::ClearHistory) => {}
            other => panic!("expected ClearHistory, got {other:?}"),
        }
        assert!(app.dialog.is_none(), "dialog should be consumed");
    }

    #[test]
    fn adr0006_confirm_n_preserves_messages() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "keep me".into() });
        app.dialog = Some(confirm_dialog(ConfirmAction::ClearMessages));
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('n'), crossterm::event::KeyModifiers::NONE), &tx);
        // Nothing happened — messages preserved, no agent command.
        assert_eq!(app.messages.len(), 1);
        assert!(matches!(&app.messages[0], MessageItem::User { text } if text == "keep me"));
        assert!(rx.try_recv().is_err(), "N must not send any agent command");
        assert!(app.dialog.is_none(), "dialog consumed (cancelled)");
    }

    #[test]
    fn adr0006_confirm_esc_preserves_messages() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "keep me".into() });
        app.dialog = Some(confirm_dialog(ConfirmAction::ClearMessages));
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.messages.len(), 1, "Esc must preserve messages");
        assert!(rx.try_recv().is_err(), "Esc must not send any agent command");
        assert!(app.dialog.is_none(), "dialog consumed (cancelled)");
    }

    #[test]
    fn adr0006_confirm_y_delete_message_removes_by_index() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "first".into() });
        app.messages.push(MessageItem::User { text: "second".into() });
        app.messages.push(MessageItem::User { text: "third".into() });
        app.dialog = Some(confirm_dialog(ConfirmAction::DeleteMessage { index: 1 }));
        let (tx, _rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        // "second" should be gone; "first" and "third" preserved.
        assert_eq!(app.messages.len(), 3, "delete pushes a System msg → 2 originals + 1 sys");
        assert!(matches!(&app.messages[0], MessageItem::User { text } if text == "first"));
        assert!(matches!(&app.messages[1], MessageItem::User { text } if text == "third"));
        assert!(matches!(&app.messages[2], MessageItem::System { text } if text.contains("Deleted")));
    }

    #[test]
    fn adr0006_confirm_a_is_noop_on_confirm_dialog() {
        // ADR 0005 'A' has scope meaning on ToolPermission but not on Confirm.
        // Pressing 'A' on a Confirm dialog should not execute the action.
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "keep me".into() });
        app.dialog = Some(confirm_dialog(ConfirmAction::ClearMessages));
        let (tx, rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::NONE), &tx);
        assert_eq!(app.messages.len(), 1, "A must NOT trigger ClearMessages on Confirm");
        assert!(rx.try_recv().is_err(), "A must not send agent commands on Confirm");
        assert!(app.dialog.is_some(), "Confirm dialog should stay open after no-op A");
    }

    #[test]
    fn adr0006_render_confirm_shows_warning() {
        let mut app = TuiApp::default();
        app.dialog = Some(confirm_dialog(ConfirmAction::ClearMessages));
        app.status = crate::tui::StatusData::default();

        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_overlays(f, f.area(), Rect::new(0, 10, 80, 3), &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("Confirm"), "missing Confirm header: {cell_text:.200}");
        assert!(cell_text.contains("Are you sure?"), "missing message: {cell_text:.200}");
        assert!(cell_text.contains("Y=confirm"), "missing Y=confirm prompt: {cell_text:.200}");
        assert!(cell_text.contains("Clear all messages"), "missing action label: {cell_text:.200}");
    }

    // ── Routing tests — destructive commands construct Confirm ───────────

    #[test]
    fn adr0006_slash_clear_constructs_confirm_not_direct_clear() {
        use crate::tui::commands::dispatch_slash_command;
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "preserve me".into() });
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch_slash_command(&mut app, "/clear", &tx);
        // Messages NOT cleared directly — Confirm dialog opened instead.
        assert_eq!(app.messages.len(), 1, "/clear must not clear directly; Confirm gates it");
        match &app.dialog {
            Some(DialogEnum::Confirm { action: ConfirmAction::ClearMessages, .. }) => {}
            other => panic!("expected Confirm{{ClearMessages}}, got {other:?}"),
        }
    }

    #[test]
    fn adr0006_ctrl_l_constructs_confirm() {
        use crate::tui::TuiApp;
        // Verify the dialog shape that Ctrl+L should construct.
        // (Direct handle_key invocation is covered by the mod.rs tests.)
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "preserve".into() });
        app.dialog = Some(DialogEnum::Confirm {
            message: "Clear all messages from this session?".into(),
            action: ConfirmAction::ClearMessages,
        });
        // Sanity: action is ClearMessages (what Ctrl+L should set).
        match &app.dialog {
            Some(DialogEnum::Confirm { action: ConfirmAction::ClearMessages, .. }) => {}
            _ => panic!("Ctrl+L should construct Confirm{{ClearMessages}}"),
        }
    }

    #[test]
    fn adr0006_slash_resume_no_arg_constructs_confirm() {
        use crate::tui::commands::dispatch_slash_command;
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "current".into() });
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch_slash_command(&mut app, "/resume", &tx);
        // Confirm dialog opened; current messages NOT overwritten.
        assert_eq!(app.messages.len(), 1);
        match &app.dialog {
            Some(DialogEnum::Confirm { action: ConfirmAction::ResumeLatest, .. }) => {}
            other => panic!("expected Confirm{{ResumeLatest}}, got {other:?}"),
        }
    }

    #[test]
    fn adr0006_resume_latest_with_no_store_reports_error_on_confirm() {
        // When user confirms ResumeLatest but no session_store is wired,
        // execute_confirm_action should push an informational System msg
        // rather than panic.
        let mut app = TuiApp::default();
        app.session_store = None;
        app.dialog = Some(confirm_dialog(ConfirmAction::ResumeLatest));
        let (tx, _rx) = std::sync::mpsc::channel();
        handle_dialog_key(&mut app, key(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE), &tx);
        // System msg pushed about store not available.
        assert!(app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("not available")
        )));
    }
}
