//! Line-based markdown chunker — splits documents into semantic chunks.
//!
//! This module re-exports chunker operations from `crate::memory::chunker` to maintain
//! backward compatibility. The main implementation includes offset tracking and
//! configurable chunking options.
//!
//! ## Migration Note
//!
//! This module previously contained a duplicate implementation without offset tracking.
//! It now re-exports from `crate::memory::chunker` to consolidate code and
//! ensure consistent behavior across the codebase.

// Re-export all chunker operations from the main memory::chunker module
pub use crate::memory::chunker::{
    // Core functions
    chunk_markdown,
    chunk_markdown_with_config,
    estimate_tokens,
    // Types
    Chunk,
    ChunkerConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic smoke tests to verify re-exports work correctly

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
    fn indexes_are_sequential() {
        let text = "# A\nContent A\n\n# B\nContent B\n\n# C\nContent C";
        let chunks = chunk_markdown(text, 512);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
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
}
