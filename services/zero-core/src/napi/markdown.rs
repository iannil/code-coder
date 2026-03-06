//! NAPI bindings for markdown module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Markdown parsing to AST
//! - Heading extraction
//! - Code block extraction
//! - Link extraction
//! - HTML rendering

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::markdown::{
    self,
    CodeBlock as RustCodeBlock,
    Heading as RustHeading,
    Image as RustImage,
    Link as RustLink,
    MarkdownNode as RustMarkdownNode,
};

// ============================================================================
// NAPI Types
// ============================================================================

/// Markdown heading with level and position info
#[napi(object)]
pub struct NapiMarkdownHeading {
    /// Heading level (1-6)
    pub level: u32,
    /// Heading text content
    pub text: String,
    /// Line number in source (1-indexed)
    pub line: u32,
}

impl From<RustHeading> for NapiMarkdownHeading {
    fn from(h: RustHeading) -> Self {
        Self {
            level: h.level as u32,
            text: h.text,
            line: h.line as u32,
        }
    }
}

/// Fenced or indented code block
#[napi(object)]
pub struct NapiMarkdownCodeBlock {
    /// Language identifier (e.g., "rust", "python")
    pub language: Option<String>,
    /// Code content
    pub code: String,
    /// Start line in source (1-indexed)
    pub start_line: u32,
    /// End line in source (1-indexed)
    pub end_line: u32,
}

impl From<RustCodeBlock> for NapiMarkdownCodeBlock {
    fn from(b: RustCodeBlock) -> Self {
        Self {
            language: b.language,
            code: b.code,
            start_line: b.start_line as u32,
            end_line: b.end_line as u32,
        }
    }
}

/// Link (inline or reference)
#[napi(object)]
pub struct NapiMarkdownLink {
    /// URL destination
    pub url: String,
    /// Link text
    pub text: String,
    /// Optional title attribute
    pub title: Option<String>,
    /// Line number in source (1-indexed)
    pub line: u32,
}

impl From<RustLink> for NapiMarkdownLink {
    fn from(l: RustLink) -> Self {
        Self {
            url: l.url,
            text: l.text,
            title: l.title,
            line: l.line as u32,
        }
    }
}

/// Image reference
#[napi(object)]
pub struct NapiMarkdownImage {
    /// Image URL/path
    pub url: String,
    /// Alt text
    pub alt: String,
    /// Optional title attribute
    pub title: Option<String>,
    /// Line number in source (1-indexed)
    pub line: u32,
}

impl From<RustImage> for NapiMarkdownImage {
    fn from(i: RustImage) -> Self {
        Self {
            url: i.url,
            alt: i.alt,
            title: i.title,
            line: i.line as u32,
        }
    }
}

/// Simplified markdown node for NAPI - uses JSON serialization for complex types
#[napi(object)]
pub struct NapiMarkdownNode {
    /// Node type: "heading", "paragraph", "code_block", "inline_code", "list",
    /// "block_quote", "link", "image", "horizontal_rule", "table", "html",
    /// "task_list_item", "footnote_definition"
    pub node_type: String,
    /// JSON-serialized content (structure depends on node_type)
    pub content: String,
}

impl From<RustMarkdownNode> for NapiMarkdownNode {
    fn from(node: RustMarkdownNode) -> Self {
        let (node_type, content) = match &node {
            RustMarkdownNode::Heading { level, text } => {
                ("heading".to_string(), serde_json::json!({ "level": level, "text": text }).to_string())
            }
            RustMarkdownNode::Paragraph { text } => {
                ("paragraph".to_string(), serde_json::json!({ "text": text }).to_string())
            }
            RustMarkdownNode::CodeBlock { language, code } => {
                ("code_block".to_string(), serde_json::json!({ "language": language, "code": code }).to_string())
            }
            RustMarkdownNode::InlineCode { code } => {
                ("inline_code".to_string(), serde_json::json!({ "code": code }).to_string())
            }
            RustMarkdownNode::List { ordered, items } => {
                ("list".to_string(), serde_json::json!({ "ordered": ordered, "items": items }).to_string())
            }
            RustMarkdownNode::BlockQuote { text } => {
                ("block_quote".to_string(), serde_json::json!({ "text": text }).to_string())
            }
            RustMarkdownNode::Link { url, text, title } => {
                ("link".to_string(), serde_json::json!({ "url": url, "text": text, "title": title }).to_string())
            }
            RustMarkdownNode::Image { url, alt, title } => {
                ("image".to_string(), serde_json::json!({ "url": url, "alt": alt, "title": title }).to_string())
            }
            RustMarkdownNode::HorizontalRule => {
                ("horizontal_rule".to_string(), "{}".to_string())
            }
            RustMarkdownNode::Table { headers, rows } => {
                ("table".to_string(), serde_json::json!({ "headers": headers, "rows": rows }).to_string())
            }
            RustMarkdownNode::Html { content } => {
                ("html".to_string(), serde_json::json!({ "content": content }).to_string())
            }
            RustMarkdownNode::TaskListItem { checked, text } => {
                ("task_list_item".to_string(), serde_json::json!({ "checked": checked, "text": text }).to_string())
            }
            RustMarkdownNode::FootnoteDefinition { label, content } => {
                ("footnote_definition".to_string(), serde_json::json!({ "label": label, "content": content }).to_string())
            }
        };
        Self { node_type, content }
    }
}

// ============================================================================
// NAPI Functions
// ============================================================================

/// Parse markdown text into a vector of typed nodes.
///
/// Returns an array of nodes, each with a `node_type` and JSON-serialized `content`.
#[napi]
pub fn parse_markdown(text: String) -> Vec<NapiMarkdownNode> {
    markdown::parse_markdown(&text)
        .into_iter()
        .map(|n| n.into())
        .collect()
}

/// Extract all headings from markdown text.
///
/// Returns headings with their level (1-6), text content, and line number.
#[napi]
pub fn extract_markdown_headings(text: String) -> Vec<NapiMarkdownHeading> {
    markdown::extract_headings(&text)
        .into_iter()
        .map(|h| h.into())
        .collect()
}

/// Extract all fenced and indented code blocks from markdown text.
///
/// Returns code blocks with language tag, code content, and line range.
#[napi]
pub fn extract_markdown_code_blocks(text: String) -> Vec<NapiMarkdownCodeBlock> {
    markdown::extract_code_blocks(&text)
        .into_iter()
        .map(|b| b.into())
        .collect()
}

/// Extract all links from markdown text.
///
/// Returns links with URL, text, optional title, and line number.
#[napi]
pub fn extract_markdown_links(text: String) -> Vec<NapiMarkdownLink> {
    markdown::extract_links(&text)
        .into_iter()
        .map(|l| l.into())
        .collect()
}

/// Extract all images from markdown text.
///
/// Returns images with URL, alt text, optional title, and line number.
#[napi]
pub fn extract_markdown_images(text: String) -> Vec<NapiMarkdownImage> {
    markdown::extract_images(&text)
        .into_iter()
        .map(|i| i.into())
        .collect()
}

/// Render markdown text to HTML.
///
/// Uses pulldown-cmark's HTML renderer with common extensions enabled
/// (tables, footnotes, strikethrough, task lists).
#[napi]
pub fn render_markdown_to_html(text: String) -> String {
    markdown::render_to_html(&text)
}

/// Extract frontmatter from markdown text (YAML between --- delimiters).
///
/// Returns the frontmatter content without the delimiters, or null if not present.
#[napi]
pub fn extract_markdown_frontmatter(text: String) -> Option<String> {
    markdown::extract_frontmatter(&text)
}

/// Remove frontmatter from markdown text.
///
/// Returns the markdown content without the frontmatter block.
#[napi]
pub fn strip_markdown_frontmatter(text: String) -> String {
    markdown::strip_frontmatter(&text).to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown() {
        let nodes = parse_markdown("# Hello\n\nWorld".to_string());
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].node_type, "heading");
    }

    #[test]
    fn test_extract_headings() {
        let headings = extract_markdown_headings("# Title\n\n## Section".to_string());
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].level, 2);
    }

    #[test]
    fn test_extract_code_blocks() {
        let blocks = extract_markdown_code_blocks("```rust\nfn main() {}\n```".to_string());
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, Some("rust".to_string()));
    }

    #[test]
    fn test_extract_links() {
        let links = extract_markdown_links("[Rust](https://rust-lang.org)".to_string());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "Rust");
        assert_eq!(links[0].url, "https://rust-lang.org");
    }

    #[test]
    fn test_render_to_html() {
        let html = render_markdown_to_html("# Hello".to_string());
        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_frontmatter() {
        let fm = extract_markdown_frontmatter("---\ntitle: Test\n---\n\n# Content".to_string());
        assert!(fm.is_some());
        assert!(fm.unwrap().contains("title: Test"));
    }

    #[test]
    fn test_strip_frontmatter() {
        let stripped = strip_markdown_frontmatter("---\ntitle: Test\n---\n\n# Content".to_string());
        assert!(!stripped.contains("---"));
        assert!(stripped.contains("# Content"));
    }
}
