//! Grep tool - high-performance content search
//!
//! This module provides regex-based content search using the grep-regex and
//! grep-searcher crates, which are the same libraries used by ripgrep.
//!
//! # Example
//!
//! ```rust,no_run
//! use zero_core::tools::grep::{Grep, GrepOptions};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let grep = Grep::new();
//! let options = GrepOptions {
//!     pattern: "fn main".to_string(),
//!     path: Some(".".to_string()),
//!     glob: Some("*.rs".to_string()),
//!     case_insensitive: false,
//!     ..Default::default()
//! };
//!
//! let result = grep.search(&options).await?;
//! for m in result.matches {
//!     println!("{}:{}: {}", m.path, m.line_number, m.line_content);
//! }
//! # Ok(())
//! # }
//! ```

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::{BinaryDetection, SearcherBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Options for grep search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepOptions {
    /// The regex pattern to search for
    pub pattern: String,

    /// The path to search in (file or directory)
    pub path: Option<String>,

    /// Glob pattern to filter files (e.g., "*.rs", "*.{ts,tsx}")
    pub glob: Option<String>,

    /// File type to search (e.g., "rust", "typescript")
    pub file_type: Option<String>,

    /// Whether to perform case-insensitive search
    #[serde(default)]
    pub case_insensitive: bool,

    /// Output mode: "content", "files_with_matches", or "count"
    #[serde(default = "default_output_mode")]
    pub output_mode: String,

    /// Number of context lines before match
    #[serde(default)]
    pub context_before: usize,

    /// Number of context lines after match
    #[serde(default)]
    pub context_after: usize,

    /// Limit the number of results
    pub limit: Option<usize>,

    /// Skip first N results
    #[serde(default)]
    pub offset: usize,

    /// Enable multiline matching
    #[serde(default)]
    pub multiline: bool,

    /// Show line numbers
    #[serde(default = "default_true")]
    pub line_numbers: bool,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            path: None,
            glob: None,
            file_type: None,
            case_insensitive: false,
            output_mode: "files_with_matches".to_string(),
            context_before: 0,
            context_after: 0,
            limit: None,
            offset: 0,
            multiline: false,
            line_numbers: true,
        }
    }
}

fn default_output_mode() -> String {
    "files_with_matches".to_string()
}

fn default_true() -> bool {
    true
}

/// A single grep match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    /// Path to the file containing the match
    pub path: String,

    /// Line number (1-indexed)
    pub line_number: u64,

    /// Column offset of the match (0-indexed)
    pub column: usize,

    /// The matched line content
    pub line_content: String,

    /// Context lines before the match
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context_before: Vec<String>,

    /// Context lines after the match
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context_after: Vec<String>,
}

/// Result of a grep search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    /// List of matches found
    pub matches: Vec<GrepMatch>,

    /// List of files with matches (for files_with_matches mode)
    pub files: Vec<String>,

    /// Count per file (for count mode)
    pub counts: Vec<(String, usize)>,

    /// Total number of matches
    pub total_matches: usize,

    /// Total number of files searched
    pub files_searched: usize,

    /// Whether the search was truncated due to limits
    pub truncated: bool,
}

impl Default for GrepResult {
    fn default() -> Self {
        Self {
            matches: Vec::new(),
            files: Vec::new(),
            counts: Vec::new(),
            total_matches: 0,
            files_searched: 0,
            truncated: false,
        }
    }
}

/// Grep search engine
pub struct Grep {
    /// Maximum number of matches before stopping
    max_matches: usize,
    /// Maximum file size to search (skip larger files)
    max_file_size: u64,
}

impl Default for Grep {
    fn default() -> Self {
        Self::new()
    }
}

impl Grep {
    /// Create a new Grep instance with default settings
    pub fn new() -> Self {
        Self {
            max_matches: 10_000,
            max_file_size: 50 * 1024 * 1024, // 50MB
        }
    }

    /// Create a new Grep instance with custom limits
    pub fn with_limits(max_matches: usize, max_file_size: u64) -> Self {
        Self {
            max_matches,
            max_file_size,
        }
    }

    /// Perform a grep search with the given options
    pub async fn search(&self, options: &GrepOptions) -> Result<GrepResult> {
        let pattern = &options.pattern;
        let path = options.path.as_deref().unwrap_or(".");
        let path = Path::new(path);

        // Build the regex matcher
        let matcher = RegexMatcher::new_line_matcher(pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        // Build the searcher
        let mut searcher_builder = SearcherBuilder::new();
        searcher_builder
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(options.line_numbers);

        if options.context_before > 0 {
            searcher_builder.before_context(options.context_before);
        }
        if options.context_after > 0 {
            searcher_builder.after_context(options.context_after);
        }
        if options.multiline {
            searcher_builder.multi_line(true);
        }

        let searcher = searcher_builder.build();

        // Build the file walker
        let mut walk_builder = WalkBuilder::new(path);
        walk_builder
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .max_filesize(Some(self.max_file_size))
            .threads(num_cpus::get().min(8));

        // Apply glob filter if provided
        if let Some(glob) = &options.glob {
            let mut override_builder = OverrideBuilder::new(path);
            override_builder
                .add(glob)
                .with_context(|| format!("Invalid glob pattern: {}", glob))?;
            walk_builder.overrides(override_builder.build()?);
        }

        // Apply file type filter if provided
        if let Some(file_type) = &options.file_type {
            let mut types_builder = ignore::types::TypesBuilder::new();
            types_builder.add_defaults();
            types_builder.select(file_type);
            walk_builder.types(types_builder.build()?);
        }

        // Collect results
        let result = Arc::new(Mutex::new(GrepResult::default()));
        let limit = options.limit.unwrap_or(self.max_matches);
        let offset = options.offset;
        let output_mode = options.output_mode.clone();

        let walk = walk_builder.build_parallel();

        // Scope the Arc clone so it gets dropped after walk.run()
        {
            let result_clone = Arc::clone(&result);
            let matcher_clone = matcher.clone();

            walk.run(|| {
                let searcher = searcher.clone();
                let matcher = matcher_clone.clone();
                let result = Arc::clone(&result_clone);
                let output_mode = output_mode.clone();

                Box::new(move |entry| {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => return ignore::WalkState::Continue,
                    };

                    // Skip directories
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        return ignore::WalkState::Continue;
                    }

                    let path = entry.path();
                    let path_str = path.to_string_lossy().to_string();

                    // Search the file
                    let mut file_matches = Vec::new();
                    let mut match_count = 0;

                    let search_result = searcher.clone().search_path(
                        &matcher,
                        path,
                        UTF8(|line_num, line| {
                            match_count += 1;

                            if output_mode == "content" {
                                file_matches.push(GrepMatch {
                                    path: path_str.clone(),
                                    line_number: line_num,
                                    column: 0, // TODO: Calculate column offset
                                    line_content: line.trim_end().to_string(),
                                    context_before: Vec::new(),
                                    context_after: Vec::new(),
                                });
                            }

                            Ok(true)
                        }),
                    );

                    if search_result.is_err() {
                        return ignore::WalkState::Continue;
                    }

                    if match_count > 0 {
                        let mut result = result.blocking_lock();
                        result.files_searched += 1;

                        match output_mode.as_str() {
                            "content" => {
                                // Apply offset and limit
                                let start = offset.min(file_matches.len());
                                let end = (offset + limit).min(file_matches.len());
                                result.matches.extend(file_matches[start..end].to_vec());
                                result.total_matches += match_count;

                                if result.matches.len() >= limit {
                                    result.truncated = true;
                                    return ignore::WalkState::Quit;
                                }
                            }
                            "files_with_matches" => {
                                result.files.push(path_str);
                                result.total_matches += match_count;

                                if result.files.len() >= limit {
                                    result.truncated = true;
                                    return ignore::WalkState::Quit;
                                }
                            }
                            "count" => {
                                result.counts.push((path_str, match_count));
                                result.total_matches += match_count;
                            }
                            _ => {}
                        }
                    }

                    ignore::WalkState::Continue
                })
            });
        } // result_clone dropped here

        let result = Arc::try_unwrap(result)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap result"))?
            .into_inner();

        Ok(result)
    }

    /// Search a single file for matches
    pub async fn search_file(&self, path: &Path, options: &GrepOptions) -> Result<Vec<GrepMatch>> {
        let matcher = RegexMatcher::new_line_matcher(&options.pattern)
            .with_context(|| format!("Invalid regex pattern: {}", options.pattern))?;

        let mut searcher_builder = SearcherBuilder::new();
        searcher_builder
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(options.line_numbers);

        if options.multiline {
            searcher_builder.multi_line(true);
        }

        let mut searcher = searcher_builder.build();
        let mut matches = Vec::new();

        searcher.search_path(
            &matcher,
            path,
            UTF8(|line_num, line| {
                matches.push(GrepMatch {
                    path: path.to_string_lossy().to_string(),
                    line_number: line_num,
                    column: 0,
                    line_content: line.trim_end().to_string(),
                    context_before: Vec::new(),
                    context_after: Vec::new(),
                });
                Ok(true)
            }),
        )?;

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test files
        fs::write(dir.path().join("hello.rs"), "fn main() {\n    println!(\"Hello, world!\");\n}\n").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn greet(name: &str) -> String {\n    format!(\"Hello, {}!\", name)\n}\n").unwrap();
        fs::write(dir.path().join("test.txt"), "This is a test file\nWith multiple lines\n").unwrap();

        dir
    }

    #[tokio::test]
    async fn test_grep_basic() {
        let dir = setup_test_dir();
        let grep = Grep::new();

        let options = GrepOptions {
            pattern: "Hello".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            output_mode: "content".to_string(),
            ..Default::default()
        };

        let result = grep.search(&options).await.unwrap();
        assert!(result.total_matches >= 2);
    }

    #[tokio::test]
    async fn test_grep_glob_filter() {
        let dir = setup_test_dir();
        let grep = Grep::new();

        let options = GrepOptions {
            pattern: "fn".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            glob: Some("*.rs".to_string()),
            output_mode: "files_with_matches".to_string(),
            ..Default::default()
        };

        let result = grep.search(&options).await.unwrap();
        assert!(result.files.iter().all(|f| f.ends_with(".rs")));
    }

    #[tokio::test]
    async fn test_grep_files_with_matches_mode() {
        let dir = setup_test_dir();
        let grep = Grep::new();

        let options = GrepOptions {
            pattern: "fn".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            output_mode: "files_with_matches".to_string(),
            ..Default::default()
        };

        let result = grep.search(&options).await.unwrap();
        assert!(!result.files.is_empty());
        assert!(result.matches.is_empty()); // Should not include matches in this mode
    }

    #[tokio::test]
    async fn test_grep_count_mode() {
        let dir = setup_test_dir();
        let grep = Grep::new();

        let options = GrepOptions {
            pattern: "fn".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            output_mode: "count".to_string(),
            ..Default::default()
        };

        let result = grep.search(&options).await.unwrap();
        assert!(!result.counts.is_empty());
    }
}
