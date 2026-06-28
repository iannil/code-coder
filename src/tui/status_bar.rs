/// ─── Status Bar ────────────────────────────────────────────────────────────
///
/// 底部三栏式状态栏（类 tmux）：
///   左：模型名 (Cyan Bold)
///   中：工具+耗时+轮次 (Gray)，仅 busy 时显示
///   右：CWD+token (Dark Gray)

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::StatusData;

/// Render status bar as three-column tmux-style bar
pub fn render(frame: &mut Frame, area: Rect, status: &StatusData, frame_count: u64) {
    let spinner = if status.agent_busy {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (frame_count / 3) as usize % frames.len();
        frames[idx].to_string()
    } else {
        String::new()
    };

    let total_width = area.width as usize;

    // Left column: model name
    let left = Span::styled(
        &status.model,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );

    // Right column: cwd + token count
    let cwd = compact_cwd(&status.cwd);
    let right_str = if status.token_count > 0 {
        format!("{}  {}t", cwd, status.token_count)
    } else {
        cwd
    };
    let right = Span::styled(
        &right_str,
        Style::default().fg(Color::DarkGray),
    );

    // Center column: tool + elapsed + round (only when busy)
    let center_str = if status.agent_busy {
        let mut parts = Vec::new();
        if !spinner.is_empty() {
            parts.push(spinner);
        }
        if let Some(ref tool) = status.current_tool {
            parts.push(format!("{}", tool));
        }
        if status.elapsed_secs > 0 {
            parts.push(format!("{}s", status.elapsed_secs));
        }
        if status.current_round > 0 {
            parts.push(format!("R{}", status.current_round));
        }
        parts.join(" ")
    } else {
        String::new()
    };
    let center = Span::styled(
        &center_str,
        Style::default().fg(Color::DarkGray),
    );

    // Build line with left/center/right alignment
    let left_len = left.content.len();
    let right_len = right.content.len();
    let center_len = center.content.len();

    let left_pad = 1;
    let needed = left_len + left_pad + center_len + 2 + right_len;

    let spans = if needed < total_width {
        // Three columns with padding
        let mid_gap = total_width - left_len - right_len;
        let _center_start = left_len + left_pad;
        let fill_mid = if center_len > 0 {
            let before = (mid_gap - center_len) / 2;
            let after = mid_gap - center_len - before;
            vec![
                Span::raw(" ".repeat(before.saturating_sub(left_pad))),
                center,
                Span::raw(" ".repeat(after)),
            ]
        } else {
            vec![Span::raw(" ".repeat(mid_gap.saturating_sub(left_pad)))]
        };
        vec![
            left,
            Span::raw(" "),
        ]
        .into_iter()
        .chain(fill_mid)
        .chain(vec![
            Span::raw(" ".repeat(1)),
            right,
        ])
        .collect()
    } else {
        // Overflow: just show left + truncated center
        vec![
            left,
            Span::raw(" "),
            center,
        ]
    };

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

/// Compact cwd: show only basename, or full path if it's short
pub(crate) fn compact_cwd(cwd: &str) -> String {
    if cwd.len() <= 20 {
        return cwd.to_string();
    }
    // Show last 2 components
    let path = std::path::Path::new(cwd);
    let components: Vec<_> = path.components().collect();
    if components.len() >= 2 {
        let last = components
            .last()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_default();
        let second_last = components
            .get(components.len().saturating_sub(2))
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_default();
        format!("…{}/{}", second_last, last)
    } else {
        path.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

/// 格式化 context 用量进度条
pub(crate) fn format_context_bar(pct: f32) -> String {
    let bar_width = 8;
    let filled = (pct * bar_width as f32).round() as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    let bar: String = format!(
        "{}{}",
        "▓".repeat(filled),
        "░".repeat(empty)
    );

    format!("[{}] {:.0}%", bar, pct * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_context_bar_zero() {
        let bar = format_context_bar(0.0);
        assert_eq!(bar, "[░░░░░░░░] 0%");
    }

    #[test]
    fn test_format_context_bar_full() {
        let bar = format_context_bar(1.0);
        assert_eq!(bar, "[▓▓▓▓▓▓▓▓] 100%");
    }

    #[test]
    fn test_format_context_bar_half() {
        let bar = format_context_bar(0.5);
        assert_eq!(bar, "[▓▓▓▓░░░░] 50%");
    }

    #[test]
    fn test_format_context_bar_quarter() {
        let bar = format_context_bar(0.25);
        assert_eq!(bar, "[▓▓░░░░░░] 25%");
    }

    #[test]
    fn test_format_context_bar_overflow() {
        let bar = format_context_bar(1.5);
        assert_eq!(bar, "[▓▓▓▓▓▓▓▓] 150%");
    }

    #[test]
    fn test_format_context_bar_tiny() {
        let bar = format_context_bar(0.01);
        assert_eq!(bar, "[░░░░░░░░] 1%");
    }

    #[test]
    fn test_format_context_bar_rounding_boundary() {
        let bar = format_context_bar(0.125);
        // 0.125 * 8 = 1.0 → 1 filled
        assert!(bar.contains("▓"));
        assert_eq!(bar, "[▓░░░░░░░] 12%");
    }

    #[test]
    fn test_render_status_bar_busy() {
        let status = StatusData {
            model: "gpt-4o".into(),
            cwd: "/home/user/project".into(),
            context_pct: 0.5,
            token_count: 1234,
            api_key_set: true,
            agent_busy: true,
            current_tool: Some("search_web".into()),
            connection_type: "OpenAI".into(),
            elapsed_secs: 42,
            current_round: 3,
        };

        let mut app = crate::tui::TuiApp {
            status,
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            input_history: Vec::new(),
            history_pos: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            completion: crate::tui::CompletionState::default(),
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
            available_models: vec![],
            permission_pending: None,
            dark_mode: true,
            selected_msg: None,
            slash_completion: crate::tui::SlashCompletionState::default(),
            help_active: false,
            cached_lines: Vec::new(),
            cached_msg_count: 0,
            cached_search_query: String::new(),
            thinking_start_time: None,
            current_round: 0,
            should_quit: false,
            session_store: None,
            current_session_id: None,
            config_store: None,
            mcp_registry: None,
        };

        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 80, 1);
                render(f, area, &app.status, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_count = buffer.content.len();
        assert!(cell_count > 0);
        // Busy state should show elapsed time
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(cell_text.contains("42s"), "Should show elapsed time: got {cell_text:?}");
        assert!(cell_text.contains("R3"), "Should show round: got {cell_text:?}");
        assert!(cell_text.contains("search_web"), "Should show current tool: got {cell_text:?}");
    }

    #[test]
    fn test_render_status_bar_idle() {
        let status = StatusData {
            model: "claude-sonnet-4".into(),
            cwd: "/tmp".into(),
            context_pct: 0.25,
            token_count: 500,
            api_key_set: false,
            agent_busy: false,
            current_tool: None,
            connection_type: "api".into(),
            elapsed_secs: 0,
            current_round: 0,
        };

        let backend = ratatui::backend::TestBackend::new(60, 1);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 60, 1);
                render(f, area, &status, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let cell_text: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Idle state should show model name
        assert!(cell_text.contains("claude"), "Should show model: got {cell_text:?}");
        // Should show cwd path
        assert!(cell_text.contains("tmp"), "Should show cwd: got {cell_text:?}");
    }
}
