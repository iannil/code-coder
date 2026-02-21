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
pub fn convert_to_telegram_markdown(input: &str) -> String {
    // First, convert fenced code blocks to inline code
    let with_code_blocks = convert_code_blocks(input);

    // Then process line by line
    let lines: Vec<String> = with_code_blocks.lines().map(convert_line).collect();

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
    BOLD_DOUBLE_ASTERISK.replace_all(text, "*$1*").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_h1() {
        assert_eq!(convert_to_telegram_markdown("# Title"), "*Title*");
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
    fn convert_dash_list() {
        assert_eq!(convert_to_telegram_markdown("- Item"), "• Item");
    }

    #[test]
    fn convert_quote() {
        assert_eq!(convert_to_telegram_markdown("> Quote"), "│ Quote");
    }

    #[test]
    fn convert_code_block() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(convert_to_telegram_markdown(input), "`fn main() {}`");
    }

    #[test]
    fn convert_bold() {
        assert_eq!(convert_to_telegram_markdown("**bold**"), "*bold*");
    }

    #[test]
    fn mixed_content() {
        let input = "## Summary\n- Point 1\n- Point 2";
        let expected = "*Summary*\n• Point 1\n• Point 2";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }

    #[test]
    fn chinese_content() {
        let input = "## 总结\n- 第一项";
        let expected = "*总结*\n• 第一项";
        assert_eq!(convert_to_telegram_markdown(input), expected);
    }
}
