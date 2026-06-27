/// ─── Status Bar ────────────────────────────────────────────────────────────
///
/// 底部状态栏：模型名 / cwd / context 用量% / token 计数 / API key 状态
/// 参考 Claude Code 的 StatusLine — 极简、紧凑、全 dimmed

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::StatusData;

/// 渲染状态栏
pub fn render(frame: &mut Frame, area: Rect, status: &StatusData, frame_count: u64) {
    let spinner = if status.agent_busy {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (frame_count / 3) as usize % frames.len();
        format!(" {}", frames[idx])
    } else {
        String::new()
    };

    let context_bar = format_context_bar(status.context_pct);
    let key_indicator = if status.api_key_set {
        "🔑"
    } else {
        "⚠"
    };

    let line = Line::from(vec![
        // Model name — cyan highlight
        Span::styled(
            &status.model,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        // Separator
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        // CWD (compact: basename only)
        Span::styled(
            compact_cwd(&status.cwd),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        // Context bar
        Span::styled(context_bar, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        // Token count
        Span::styled(
            format!("{}t", status.token_count),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        // API key / spinner (pushed by filler)
        Span::styled(key_indicator, Style::default().fg(Color::DarkGray)),
        Span::styled(spinner, Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

/// Compact cwd: show only basename, or full path if it's short
fn compact_cwd(cwd: &str) -> String {
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
fn format_context_bar(pct: f32) -> String {
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
