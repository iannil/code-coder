//! NAPI bindings for Ignore Engine
//!
//! Provides Node.js bindings for the high-performance file ignore engine.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

use crate::foundation::ignore::{IgnoreConfig, IgnoreEngine};

// ============================================================================
// NAPI Types
// ============================================================================

/// Configuration for the ignore engine
#[napi(object)]
#[derive(Default)]
pub struct NapiIgnoreConfig {
    /// Use default folder patterns (default: true)
    pub use_default_folders: Option<bool>,
    /// Use default file patterns (default: true)
    pub use_default_files: Option<bool>,
    /// Additional patterns to add
    pub additional_patterns: Option<Vec<String>>,
    /// Patterns to whitelist (never ignore)
    pub whitelist_patterns: Option<Vec<String>>,
    /// Whether to respect .gitignore files (default: true)
    pub respect_gitignore: Option<bool>,
    /// Whether to respect .ccignore files (default: true)
    pub respect_ccignore: Option<bool>,
}

impl From<NapiIgnoreConfig> for IgnoreConfig {
    fn from(config: NapiIgnoreConfig) -> Self {
        IgnoreConfig {
            use_default_folders: config.use_default_folders.unwrap_or(true),
            use_default_files: config.use_default_files.unwrap_or(true),
            additional_patterns: config.additional_patterns.unwrap_or_default(),
            whitelist_patterns: config.whitelist_patterns.unwrap_or_default(),
            respect_gitignore: config.respect_gitignore.unwrap_or(true),
            respect_ccignore: config.respect_ccignore.unwrap_or(true),
        }
    }
}

/// Result of checking if a path should be ignored
#[napi(object)]
pub struct NapiIgnoreCheckResult {
    /// Whether the path should be ignored
    pub ignored: bool,
    /// Which pattern caused the ignore (if any)
    pub matched_pattern: Option<String>,
    /// Whether the match was from a negation rule
    pub negated: bool,
}

// ============================================================================
// NAPI Handle
// ============================================================================

/// Handle to a compiled ignore engine for efficient reuse
#[napi]
pub struct IgnoreEngineHandle {
    inner: IgnoreEngine,
}

#[napi]
impl IgnoreEngineHandle {
    /// Create a new ignore engine with default configuration
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: IgnoreEngine::new(),
        }
    }

    /// Create a new ignore engine with custom configuration
    #[napi(factory)]
    pub fn with_config(config: NapiIgnoreConfig) -> Self {
        Self {
            inner: IgnoreEngine::with_config(config.into()),
        }
    }

    /// Load gitignore rules from a specific file
    #[napi]
    pub fn load_gitignore(&mut self, path: String) -> Result<()> {
        self.inner
            .load_gitignore(Path::new(&path))
            .map_err(|e| Error::from_reason(e))
    }

    /// Add a single pattern to the engine
    #[napi]
    pub fn add_pattern(&mut self, pattern: String) -> Result<()> {
        self.inner
            .add_pattern(&pattern)
            .map_err(|e| Error::from_reason(e))
    }

    /// Check if a path should be ignored
    #[napi]
    pub fn is_ignored(&self, path: String) -> bool {
        self.inner.is_ignored_str(&path)
    }

    /// Check multiple paths at once
    #[napi]
    pub fn is_ignored_batch(&self, paths: Vec<String>) -> Vec<bool> {
        paths
            .iter()
            .map(|p| self.inner.is_ignored_str(p))
            .collect()
    }

    /// Check if a path should be ignored with detailed result
    #[napi]
    pub fn check(&self, path: String) -> NapiIgnoreCheckResult {
        let result = self.inner.check(Path::new(&path));
        NapiIgnoreCheckResult {
            ignored: result.ignored,
            matched_pattern: result.matched_pattern,
            negated: result.negated,
        }
    }

    /// Get all files in a directory that are not ignored
    #[napi]
    pub fn list_files(&self, root: String) -> Vec<String> {
        self.inner
            .list_files(Path::new(&root))
            .into_iter()
            .filter_map(|p| p.to_str().map(String::from))
            .collect()
    }
}

impl Default for IgnoreEngineHandle {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Quick check if a path matches default ignore patterns
#[napi]
pub fn should_ignore_path(path: String) -> bool {
    crate::foundation::ignore::should_ignore(&path)
}

/// Get the default ignore patterns
#[napi]
pub fn get_ignore_default_patterns() -> Vec<String> {
    crate::foundation::ignore::get_default_patterns()
}

/// Get the default ignored folder names
#[napi]
pub fn get_ignore_default_folders() -> Vec<String> {
    crate::foundation::ignore::get_default_folders()
        .into_iter()
        .map(String::from)
        .collect()
}

/// Create an ignore engine with default configuration
#[napi]
pub fn create_ignore_engine() -> IgnoreEngineHandle {
    IgnoreEngineHandle::new()
}

/// Create an ignore engine with custom configuration
#[napi]
pub fn create_ignore_engine_with_config(config: NapiIgnoreConfig) -> IgnoreEngineHandle {
    IgnoreEngineHandle::with_config(config)
}

/// Filter a list of paths, returning only those that are not ignored
#[napi]
pub fn filter_ignored_paths(paths: Vec<String>) -> Vec<String> {
    let engine = IgnoreEngine::new();
    paths
        .into_iter()
        .filter(|p| !engine.is_ignored_str(p))
        .collect()
}

/// Filter a list of paths using custom patterns
#[napi]
pub fn filter_paths_with_patterns(
    paths: Vec<String>,
    additional_patterns: Vec<String>,
) -> Vec<String> {
    let config = IgnoreConfig {
        additional_patterns,
        ..Default::default()
    };
    let engine = IgnoreEngine::with_config(config);
    paths
        .into_iter()
        .filter(|p| !engine.is_ignored_str(p))
        .collect()
}
