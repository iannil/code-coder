//! Telegram Markdown format converter.
//!
//! Converts standard Markdown to Telegram-compatible legacy Markdown format.
//! Telegram's legacy Markdown mode has limited support and doesn't handle
//! `## headings`, `- lists`, `> quotes`, or fenced code blocks well.
//!
//! This module provides intelligent conversion to preserve basic formatting
//! while avoiding parse errors that would trigger plain-text fallback.

use regex::Regex;
use std::sync::LazyLock;

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
static CODE_BLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"```[a-zA-Z0-9]*\n([\s\S]*?)\n```").unwrap());

/// Convert standard Markdown to Telegram-compatible format.
///
/// # Conversion Rules
///
/// | Input              | Output         |
/// |--------------------|----------------|
/// | `# Title`          | `*Title*`      |
/// | `## Subtitle`      | `*Subtitle*`   |
/// | `### Section`      | `_Section_`    |
/// | `#### Subsection`  | `_Subsection_` |
/// | `- Item`           | `• Item`       |
/// | `* Item`           | `• Item`       |
/// | `1. Item`          | `1. Item`      |
/// | `> Quote`          | `│ Quote`      |
/// | ` ```lang\ncode``` ` | `` `code` ``   |
/// | `**bold**`         | `*bold*`       |
///
/// # Examples
///
/// ```ignore
/// use zero_bot::channels::telegram_format::convert_to_telegram_markdown;
///
/// let input = "## Summary\n- Point 1\n- Point 2";
/// let output = convert_to_telegram_markdown(input);
/// assert_eq!(output, "*Summary*\n• Point 1\n• Point 2");
/// ```
pub fn convert_to_telegram_markdown(input: &str) -> String {
    // First, convert fenced code blocks to inline code
    let with_code_blocks = convert_code_blocks(input);

    // Then process line by line
    let lines: Vec<String> = with_code_blocks
        .lines()
        .map(convert_line)
        .collect();

    // Join and apply inline conversions
    let joined = lines.join("\n");
    convert_inline_formatting(&joined)
}

/// Convert fenced code blocks to inline code.
///
/// Multi-line code blocks are joined with semicolons to fit in inline format.
/// This is a trade-off: long code loses newlines but preserves monospace formatting.
fn convert_code_blocks(input: &str) -> String {
    CODE_BLOCK_PATTERN
        .replace_all(input, |caps: &regex::Captures| {
            let code = caps.get(1).map_or("", |m| m.as_str());
            // Join multiple lines with semicolons, trim each line
            let inline_code: String = code
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            format!("`{inline_code}`")
        })
        .to_string()
}

/// Convert a single line according to Telegram formatting rules.
fn convert_line(line: &str) -> String {
    // H1: # Title -> *Title*
    if let Some(caps) = H1_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("*{title}*");
    }

    // H2: ## Title -> *Title*
    if let Some(caps) = H2_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("*{title}*");
    }

    // H3: ### Title -> _Title_
    if let Some(caps) = H3_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("_{title}_");
    }

    // H4: #### Title -> _Title_
    if let Some(caps) = H4_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("_{title}_");
    }

    // Quote: > text -> │ text
    if let Some(caps) = QUOTE_PATTERN.captures(line) {
        let text = caps.get(1).map_or("", |m| m.as_str());
        return format!("│ {text}");
    }

    // Dash list: - item -> • item (preserve indentation)
    if let Some(caps) = DASH_LIST_PATTERN.captures(line) {
        let indent = caps.get(1).map_or("", |m| m.as_str());
        let item = caps.get(2).map_or("", |m| m.as_str());
        return format!("{indent}• {item}");
    }

    // Asterisk list: * item -> • item (preserve indentation)
    // But only at line start to avoid matching *bold* text
    if let Some(caps) = ASTERISK_LIST_PATTERN.captures(line) {
        let indent = caps.get(1).map_or("", |m| m.as_str());
        let item = caps.get(2).map_or("", |m| m.as_str());
        return format!("{indent}• {item}");
    }

    // Numbered list: keep as-is (Telegram handles these)
    if NUMBERED_LIST_PATTERN.is_match(line) {
        return line.to_string();
    }

    line.to_string()
}

/// Convert inline formatting elements.
fn convert_inline_formatting(text: &str) -> String {
    // Convert **bold** to *bold*
    // Telegram's legacy Markdown uses single asterisks for bold
    BOLD_DOUBLE_ASTERISK
        .replace_all(text, "*$1*")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Heading conversions ──────────────────────────────────────────

    #[test]
    fn convert_h1() {
        assert_eq!(convert_to_telegram_markdown("# Title"), "*Title*");
    }

    #[test]
    fn convert_h1_with_content_after() {
        let input = "# Title\nSome content below";
        let expected = "*Title*\nSome content below";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn convert_h2() {
        assert_eq!(convert_to_telegram_markdown("## Subtitle"), "*Subtitle*");
    }

    #[test]
    fn convert_h3() {
        assert_eq!(convert_to_telegram_markdown("### Section"), "_Section_");
    }

    #[test]
    fn convert_h4() {
        assert_eq!(
            convert_to_telegram_markdown("#### Subsection"),
            "_Subsection_"
        );
    }

    #[test]
    fn heading_with_special_chars() {
        assert_eq!(
            convert_to_telegram_markdown("## Hello, World!"),
            "*Hello, World!*"
        );
    }

    // ── List conversions ─────────────────────────────────────────────

    #[test]
    fn convert_dash_list() {
        assert_eq!(convert_to_telegram_markdown("- Item"), "• Item");
    }

    #[test]
    fn convert_asterisk_list() {
        assert_eq!(convert_to_telegram_markdown("* Item"), "• Item");
    }

    #[test]
    fn convert_multiple_list_items() {
        let input = "- First\n- Second\n- Third";
        let expected = "• First\n• Second\n• Third";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn convert_nested_list() {
        let input = "- Item\n  - Nested";
        let expected = "• Item\n  • Nested";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn numbered_list_preserved() {
        let input = "1. First\n2. Second\n3. Third";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    // ── Quote conversions ────────────────────────────────────────────

    #[test]
    fn convert_quote() {
        assert_eq!(convert_to_telegram_markdown("> Quote"), "│ Quote");
    }

    #[test]
    fn convert_multi_line_quote() {
        let input = "> First line\n> Second line";
        let expected = "│ First line\n│ Second line";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    // ── Code block conversions ───────────────────────────────────────

    #[test]
    fn convert_code_block() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(convert_to_telegram_markdown(input), "`fn main() {}`");
    }

    #[test]
    fn convert_code_block_multiline() {
        let input = "```python\ndef foo():\n    return 42\n```";
        let expected = "`def foo():; return 42`";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn convert_code_block_no_language() {
        let input = "```\ncode here\n```";
        assert_eq!(convert_to_telegram_markdown(input), "`code here`");
    }

    #[test]
    fn preserve_inline_code() {
        let input = "Use `inline code` here";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    // ── Bold conversions ─────────────────────────────────────────────

    #[test]
    fn convert_bold() {
        assert_eq!(convert_to_telegram_markdown("**bold**"), "*bold*");
    }

    #[test]
    fn convert_multiple_bold() {
        assert_eq!(
            convert_to_telegram_markdown("**one** and **two**"),
            "*one* and *two*"
        );
    }

    #[test]
    fn preserve_existing_telegram_format() {
        assert_eq!(
            convert_to_telegram_markdown("*already bold*"),
            "*already bold*"
        );
    }

    #[test]
    fn preserve_existing_italic() {
        assert_eq!(
            convert_to_telegram_markdown("_already italic_"),
            "_already italic_"
        );
    }

    // ── Mixed content ────────────────────────────────────────────────

    #[test]
    fn mixed_content() {
        let input = "## Summary\n- Point 1\n- Point 2";
        let expected = "*Summary*\n• Point 1\n• Point 2";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn mixed_content_complex() {
        let input = "# Main Title\n\n## Section 1\n\n- Item with **bold** text\n- Another item\n\n> A quote here\n\n### Subsection\n\nRegular text.";
        let expected = "*Main Title*\n\n*Section 1*\n\n• Item with *bold* text\n• Another item\n\n│ A quote here\n\n_Subsection_\n\nRegular text.";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn code_block_with_surrounding_text() {
        let input = "Here's some code:\n\n```rust\nlet x = 1;\n```\n\nAnd more text.";
        let expected = "Here's some code:\n\n`let x = 1;`\n\nAnd more text.";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    // ── Edge cases ───────────────────────────────────────────────────

    #[test]
    fn empty_string() {
        assert_eq!(convert_to_telegram_markdown(""), "");
    }

    #[test]
    fn plain_text_unchanged() {
        let input = "Just some plain text without any formatting.";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    #[test]
    fn heading_in_middle_of_text_not_converted() {
        // Hash in middle of line should not be converted
        let input = "This has a # in the middle";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    #[test]
    fn dash_in_middle_of_text_not_converted() {
        // Dash in middle of line should not be converted to bullet
        let input = "This has a - dash in it";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    #[test]
    fn empty_list_item() {
        assert_eq!(convert_to_telegram_markdown("- "), "• ");
    }

    #[test]
    fn whitespace_only_input() {
        assert_eq!(convert_to_telegram_markdown("   "), "   ");
    }

    #[test]
    fn newlines_preserved() {
        let input = "Line 1\n\nLine 2\n\nLine 3";
        assert_eq!(convert_to_telegram_markdown(input), input);
    }

    // ── Chinese/Unicode content ──────────────────────────────────────

    #[test]
    fn chinese_heading() {
        assert_eq!(convert_to_telegram_markdown("## 总结"), "*总结*");
    }

    #[test]
    fn chinese_list() {
        let input = "- 第一项\n- 第二项";
        let expected = "• 第一项\n• 第二项";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn chinese_quote() {
        assert_eq!(convert_to_telegram_markdown("> 引用内容"), "│ 引用内容");
    }

    #[test]
    fn mixed_chinese_english() {
        let input = "## Summary 总结\n\n- Point 要点 1\n- Point 要点 2";
        let expected = "*Summary 总结*\n\n• Point 要点 1\n• Point 要点 2";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }
}
