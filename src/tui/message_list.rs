/// ─── Message List Component ──────────────────────────────────────────────────
///
/// 消息列表渲染、虚拟滚动、搜索高亮、行缓存。
/// 缓存使用 thread_local RefCell，单线程 TUI 安全。

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;
use std::cell::RefCell;

use super::{MessageItem, TuiApp};

/// ─── Module-Level Cache ─────────────────────────────────────────────────────

struct MessageCache {
    lines: Vec<Line<'static>>,
    msg_count: usize,
    search_query: String,
}

impl Default for MessageCache {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            msg_count: 0,
            search_query: String::new(),
        }
    }
}

thread_local! {
    static MSG_CACHE: RefCell<MessageCache> = const { RefCell::new(MessageCache { lines: Vec::new(), msg_count: 0, search_query: String::new() }) };
}

/// Force cache rebuild on next build_message_lines call.
pub fn invalidate_cache() {
    MSG_CACHE.with(|c| c.borrow_mut().msg_count = 0);
}

/// ─── Display Row Calculations ───────────────────────────────────────────────

/// 计算逻辑行列表在给定宽度下实际占用的显示行数（考虑折行）
fn count_display_rows(lines: &[Line<'_>], area_width: u16) -> usize {
    if area_width == 0 {
        return lines.len();
    }
    lines.iter().map(|line| {
        let w = line.width();
        if w == 0 { 1 } else { (w + area_width as usize - 1) / area_width as usize }
    }).sum()
}

/// 从末尾开始算，返回 scroll_offset 使得最后 msg_height 显示行可见
fn bottom_scroll_offset(lines: &[Line<'_>], area_width: u16, msg_height: usize) -> usize {
    if area_width == 0 || msg_height == 0 || lines.is_empty() {
        return 0;
    }
    let mut rows = 0usize;
    let mut skipped = lines.len();
    for line in lines.iter().rev() {
        let w = line.width();
        let line_rows = if w == 0 { 1 } else { (w + area_width as usize - 1) / area_width as usize };
        if rows + line_rows > msg_height {
            break;
        }
        rows += line_rows;
        skipped -= 1;
    }
    skipped
}

/// ─── Search ─────────────────────────────────────────────────────────────────

/// Count how many lines match the search query
pub fn count_search_matches(app: &mut TuiApp) {
    if app.search_query.is_empty() {
        app.search_match_count = 0;
        app.search_current_match = 0;
        return;
    }
    let query = app.search_query.to_lowercase();
    let mut count = 0;
    for m in &app.messages {
        let text = match m {
            MessageItem::User { text } => text,
            MessageItem::Assistant { text } => text,
            MessageItem::ToolCall { output, .. } => output,
            MessageItem::System { text } => text,
            MessageItem::Reasoning { text, .. } => text,
        };
        if text.to_lowercase().contains(&query) {
            count += 1;
        }
    }
    app.search_match_count = count;
}

/// Update reverse search results
pub fn update_reverse_search(app: &mut TuiApp) {
    app.reverse_search_results.clear();
    if app.reverse_search_query.is_empty() {
        return;
    }
    let query = app.reverse_search_query.to_lowercase();
    for (i, m) in app.messages.iter().enumerate() {
        let text = match m {
            MessageItem::User { text } => text,
            MessageItem::Assistant { text } => text,
            MessageItem::ToolCall { output, .. } => output,
            MessageItem::System { text } => text,
            MessageItem::Reasoning { text, .. } => text,
        };
        if text.to_lowercase().contains(&query) {
            app.reverse_search_results.push(i);
        }
    }
    if !app.reverse_search_results.is_empty() {
        app.reverse_search_idx = app.reverse_search_results.len() - 1;
        app.auto_scroll = false;
    }
}

/// ─── Line Building ──────────────────────────────────────────────────────────

/// Build the rendered message lines with caching
fn build_message_lines(app: &TuiApp) -> Vec<Line<'static>> {
    MSG_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if app.messages.len() == cache.msg_count
            && !cache.lines.is_empty()
            && cache.search_query == app.search_query
        {
            return cache.lines.clone();
        }

        let highlight = if app.search_active && !app.search_query.is_empty() {
            Some(app.search_query.as_str())
        } else {
            None
        };

        cache.lines = build_message_lines_inner(app, highlight);
        cache.msg_count = app.messages.len();
        cache.search_query = app.search_query.clone();
        cache.lines.clone()
    })
}

/// The actual rendering logic (separated for cache clarity)
fn build_message_lines_inner(app: &TuiApp, highlight: Option<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (msg_idx, m) in app.messages.iter().enumerate() {
        let is_selected = app.selected_msg == Some(msg_idx);
        let prefix_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        match m {
            MessageItem::User { text } => {
                let prefix = if is_selected { "▸ " } else { "▶ " };
                lines.push(Line::styled(
                    format!("{}{}", prefix, text.lines().next().unwrap_or("")),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
                for line in text.lines().skip(1) {
                    lines.push(Line::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                }
            }
            MessageItem::Assistant { text } => {
                let prefix = if is_selected { "▸ " } else { "▷ " };
                let md_lines = super::markdown::render_markdown_with_highlight(text, highlight);
                if let Some(first) = md_lines.first() {
                    let styled_line = Line::from(
                        std::iter::once(Span::styled(prefix, prefix_style))
                            .chain(first.spans.iter().cloned())
                            .collect::<Vec<_>>(),
                    );
                    lines.push(styled_line);
                }
                for line in md_lines.iter().skip(1) {
                    lines.push(Line::from(
                        std::iter::once(Span::raw("  "))
                            .chain(line.spans.iter().cloned())
                            .collect::<Vec<_>>(),
                    ));
                }
            }
            MessageItem::Reasoning { text, .. } => {
                for (i, line) in text.lines().enumerate() {
                    if i == 0 {
                        lines.push(Line::styled(
                            format!("· {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    } else {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            MessageItem::ToolCall { name, input, output, .. } => {
                lines.push(Line::styled(
                    format!("⚙ {}", name),
                    Style::default().fg(Color::DarkGray),
                ));
                if !input.is_empty() {
                    for line in input.lines() {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
                if !output.is_empty() {
                    for line in output.lines() {
                        lines.push(Line::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            MessageItem::System { text } => {
                let style = if text.starts_with("[end]") || text.starts_with("[error]") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                lines.push(Line::styled(format!("  {}", text), style));
            }
        }
    }
    lines
}

/// ─── Render ─────────────────────────────────────────────────────────────────

/// Render the message list area (no layout calculation — caller provides area).
pub fn render(frame: &mut Frame, area: Rect, app: &mut TuiApp, _frame_count: u64) {
    let rendered_lines = build_message_lines(app);
    let msg_height = area.height.saturating_sub(1) as usize;
    let text_width = area.width;
    let total_display_rows = count_display_rows(&rendered_lines, text_width);

    // Auto-scroll or clamp
    if app.auto_scroll {
        app.scroll_offset = bottom_scroll_offset(&rendered_lines, text_width, msg_height);
    } else {
        let max_offset = bottom_scroll_offset(&rendered_lines, text_width, msg_height);
        if app.scroll_offset > max_offset {
            app.scroll_offset = max_offset;
        }
    }

    // Scrolled display rows
    let scrolled_display_rows =
        if app.auto_scroll && total_display_rows > msg_height {
            total_display_rows - msg_height
        } else if app.scroll_offset > 0 {
            let clamped = app.scroll_offset.min(rendered_lines.len());
            count_display_rows(&rendered_lines[..clamped], text_width)
        } else {
            0
        };

    // Block title
    let msg_block = Block::default()
        .borders(ratatui::widgets::Borders::TOP)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(
            "{} CodeCoder{}{}",
            if app.status.agent_busy { "• " } else { "" },
            if !app.auto_scroll { " (↑ scroll)" } else { "" },
            if app.reverse_search_active {
                format!(
                    " \u{2315}`{}` {} hits",
                    app.reverse_search_query,
                    app.reverse_search_results.len(),
                )
            } else if app.search_active {
                format!(
                    " [search] {} ({} hits)",
                    app.search_query,
                    app.search_match_count
                )
            } else {
                String::new()
            }
        ))
        .title_alignment(ratatui::layout::Alignment::Left);

    // Virtual scrolling
    let visible_slice = if scrolled_display_rows > 0 && total_display_rows > msg_height {
        let mut logical_start = 0usize;
        let mut accumulated = 0usize;
        for (i, l) in rendered_lines.iter().enumerate() {
            let w = l.width();
            let rows = if w == 0 { 1 } else { (w + text_width as usize - 1) / text_width as usize };
            if accumulated + rows > scrolled_display_rows {
                logical_start = i;
                break;
            }
            accumulated += rows;
        }

        let mut visible_end = logical_start;
        accumulated = 0;
        for (i, l) in rendered_lines[logical_start..].iter().enumerate() {
            let w = l.width();
            let rows = if w == 0 { 1 } else { (w + text_width as usize - 1) / text_width as usize };
            if accumulated + rows > msg_height {
                visible_end = logical_start + i + 1;
                break;
            }
            accumulated += rows;
        }
        if visible_end <= logical_start {
            visible_end = rendered_lines.len().min(logical_start + msg_height);
        }
        &rendered_lines[logical_start..visible_end.min(rendered_lines.len())]
    } else {
        &rendered_lines[..]
    };

    let msg_paragraph = Paragraph::new(visible_slice.to_vec())
        .block(msg_block)
        .scroll((0, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(msg_paragraph, area);

    // Scrollbar
    if total_display_rows > msg_height {
        let thumb_color = if app.dark_mode { Color::White } else { Color::Black };
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(Style::default())
            .thumb_style(Style::default().fg(thumb_color))
            .begin_style(Style::default())
            .end_style(Style::default());

        let mut scrollbar_state = ScrollbarState::new(total_display_rows)
            .position(scrolled_display_rows)
            .viewport_content_length(msg_height);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

/// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::MessageItem;

    // ── count_display_rows ───────────────────────────────────────────────────

    #[test]
    fn test_count_display_rows_empty() {
        assert_eq!(count_display_rows(&[], 40), 0);
    }

    #[test]
    fn test_count_display_rows_zero_width() {
        let lines = vec![Line::raw("hello")];
        assert_eq!(count_display_rows(&lines, 0), 1);
    }

    #[test]
    fn test_count_display_rows_no_wrap() {
        let lines = vec![Line::raw("hello")];
        assert_eq!(count_display_rows(&lines, 40), 1);
    }

    #[test]
    fn test_count_display_rows_wraps() {
        let lines = vec![Line::raw("hello world")];
        assert_eq!(count_display_rows(&lines, 6), 2);
    }

    #[test]
    fn test_count_display_rows_multi_line() {
        let lines = vec![Line::raw("short"), Line::raw("a very long line indeed")];
        assert_eq!(count_display_rows(&lines, 10), 4);
    }

    #[test]
    fn test_count_display_rows_exact_width() {
        let lines = vec![Line::raw("12345")];
        assert_eq!(count_display_rows(&lines, 5), 1);
    }

    // ── bottom_scroll_offset ─────────────────────────────────────────────────

    #[test]
    fn test_bottom_scroll_offset_zero_height() {
        assert_eq!(bottom_scroll_offset(&[Line::raw("hi")], 40, 0), 0);
    }

    #[test]
    fn test_bottom_scroll_offset_fits() {
        let lines = vec![Line::raw("hello")];
        assert_eq!(bottom_scroll_offset(&lines, 40, 10), 0);
    }

    #[test]
    fn test_bottom_scroll_offset_scroll_needed() {
        let lines: Vec<Line> = (0..20).map(|i| Line::raw(format!("line {}", i))).collect();
        let offset = bottom_scroll_offset(&lines, 40, 5);
        assert!(offset > 0, "should scroll when content exceeds height");
    }

    #[test]
    fn test_bottom_scroll_offset_empty_lines() {
        assert_eq!(bottom_scroll_offset(&[], 40, 10), 0);
    }

    // ── count_search_matches ─────────────────────────────────────────────────

    #[test]
    fn test_search_matches_empty_query() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.search_query = "".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 0);
    }

    #[test]
    fn test_search_matches_found() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello world".into() });
        app.messages.push(MessageItem::Assistant { text: "world peace".into() });
        app.search_query = "world".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 2);
    }

    #[test]
    fn test_search_matches_not_found() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.messages.push(MessageItem::System { text: "hi".into() });
        app.search_query = "xyz".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 0);
    }

    #[test]
    fn test_search_matches_case_insensitive() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "Hello World".into() });
        app.search_query = "hello".into();
        count_search_matches(&mut app);
        assert_eq!(app.search_match_count, 1);
    }

    // ── update_reverse_search ────────────────────────────────────────────────

    #[test]
    fn test_reverse_search_empty_query() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hi".into() });
        update_reverse_search(&mut app);
        assert!(app.reverse_search_results.is_empty());
    }

    #[test]
    fn test_reverse_search_finds_matches() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "first".into() });
        app.messages.push(MessageItem::Assistant { text: "second".into() });
        app.reverse_search_query = "second".into();
        update_reverse_search(&mut app);
        assert_eq!(app.reverse_search_results.len(), 1);
        assert_eq!(app.reverse_search_results[0], 1);
    }

    #[test]
    fn test_reverse_search_no_matches() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "hello".into() });
        app.reverse_search_query = "xyz".into();
        update_reverse_search(&mut app);
        assert!(app.reverse_search_results.is_empty());
    }

    #[test]
    fn test_reverse_search_sets_last_match_index() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "a".into() });
        app.messages.push(MessageItem::Assistant { text: "a".into() });
        app.messages.push(MessageItem::System { text: "a".into() });
        app.reverse_search_query = "a".into();
        update_reverse_search(&mut app);
        assert_eq!(app.reverse_search_results.len(), 3);
        assert_eq!(app.reverse_search_idx, 2, "should select the last (newest) match");
        assert!(!app.auto_scroll, "reverse search should disable auto-scroll");
    }

    // ── invalidate_cache ─────────────────────────────────────────────────────

    #[test]
    fn test_invalidate_cache_resets_state() {
        // Prime cache by building lines
        let app = TuiApp::default();
        build_message_lines(&app); // primes cache
        invalidate_cache();
        // Rebuild should not use stale cache
        MSG_CACHE.with(|c| {
            assert_eq!(c.borrow().msg_count, 0, "cache should be invalidated");
        });
    }

    // ── build_message_lines caching ──────────────────────────────────────────

    #[test]
    fn test_build_message_lines_primes_cache() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "hello".into() });
        let lines = build_message_lines(&app);
        assert!(!lines.is_empty());
        MSG_CACHE.with(|c| {
            assert_eq!(c.borrow().msg_count, 1, "cache should be primed");
        });
    }

    #[test]
    fn test_build_message_lines_cache_hit() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "test".into() });
        let _first = build_message_lines(&app);
        let _second = build_message_lines(&app); // cache hit
        MSG_CACHE.with(|c| {
            assert_eq!(c.borrow().msg_count, 1, "cache should be preserved");
        });
    }

    #[test]
    fn test_build_message_lines_cache_miss_after_change() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "first".into() });
        let _first = build_message_lines(&app);
        app.messages.push(MessageItem::System { text: "second".into() });
        let _second = build_message_lines(&app); // cache miss — msg_count changed
        MSG_CACHE.with(|c| {
            assert_eq!(c.borrow().msg_count, 2, "cache should update after new message");
        });
    }

    #[test]
    fn test_build_message_lines_cache_miss_search_query() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::Assistant { text: "hello world".into() });
        let _first = build_message_lines(&app);
        app.search_active = true;
        app.search_query = "hello".into();
        let _second = build_message_lines(&app); // cache miss — search_query changed
        MSG_CACHE.with(|c| {
            assert_eq!(c.borrow().search_query, "hello");
        });
    }

    // ── System message prefix styling ────────────────────────────────────────

    #[test]
    fn test_build_system_message_has_correct_prefix() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "test".into() });
        let lines = build_message_lines(&app);
        assert!(lines[0].spans[0].content.contains("test"), "system message should be rendered");
    }

    #[test]
    fn test_build_end_message_is_dim() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "[end] (1.5s)".into() });
        let lines = build_message_lines(&app);
        assert_eq!(lines.len(), 1, "should render one line");
    }

    #[test]
    fn test_build_error_message_is_dim() {
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::System { text: "[error] (1.5s): something broke".into() });
        let lines = build_message_lines(&app);
        assert_eq!(lines.len(), 1, "should render one line");
    }

    #[test]
    fn test_build_message_empty_input() {
        let app = TuiApp::default();
        let lines = build_message_lines(&app);
        assert!(lines.is_empty(), "empty app should have no lines");
    }
}
