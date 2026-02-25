//! Telegram HTML format converter.
//!
//! Converts standard Markdown to Telegram-compatible HTML format.
//! HTML mode is more reliable than Markdown modes because:
//! - Fewer escaping requirements
//! - Better support for nested formatting
//! - Consistent rendering across clients
//!
//! This module handles tables, code blocks, headings, lists, and inline formatting.

use regex::Regex;
use std::sync::LazyLock;

// ============================================================================
// Regex Patterns
// ============================================================================

/// Pre-compiled regex patterns for Markdown conversion
static H1_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^# (.+)$").unwrap());
static H2_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^## (.+)$").unwrap());
static H3_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^### (.+)$").unwrap());
static H4_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#### (.+)$").unwrap());
static DASH_LIST_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)- (.*)$").unwrap());
static ASTERISK_LIST_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)\* (.*)$").unwrap());
static NUMBERED_LIST_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)\d+\. (.+)$").unwrap());
static QUOTE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^> (.+)$").unwrap());
static BOLD_DOUBLE_ASTERISK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*").unwrap());
// Note: Single asterisk bold is handled after double asterisk replacement
// to avoid conflicts. Uses simple pattern without look-around.
static BOLD_SINGLE_ASTERISK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*([^*\n]+?)\*").unwrap());
static ITALIC_UNDERSCORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"_([^_\n]+?)_").unwrap());
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`\n]+?)`").unwrap());
static LINK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
static CODE_BLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"```[a-zA-Z0-9]*\n?([\s\S]*?)```").unwrap());
static TABLE_HEADER_SEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\|?[\s:]*-+[\s:|-]*\|?$").unwrap());

// ============================================================================
// Public API
// ============================================================================

/// Convert standard Markdown to Telegram-compatible HTML format.
///
/// # Conversion Rules
///
/// | Input              | Output                    |
/// |--------------------|---------------------------|
/// | `# Title`          | `<b>📌 Title</b>`         |
/// | `## Subtitle`      | `<b>Subtitle</b>`         |
/// | `### Section`      | `<i>Section</i>`          |
/// | `- Item`           | `• Item`                  |
/// | `> Quote`          | `┃ Quote`                 |
/// | ` ```code``` `     | `<pre>code</pre>`         |
/// | `**bold**`         | `<b>bold</b>`             |
/// | `_italic_`         | `<i>italic</i>`           |
/// | `` `code` ``       | `<code>code</code>`       |
/// | `[text](url)`      | `<a href="url">text</a>`  |
/// | Markdown tables    | Monospace ASCII table     |
pub fn convert_to_telegram_html(input: &str) -> String {
    // Step 1: Convert tables first (before other processing)
    let with_tables = convert_tables(input);

    // Step 2: Convert fenced code blocks
    let with_code_blocks = convert_code_blocks(&with_tables);

    // Step 3: Process line by line
    let lines: Vec<String> = with_code_blocks.lines().map(convert_line).collect();

    // Step 4: Join and apply inline conversions
    let joined = lines.join("\n");
    convert_inline_formatting(&joined)
}

/// Legacy function for backward compatibility.
/// Internally uses HTML conversion but output is still usable with HTML parse_mode.
#[deprecated(since = "0.2.0", note = "Use convert_to_telegram_html instead")]
pub fn convert_to_telegram_markdown(input: &str) -> String {
    convert_to_telegram_html(input)
}

// ============================================================================
// Table Conversion
// ============================================================================

/// Convert Markdown tables to monospace ASCII format.
///
/// Telegram doesn't support native tables, so we render them as
/// preformatted text with box-drawing characters.
fn convert_tables(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Check if this is the start of a table
        if is_table_row(lines[i]) && i + 1 < lines.len() && TABLE_HEADER_SEP.is_match(lines[i + 1])
        {
            // Found a table - collect all rows
            let mut table_lines = vec![lines[i]];
            let mut j = i + 1;

            // Skip the separator line
            j += 1;

            // Collect data rows
            while j < lines.len() && is_table_row(lines[j]) {
                table_lines.push(lines[j]);
                j += 1;
            }

            // Convert the table
            let ascii_table = render_ascii_table(&table_lines);
            result.push(ascii_table);

            i = j;
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Check if a line looks like a table row (contains | separators).
fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|') && !TABLE_HEADER_SEP.is_match(trimmed)
}

/// Render a Markdown table as ASCII art with box-drawing characters.
fn render_ascii_table(rows: &[&str]) -> String {
    // Parse all rows into cells
    let parsed_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if parsed_rows.is_empty() {
        return String::new();
    }

    // Calculate column widths (max width of each column)
    let col_count = parsed_rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut col_widths: Vec<usize> = vec![0; col_count];

    for row in &parsed_rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(unicode_width(cell));
            }
        }
    }

    // Ensure minimum width of 3 for readability
    for width in &mut col_widths {
        *width = (*width).max(3);
    }

    // Build the ASCII table
    let mut output = Vec::new();

    // Top border
    output.push(build_border(&col_widths, '┌', '┬', '┐'));

    // Header row (first row)
    if let Some(header) = parsed_rows.first() {
        output.push(build_row(header, &col_widths));
        output.push(build_border(&col_widths, '├', '┼', '┤'));
    }

    // Data rows
    for row in parsed_rows.iter().skip(1) {
        output.push(build_row(row, &col_widths));
    }

    // Bottom border
    output.push(build_border(&col_widths, '└', '┴', '┘'));

    // Wrap in <pre> tags for monospace rendering
    format!("<pre>{}</pre>", output.join("\n"))
}

/// Build a table border line.
fn build_border(widths: &[usize], left: char, mid: char, right: char) -> String {
    let segments: Vec<String> = widths.iter().map(|&w| "─".repeat(w + 2)).collect();
    format!("{left}{}{right}", segments.join(&mid.to_string()))
}

/// Build a table data row.
fn build_row(cells: &[String], widths: &[usize]) -> String {
    let padded: Vec<String> = widths
        .iter()
        .enumerate()
        .map(|(i, &width)| {
            let cell = cells.get(i).map(String::as_str).unwrap_or("");
            let cell_width = unicode_width(cell);
            let padding = width.saturating_sub(cell_width);
            format!(" {cell}{} ", " ".repeat(padding))
        })
        .collect();
    format!("│{}│", padded.join("│"))
}

/// Calculate the display width of a string (accounting for CJK characters).
fn unicode_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if is_wide_char(c) {
                2
            } else {
                1
            }
        })
        .sum()
}

/// Check if a character is a wide (CJK) character.
fn is_wide_char(c: char) -> bool {
    let code = c as u32;
    // CJK Unified Ideographs and common wide ranges
    (0x4E00..=0x9FFF).contains(&code)  // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&code)  // CJK Extension A
        || (0xF900..=0xFAFF).contains(&code)  // CJK Compatibility Ideographs
        || (0x3000..=0x303F).contains(&code)  // CJK Punctuation
        || (0xFF00..=0xFFEF).contains(&code)  // Halfwidth and Fullwidth Forms
        || (0xAC00..=0xD7AF).contains(&code) // Hangul Syllables
}

// ============================================================================
// Code Block Conversion
// ============================================================================

/// Convert fenced code blocks to HTML <pre> tags.
///
/// Preserves newlines and formatting within code blocks.
fn convert_code_blocks(input: &str) -> String {
    CODE_BLOCK_PATTERN
        .replace_all(input, |caps: &regex::Captures| {
            let code = caps.get(1).map_or("", |m| m.as_str());
            let escaped = escape_html(code.trim());
            format!("<pre>{escaped}</pre>")
        })
        .to_string()
}

// ============================================================================
// Line Conversion
// ============================================================================

/// Convert a single line according to Telegram HTML formatting rules.
fn convert_line(line: &str) -> String {
    // Skip lines inside <pre> blocks (already processed)
    if line.contains("<pre>") || line.contains("</pre>") {
        return line.to_string();
    }

    // H1: # Title -> <b>📌 Title</b>
    if let Some(caps) = H1_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("<b>📌 {}</b>", escape_html(title));
    }

    // H2: ## Title -> <b>Title</b>
    if let Some(caps) = H2_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("<b>{}</b>", escape_html(title));
    }

    // H3: ### Title -> <i>Title</i>
    if let Some(caps) = H3_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("<i>{}</i>", escape_html(title));
    }

    // H4: #### Title -> <i>Title</i>
    if let Some(caps) = H4_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("<i>{}</i>", escape_html(title));
    }

    // Quote: > text -> ┃ text (blockquote style)
    if let Some(caps) = QUOTE_PATTERN.captures(line) {
        let text = caps.get(1).map_or("", |m| m.as_str());
        return format!("┃ <i>{}</i>", escape_html(text));
    }

    // Dash list: - item -> • item (preserve indentation)
    if let Some(caps) = DASH_LIST_PATTERN.captures(line) {
        let indent = caps.get(1).map_or("", |m| m.as_str());
        let item = caps.get(2).map_or("", |m| m.as_str());
        return format!("{indent}• {item}");
    }

    // Asterisk list: * item -> • item (preserve indentation)
    if let Some(caps) = ASTERISK_LIST_PATTERN.captures(line) {
        let indent = caps.get(1).map_or("", |m| m.as_str());
        let item = caps.get(2).map_or("", |m| m.as_str());
        return format!("{indent}• {item}");
    }

    // Numbered list: keep as-is
    if NUMBERED_LIST_PATTERN.is_match(line) {
        return line.to_string();
    }

    line.to_string()
}

// ============================================================================
// Inline Formatting
// ============================================================================

/// Convert inline Markdown formatting to HTML.
fn convert_inline_formatting(text: &str) -> String {
    let mut result = text.to_string();

    // Convert links first (before escaping)
    result = LINK_PATTERN
        .replace_all(&result, |caps: &regex::Captures| {
            let text = caps.get(1).map_or("", |m| m.as_str());
            let url = caps.get(2).map_or("", |m| m.as_str());
            format!("<a href=\"{url}\">{text}</a>")
        })
        .to_string();

    // Convert **bold** to <b>bold</b>
    result = BOLD_DOUBLE_ASTERISK
        .replace_all(&result, "<b>$1</b>")
        .to_string();

    // Convert *bold* to <b>bold</b> (single asterisk bold in many Markdown dialects)
    result = BOLD_SINGLE_ASTERISK
        .replace_all(&result, "<b>$1</b>")
        .to_string();

    // Convert _italic_ to <i>italic</i>
    result = ITALIC_UNDERSCORE
        .replace_all(&result, "<i>$1</i>")
        .to_string();

    // Convert `code` to <code>code</code>
    result = INLINE_CODE
        .replace_all(&result, |caps: &regex::Captures| {
            let code = caps.get(1).map_or("", |m| m.as_str());
            format!("<code>{}</code>", escape_html(code))
        })
        .to_string();

    result
}

/// Escape HTML special characters.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Heading Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_h1() {
        assert_eq!(convert_to_telegram_html("# Title"), "<b>📌 Title</b>");
    }

    #[test]
    fn convert_h2() {
        assert_eq!(convert_to_telegram_html("## Subtitle"), "<b>Subtitle</b>");
    }

    #[test]
    fn convert_h3() {
        assert_eq!(convert_to_telegram_html("### Section"), "<i>Section</i>");
    }

    // ------------------------------------------------------------------------
    // List Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_dash_list() {
        assert_eq!(convert_to_telegram_html("- Item"), "• Item");
    }

    #[test]
    fn convert_nested_list() {
        let input = "- Item 1\n  - Nested";
        let expected = "• Item 1\n  • Nested";
        assert_eq!(convert_to_telegram_html(input), expected);
    }

    // ------------------------------------------------------------------------
    // Quote Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_quote() {
        assert_eq!(convert_to_telegram_html("> Quote"), "┃ <i>Quote</i>");
    }

    // ------------------------------------------------------------------------
    // Code Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_code_block() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(convert_to_telegram_html(input), "<pre>fn main() {}</pre>");
    }

    #[test]
    fn convert_code_block_multiline() {
        let input = "```\nline 1\nline 2\n```";
        assert_eq!(convert_to_telegram_html(input), "<pre>line 1\nline 2</pre>");
    }

    #[test]
    fn convert_inline_code() {
        assert_eq!(convert_to_telegram_html("Use `git status`"), "Use <code>git status</code>");
    }

    // ------------------------------------------------------------------------
    // Inline Formatting Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_bold_double_asterisk() {
        assert_eq!(convert_to_telegram_html("**bold**"), "<b>bold</b>");
    }

    #[test]
    fn convert_bold_single_asterisk() {
        assert_eq!(convert_to_telegram_html("*bold*"), "<b>bold</b>");
    }

    #[test]
    fn convert_italic() {
        assert_eq!(convert_to_telegram_html("_italic_"), "<i>italic</i>");
    }

    #[test]
    fn convert_link() {
        assert_eq!(
            convert_to_telegram_html("[Google](https://google.com)"),
            "<a href=\"https://google.com\">Google</a>"
        );
    }

    // ------------------------------------------------------------------------
    // Table Tests
    // ------------------------------------------------------------------------

    #[test]
    fn convert_simple_table() {
        let input = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
        let result = convert_to_telegram_html(input);
        assert!(result.contains("<pre>"));
        assert!(result.contains("Alice"));
        assert!(result.contains("30"));
        assert!(result.contains("┌"));
        assert!(result.contains("└"));
    }

    #[test]
    fn convert_table_with_cjk() {
        let input = "| 名称 | 数量 |\n|------|------|\n| 苹果 | 5 |";
        let result = convert_to_telegram_html(input);
        assert!(result.contains("<pre>"));
        assert!(result.contains("名称"));
        assert!(result.contains("苹果"));
    }

    // ------------------------------------------------------------------------
    // Mixed Content Tests
    // ------------------------------------------------------------------------

    #[test]
    fn mixed_content() {
        let input = "## Summary\n- Point 1\n- Point 2";
        let expected = "<b>Summary</b>\n• Point 1\n• Point 2";
        assert_eq!(convert_to_telegram_html(input), expected);
    }

    #[test]
    fn chinese_content() {
        let input = "## 总结\n- 第一项";
        let expected = "<b>总结</b>\n• 第一项";
        assert_eq!(convert_to_telegram_html(input), expected);
    }

    // ------------------------------------------------------------------------
    // HTML Escaping Tests
    // ------------------------------------------------------------------------

    #[test]
    fn escape_html_in_headings() {
        assert_eq!(convert_to_telegram_html("# A < B & C > D"), "<b>📌 A &lt; B &amp; C &gt; D</b>");
    }

    #[test]
    fn escape_html_in_code() {
        assert_eq!(
            convert_to_telegram_html("`<script>alert('xss')</script>`"),
            "<code>&lt;script&gt;alert('xss')&lt;/script&gt;</code>"
        );
    }

    // ------------------------------------------------------------------------
    // Unicode Width Tests
    // ------------------------------------------------------------------------

    #[test]
    fn unicode_width_ascii() {
        assert_eq!(unicode_width("hello"), 5);
    }

    #[test]
    fn unicode_width_cjk() {
        assert_eq!(unicode_width("你好"), 4); // Each CJK char is width 2
    }

    #[test]
    fn unicode_width_mixed() {
        assert_eq!(unicode_width("hello你好"), 9); // 5 + 4
    }
}
