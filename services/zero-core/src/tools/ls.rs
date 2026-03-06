//! List tool - directory listing with ignore patterns
//!
//! This module provides directory listing with:
//! - Gitignore-aware file walking
//! - Configurable ignore patterns
//! - Tree-style output formatting
//! - Limit support to prevent excessive output

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

/// Default patterns to ignore when listing directories
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "node_modules/",
    "__pycache__/",
    ".git/",
    "dist/",
    "build/",
    "target/",
    "vendor/",
    "bin/",
    "obj/",
    ".idea/",
    ".vscode/",
    ".zig-cache/",
    "zig-out/",
    ".coverage/",
    "coverage/",
    "tmp/",
    "temp/",
    ".cache/",
    "cache/",
    "logs/",
    ".venv/",
    "venv/",
    "env/",
];

/// Default limit for number of files to list
pub const DEFAULT_LIMIT: usize = 100;

/// Options for listing directories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsOptions {
    /// Additional glob patterns to ignore
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Maximum number of files to return
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Whether to use default ignore patterns
    #[serde(default = "default_true")]
    pub use_default_ignores: bool,

    /// Whether to show hidden files
    #[serde(default)]
    pub show_hidden: bool,

    /// Whether to follow symlinks
    #[serde(default)]
    pub follow_symlinks: bool,
}

impl Default for LsOptions {
    fn default() -> Self {
        Self {
            ignore: Vec::new(),
            limit: DEFAULT_LIMIT,
            use_default_ignores: true,
            show_hidden: false,
            follow_symlinks: false,
        }
    }
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

fn default_true() -> bool {
    true
}

/// Result of a directory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsResult {
    /// The directory that was listed
    pub path: String,

    /// Files found (relative paths)
    pub files: Vec<String>,

    /// Tree-formatted output
    pub output: String,

    /// Total number of files found
    pub count: usize,

    /// Whether the result was truncated due to limit
    pub truncated: bool,
}

/// Directory lister with ignore support
pub struct Ls {
    /// Default options
    default_options: LsOptions,
}

impl Default for Ls {
    fn default() -> Self {
        Self::new()
    }
}

impl Ls {
    /// Create a new Ls instance with default options
    pub fn new() -> Self {
        Self {
            default_options: LsOptions::default(),
        }
    }

    /// Create a new Ls instance with custom default options
    pub fn with_defaults(options: LsOptions) -> Self {
        Self {
            default_options: options,
        }
    }

    /// List files in a directory
    pub fn list(&self, path: &Path, options: Option<&LsOptions>) -> Result<LsResult> {
        let options = options.unwrap_or(&self.default_options);

        if !path.exists() {
            anyhow::bail!("Directory does not exist: {}", path.display());
        }

        if !path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", path.display());
        }

        // Build the walker
        let mut builder = WalkBuilder::new(path);
        builder
            .hidden(!options.show_hidden)
            .follow_links(options.follow_symlinks)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .ignore(true);

        // Build ignore overrides
        let mut override_builder = ignore::overrides::OverrideBuilder::new(path);

        // Add default ignore patterns
        if options.use_default_ignores {
            for pattern in DEFAULT_IGNORE_PATTERNS {
                override_builder
                    .add(&format!("!{}", pattern))
                    .with_context(|| format!("Invalid ignore pattern: {}", pattern))?;
            }
        }

        // Add custom ignore patterns
        for pattern in &options.ignore {
            override_builder
                .add(&format!("!{}", pattern))
                .with_context(|| format!("Invalid ignore pattern: {}", pattern))?;
        }

        builder.overrides(override_builder.build()?);

        // Collect files
        let mut files = Vec::new();
        let mut truncated = false;

        for entry in builder.build() {
            let entry = entry.with_context(|| "Failed to read directory entry")?;

            // Skip directories
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            // Get relative path
            let relative = entry
                .path()
                .strip_prefix(path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();

            if relative.is_empty() {
                continue;
            }

            files.push(relative);

            if files.len() >= options.limit {
                truncated = true;
                break;
            }
        }

        // Sort files for consistent output
        files.sort();

        // Build tree output
        let output = self.format_tree(path, &files);

        Ok(LsResult {
            path: path.to_string_lossy().to_string(),
            count: files.len(),
            files,
            output,
            truncated,
        })
    }

    /// Format files as a tree structure
    fn format_tree(&self, root: &Path, files: &[String]) -> String {
        // Build directory structure
        let mut dirs: HashSet<String> = HashSet::new();
        let mut files_by_dir: HashMap<String, Vec<String>> = HashMap::new();

        for file in files {
            let parts: Vec<&str> = file.split('/').collect();

            // Add all parent directories
            for i in 0..parts.len() - 1 {
                let dir_path = if i == 0 {
                    parts[0].to_string()
                } else {
                    parts[..=i].join("/")
                };
                dirs.insert(dir_path);
            }

            // Add file to its directory
            let dir = if parts.len() == 1 {
                ".".to_string()
            } else {
                parts[..parts.len() - 1].join("/")
            };

            let filename = parts.last().unwrap().to_string();
            files_by_dir.entry(dir).or_default().push(filename);
        }

        // Render tree
        let mut output = format!("{}/\n", root.display());
        output.push_str(&self.render_dir(".", 0, &dirs, &files_by_dir));
        output
    }

    /// Render a directory and its contents
    fn render_dir(
        &self,
        dir_path: &str,
        depth: usize,
        dirs: &HashSet<String>,
        files_by_dir: &HashMap<String, Vec<String>>,
    ) -> String {
        let mut output = String::new();
        let indent = "  ".repeat(depth);

        // Get child directories
        let mut children: Vec<&String> = dirs
            .iter()
            .filter(|d| {
                let parent = if d.contains('/') {
                    d.rsplit_once('/').map(|(p, _)| p).unwrap_or(".")
                } else {
                    "."
                };
                parent == dir_path && *d != dir_path
            })
            .collect();
        children.sort();

        // Render subdirectories
        for child in children {
            let name = child.rsplit('/').next().unwrap_or(child);
            output.push_str(&format!("{}  {}/\n", indent, name));
            output.push_str(&self.render_dir(child, depth + 1, dirs, files_by_dir));
        }

        // Render files
        if let Some(files) = files_by_dir.get(dir_path) {
            let mut sorted_files = files.clone();
            sorted_files.sort();
            for file in sorted_files {
                output.push_str(&format!("{}  {}\n", indent, file));
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

        // Create test structure
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();

        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(dir.path().join("tests/test.rs"), "// tests").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("node_modules/pkg/index.js"), "// js").unwrap();

        dir
    }

    #[test]
    fn test_ls_basic() {
        let dir = setup_test_dir();
        let ls = Ls::new();

        let result = ls.list(dir.path(), None).unwrap();
        assert!(result.count >= 3); // At least src/main.rs, src/lib.rs, Cargo.toml
        assert!(!result.truncated);
    }

    #[test]
    fn test_ls_ignores_node_modules() {
        let dir = setup_test_dir();
        let ls = Ls::new();

        let result = ls.list(dir.path(), None).unwrap();
        // node_modules should be ignored by default
        assert!(!result.files.iter().any(|f| f.contains("node_modules")));
    }

    #[test]
    fn test_ls_with_limit() {
        let dir = setup_test_dir();
        let ls = Ls::new();

        let options = LsOptions {
            limit: 2,
            ..Default::default()
        };

        let result = ls.list(dir.path(), Some(&options)).unwrap();
        assert_eq!(result.count, 2);
        assert!(result.truncated);
    }

    #[test]
    fn test_ls_custom_ignore() {
        let dir = setup_test_dir();
        let ls = Ls::new();

        let options = LsOptions {
            ignore: vec!["tests/".to_string()],
            ..Default::default()
        };

        let result = ls.list(dir.path(), Some(&options)).unwrap();
        assert!(!result.files.iter().any(|f| f.starts_with("tests/")));
    }

    #[test]
    fn test_ls_nonexistent_dir() {
        let ls = Ls::new();
        let result = ls.list(Path::new("/nonexistent/path"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_ls_tree_output() {
        let dir = setup_test_dir();
        let ls = Ls::new();

        let result = ls.list(dir.path(), None).unwrap();
        assert!(result.output.contains("src/"));
        assert!(result.output.contains("main.rs"));
    }
}
