//! Slack mrkdwn format converter.
//!
//! Converts standard Markdown to Slack-compatible mrkdwn format.
//! Slack uses a custom format called "mrkdwn" with different syntax:
//!
//! - Uses `*bold*` instead of `**bold**`
//! - Uses `_italic_` instead of `*italic*`
//! - Uses `~strikethrough~` instead of `~~strikethrough~~`
//! - Uses `>` for single-line quotes only
//! - Code blocks use triple backticks (same as standard)
//! - Links use `<url|text>` format
//!
//! This module provides intelligent conversion to preserve formatting.

use regex::Regex;
use std::sync::LazyLock;

/// Maximum message length for Slack.
pub const MAX_MESSAGE_LENGTH: usize = 40000;

/// Maximum length for attachment/block text fields.
pub const MAX_BLOCK_TEXT_LENGTH: usize = 3000;

/// Pre-compiled regex patterns for Markdown conversion
static H1_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^# (.+)$").unwrap());
static H2_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^## (.+)$").unwrap());
static H3_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^### (.+)$").unwrap());
static H4_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#### (.+)$").unwrap());
static BOLD_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*").unwrap());
#[allow(dead_code)] // Reserved for future italic handling improvements
static ITALIC_ASTERISK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?<!\*)\*([^*]+)\*(?!\*)").unwrap());
static STRIKETHROUGH_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"~~(.+?)~~").unwrap());
static LINK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
static DASH_LIST_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)- (.*)$").unwrap());
static CODE_BLOCK_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^```(\w*)$").unwrap());

/// Convert standard Markdown to Slack mrkdwn format.
///
/// # Conversion Rules
///
/// | Input                | Output              |
/// |----------------------|---------------------|
/// | `# Title`            | `*Title*` (bold)    |
/// | `## Subtitle`        | `*Subtitle*`        |
/// | `### Section`        | `_Section_` (italic)|
/// | `#### Subsection`    | `_Subsection_`      |
/// | `**bold**`           | `*bold*`            |
/// | `*italic*`           | `_italic_`          |
/// | `~~strike~~`         | `~strike~`          |
/// | `[text](url)`        | `<url|text>`        |
/// | `- item`             | `• item`            |
/// | `> quote`            | `> quote` (preserved)|
/// | Code blocks          | Preserved           |
pub fn convert_to_slack_mrkdwn(input: &str) -> String {
    // Process lines for structure conversion
    let mut in_code_block = false;
    let lines: Vec<String> = input
        .lines()
        .map(|line| {
            // Track code block state
            if CODE_BLOCK_START.is_match(line) || line == "```" {
                in_code_block = !in_code_block;
                return line.to_string();
            }

            // Don't convert inside code blocks
            if in_code_block {
                return line.to_string();
            }

            convert_line(line)
        })
        .collect();

    // Join and apply inline conversions
    let joined = lines.join("\n");
    convert_inline_formatting(&joined)
}

/// Convert a single line according to Slack formatting rules.
fn convert_line(line: &str) -> String {
    // H1: # Title -> *Title* (bold in Slack)
    if let Some(caps) = H1_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("*{}*", title);
    }

    // H2: ## Title -> *Title*
    if let Some(caps) = H2_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("*{}*", title);
    }

    // H3: ### Title -> _Title_ (italic)
    if let Some(caps) = H3_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("_{}_", title);
    }

    // H4: #### Title -> _Title_
    if let Some(caps) = H4_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("_{}_", title);
    }

    // Dash list: - item -> • item (preserve indentation)
    if let Some(caps) = DASH_LIST_PATTERN.captures(line) {
        let indent = caps.get(1).map_or("", |m| m.as_str());
        let item = caps.get(2).map_or("", |m| m.as_str());
        return format!("{}• {}", indent, item);
    }

    line.to_string()
}

/// Convert inline formatting elements.
fn convert_inline_formatting(text: &str) -> String {
    let mut result = text.to_string();

    // Convert **bold** to *bold* (must do before italic)
    result = BOLD_PATTERN.replace_all(&result, "*$1*").to_string();

    // Convert standalone *italic* to _italic_
    // This is tricky because *bold* in Slack also uses single asterisks
    // We handle this by checking context (after bold conversion)
    result = convert_italic(&result);

    // Convert ~~strikethrough~~ to ~strikethrough~
    result = STRIKETHROUGH_PATTERN.replace_all(&result, "~$1~").to_string();

    // Convert [text](url) to <url|text>
    result = LINK_PATTERN.replace_all(&result, "<$2|$1>").to_string();

    result
}

/// Convert italic formatting carefully.
///
/// This is complex because:
/// - Standard MD uses `*text*` for italic
/// - Slack uses `_text_` for italic
/// - Slack uses `*text*` for bold
/// - We've already converted `**bold**` to `*bold*`
///
/// We need to find remaining `*text*` patterns that aren't bold markers.
fn convert_italic(text: &str) -> String {
    // Simple heuristic: if we see *word* patterns that aren't adjacent to other asterisks,
    // convert them to _word_
    // This is a simplified approach - full parsing would require a proper tokenizer

    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '*' {
            // Check if this is a standalone *word* pattern
            // (not **bold** which we've already converted)
            if i + 1 < chars.len() && chars[i + 1] != '*' {
                // Look for closing *
                if let Some(end) = find_closing_asterisk(&chars, i + 1) {
                    // Check this isn't part of a ** pair
                    let is_standalone = (i == 0 || chars[i - 1] != '*')
                        && (end + 1 >= chars.len() || chars[end + 1] != '*');

                    if is_standalone {
                        let content: String = chars[i + 1..end].iter().collect();
                        // Only convert if content doesn't contain spaces at boundaries
                        // (Slack mrkdwn requires no space after opening marker)
                        if !content.starts_with(' ') && !content.ends_with(' ') {
                            result.push('_');
                            result.push_str(&content);
                            result.push('_');
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Find the closing asterisk for an italic span.
fn find_closing_asterisk(chars: &[char], start: usize) -> Option<usize> {
    for i in start..chars.len() {
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '\\') {
            // Ensure it's not an empty span
            if i > start {
                return Some(i);
            }
        }
    }
    None
}

/// Split a message into chunks that fit Slack's character limit.
pub fn split_message(text: &str) -> Vec<String> {
    if text.len() <= MAX_BLOCK_TEXT_LENGTH {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for line in text.lines() {
        let line_with_newline = if current_chunk.is_empty() {
            line.to_string()
        } else {
            format!("\n{}", line)
        };

        if current_chunk.len() + line_with_newline.len() > MAX_BLOCK_TEXT_LENGTH {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
            }

            // If a single line is too long, split it
            if line.len() > MAX_BLOCK_TEXT_LENGTH {
                let mut remaining = line;
                while remaining.len() > MAX_BLOCK_TEXT_LENGTH {
                    let split_point = find_split_point(remaining, MAX_BLOCK_TEXT_LENGTH);
                    chunks.push(remaining[..split_point].to_string());
                    remaining = &remaining[split_point..];
                }
                current_chunk = remaining.to_string();
            } else {
                current_chunk = line.to_string();
            }
        } else {
            current_chunk.push_str(&line_with_newline);
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

/// Find a good point to split text (preferring spaces, punctuation).
fn find_split_point(text: &str, max_len: usize) -> usize {
    if text.len() <= max_len {
        return text.len();
    }

    // Look for last space before max_len
    if let Some(pos) = text[..max_len].rfind(' ') {
        return pos + 1;
    }

    max_len
}

/// Format a message with Slack Block Kit-like structure (text approximation).
///
/// This creates a text-based approximation of Slack blocks.
pub fn format_as_blocks(
    header: Option<&str>,
    sections: &[(&str, Option<&str>)],
    context: Option<&str>,
) -> String {
    let mut result = String::new();

    // Header
    if let Some(h) = header {
        result.push_str(&format!("*{}*\n\n", h));
    }

    // Sections
    for (text, accessory) in sections {
        result.push_str(text);
        if let Some(acc) = accessory {
            result.push_str(&format!("  _{}_", acc));
        }
        result.push('\n');
    }

    // Context (smaller text)
    if let Some(ctx) = context {
        result.push_str(&format!("\n_{}_", ctx));
    }

    result.trim_end().to_string()
}

/// Escape special characters for Slack mrkdwn.
pub fn escape_mrkdwn(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Create a mention for a user.
pub fn mention_user(user_id: &str) -> String {
    format!("<@{}>", user_id)
}

/// Create a mention for a channel.
pub fn mention_channel(channel_id: &str) -> String {
    format!("<#{}>", channel_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_h1() {
        assert_eq!(convert_to_slack_mrkdwn("# Title"), "*Title*");
    }

    #[test]
    fn convert_h2() {
        assert_eq!(convert_to_slack_mrkdwn("## Subtitle"), "*Subtitle*");
    }

    #[test]
    fn convert_h3() {
        assert_eq!(convert_to_slack_mrkdwn("### Section"), "_Section_");
    }

    #[test]
    fn convert_bold() {
        assert_eq!(convert_to_slack_mrkdwn("**bold text**"), "*bold text*");
    }

    #[test]
    fn convert_strikethrough() {
        assert_eq!(convert_to_slack_mrkdwn("~~deleted~~"), "~deleted~");
    }

    #[test]
    fn convert_link() {
        assert_eq!(
            convert_to_slack_mrkdwn("[Click here](https://example.com)"),
            "<https://example.com|Click here>"
        );
    }

    #[test]
    fn convert_list() {
        assert_eq!(convert_to_slack_mrkdwn("- Item 1"), "• Item 1");
    }

    #[test]
    fn preserve_code_blocks() {
        let input = "```rust\nfn main() {}\n```";
        let result = convert_to_slack_mrkdwn(input);
        assert!(result.contains("```"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn escape_special_chars() {
        assert_eq!(escape_mrkdwn("<script>alert('xss')</script>"),
                   "&lt;script&gt;alert('xss')&lt;/script&gt;");
    }

    #[test]
    fn mention_formatting() {
        assert_eq!(mention_user("U12345"), "<@U12345>");
        assert_eq!(mention_channel("C12345"), "<#C12345>");
    }

    #[test]
    fn format_blocks_basic() {
        let result = format_as_blocks(
            Some("Header"),
            &[("Section 1 text", None)],
            Some("Context"),
        );
        assert!(result.contains("*Header*"));
        assert!(result.contains("Section 1"));
        assert!(result.contains("_Context_"));
    }

    #[test]
    fn mixed_content() {
        let input = "## Summary\n- Point 1\n- Point 2";
        let result = convert_to_slack_mrkdwn(input);
        assert!(result.contains("*Summary*"));
        assert!(result.contains("• Point 1"));
        assert!(result.contains("• Point 2"));
    }

    #[test]
    fn chinese_content() {
        let input = "## 总结\n- 第一项";
        let result = convert_to_slack_mrkdwn(input);
        assert!(result.contains("*总结*"));
        assert!(result.contains("• 第一项"));
    }

    #[test]
    fn split_short_message() {
        let text = "Short message";
        let chunks = split_message(text);
        assert_eq!(chunks.len(), 1);
    }
}
