/// ─── Markdown Renderer ──────────────────────────────────────────────────────
///
/// Parses markdown text and produces ratatui `Vec<Line<'static>>` with syntax
/// highlighting for code blocks. Used by the message list widget.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render markdown text into ratatui lines with syntax highlighting.
/// This is a streaming processor that handles the most common elements:
/// headings, bold/italic, inline code, code blocks, lists, and links.
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    render_markdown_with_highlight(text, None)
}

/// Render markdown with optional search highlighting
pub fn render_markdown_with_highlight(text: &str, highlight: Option<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current_line = Vec::<Span<'static>>::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();

    // Simple line-by-line markdown processor
    // Table detection needs to look ahead, so we use a peekable approach
    let all_lines: Vec<&str> = text.lines().collect();
    let mut skip_until: Option<usize> = None;
    let hl = highlight.filter(|h| !h.is_empty());

    for (i, raw_line) in all_lines.iter().enumerate() {
        // Skip lines already consumed by table rendering
        if let Some(limit) = skip_until {
            if i < limit {
                continue;
            }
            skip_until = None;
        }

        // Handle code blocks
        if raw_line.starts_with("```") {
            if in_code_block {
                // End code block — render it now
                let lang = std::mem::take(&mut code_block_lang);
                let content = std::mem::take(&mut code_block_content);
                render_code_block(&mut lines, &lang, &content);
                in_code_block = false;
            } else {
                // Flush current inline content
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
                in_code_block = true;
                code_block_lang = raw_line[3..].trim().to_string();
            }
            continue;
        }

        if in_code_block {
            code_block_content.push_str(raw_line);
            code_block_content.push('\n');
            continue;
        }

        // Empty line — flush current paragraph
        if raw_line.trim().is_empty() {
            if !current_line.is_empty() {
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }
            lines.push(Line::from(""));
            continue;
        }

        // Headings
        if raw_line.starts_with("### ") {
            render_inline(&raw_line[4..], &mut current_line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }
        if raw_line.starts_with("## ") {
            render_inline(&raw_line[3..], &mut current_line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }
        if raw_line.starts_with("# ") {
            render_inline(&raw_line[2..], &mut current_line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }

        // Unordered list
        if raw_line.starts_with("- ") || raw_line.starts_with("* ") {
            current_line.push(Span::styled("  • ", Style::default().fg(Color::Yellow)));
            render_inline(&raw_line[2..], &mut current_line, Style::default(), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }

        // Ordered list
        if let Some(rest) = raw_line.strip_prefix(|c: char| c.is_ascii_digit())
            .and_then(|s| s.strip_prefix(". "))
        {
            current_line.push(Span::styled("  1. ", Style::default().fg(Color::Yellow)));
            render_inline(rest, &mut current_line, Style::default(), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }

        // Table — detect and render as a block
        if raw_line.starts_with('|') && raw_line.contains("---") {
            // This is a table separator row — consume the whole table
            let table_start = i.saturating_sub(1); // header row
            let mut table_rows: Vec<String> = Vec::new();

            for j in table_start..all_lines.len() {
                let line = all_lines[j].trim();
                if line.starts_with('|') {
                    let cleaned = line.trim_matches('|').trim().to_string();
                    table_rows.push(cleaned);
                } else {
                    break;
                }
            }

            if table_rows.len() >= 3 {
                render_table(&mut lines, &table_rows);
                skip_until = Some(table_start + table_rows.len());
            }
            continue;
        } else if raw_line.starts_with('|') {
            // Check if next line has |---|---| pattern
            if i + 1 < all_lines.len() && all_lines[i + 1].starts_with('|') && all_lines[i + 1].contains("---") {
                // Will be handled when we encounter the separator
                // For now, just render as plain text
                render_inline(raw_line, &mut current_line, Style::default(), hl);
                lines.push(Line::from(std::mem::take(&mut current_line)));
                continue;
            }
            // Otherwise: treat as inline
            render_inline(raw_line, &mut current_line, Style::default(), hl);
            lines.push(Line::from(std::mem::take(&mut current_line)));
            continue;
        }

        // Empty line — flush current paragraph

        // Regular paragraph line
        if !current_line.is_empty() && !raw_line.starts_with(' ') {
            // Continuing paragraph — just add the line as-is for now
            current_line.push(Span::raw(" "));
        }
        render_inline(raw_line, &mut current_line, Style::default(), hl);
        lines.push(Line::from(std::mem::take(&mut current_line)));
    }

    // Handle unclosed code block at EOF
    if in_code_block {
        let lang = std::mem::take(&mut code_block_lang);
        let content = std::mem::take(&mut code_block_content);
        render_code_block(&mut lines, &lang, &content);
    }

    // Flush remaining inline content
    if !current_line.is_empty() {
        lines.push(Line::from(std::mem::take(&mut current_line)));
    }

    lines
}

/// Syntect 缓存（延迟初始化）
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static DARK_MODE: AtomicBool = AtomicBool::new(true);

/// Set the global theme mode. Call before rendering.
pub fn set_dark_mode(dark: bool) {
    DARK_MODE.store(dark, Ordering::Relaxed);
}

/// Check current theme mode.
#[allow(dead_code)]
pub fn is_dark_mode() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

fn get_syntax_set() -> &'static syntect::parsing::SyntaxSet {
    static SS: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
    SS.get_or_init(|| {
        syntect::parsing::SyntaxSet::load_defaults_newlines()
    })
}

static THEMES: OnceLock<(syntect::highlighting::Theme, syntect::highlighting::Theme)> = OnceLock::new();

fn load_themes() -> &'static (syntect::highlighting::Theme, syntect::highlighting::Theme) {
    THEMES.get_or_init(|| {
        let ts = syntect::highlighting::ThemeSet::load_defaults();
        let dark = ts.themes["base16-ocean.dark"].clone();
        // Light theme: try base16-ocean.light first, fall back to InspiredGitHub
        let light = ts.themes.get("base16-ocean.light")
            .cloned()
            .unwrap_or_else(|| ts.themes["InspiredGitHub"].clone());
        (dark, light)
    })
}

fn get_theme() -> &'static syntect::highlighting::Theme {
    let themes = load_themes();
    if DARK_MODE.load(Ordering::Relaxed) {
        &themes.0
    } else {
        &themes.1
    }
}

/// Render a code block with syntax highlighting (via syntect)
fn render_code_block(
    lines: &mut Vec<Line<'static>>,
    lang: &str,
    content: &str,
) {
    let header = if lang.is_empty() {
        " ┌─ code ".to_string()
    } else if lang == "diff" {
        " ┌─ diff ".to_string()
    } else {
        format!(" ┌─ {lang} ")
    };
    lines.push(Line::styled(header, Style::default().fg(Color::DarkGray)));

    // Check if this is a diff block
    if lang == "diff" {
        let diff_lines = render_diff_text(content);
        if !diff_lines.is_empty() {
            for dl in diff_lines {
                let mut spans = vec![Span::raw(" │ ")];
                spans.extend(dl.spans);
                lines.push(Line::from(spans));
            }
            lines.push(Line::styled(
                " └─",
                Style::default().fg(Color::DarkGray),
            ));
            return;
        }
    }

    let ss = get_syntax_set();
    let theme = get_theme();

    // Try to find syntax for this language
    let syntax = if lang.is_empty() {
        None
    } else {
        ss.find_syntax_by_token(lang)
    };

    if let Some(syntax) = syntax {
        // Use syntect's line-based highlighting
        use syntect::highlighting::FontStyle;
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

        for line in content.lines() {
            let ranges = highlighter.highlight_line(line, ss)
                .unwrap_or_else(|_| Vec::new());

            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::raw(" │ "));

            for (style, text) in ranges {
                let fg = syntect_color_to_ratatui(style.foreground);
                let mut ratatui_style = Style::default().fg(fg);

                if style.font_style.contains(FontStyle::BOLD) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                }
                if style.font_style.contains(FontStyle::ITALIC) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                }
                if style.font_style.contains(FontStyle::UNDERLINE) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                }

                spans.push(Span::styled(text.to_string(), ratatui_style));
            }

            lines.push(Line::from(spans));
        }
    } else {
        // No syntax found — plain dim style
        for line in content.lines() {
            lines.push(Line::styled(
                format!(" │ {}", line),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // Code block footer
    lines.push(Line::styled(
        " └─",
        Style::default().fg(Color::DarkGray),
    ));
}

/// Convert syntect Color to ratatui Color
fn syntect_color_to_ratatui(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Detect if text is a unified diff and render with +/- colors
pub fn render_diff_text(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_diff = false;

    for line in text.lines() {
        let line_owned = line.to_string();
        if line.starts_with("diff --git") || line.starts_with("--- ") || line.starts_with("+++ ") {
            in_diff = true;
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        } else if line.starts_with("@@") {
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::Blue),
            ));
        } else if line.starts_with('+') && !line.starts_with("+++") {
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::Green),
            ));
        } else if line.starts_with('-') && !line.starts_with("---") {
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::Red),
            ));
        } else if line.starts_with(" ") {
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::DarkGray),
            ));
        } else if in_diff {
            lines.push(Line::styled(
                line_owned,
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            return Vec::new(); // Not a diff
        }
    }

    lines
}

/// Render a table from pre-collected rows (each row is pipe-delimited)
fn render_table(lines: &mut Vec<Line<'static>>, rows: &[String]) {
    if rows.is_empty() {
        return;
    }

    // Parse cells for each row
    let parsed: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    // Calculate column widths (skip separator row, index 1)
    let col_count = parsed.first().map(|r| r.len()).unwrap_or(0);
    if col_count == 0 {
        return;
    }

    let mut col_widths = vec![0usize; col_count];
    for row in &parsed {
        for (j, cell) in row.iter().enumerate() {
            if j < col_widths.len() {
                col_widths[j] = col_widths[j].max(cell.len());
            }
        }
    }

    // Clamp to max display width
    let max_col_width = 40;
    for w in &mut col_widths {
        *w = (*w).min(max_col_width);
    }

    // Render header (row 0)
    render_table_row(lines, &parsed[0], &col_widths, true);

    // Render data rows (skip separator row 1)
    for row in parsed.iter().skip(2) {
        render_table_row(lines, row, &col_widths, false);
    }
}

fn render_table_row(
    lines: &mut Vec<Line<'static>>,
    cells: &[String],
    widths: &[usize],
    is_header: bool,
) {
    let mut spans = Vec::new();
    spans.push(Span::raw(" │ "));

    for (i, cell) in cells.iter().enumerate() {
        let width = widths.get(i).copied().unwrap_or(0);
        let padded = if cell.len() >= width {
            cell[..width].to_string()
        } else {
            format!("{}{}", cell, " ".repeat(width - cell.len()))
        };

        let style = if is_header {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        spans.push(Span::styled(padded, style));
        spans.push(Span::raw(" │ ".to_string()));
    }

    lines.push(Line::from(spans));

    // Header underline
    if is_header {
        let total_width: usize = widths.iter().sum::<usize>() + widths.len() * 3 + 1;
        let underline = "─".repeat(total_width.min(80));
        lines.push(Line::styled(
            format!(" ├{underline}┤"),
            Style::default().fg(Color::DarkGray),
        ));
    }
}

/// Render inline elements from a markdown line
/// Push text to spans, optionally highlighting substrings matching `highlight`.
fn push_highlighted_text(
    buf: &mut String,
    spans: &mut Vec<Span<'static>>,
    style: Style,
    highlight: Option<&str>,
) {
    let text = std::mem::take(buf);
    if text.is_empty() {
        return;
    }
    let Some(hl) = highlight else {
        spans.push(Span::styled(text, style));
        return;
    };
    if hl.is_empty() {
        spans.push(Span::styled(text, style));
        return;
    }
    let hl_lower = hl.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&hl_lower) {
        let abs_pos = start + pos;
        // Push non-matching prefix
        if abs_pos > start {
            spans.push(Span::styled(text[start..abs_pos].to_string(), style));
        }
        // Push matching text with highlight
        let end = abs_pos + hl.len();
        spans.push(Span::styled(
            text[abs_pos..end].to_string(),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        start = end;
    }
    // Push remaining non-matching suffix
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), style));
    }
}

fn render_inline(text: &str, spans: &mut Vec<Span<'static>>, base_style: Style, highlight: Option<&str>) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut buf = String::new();

    while i < chars.len() {
        // Inline code (backticks)
        if chars[i] == '`' {
            if !buf.is_empty() {
                push_highlighted_text(&mut buf, spans, base_style, highlight);
            }
            i += 1;
            // Find closing backtick
            let _code_start = i;
            while i < chars.len() && chars[i] != '`' {
                buf.push(chars[i]);
                i += 1;
            }
            let code_text = std::mem::take(&mut buf);
            spans.push(Span::styled(
                code_text,
                base_style
                    .fg(Color::Green)
                    .add_modifier(Modifier::DIM),
            ));
            if i < chars.len() {
                i += 1; // skip closing backtick
            }
            continue;
        }

        // Bold (**text**)
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                push_highlighted_text(&mut buf, spans, base_style, highlight);
            }
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                buf.push(chars[i]);
                i += 1;
            }
            let bold_text = std::mem::take(&mut buf);
            spans.push(Span::styled(
                bold_text,
                base_style.add_modifier(Modifier::BOLD),
            ));
            i += 2; // skip closing **
            continue;
        }

        // Italic (*text*)
        if chars[i] == '*' {
            if !buf.is_empty() {
                push_highlighted_text(&mut buf, spans, base_style, highlight);
            }
            i += 1;
            while i < chars.len() && chars[i] != '*' {
                buf.push(chars[i]);
                i += 1;
            }
            let italic_text = std::mem::take(&mut buf);
            spans.push(Span::styled(
                italic_text,
                base_style.add_modifier(Modifier::ITALIC),
            ));
            if i < chars.len() {
                i += 1; // skip closing *
            }
            continue;
        }

        // Link [text](url) — render as underlined text
        if chars[i] == '[' {
            if !buf.is_empty() {
                push_highlighted_text(&mut buf, spans, base_style, highlight);
            }
            i += 1;
            let _link_start = i;
            while i < chars.len() && chars[i] != ']' {
                buf.push(chars[i]);
                i += 1;
            }
            let link_text = std::mem::take(&mut buf);
            i += 1; // skip ]
            if i < chars.len() && chars[i] == '(' {
                i += 1;
                while i < chars.len() && chars[i] != ')' {
                    buf.push(chars[i]);
                    i += 1;
                }
                let _url = std::mem::take(&mut buf);
                i += 1; // skip )
                spans.push(Span::styled(
                    link_text,
                    base_style
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            } else {
                // No URL found — just render as regular text
                spans.push(Span::styled(link_text, base_style));
            }
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        push_highlighted_text(&mut buf, spans, base_style, highlight);
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renders_plain_text() {
        let lines = render_markdown("hello world");
        assert!(!lines.is_empty());
        assert!(lines[0].to_string().contains("hello"));
    }

    #[test]
    fn test_render_heading() {
        let lines = render_markdown("# Title");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_code_block() {
        let lines = render_markdown("```rust\nfn main() {}\n```");
        // Should have header + code line + footer
        assert_eq!(lines.len(), 3, "code block: header + content + footer");
    }

    #[test]
    fn test_render_bold_italic() {
        let lines = render_markdown("**bold** and *italic*");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_list() {
        let lines = render_markdown("- item one\n- item two");
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_render_inline_code() {
        let lines = render_markdown("use `std::collections`");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_link() {
        let lines = render_markdown("[click here](https://example.com)");
        assert!(!lines.is_empty());
    }
}
