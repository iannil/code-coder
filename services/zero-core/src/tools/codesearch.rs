//! Code Search tool - semantic code search with context
//!
//! This module provides code-aware search with:
//! - Syntax-aware context extraction
//! - Function/class boundary detection
//! - Multi-file aggregated results
//! - Relevance ranking

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::grep::{Grep, GrepMatch, GrepOptions};

/// Options for code search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchOptions {
    /// The search query
    pub query: String,

    /// Directory to search in
    pub path: Option<String>,

    /// File type filter (e.g., "rust", "typescript")
    pub file_type: Option<String>,

    /// Glob pattern filter
    pub glob: Option<String>,

    /// Include function/class context
    #[serde(default = "default_true")]
    pub include_context: bool,

    /// Maximum context lines to include
    #[serde(default = "default_context_lines")]
    pub context_lines: usize,

    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Case insensitive search
    #[serde(default)]
    pub case_insensitive: bool,

    /// Group results by file
    #[serde(default = "default_true")]
    pub group_by_file: bool,
}

impl Default for CodeSearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            path: None,
            file_type: None,
            glob: None,
            include_context: true,
            context_lines: 3,
            limit: 50,
            case_insensitive: false,
            group_by_file: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_context_lines() -> usize {
    3
}

fn default_limit() -> usize {
    50
}

/// A single code search match with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchMatch {
    /// File path
    pub file: String,

    /// Line number
    pub line: u64,

    /// Column offset
    pub column: usize,

    /// The matching line
    pub content: String,

    /// Context before the match
    pub context_before: Vec<String>,

    /// Context after the match
    pub context_after: Vec<String>,

    /// Enclosing function/class name (if detected)
    pub enclosing_scope: Option<String>,

    /// Relevance score (0.0 - 1.0)
    pub relevance: f64,
}

/// Results grouped by file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    /// File path
    pub file: String,

    /// Matches in this file
    pub matches: Vec<CodeSearchMatch>,

    /// Total matches in this file
    pub match_count: usize,
}

/// Result of a code search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    /// All matches (ungrouped)
    pub matches: Vec<CodeSearchMatch>,

    /// Matches grouped by file
    pub by_file: Vec<FileSearchResult>,

    /// Total number of matches
    pub total_matches: usize,

    /// Total files with matches
    pub files_with_matches: usize,

    /// Whether results were truncated
    pub truncated: bool,

    /// Formatted output
    pub output: String,
}

/// Code search engine
pub struct CodeSearch {
    /// Underlying grep engine
    grep: Grep,
}

impl Default for CodeSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeSearch {
    /// Create a new CodeSearch instance
    pub fn new() -> Self {
        Self { grep: Grep::new() }
    }

    /// Perform a code search
    pub async fn search(&self, options: &CodeSearchOptions) -> Result<CodeSearchResult> {
        // Build grep options
        let grep_options = GrepOptions {
            pattern: options.query.clone(),
            path: options.path.clone(),
            glob: options.glob.clone(),
            file_type: options.file_type.clone(),
            case_insensitive: options.case_insensitive,
            output_mode: "content".to_string(),
            context_before: if options.include_context { options.context_lines } else { 0 },
            context_after: if options.include_context { options.context_lines } else { 0 },
            limit: Some(options.limit),
            ..Default::default()
        };

        // Execute search
        let grep_result = self.grep.search(&grep_options).await?;

        // Convert to code search results
        let matches: Vec<CodeSearchMatch> = grep_result
            .matches
            .into_iter()
            .map(|m| self.enhance_match(m, options))
            .collect();

        // Group by file if requested
        let by_file = if options.group_by_file {
            self.group_by_file(&matches)
        } else {
            Vec::new()
        };

        let total_matches = matches.len();
        let files_with_matches = by_file.len();
        let truncated = grep_result.truncated;

        // Format output
        let output = self.format_output(&matches, &by_file, options);

        Ok(CodeSearchResult {
            matches,
            by_file,
            total_matches,
            files_with_matches,
            truncated,
            output,
        })
    }

    /// Enhance a grep match with code-aware context
    fn enhance_match(&self, m: GrepMatch, options: &CodeSearchOptions) -> CodeSearchMatch {
        // Try to detect enclosing scope
        let enclosing_scope = self.detect_scope(&m.context_before, &m.line_content, &m.path);

        // Calculate relevance score
        let relevance = self.calculate_relevance(&m, options);

        CodeSearchMatch {
            file: m.path,
            line: m.line_number,
            column: m.column,
            content: m.line_content,
            context_before: m.context_before,
            context_after: m.context_after,
            enclosing_scope,
            relevance,
        }
    }

    /// Detect the enclosing function/class/method
    fn detect_scope(&self, context: &[String], _line: &str, path: &str) -> Option<String> {
        // Simple heuristic: look for function/class definitions in context
        let patterns = if path.ends_with(".rs") {
            vec!["fn ", "impl ", "struct ", "enum ", "trait "]
        } else if path.ends_with(".ts") || path.ends_with(".js") {
            vec!["function ", "class ", "const ", "=> {", "async "]
        } else if path.ends_with(".py") {
            vec!["def ", "class ", "async def "]
        } else if path.ends_with(".go") {
            vec!["func ", "type ", "struct "]
        } else {
            vec!["function", "class", "def ", "fn "]
        };

        for line in context.iter().rev() {
            for pattern in &patterns {
                if line.contains(pattern) {
                    // Extract the name
                    return Some(line.trim().to_string());
                }
            }
        }

        None
    }

    /// Calculate relevance score for a match
    fn calculate_relevance(&self, m: &GrepMatch, options: &CodeSearchOptions) -> f64 {
        let mut score: f64 = 0.5;

        // Exact case match is more relevant
        if m.line_content.contains(&options.query) {
            score += 0.2;
        }

        // Matches at word boundaries are more relevant
        let query_lower = options.query.to_lowercase();
        let content_lower = m.line_content.to_lowercase();
        if let Some(pos) = content_lower.find(&query_lower) {
            // Check if it's at a word boundary
            let at_start = pos == 0 || !content_lower.chars().nth(pos - 1).unwrap().is_alphanumeric();
            let at_end = pos + query_lower.len() >= content_lower.len()
                || !content_lower.chars().nth(pos + query_lower.len()).unwrap().is_alphanumeric();
            if at_start && at_end {
                score += 0.2;
            }
        }

        // Shorter lines tend to be more relevant
        if m.line_content.len() < 80 {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Group matches by file
    fn group_by_file(&self, matches: &[CodeSearchMatch]) -> Vec<FileSearchResult> {
        let mut by_file: std::collections::HashMap<String, Vec<CodeSearchMatch>> =
            std::collections::HashMap::new();

        for m in matches {
            by_file.entry(m.file.clone()).or_default().push(m.clone());
        }

        by_file
            .into_iter()
            .map(|(file, matches)| FileSearchResult {
                file,
                match_count: matches.len(),
                matches,
            })
            .collect()
    }

    /// Format output for display
    fn format_output(
        &self,
        matches: &[CodeSearchMatch],
        by_file: &[FileSearchResult],
        options: &CodeSearchOptions,
    ) -> String {
        let mut output = String::new();

        if options.group_by_file && !by_file.is_empty() {
            for file_result in by_file {
                output.push_str(&format!("## {}\n\n", file_result.file));
                for m in &file_result.matches {
                    output.push_str(&format!("{}:{}: {}\n", m.file, m.line, m.content));
                }
                output.push('\n');
            }
        } else {
            for m in matches {
                output.push_str(&format!("{}:{}: {}\n", m.file, m.line, m.content));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test Rust file
        fs::write(
            dir.path().join("lib.rs"),
            r#"
fn hello_world() {
    println!("Hello, world!");
}

struct Greeter {
    name: String,
}

impl Greeter {
    fn greet(&self) {
        println!("Hello, {}!", self.name);
    }
}
"#,
        )
        .unwrap();

        // Create test TypeScript file
        fs::write(
            dir.path().join("index.ts"),
            r#"
function sayHello(name: string) {
    console.log(`Hello, ${name}!`);
}

class Greeter {
    constructor(private name: string) {}

    greet() {
        console.log(`Hello, ${this.name}!`);
    }
}
"#,
        )
        .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_basic_search() {
        let dir = setup_test_dir();
        let search = CodeSearch::new();

        let options = CodeSearchOptions {
            query: "Hello".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = search.search(&options).await.unwrap();
        assert!(result.total_matches >= 2); // At least 2 matches across files
    }

    #[tokio::test]
    async fn test_file_type_filter() {
        let dir = setup_test_dir();
        let search = CodeSearch::new();

        let options = CodeSearchOptions {
            query: "Hello".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            glob: Some("*.rs".to_string()),
            ..Default::default()
        };

        let result = search.search(&options).await.unwrap();
        // Should only find matches in .rs files
        assert!(result.matches.iter().all(|m| m.file.ends_with(".rs")));
    }

    #[tokio::test]
    async fn test_case_insensitive() {
        let dir = setup_test_dir();
        let search = CodeSearch::new();

        let options = CodeSearchOptions {
            query: "hello".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            case_insensitive: true,
            ..Default::default()
        };

        let result = search.search(&options).await.unwrap();
        // Should find matches containing Hello (case insensitive)
        assert!(result.total_matches >= 1);
    }

    #[tokio::test]
    async fn test_group_by_file() {
        let dir = setup_test_dir();
        let search = CodeSearch::new();

        let options = CodeSearchOptions {
            query: "Hello".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            group_by_file: true,
            ..Default::default()
        };

        let result = search.search(&options).await.unwrap();
        assert!(!result.by_file.is_empty());
        // Each file result should have matches
        for file_result in &result.by_file {
            assert!(!file_result.matches.is_empty());
        }
    }
}
