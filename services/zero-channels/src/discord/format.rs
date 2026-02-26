//! Discord Markdown format converter.
//!
//! Converts standard Markdown to Discord-compatible Markdown format.
//! Discord supports most Markdown but has some differences:
//!
//! - Uses `**bold**` and `*italic*` (same as standard)
//! - Uses `~~strikethrough~~`
//! - Uses `||spoiler||` for spoilers
//! - Uses `>>> block quote` for multi-line quotes
//! - Code blocks use triple backticks with optional language
//! - Has 2000 character limit per message
//!
//! This module provides intelligent conversion and message splitting.

use regex::Regex;
use std::sync::LazyLock;

/// Maximum message length for Discord.
pub const MAX_MESSAGE_LENGTH: usize = 2000;

/// Pre-compiled regex patterns for Markdown conversion
static H1_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^# (.+)$").unwrap());
static H2_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^## (.+)$").unwrap());
static H3_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^### (.+)$").unwrap());
static H4_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#### (.+)$").unwrap());
#[allow(dead_code)] // Reserved for future quote handling improvements
static MULTILINE_QUOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^> (.+)$").unwrap());

/// Convert standard Markdown to Discord-compatible format.
///
/// # Conversion Rules
///
/// | Input              | Output                  |
/// |--------------------|-------------------------|
/// | `# Title`          | `**__Title__**`         |
/// | `## Subtitle`      | `**Subtitle**`          |
/// | `### Section`      | `__Section__`           |
/// | `#### Subsection`  | `*Subsection*`          |
/// | `> Quote`          | `>>> Quote` (preserved) |
/// | Code blocks        | Preserved as-is         |
/// | `**bold**`         | Preserved as-is         |
pub fn convert_to_discord_markdown(input: &str) -> String {
    // Process line by line
    let lines: Vec<String> = input.lines().map(convert_line).collect();

    // Join lines
    let result = lines.join("\n");

    // Convert multi-line quotes to Discord block quotes
    convert_quotes(&result)
}

/// Convert a single line according to Discord formatting rules.
fn convert_line(line: &str) -> String {
    // H1: # Title -> **__Title__** (bold + underline for emphasis)
    if let Some(caps) = H1_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("**__{}__**", title);
    }

    // H2: ## Title -> **Title** (bold)
    if let Some(caps) = H2_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("**{}**", title);
    }

    // H3: ### Title -> __Title__ (underline)
    if let Some(caps) = H3_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("__{}__", title);
    }

    // H4: #### Title -> *Title* (italic)
    if let Some(caps) = H4_PATTERN.captures(line) {
        let title = caps.get(1).map_or("", |m| m.as_str());
        return format!("*{}*", title);
    }

    // Everything else passes through unchanged
    // Discord supports most standard Markdown natively
    line.to_string()
}

/// Convert standard `> quote` lines to Discord block quotes.
///
/// Discord uses `>>> ` for multi-line block quotes.
/// Single `> ` quotes are kept for single lines.
fn convert_quotes(text: &str) -> String {
    // Check if there are consecutive quote lines
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut in_quote_block = false;
    let mut quote_lines = Vec::new();

    for line in &lines {
        if let Some(stripped) = line.strip_prefix("> ") {
            if !in_quote_block {
                in_quote_block = true;
            }
            quote_lines.push(stripped);
        } else {
            if in_quote_block {
                // End of quote block
                if quote_lines.len() > 1 {
                    // Multi-line: use >>>
                    result.push(format!(">>> {}", quote_lines.join("\n")));
                } else {
                    // Single line: keep as >
                    result.push(format!("> {}", quote_lines[0]));
                }
                quote_lines.clear();
                in_quote_block = false;
            }
            result.push((*line).to_string());
        }
    }

    // Handle trailing quote block
    if !quote_lines.is_empty() {
        if quote_lines.len() > 1 {
            result.push(format!(">>> {}", quote_lines.join("\n")));
        } else {
            result.push(format!("> {}", quote_lines[0]));
        }
    }

    result.join("\n")
}

/// Split a message into chunks that fit Discord's character limit.
///
/// Attempts to split at natural boundaries (newlines, sentences).
pub fn split_message(text: &str) -> Vec<String> {
    if text.len() <= MAX_MESSAGE_LENGTH {
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

        if current_chunk.len() + line_with_newline.len() > MAX_MESSAGE_LENGTH {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
            }

            // If a single line is too long, split it
            if line.len() > MAX_MESSAGE_LENGTH {
                let mut remaining = line;
                while remaining.len() > MAX_MESSAGE_LENGTH {
                    let split_point = find_split_point(remaining, MAX_MESSAGE_LENGTH);
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

    // Fall back to hard split
    max_len
}

/// Create a Discord embed-like format (text-based fallback).
///
/// Discord embeds are typically created via API with structured data,
/// but this provides a text-based approximation for plain messages.
pub fn format_as_embed(title: &str, description: &str, fields: &[(&str, &str)]) -> String {
    let mut result = String::new();

    // Title
    if !title.is_empty() {
        result.push_str(&format!("**__{}__**\n", title));
    }

    // Description
    if !description.is_empty() {
        result.push_str(description);
        result.push('\n');
    }

    // Fields
    if !fields.is_empty() {
        result.push('\n');
        for (name, value) in fields {
            result.push_str(&format!("**{}**\n{}\n", name, value));
        }
    }

    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_h1() {
        assert_eq!(
            convert_to_discord_markdown("# Title"),
            "**__Title__**"
        );
    }

    #[test]
    fn convert_h2() {
        assert_eq!(convert_to_discord_markdown("## Subtitle"), "**Subtitle**");
    }

    #[test]
    fn convert_h3() {
        assert_eq!(convert_to_discord_markdown("### Section"), "__Section__");
    }

    #[test]
    fn convert_h4() {
        assert_eq!(convert_to_discord_markdown("#### Subsection"), "*Subsection*");
    }

    #[test]
    fn preserve_code_blocks() {
        let input = "```rust\nfn main() {}\n```";
        assert_eq!(convert_to_discord_markdown(input), input);
    }

    #[test]
    fn convert_single_quote() {
        let input = "> This is a quote";
        assert_eq!(convert_to_discord_markdown(input), "> This is a quote");
    }

    #[test]
    fn convert_multi_quote() {
        let input = "> Line 1\n> Line 2";
        assert_eq!(convert_to_discord_markdown(input), ">>> Line 1\nLine 2");
    }

    #[test]
    fn split_short_message() {
        let text = "Short message";
        let chunks = split_message(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn split_long_message() {
        let text = "a ".repeat(1500);
        let chunks = split_message(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= MAX_MESSAGE_LENGTH);
        }
    }

    #[test]
    fn format_embed_basic() {
        let result = format_as_embed("Test Title", "Description here", &[]);
        assert!(result.contains("**__Test Title__**"));
        assert!(result.contains("Description here"));
    }

    #[test]
    fn format_embed_with_fields() {
        let result = format_as_embed(
            "Status",
            "System status report",
            &[("CPU", "50%"), ("Memory", "4GB")],
        );
        assert!(result.contains("**CPU**"));
        assert!(result.contains("50%"));
    }

    #[test]
    fn mixed_content() {
        let input = "## Summary\n- Point 1\n- Point 2\n```\ncode\n```";
        let result = convert_to_discord_markdown(input);
        assert!(result.contains("**Summary**"));
        assert!(result.contains("- Point 1"));
        assert!(result.contains("```"));
    }

    #[test]
    fn chinese_content() {
        let input = "## 总结\n- 第一项";
        let expected = "**总结**\n- 第一项";
        assert_eq!(convert_to_discord_markdown(input), expected);
    }
}
