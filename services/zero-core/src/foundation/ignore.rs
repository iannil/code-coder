//! File Ignore Engine - gitignore-compatible file filtering
//!
//! Provides high-performance file path matching using the `ignore` crate
//! (same implementation as ripgrep). Supports:
//! - Standard gitignore patterns
//! - Negation patterns (!important.log)
//! - Custom ignore files (.ccignore)
//! - Default patterns for common ignore paths

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};

// ============================================================================
// Default Ignore Patterns
// ============================================================================

/// Default folder names that should always be ignored
const DEFAULT_FOLDERS: &[&str] = &[
    // Package managers
    "node_modules",
    "bower_components",
    ".pnpm-store",
    "vendor",
    ".npm",
    // Build outputs
    "dist",
    "build",
    "out",
    ".next",
    "target",
    "bin",
    "obj",
    // VCS
    ".git",
    ".svn",
    ".hg",
    // IDE
    ".vscode",
    ".idea",
    ".turbo",
    ".output",
    "desktop",
    ".sst",
    // Cache
    ".cache",
    ".webkit-cache",
    "__pycache__",
    ".pytest_cache",
    "mypy_cache",
    // Misc
    ".history",
    ".gradle",
];

/// Default file patterns that should be ignored
const DEFAULT_FILE_PATTERNS: &[&str] = &[
    // Editor swap files
    "**/*.swp",
    "**/*.swo",
    // Python bytecode
    "**/*.pyc",
    // OS files
    "**/.DS_Store",
    "**/Thumbs.db",
    // Logs & temp
    "**/logs/**",
    "**/tmp/**",
    "**/temp/**",
    "**/*.log",
    // Coverage/test outputs
    "**/coverage/**",
    "**/.nyc_output/**",
];

// ============================================================================
// Types
// ============================================================================

/// Result of checking if a path should be ignored
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreCheckResult {
    /// Whether the path should be ignored
    pub ignored: bool,
    /// Which pattern caused the ignore (if any)
    pub matched_pattern: Option<String>,
    /// Whether the match was from a negation rule
    pub negated: bool,
}

/// Configuration for the ignore engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreConfig {
    /// Use default folder patterns
    pub use_default_folders: bool,
    /// Use default file patterns
    pub use_default_files: bool,
    /// Additional patterns to add
    pub additional_patterns: Vec<String>,
    /// Patterns to whitelist (never ignore)
    pub whitelist_patterns: Vec<String>,
    /// Whether to respect .gitignore files
    pub respect_gitignore: bool,
    /// Whether to respect .ccignore files
    pub respect_ccignore: bool,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            use_default_folders: true,
            use_default_files: true,
            additional_patterns: Vec::new(),
            whitelist_patterns: Vec::new(),
            respect_gitignore: true,
            respect_ccignore: true,
        }
    }
}

/// Statistics about ignored files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreStats {
    /// Total paths checked
    pub total_checked: usize,
    /// Total paths ignored
    pub total_ignored: usize,
    /// Total paths allowed
    pub total_allowed: usize,
    /// Breakdown by pattern type
    pub by_pattern: std::collections::HashMap<String, usize>,
}

// ============================================================================
// Ignore Engine Implementation
// ============================================================================

/// High-performance file ignore engine
pub struct IgnoreEngine {
    /// Compiled gitignore patterns
    gitignore: Gitignore,
    /// Set of folder names for O(1) lookup
    folder_set: HashSet<String>,
    /// Whitelist patterns (compiled)
    whitelist: Option<Gitignore>,
    /// Configuration
    config: IgnoreConfig,
}

impl IgnoreEngine {
    /// Create a new ignore engine with default configuration
    pub fn new() -> Self {
        Self::with_config(IgnoreConfig::default())
    }

    /// Create a new ignore engine with custom configuration
    pub fn with_config(config: IgnoreConfig) -> Self {
        let mut builder = GitignoreBuilder::new("");

        // Add default folder patterns
        if config.use_default_folders {
            for folder in DEFAULT_FOLDERS {
                let _ = builder.add_line(None, &format!("**/{}/", folder));
                let _ = builder.add_line(None, &format!("{}/", folder));
            }
        }

        // Add default file patterns
        if config.use_default_files {
            for pattern in DEFAULT_FILE_PATTERNS {
                let _ = builder.add_line(None, pattern);
            }
        }

        // Add additional patterns
        for pattern in &config.additional_patterns {
            let _ = builder.add_line(None, pattern);
        }

        let gitignore = builder.build().unwrap_or_else(|_| {
            GitignoreBuilder::new("").build().unwrap()
        });

        // Build folder set for fast lookup
        let mut folder_set = HashSet::new();
        if config.use_default_folders {
            for folder in DEFAULT_FOLDERS {
                folder_set.insert(folder.to_string());
            }
        }

        // Build whitelist patterns
        let whitelist = if !config.whitelist_patterns.is_empty() {
            let mut wl_builder = GitignoreBuilder::new("");
            for pattern in &config.whitelist_patterns {
                let _ = wl_builder.add_line(None, pattern);
            }
            wl_builder.build().ok()
        } else {
            None
        };

        Self {
            gitignore,
            folder_set,
            whitelist,
            config,
        }
    }

    /// Load gitignore rules from a specific file
    pub fn load_gitignore(&mut self, path: &Path) -> Result<(), String> {
        let mut builder = GitignoreBuilder::new(path.parent().unwrap_or(path));

        // Re-add existing patterns
        if self.config.use_default_folders {
            for folder in DEFAULT_FOLDERS {
                let _ = builder.add_line(None, &format!("**/{}/", folder));
                let _ = builder.add_line(None, &format!("{}/", folder));
            }
        }

        if self.config.use_default_files {
            for pattern in DEFAULT_FILE_PATTERNS {
                let _ = builder.add_line(None, pattern);
            }
        }

        for pattern in &self.config.additional_patterns {
            let _ = builder.add_line(None, pattern);
        }

        // Add patterns from file
        if path.exists() {
            if let Some(err) = builder.add(path) {
                return Err(format!("Failed to load gitignore: {}", err));
            }
        }

        self.gitignore = builder
            .build()
            .map_err(|e| format!("Failed to build gitignore: {}", e))?;

        Ok(())
    }

    /// Add a single pattern to the engine
    pub fn add_pattern(&mut self, pattern: &str) -> Result<(), String> {
        self.config.additional_patterns.push(pattern.to_string());

        let mut builder = GitignoreBuilder::new("");

        // Re-add all patterns
        if self.config.use_default_folders {
            for folder in DEFAULT_FOLDERS {
                let _ = builder.add_line(None, &format!("**/{}/", folder));
            }
        }

        if self.config.use_default_files {
            for pat in DEFAULT_FILE_PATTERNS {
                let _ = builder.add_line(None, pat);
            }
        }

        for pat in &self.config.additional_patterns {
            let _ = builder.add_line(None, pat);
        }

        self.gitignore = builder
            .build()
            .map_err(|e| format!("Failed to build gitignore: {}", e))?;

        Ok(())
    }

    /// Check if a path should be ignored
    pub fn is_ignored(&self, path: &Path) -> bool {
        // Fast path: check folder set first
        for component in path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if let Some(s) = os_str.to_str() {
                    if self.folder_set.contains(s) {
                        return true;
                    }
                }
            }
        }

        // Check whitelist first
        if let Some(ref whitelist) = self.whitelist {
            if whitelist.matched(path, path.is_dir()).is_whitelist() {
                return false;
            }
        }

        // Check gitignore patterns
        let is_dir = path.is_dir();
        self.gitignore.matched(path, is_dir).is_ignore()
    }

    /// Check if a path should be ignored with detailed result
    pub fn check(&self, path: &Path) -> IgnoreCheckResult {
        // Fast path: check folder set first
        for component in path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if let Some(s) = os_str.to_str() {
                    if self.folder_set.contains(s) {
                        return IgnoreCheckResult {
                            ignored: true,
                            matched_pattern: Some(format!("**/{}/", s)),
                            negated: false,
                        };
                    }
                }
            }
        }

        // Check whitelist
        if let Some(ref whitelist) = self.whitelist {
            let matched = whitelist.matched(path, path.is_dir());
            if matched.is_whitelist() {
                return IgnoreCheckResult {
                    ignored: false,
                    matched_pattern: None,
                    negated: true,
                };
            }
        }

        // Check gitignore patterns
        let is_dir = path.is_dir();
        let matched = self.gitignore.matched(path, is_dir);

        IgnoreCheckResult {
            ignored: matched.is_ignore(),
            matched_pattern: None, // The ignore crate doesn't expose which pattern matched
            negated: matched.is_whitelist(),
        }
    }

    /// Check if a path string should be ignored (for NAPI convenience)
    pub fn is_ignored_str(&self, path: &str) -> bool {
        self.is_ignored(Path::new(path))
    }

    /// Get all default patterns as strings
    pub fn default_patterns() -> Vec<String> {
        let mut patterns = Vec::new();

        for folder in DEFAULT_FOLDERS {
            patterns.push(format!("**/{}/", folder));
        }

        for pattern in DEFAULT_FILE_PATTERNS {
            patterns.push(pattern.to_string());
        }

        patterns
    }

    /// Create a walker that respects ignore rules and collect results
    /// Returns all non-ignored directory entries
    pub fn walk(&self, root: &Path) -> Vec<DirEntry> {
        let mut builder = WalkBuilder::new(root);

        builder
            .git_global(self.config.respect_gitignore)
            .git_ignore(self.config.respect_gitignore)
            .git_exclude(self.config.respect_gitignore)
            .hidden(true) // Don't skip hidden by default, let patterns decide
            .parents(true);

        if self.config.respect_ccignore {
            builder.add_custom_ignore_filename(".ccignore");
        }

        builder
            .build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| !self.is_ignored(entry.path()))
            .collect()
    }

    /// Get all files in a directory that are not ignored
    pub fn list_files(&self, root: &Path) -> Vec<PathBuf> {
        self.walk(root)
            .into_iter()
            .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .map(|entry| entry.into_path())
            .collect()
    }
}

impl Default for IgnoreEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Quick check if a path matches default ignore patterns
pub fn should_ignore(path: &str) -> bool {
    let engine = IgnoreEngine::new();
    engine.is_ignored_str(path)
}

/// Get the default ignore patterns
pub fn get_default_patterns() -> Vec<String> {
    IgnoreEngine::default_patterns()
}

/// Get the default ignored folder names
pub fn get_default_folders() -> Vec<&'static str> {
    DEFAULT_FOLDERS.to_vec()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ignore() {
        let engine = IgnoreEngine::new();

        // Folders
        assert!(engine.is_ignored(Path::new("node_modules")));
        assert!(engine.is_ignored(Path::new("foo/node_modules/bar")));
        assert!(engine.is_ignored(Path::new(".git")));
        assert!(engine.is_ignored(Path::new("dist")));

        // Files
        assert!(engine.is_ignored(Path::new("foo.swp")));
        assert!(engine.is_ignored(Path::new("dir/file.swp")));
        assert!(engine.is_ignored(Path::new(".DS_Store")));
    }

    #[test]
    fn test_not_ignored() {
        let engine = IgnoreEngine::new();

        assert!(!engine.is_ignored(Path::new("src/main.rs")));
        assert!(!engine.is_ignored(Path::new("package.json")));
        assert!(!engine.is_ignored(Path::new("README.md")));
    }

    #[test]
    fn test_add_pattern() {
        let mut engine = IgnoreEngine::new();
        engine.add_pattern("**/*.test.ts").unwrap();

        assert!(engine.is_ignored(Path::new("foo.test.ts")));
        assert!(engine.is_ignored(Path::new("src/utils.test.ts")));
        assert!(!engine.is_ignored(Path::new("src/utils.ts")));
    }

    #[test]
    fn test_whitelist() {
        let config = IgnoreConfig {
            whitelist_patterns: vec!["!important.log".to_string()],
            ..Default::default()
        };
        let engine = IgnoreEngine::with_config(config);

        // Note: whitelist patterns with ! prefix should allow the file
        // The ignore crate handles this automatically
    }

    #[test]
    fn test_default_patterns() {
        let patterns = IgnoreEngine::default_patterns();
        assert!(patterns.contains(&"**/node_modules/".to_string()));
        assert!(patterns.contains(&"**/*.swp".to_string()));
    }

    #[test]
    fn test_check_detailed() {
        let engine = IgnoreEngine::new();

        let result = engine.check(Path::new("node_modules/foo"));
        assert!(result.ignored);
        assert!(result.matched_pattern.is_some());

        let result = engine.check(Path::new("src/main.rs"));
        assert!(!result.ignored);
    }
}
