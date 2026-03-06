//! Glob tool - fast file pattern matching
//!
//! This module provides parallel file traversal with glob pattern matching
//! using the ignore crate, which respects .gitignore rules by default.
//!
//! # Example
//!
//! ```rust,no_run
//! use zero_core::tools::glob::{Glob, GlobOptions};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let glob = Glob::new();
//! let options = GlobOptions {
//!     pattern: "**/*.rs".to_string(),
//!     path: Some(".".to_string()),
//!     ..Default::default()
//! };
//!
//! let result = glob.find(&options).await?;
//! for file in result.files {
//!     println!("{}", file.path);
//! }
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use globset::Glob as GlobPattern;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::FileInfo;

/// Options for glob search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobOptions {
    /// The glob pattern to match (e.g., "**/*.rs", "src/**/*.ts")
    pub pattern: String,

    /// The path to search in (defaults to current directory)
    pub path: Option<String>,

    /// Whether to include hidden files (starting with .)
    #[serde(default)]
    pub include_hidden: bool,

    /// Whether to respect .gitignore files
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,

    /// Maximum depth to traverse (None for unlimited)
    pub max_depth: Option<usize>,

    /// Limit the number of results
    pub limit: Option<usize>,

    /// Sort results by modification time (newest first)
    #[serde(default)]
    pub sort_by_mtime: bool,

    /// Only include files (no directories)
    #[serde(default = "default_true")]
    pub files_only: bool,

    /// Follow symbolic links
    #[serde(default)]
    pub follow_symlinks: bool,
}

impl Default for GlobOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            path: None,
            include_hidden: false,
            respect_gitignore: true,
            max_depth: None,
            limit: None,
            sort_by_mtime: false,
            files_only: true,
            follow_symlinks: false,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Result of a glob search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobResult {
    /// List of matching files/directories
    pub files: Vec<FileInfo>,

    /// Total number of matches (before limit)
    pub total_matches: usize,

    /// Whether the search was truncated due to limits
    pub truncated: bool,

    /// Search duration in milliseconds
    pub duration_ms: u64,
}

impl Default for GlobResult {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            total_matches: 0,
            truncated: false,
            duration_ms: 0,
        }
    }
}

/// Glob search engine
pub struct Glob {
    /// Maximum number of results
    max_results: usize,
}

impl Default for Glob {
    fn default() -> Self {
        Self::new()
    }
}

impl Glob {
    /// Create a new Glob instance with default settings
    pub fn new() -> Self {
        Self {
            max_results: 10_000,
        }
    }

    /// Create a new Glob instance with custom limits
    pub fn with_limits(max_results: usize) -> Self {
        Self { max_results }
    }

    /// Find files matching the glob pattern
    pub async fn find(&self, options: &GlobOptions) -> Result<GlobResult> {
        let start = std::time::Instant::now();
        let pattern = &options.pattern;
        let path = options.path.as_deref().unwrap_or(".");
        let path = Path::new(path);

        // Build the file walker
        let mut walk_builder = WalkBuilder::new(path);
        walk_builder
            .hidden(!options.include_hidden)
            .ignore(options.respect_gitignore)
            .git_ignore(options.respect_gitignore)
            .git_global(options.respect_gitignore)
            .git_exclude(options.respect_gitignore)
            .follow_links(options.follow_symlinks)
            .threads(num_cpus::get().min(8));

        if let Some(depth) = options.max_depth {
            walk_builder.max_depth(Some(depth));
        }

        // Apply glob pattern
        let mut override_builder = OverrideBuilder::new(path);
        override_builder
            .add(pattern)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?;
        walk_builder.overrides(override_builder.build()?);

        // Collect results
        let limit = options.limit.unwrap_or(self.max_results);
        let files_only = options.files_only;

        let results = Arc::new(Mutex::new(Vec::new()));
        let total_matches = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let truncated = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let walk = walk_builder.build_parallel();

        // Scope the Arc clones so they get dropped after walk.run()
        {
            let results_clone = Arc::clone(&results);
            let total_matches_clone = Arc::clone(&total_matches);
            let truncated_clone = Arc::clone(&truncated);

            walk.run(|| {
                let results = Arc::clone(&results_clone);
                let total_matches = Arc::clone(&total_matches_clone);
                let truncated = Arc::clone(&truncated_clone);

                Box::new(move |entry| {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => return ignore::WalkState::Continue,
                    };

                    // Skip directories if files_only is true
                    if files_only && entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        return ignore::WalkState::Continue;
                    }

                    // Skip the root path itself
                    if entry.depth() == 0 {
                        return ignore::WalkState::Continue;
                    }

                    let count = total_matches.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                    if count > limit {
                        truncated.store(true, std::sync::atomic::Ordering::Relaxed);
                        return ignore::WalkState::Quit;
                    }

                    if let Ok(file_info) = FileInfo::from_path(entry.path()) {
                        let mut results = results.blocking_lock();
                        results.push(file_info);
                    }

                    ignore::WalkState::Continue
                })
            });
        } // results_clone, total_matches_clone, truncated_clone dropped here

        let mut files = Arc::try_unwrap(results)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap results"))?
            .into_inner();

        // Sort by modification time if requested
        if options.sort_by_mtime {
            files.sort_by(|a, b| b.modified.cmp(&a.modified));
        }

        let total_matches = total_matches.load(std::sync::atomic::Ordering::Relaxed);
        let truncated = truncated.load(std::sync::atomic::Ordering::Relaxed);

        Ok(GlobResult {
            files,
            total_matches,
            truncated,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Check if a path matches a glob pattern
    pub fn matches(pattern: &str, path: &str) -> Result<bool> {
        let glob = GlobPattern::new(pattern)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?;
        let matcher = glob.compile_matcher();
        Ok(matcher.is_match(path))
    }

    /// Expand a glob pattern to a list of files (blocking)
    pub fn expand_sync(pattern: &str, base_path: Option<&str>) -> Result<Vec<PathBuf>> {
        let path = base_path.unwrap_or(".");
        let path = Path::new(path);

        let mut override_builder = OverrideBuilder::new(path);
        override_builder
            .add(pattern)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        let mut walk_builder = WalkBuilder::new(path);
        walk_builder
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .overrides(override_builder.build()?);

        let mut results = Vec::new();

        for entry in walk_builder.build() {
            if let Ok(entry) = entry {
                if entry.depth() > 0 && entry.file_type().is_some_and(|ft| ft.is_file()) {
                    results.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test directory structure
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();

        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(dir.path().join("tests/test.rs"), "#[test] fn test() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        dir
    }

    #[tokio::test]
    async fn test_glob_basic() {
        let dir = setup_test_dir();
        let glob = Glob::new();

        let options = GlobOptions {
            pattern: "**/*.rs".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = glob.find(&options).await.unwrap();
        assert_eq!(result.files.len(), 3); // main.rs, lib.rs, test.rs
    }

    #[tokio::test]
    async fn test_glob_specific_dir() {
        let dir = setup_test_dir();
        let glob = Glob::new();

        let options = GlobOptions {
            pattern: "src/*.rs".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = glob.find(&options).await.unwrap();
        assert_eq!(result.files.len(), 2); // main.rs, lib.rs
    }

    #[tokio::test]
    async fn test_glob_with_limit() {
        let dir = setup_test_dir();
        let glob = Glob::new();

        let options = GlobOptions {
            pattern: "**/*".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            limit: Some(2),
            ..Default::default()
        };

        let result = glob.find(&options).await.unwrap();
        assert!(result.files.len() <= 2);
        assert!(result.truncated);
    }

    #[test]
    fn test_glob_matches() {
        assert!(Glob::matches("*.rs", "main.rs").unwrap());
        assert!(Glob::matches("**/*.rs", "src/main.rs").unwrap());
        assert!(!Glob::matches("*.rs", "main.txt").unwrap());
    }

    #[test]
    fn test_glob_expand_sync() {
        let dir = setup_test_dir();
        let files = Glob::expand_sync(
            "**/*.rs",
            Some(&dir.path().to_string_lossy()),
        )
        .unwrap();
        assert_eq!(files.len(), 3);
    }
}
