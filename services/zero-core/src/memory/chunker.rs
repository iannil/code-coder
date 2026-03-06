//! Line-based markdown chunker — splits documents into semantic chunks.
//!
//! Splits on markdown headings and paragraph boundaries, respecting
//! a max token limit per chunk. Preserves heading context.
//!
//! Adapted from zero-memory/src/chunker.rs

use serde::{Deserialize, Serialize};

/// Configuration for the chunker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Maximum tokens per chunk (~4 chars per token)
    pub max_tokens: usize,
    /// Overlap tokens between chunks (for context preservation)
    pub overlap_tokens: usize,
    /// Preserve heading context in split chunks
    pub preserve_headings: bool,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 0,
            preserve_headings: true,
        }
    }
}

/// A single chunk of text with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Chunk index (0-based)
    pub index: usize,
    /// Chunk content
    pub content: String,
    /// Heading context (if any)
    pub heading: Option<String>,
    /// Character offset in original text
    pub start_offset: usize,
    /// Character end offset in original text
    pub end_offset: usize,
}

/// Split markdown text into chunks, each under `max_tokens` approximate tokens.
///
/// Strategy:
/// 1. Split on `## ` and `# ` headings (keeps heading with its content)
/// 2. If a section exceeds `max_tokens`, split on blank lines (paragraphs)
/// 3. If a paragraph still exceeds, split on line boundaries
///
/// Token estimation: ~4 chars per token (rough English average).
pub fn chunk_markdown(text: &str, max_tokens: usize) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let max_chars = max_tokens.saturating_mul(4).max(4);
    let sections = split_on_headings(text);
    let mut chunks = Vec::new();
    let mut offset = 0;

    for (heading, body) in sections {
        let full = if let Some(ref h) = heading {
            format!("{h}\n{body}")
        } else {
            body.clone()
        };

        let section_len = full.len();

        if full.len() <= max_chars {
            if !full.trim().is_empty() {
                chunks.push(Chunk {
                    index: chunks.len(),
                    content: full.trim().to_string(),
                    heading: heading.clone(),
                    start_offset: offset,
                    end_offset: offset + section_len,
                });
            }
        } else {
            // Split on paragraphs (blank lines)
            let paragraphs = split_on_blank_lines(&body);
            let mut current = heading
                .as_ref()
                .map_or_else(String::new, |h| format!("{h}\n"));
            let mut current_start = offset;

            for para in paragraphs {
                if current.len() + para.len() > max_chars && !current.trim().is_empty() {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        content: current.trim().to_string(),
                        heading: heading.clone(),
                        start_offset: current_start,
                        end_offset: current_start + current.len(),
                    });
                    current_start = current_start + current.len();
                    current = heading
                        .as_ref()
                        .map_or_else(String::new, |h| format!("{h}\n"));
                }

                if para.len() > max_chars {
                    // Paragraph too big — split on lines
                    if !current.trim().is_empty() {
                        chunks.push(Chunk {
                            index: chunks.len(),
                            content: current.trim().to_string(),
                            heading: heading.clone(),
                            start_offset: current_start,
                            end_offset: current_start + current.len(),
                        });
                        current_start = current_start + current.len();
                        current = heading
                            .as_ref()
                            .map_or_else(String::new, |h| format!("{h}\n"));
                    }
                    for line_chunk in split_on_lines(&para, max_chars) {
                        if !line_chunk.trim().is_empty() {
                            let chunk_len = line_chunk.len();
                            chunks.push(Chunk {
                                index: chunks.len(),
                                content: line_chunk.trim().to_string(),
                                heading: heading.clone(),
                                start_offset: current_start,
                                end_offset: current_start + chunk_len,
                            });
                            current_start += chunk_len;
                        }
                    }
                } else {
                    current.push_str(&para);
                    current.push('\n');
                }
            }

            if !current.trim().is_empty() {
                chunks.push(Chunk {
                    index: chunks.len(),
                    content: current.trim().to_string(),
                    heading: heading.clone(),
                    start_offset: current_start,
                    end_offset: current_start + current.len(),
                });
            }
        }

        offset += section_len;
    }

    // Filter out empty chunks and re-index
    chunks.retain(|c| !c.content.is_empty());
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
    }

    chunks
}

/// Split markdown text with custom configuration
pub fn chunk_markdown_with_config(text: &str, config: &ChunkerConfig) -> Vec<Chunk> {
    chunk_markdown(text, config.max_tokens)
}

/// Split text into `(heading, body)` sections.
fn split_on_headings(text: &str) -> Vec<(Option<String>, String)> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body = String::new();

    for line in text.lines() {
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            if !current_body.trim().is_empty() || current_heading.is_some() {
                sections.push((current_heading.take(), current_body.clone()));
                current_body.clear();
            }
            current_heading = Some(line.to_string());
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    if !current_body.trim().is_empty() || current_heading.is_some() {
        sections.push((current_heading, current_body));
    }

    sections
}

/// Split text on blank lines (paragraph boundaries)
fn split_on_blank_lines(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                paragraphs.push(current.clone());
                current.clear();
            }
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        paragraphs.push(current);
    }

    paragraphs
}

/// Split text on line boundaries to fit within `max_chars`
fn split_on_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > max_chars && !current.is_empty() {
            chunks.push(current.clone());
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Estimate token count for text (~4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        assert!(chunk_markdown("", 512).is_empty());
        assert!(chunk_markdown("   ", 512).is_empty());
    }

    #[test]
    fn single_short_paragraph() {
        let chunks = chunk_markdown("Hello world", 512);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello world");
        assert!(chunks[0].heading.is_none());
    }

    #[test]
    fn heading_sections() {
        let text = "# Title\nSome intro.\n\n## Section A\nContent A.\n\n## Section B\nContent B.";
        let chunks = chunk_markdown(text, 512);
        assert!(chunks.len() >= 3);
    }

    #[test]
    fn respects_max_tokens() {
        let long_text: String = (0..200).fold(String::new(), |mut s, i| {
            use std::fmt::Write;
            let _ = writeln!(s, "This is sentence number {i} with some extra words.");
            s
        });
        let chunks = chunk_markdown(&long_text, 50);
        assert!(chunks.len() > 1, "Expected multiple chunks, got {}", chunks.len());
        for chunk in &chunks {
            assert!(chunk.content.len() <= 300, "Chunk too long: {} chars", chunk.content.len());
        }
    }

    #[test]
    fn preserves_heading_in_split_sections() {
        let mut text = String::from("## Big Section\n");
        for i in 0..100 {
            use std::fmt::Write;
            let _ = write!(text, "Line {i} with some content here.\n\n");
        }
        let chunks = chunk_markdown(&text, 50);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            if chunk.heading.is_some() {
                assert_eq!(chunk.heading.as_deref(), Some("## Big Section"));
            }
        }
    }

    #[test]
    fn indexes_are_sequential() {
        let text = "# A\nContent A\n\n# B\nContent B\n\n# C\nContent C";
        let chunks = chunk_markdown(text, 512);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn chunk_count_reasonable() {
        let text = "Hello world. This is a test document.";
        let chunks = chunk_markdown(text, 512);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn unicode_content() {
        let text = "# 日本語\nこんにちは世界\n\n## Émojis\n🐦‍🔥 Rust is great 🚀";
        let chunks = chunk_markdown(text, 512);
        assert!(!chunks.is_empty());
        let all: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(all.contains("こんにちは"));
        assert!(all.contains("🐦‍🔥"));
    }

    #[test]
    fn estimate_tokens_basic() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn config_defaults() {
        let config = ChunkerConfig::default();
        assert_eq!(config.max_tokens, 512);
        assert!(config.preserve_headings);
    }

    #[test]
    fn with_config() {
        let config = ChunkerConfig {
            max_tokens: 100,
            overlap_tokens: 0,
            preserve_headings: true,
        };
        let chunks = chunk_markdown_with_config("Hello world", &config);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_has_offsets() {
        let text = "# Title\nContent here";
        let chunks = chunk_markdown(text, 512);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].start_offset, 0);
        assert!(chunks[0].end_offset > 0);
    }

    #[test]
    fn max_tokens_zero_handled() {
        let chunks = chunk_markdown("Hello world", 0);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn very_long_single_line() {
        let text = "word ".repeat(5000);
        let chunks = chunk_markdown(&text, 50);
        assert!(!chunks.is_empty());
    }
}
