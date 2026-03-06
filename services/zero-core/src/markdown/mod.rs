//! Native Markdown parser using pulldown-cmark.
//!
//! Provides high-performance Markdown parsing capabilities:
//! - AST-based parsing with typed node output
//! - Heading extraction with hierarchy
//! - Code block extraction with language tags
//! - Link extraction (inline and reference)
//! - HTML rendering
//!
//! # Example
//!
//! ```rust
//! use zero_core::markdown::{parse_markdown, extract_headings, extract_code_blocks};
//!
//! let text = "# Title\n\nSome text\n\n```rust\nfn main() {}\n```";
//!
//! // Extract headings
//! let headings = extract_headings(text);
//! assert_eq!(headings[0].text, "Title");
//! assert_eq!(headings[0].level, 1);
//!
//! // Extract code blocks
//! let blocks = extract_code_blocks(text);
//! assert_eq!(blocks[0].language, Some("rust".to_string()));
//! ```

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd, Options, html};
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Markdown heading with level and position info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Heading {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text content
    pub text: String,
    /// Line number in source (1-indexed)
    pub line: usize,
}

/// Fenced or indented code block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeBlock {
    /// Language identifier (e.g., "rust", "python")
    pub language: Option<String>,
    /// Code content
    pub code: String,
    /// Start line in source (1-indexed)
    pub start_line: usize,
    /// End line in source (1-indexed)
    pub end_line: usize,
}

/// Link (inline or reference)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Link {
    /// URL destination
    pub url: String,
    /// Link text
    pub text: String,
    /// Optional title attribute
    pub title: Option<String>,
    /// Line number in source (1-indexed)
    pub line: usize,
}

/// Image reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    /// Image URL/path
    pub url: String,
    /// Alt text
    pub alt: String,
    /// Optional title attribute
    pub title: Option<String>,
    /// Line number in source (1-indexed)
    pub line: usize,
}

/// Markdown node types for AST representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum MarkdownNode {
    /// Heading (h1-h6)
    Heading {
        level: u8,
        text: String,
    },
    /// Paragraph
    Paragraph {
        text: String,
    },
    /// Fenced or indented code block
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    /// Inline code span
    InlineCode {
        code: String,
    },
    /// Unordered or ordered list
    List {
        ordered: bool,
        items: Vec<String>,
    },
    /// Block quote
    BlockQuote {
        text: String,
    },
    /// Link
    Link {
        url: String,
        text: String,
        title: Option<String>,
    },
    /// Image
    Image {
        url: String,
        alt: String,
        title: Option<String>,
    },
    /// Horizontal rule
    HorizontalRule,
    /// Table
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Raw HTML block
    Html {
        content: String,
    },
    /// Task list item
    TaskListItem {
        checked: bool,
        text: String,
    },
    /// Footnote definition
    FootnoteDefinition {
        label: String,
        content: String,
    },
}

// ============================================================================
// Parser Functions
// ============================================================================

/// Parse markdown text into a vector of typed nodes.
///
/// This provides a simplified AST representation that's easier to work with
/// than the raw pulldown-cmark events.
pub fn parse_markdown(text: &str) -> Vec<MarkdownNode> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, opts);
    let mut nodes = Vec::new();
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 0u8;
    let mut in_paragraph = false;
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_content = String::new();
    let mut in_link = false;
    let mut link_url = String::new();
    let mut link_title: Option<String> = None;
    let mut link_text = String::new();
    let mut in_list = false;
    let mut list_ordered = false;
    let mut list_items: Vec<String> = Vec::new();
    let mut current_list_item = String::new();
    let mut in_block_quote = false;
    let mut block_quote_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = heading_level_to_u8(level);
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading {
                    nodes.push(MarkdownNode::Heading {
                        level: heading_level,
                        text: current_text.trim().to_string(),
                    });
                    in_heading = false;
                    current_text.clear();
                }
            }
            Event::Start(Tag::Paragraph) => {
                if !in_list && !in_block_quote {
                    in_paragraph = true;
                    current_text.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if in_paragraph {
                    let trimmed = current_text.trim().to_string();
                    if !trimmed.is_empty() {
                        nodes.push(MarkdownNode::Paragraph { text: trimmed });
                    }
                    in_paragraph = false;
                    current_text.clear();
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        if lang_str.is_empty() { None } else { Some(lang_str) }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    nodes.push(MarkdownNode::CodeBlock {
                        language: code_lang.take(),
                        code: code_content.trim_end().to_string(),
                    });
                    in_code_block = false;
                    code_content.clear();
                }
            }
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                in_link = true;
                link_url = dest_url.to_string();
                link_title = if title.is_empty() { None } else { Some(title.to_string()) };
                link_text.clear();
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    nodes.push(MarkdownNode::Link {
                        url: link_url.clone(),
                        text: link_text.trim().to_string(),
                        title: link_title.take(),
                    });
                    in_link = false;
                    link_text.clear();
                }
            }
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                // Handle image - collect alt text
                let url = dest_url.to_string();
                let img_title = if title.is_empty() { None } else { Some(title.to_string()) };
                // We'll get the alt text from subsequent Text events
                // For now, push a placeholder that we'll handle
                nodes.push(MarkdownNode::Image {
                    url,
                    alt: String::new(),
                    title: img_title,
                });
            }
            Event::Start(Tag::List(first_item)) => {
                in_list = true;
                list_ordered = first_item.is_some();
                list_items.clear();
                current_list_item.clear();
            }
            Event::End(TagEnd::List(_)) => {
                if in_list {
                    if !current_list_item.trim().is_empty() {
                        list_items.push(current_list_item.trim().to_string());
                    }
                    if !list_items.is_empty() {
                        nodes.push(MarkdownNode::List {
                            ordered: list_ordered,
                            items: list_items.clone(),
                        });
                    }
                    in_list = false;
                    list_items.clear();
                    current_list_item.clear();
                }
            }
            Event::Start(Tag::Item) => {
                if !current_list_item.trim().is_empty() {
                    list_items.push(current_list_item.trim().to_string());
                }
                current_list_item.clear();
            }
            Event::End(TagEnd::Item) => {
                // Item content is accumulated
            }
            Event::Start(Tag::BlockQuote) => {
                in_block_quote = true;
                block_quote_text.clear();
            }
            Event::End(TagEnd::BlockQuote) => {
                if in_block_quote {
                    let trimmed = block_quote_text.trim().to_string();
                    if !trimmed.is_empty() {
                        nodes.push(MarkdownNode::BlockQuote { text: trimmed });
                    }
                    in_block_quote = false;
                    block_quote_text.clear();
                }
            }
            Event::Code(code) => {
                if in_heading || in_paragraph || in_link {
                    current_text.push_str(&code);
                } else if in_list {
                    current_list_item.push_str(&code);
                } else if in_block_quote {
                    block_quote_text.push_str(&code);
                } else {
                    nodes.push(MarkdownNode::InlineCode { code: code.to_string() });
                }
            }
            Event::Text(text_content) => {
                let t = text_content.as_ref();
                if in_code_block {
                    code_content.push_str(t);
                } else if in_heading {
                    current_text.push_str(t);
                } else if in_link {
                    link_text.push_str(t);
                } else if in_paragraph {
                    current_text.push_str(t);
                } else if in_list {
                    current_list_item.push_str(t);
                } else if in_block_quote {
                    block_quote_text.push_str(t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_heading || in_paragraph {
                    current_text.push(' ');
                } else if in_list {
                    current_list_item.push(' ');
                } else if in_block_quote {
                    block_quote_text.push(' ');
                }
            }
            Event::Rule => {
                nodes.push(MarkdownNode::HorizontalRule);
            }
            Event::Html(html_content) => {
                nodes.push(MarkdownNode::Html { content: html_content.to_string() });
            }
            Event::TaskListMarker(checked) => {
                // Task list marker - we'd need to track this with the item
                // For now, we handle it by modifying the list item
                if in_list && !current_list_item.is_empty() {
                    let prefix = if checked { "[x] " } else { "[ ] " };
                    current_list_item = format!("{}{}", prefix, current_list_item);
                }
            }
            _ => {}
        }
    }

    nodes
}

/// Extract all headings from markdown text.
///
/// Returns headings with their level (1-6), text content, and line number.
pub fn extract_headings(text: &str) -> Vec<Heading> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(text, opts);
    let mut headings = Vec::new();
    let mut current_heading_text = String::new();
    let mut current_level = 0u8;
    let mut in_heading = false;
    let mut current_line = 1usize;

    // Track line numbers by scanning through events
    let lines: Vec<&str> = text.lines().collect();
    let mut line_idx = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_level = heading_level_to_u8(level);
                current_heading_text.clear();

                // Find the line with this heading
                let marker = "#".repeat(current_level as usize);
                while line_idx < lines.len() {
                    if lines[line_idx].trim_start().starts_with(&marker) {
                        current_line = line_idx + 1;
                        break;
                    }
                    line_idx += 1;
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading {
                    headings.push(Heading {
                        level: current_level,
                        text: current_heading_text.trim().to_string(),
                        line: current_line,
                    });
                    in_heading = false;
                    current_heading_text.clear();
                }
            }
            Event::Text(t) if in_heading => {
                current_heading_text.push_str(&t);
            }
            Event::Code(c) if in_heading => {
                current_heading_text.push_str(&c);
            }
            Event::SoftBreak | Event::HardBreak if in_heading => {
                current_heading_text.push(' ');
            }
            _ => {}
        }
    }

    headings
}

/// Extract all fenced and indented code blocks from markdown text.
///
/// Returns code blocks with language tag, code content, and line range.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(text, opts);
    let mut blocks = Vec::new();
    let mut current_code = String::new();
    let mut current_lang: Option<String> = None;
    let mut in_code_block = false;

    // Line tracking
    let lines: Vec<&str> = text.lines().collect();
    let mut line_idx = 0;
    let mut start_line = 1;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                current_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        if lang_str.is_empty() { None } else { Some(lang_str) }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                current_code.clear();

                // Find the start line (looking for ``` or indented block)
                while line_idx < lines.len() {
                    let trimmed = lines[line_idx].trim_start();
                    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                        start_line = line_idx + 1;
                        line_idx += 1;
                        break;
                    }
                    // Check for indented code block (4 spaces or tab)
                    if lines[line_idx].starts_with("    ") || lines[line_idx].starts_with('\t') {
                        start_line = line_idx + 1;
                        break;
                    }
                    line_idx += 1;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    // Find end line
                    let code_lines = current_code.lines().count();
                    let end_line = start_line + code_lines;

                    blocks.push(CodeBlock {
                        language: current_lang.take(),
                        code: current_code.trim_end().to_string(),
                        start_line,
                        end_line,
                    });
                    in_code_block = false;
                    current_code.clear();
                }
            }
            Event::Text(t) if in_code_block => {
                current_code.push_str(&t);
            }
            _ => {}
        }
    }

    blocks
}

/// Extract all links from markdown text.
///
/// Returns links with URL, text, optional title, and line number.
pub fn extract_links(text: &str) -> Vec<Link> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(text, opts);
    let mut links = Vec::new();
    let mut current_text = String::new();
    let mut current_url = String::new();
    let mut current_title: Option<String> = None;
    let mut in_link = false;

    // Line tracking
    let lines: Vec<&str> = text.lines().collect();
    let mut current_line = 1;
    let mut line_idx = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                in_link = true;
                current_url = dest_url.to_string();
                current_title = if title.is_empty() { None } else { Some(title.to_string()) };
                current_text.clear();

                // Find line with this link
                let url_escaped = current_url.replace('(', "\\(").replace(')', "\\)");
                while line_idx < lines.len() {
                    if lines[line_idx].contains(&current_url) ||
                       lines[line_idx].contains(&url_escaped) ||
                       lines[line_idx].contains('[') {
                        current_line = line_idx + 1;
                        break;
                    }
                    line_idx += 1;
                }
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    links.push(Link {
                        url: current_url.clone(),
                        text: current_text.trim().to_string(),
                        title: current_title.take(),
                        line: current_line,
                    });
                    in_link = false;
                    current_text.clear();
                }
            }
            Event::Text(t) if in_link => {
                current_text.push_str(&t);
            }
            Event::Code(c) if in_link => {
                current_text.push_str(&c);
            }
            _ => {}
        }
    }

    links
}

/// Extract all images from markdown text.
pub fn extract_images(text: &str) -> Vec<Image> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(text, opts);
    let mut images = Vec::new();
    let mut current_alt = String::new();
    let mut current_url = String::new();
    let mut current_title: Option<String> = None;
    let mut in_image = false;

    let lines: Vec<&str> = text.lines().collect();
    let mut current_line = 1;
    let mut line_idx = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                in_image = true;
                current_url = dest_url.to_string();
                current_title = if title.is_empty() { None } else { Some(title.to_string()) };
                current_alt.clear();

                // Find line with this image
                while line_idx < lines.len() {
                    if lines[line_idx].contains("![") {
                        current_line = line_idx + 1;
                        break;
                    }
                    line_idx += 1;
                }
            }
            Event::End(TagEnd::Image) => {
                if in_image {
                    images.push(Image {
                        url: current_url.clone(),
                        alt: current_alt.trim().to_string(),
                        title: current_title.take(),
                        line: current_line,
                    });
                    in_image = false;
                    current_alt.clear();
                }
            }
            Event::Text(t) if in_image => {
                current_alt.push_str(&t);
            }
            _ => {}
        }
    }

    images
}

/// Render markdown text to HTML.
///
/// Uses pulldown-cmark's HTML renderer with common extensions enabled.
pub fn render_to_html(text: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, opts);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Extract frontmatter from markdown text (YAML between --- delimiters).
///
/// Returns the frontmatter content without the delimiters, or None if not present.
pub fn extract_frontmatter(text: &str) -> Option<String> {
    if !text.starts_with("---") {
        return None;
    }

    let rest = &text[3..];
    let end_match = rest.find("\n---");

    end_match.map(|end_idx| rest[..end_idx].trim().to_string())
}

/// Remove frontmatter from markdown text.
///
/// Returns the markdown content without the frontmatter block.
pub fn strip_frontmatter(text: &str) -> &str {
    if !text.starts_with("---") {
        return text;
    }

    let rest = &text[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let after_end = end_idx + 4; // Skip past "\n---"
        if after_end < rest.len() {
            rest[after_end..].trim_start_matches('\n')
        } else {
            ""
        }
    } else {
        text
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headings() {
        let text = "# Title\n\nSome text\n\n## Section 1\n\nContent\n\n### Subsection\n\nMore";
        let headings = extract_headings(text);

        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "Section 1");
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].text, "Subsection");
    }

    #[test]
    fn test_extract_code_blocks() {
        let text = "# Code Example\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n\nAnd more:\n\n```python\nprint('hi')\n```";
        let blocks = extract_code_blocks(text);

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, Some("rust".to_string()));
        assert!(blocks[0].code.contains("fn main()"));
        assert_eq!(blocks[1].language, Some("python".to_string()));
        assert!(blocks[1].code.contains("print"));
    }

    #[test]
    fn test_extract_code_blocks_no_language() {
        let text = "```\nplain code\n```";
        let blocks = extract_code_blocks(text);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, None);
        assert_eq!(blocks[0].code, "plain code");
    }

    #[test]
    fn test_extract_links() {
        let text = "Check out [Rust](https://rust-lang.org) and [this link](https://example.com \"Example\").";
        let links = extract_links(text);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].text, "Rust");
        assert_eq!(links[0].url, "https://rust-lang.org");
        assert_eq!(links[0].title, None);
        assert_eq!(links[1].text, "this link");
        assert_eq!(links[1].url, "https://example.com");
        assert_eq!(links[1].title, Some("Example".to_string()));
    }

    #[test]
    fn test_extract_images() {
        let text = "![Logo](https://example.com/logo.png \"Site Logo\")";
        let images = extract_images(text);

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].url, "https://example.com/logo.png");
        assert_eq!(images[0].alt, "Logo");
        assert_eq!(images[0].title, Some("Site Logo".to_string()));
    }

    #[test]
    fn test_render_to_html() {
        let text = "# Hello\n\nWorld";
        let html = render_to_html(text);

        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<p>World</p>"));
    }

    #[test]
    fn test_parse_markdown_headings() {
        let text = "# Title\n\n## Section";
        let nodes = parse_markdown(text);

        let headings: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::Heading { .. })).collect();
        assert_eq!(headings.len(), 2);
    }

    #[test]
    fn test_parse_markdown_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let nodes = parse_markdown(text);

        let paragraphs: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::Paragraph { .. })).collect();
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn test_parse_markdown_code_block() {
        let text = "```rust\nfn main() {}\n```";
        let nodes = parse_markdown(text);

        let code_blocks: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::CodeBlock { .. })).collect();
        assert_eq!(code_blocks.len(), 1);
        if let MarkdownNode::CodeBlock { language, code } = &code_blocks[0] {
            assert_eq!(language, &Some("rust".to_string()));
            assert!(code.contains("fn main()"));
        }
    }

    #[test]
    fn test_parse_markdown_list() {
        let text = "- Item 1\n- Item 2\n- Item 3";
        let nodes = parse_markdown(text);

        let lists: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::List { .. })).collect();
        assert_eq!(lists.len(), 1);
        if let MarkdownNode::List { ordered, items } = &lists[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        }
    }

    #[test]
    fn test_parse_markdown_block_quote() {
        let text = "> This is a quote\n> with multiple lines";
        let nodes = parse_markdown(text);

        let quotes: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::BlockQuote { .. })).collect();
        assert_eq!(quotes.len(), 1);
    }

    #[test]
    fn test_extract_frontmatter() {
        let text = "---\ntitle: Hello\nauthor: World\n---\n\n# Content";
        let frontmatter = extract_frontmatter(text);

        assert!(frontmatter.is_some());
        let fm = frontmatter.unwrap();
        assert!(fm.contains("title: Hello"));
        assert!(fm.contains("author: World"));
    }

    #[test]
    fn test_extract_frontmatter_none() {
        let text = "# No frontmatter here";
        assert!(extract_frontmatter(text).is_none());
    }

    #[test]
    fn test_strip_frontmatter() {
        let text = "---\ntitle: Test\n---\n\n# Content here";
        let stripped = strip_frontmatter(text);

        assert!(!stripped.contains("---"));
        assert!(stripped.contains("# Content here"));
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let text = "# Just markdown";
        let stripped = strip_frontmatter(text);
        assert_eq!(stripped, text);
    }

    #[test]
    fn test_heading_with_inline_code() {
        let text = "# Using `code` in headings";
        let headings = extract_headings(text);

        assert_eq!(headings.len(), 1);
        assert!(headings[0].text.contains("code"));
    }

    #[test]
    fn test_empty_markdown() {
        let nodes = parse_markdown("");
        assert!(nodes.is_empty());

        let headings = extract_headings("");
        assert!(headings.is_empty());

        let blocks = extract_code_blocks("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_unicode_content() {
        let text = "# 日本語タイトル\n\nこんにちは世界 🌍";
        let nodes = parse_markdown(text);

        assert!(!nodes.is_empty());
        let headings = extract_headings(text);
        assert_eq!(headings[0].text, "日本語タイトル");
    }

    #[test]
    fn test_horizontal_rule() {
        let text = "Before\n\n---\n\nAfter";
        let nodes = parse_markdown(text);

        let rules: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::HorizontalRule)).collect();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_ordered_list() {
        let text = "1. First\n2. Second\n3. Third";
        let nodes = parse_markdown(text);

        let lists: Vec<_> = nodes.iter().filter(|n| matches!(n, MarkdownNode::List { .. })).collect();
        assert_eq!(lists.len(), 1);
        if let MarkdownNode::List { ordered, items } = &lists[0] {
            assert!(ordered);
            assert_eq!(items.len(), 3);
        }
    }
}
