//! Truncation tool - output truncation for large results
//!
//! This module provides output truncation with:
//! - Line-based truncation
//! - Byte-based truncation
//! - Head or tail direction
//! - Full output file storage for later retrieval

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Default maximum lines before truncation
pub const DEFAULT_MAX_LINES: usize = 2000;

/// Default maximum bytes before truncation
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024; // 50KB

/// Default retention period for stored outputs (7 days in seconds)
pub const DEFAULT_RETENTION_SECS: u64 = 7 * 24 * 60 * 60;

/// Options for truncation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncateOptions {
    /// Maximum number of lines before truncation
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    /// Maximum number of bytes before truncation
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,

    /// Direction to truncate from ("head" or "tail")
    #[serde(default = "default_direction")]
    pub direction: String,

    /// Whether to save the full output to a file
    #[serde(default = "default_true")]
    pub save_full_output: bool,
}

impl Default for TruncateOptions {
    fn default() -> Self {
        Self {
            max_lines: DEFAULT_MAX_LINES,
            max_bytes: DEFAULT_MAX_BYTES,
            direction: "head".to_string(),
            save_full_output: true,
        }
    }
}

fn default_max_lines() -> usize {
    DEFAULT_MAX_LINES
}

fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}

fn default_direction() -> String {
    "head".to_string()
}

fn default_true() -> bool {
    true
}

/// Result of a truncation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncateResult {
    /// The (possibly truncated) content
    pub content: String,

    /// Whether the content was truncated
    pub truncated: bool,

    /// Path to the full output file (if saved)
    pub output_path: Option<String>,

    /// Number of lines removed
    pub lines_removed: usize,

    /// Number of bytes removed
    pub bytes_removed: usize,

    /// Original line count
    pub original_lines: usize,

    /// Original byte count
    pub original_bytes: usize,
}

impl TruncateResult {
    /// Create a result for non-truncated content
    pub fn not_truncated(content: String) -> Self {
        let lines = content.lines().count();
        let bytes = content.len();
        Self {
            content,
            truncated: false,
            output_path: None,
            lines_removed: 0,
            bytes_removed: 0,
            original_lines: lines,
            original_bytes: bytes,
        }
    }
}

/// Output truncation handler
pub struct Truncator {
    /// Directory for storing full outputs
    output_dir: PathBuf,
    /// Default options
    default_options: TruncateOptions,
}

impl Truncator {
    /// Create a new Truncator with the given output directory
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            default_options: TruncateOptions::default(),
        }
    }

    /// Create a new Truncator with custom default options
    pub fn with_defaults(output_dir: PathBuf, options: TruncateOptions) -> Self {
        Self {
            output_dir,
            default_options: options,
        }
    }

    /// Truncate text if it exceeds limits
    pub fn truncate(&self, text: &str, options: Option<&TruncateOptions>) -> Result<TruncateResult> {
        let options = options.unwrap_or(&self.default_options);

        let lines: Vec<&str> = text.lines().collect();
        let total_lines = lines.len();
        let total_bytes = text.len();

        // Check if truncation is needed
        if total_lines <= options.max_lines && total_bytes <= options.max_bytes {
            return Ok(TruncateResult::not_truncated(text.to_string()));
        }

        // Perform truncation
        let mut selected: Vec<&str> = Vec::new();
        let mut bytes = 0;
        let mut hit_byte_limit = false;

        if options.direction == "head" {
            for (i, line) in lines.iter().enumerate() {
                if i >= options.max_lines {
                    break;
                }

                let line_bytes = line.len() + if i > 0 { 1 } else { 0 }; // +1 for newline
                if bytes + line_bytes > options.max_bytes {
                    hit_byte_limit = true;
                    break;
                }

                selected.push(line);
                bytes += line_bytes;
            }
        } else {
            // Tail direction
            for (i, line) in lines.iter().rev().enumerate() {
                if i >= options.max_lines {
                    break;
                }

                let line_bytes = line.len() + if i > 0 { 1 } else { 0 };
                if bytes + line_bytes > options.max_bytes {
                    hit_byte_limit = true;
                    break;
                }

                selected.insert(0, line);
                bytes += line_bytes;
            }
        }

        let removed_count = if hit_byte_limit {
            total_bytes - bytes
        } else {
            total_lines - selected.len()
        };
        let unit = if hit_byte_limit { "bytes" } else { "lines" };

        let preview = selected.join("\n");

        // Save full output if requested
        let output_path = if options.save_full_output {
            Some(self.save_full_output(text)?)
        } else {
            None
        };

        // Format the message
        let hint = if output_path.is_some() {
            format!(
                "Full output saved to: {}\nUse Grep to search the full content or Read with offset/limit to view specific sections.",
                output_path.as_ref().unwrap()
            )
        } else {
            "Output was truncated.".to_string()
        };

        let message = if options.direction == "head" {
            format!(
                "{}\n\n...{} {} truncated...\n\n{}",
                preview, removed_count, unit, hint
            )
        } else {
            format!(
                "...{} {} truncated...\n\n{}\n\n{}",
                removed_count, unit, hint, preview
            )
        };

        Ok(TruncateResult {
            content: message,
            truncated: true,
            output_path,
            lines_removed: total_lines - selected.len(),
            bytes_removed: total_bytes - bytes,
            original_lines: total_lines,
            original_bytes: total_bytes,
        })
    }

    /// Save full output to a file and return the path
    fn save_full_output(&self, content: &str) -> Result<String> {
        // Create output directory if needed
        fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("Failed to create output directory: {}", self.output_dir.display()))?;

        // Generate unique filename
        let id = uuid::Uuid::new_v4();
        let filename = format!("tool_{}", id);
        let path = self.output_dir.join(&filename);

        fs::write(&path, content)
            .with_context(|| format!("Failed to write output file: {}", path.display()))?;

        Ok(path.to_string_lossy().to_string())
    }

    /// Clean up old output files
    pub fn cleanup(&self, retention_secs: u64) -> Result<usize> {
        let cutoff = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(retention_secs))
            .unwrap_or(std::time::UNIX_EPOCH);

        let mut removed = 0;

        if !self.output_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.output_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !filename.starts_with("tool_") {
                continue;
            }

            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        if fs::remove_file(&path).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

/// Convenience function to truncate text with default options
pub fn truncate_output(text: &str, output_dir: &Path) -> Result<TruncateResult> {
    let truncator = Truncator::new(output_dir.to_path_buf());
    truncator.truncate(text, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_truncator() -> (Truncator, TempDir) {
        let dir = TempDir::new().unwrap();
        let truncator = Truncator::new(dir.path().join("output"));
        (truncator, dir)
    }

    #[test]
    fn test_no_truncation_needed() {
        let (truncator, _dir) = setup_truncator();
        let text = "Line 1\nLine 2\nLine 3";

        let result = truncator.truncate(text, None).unwrap();
        assert!(!result.truncated);
        assert_eq!(result.content, text);
        assert!(result.output_path.is_none());
    }

    #[test]
    fn test_line_truncation_head() {
        let (truncator, _dir) = setup_truncator();
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let options = TruncateOptions {
            max_lines: 10,
            max_bytes: 1024 * 1024,
            save_full_output: false,
            ..Default::default()
        };

        let result = truncator.truncate(&text, Some(&options)).unwrap();
        assert!(result.truncated);
        assert!(result.content.contains("Line 1"));
        assert!(result.content.contains("...90 lines truncated..."));
        assert!(!result.content.contains("Line 100"));
    }

    #[test]
    fn test_line_truncation_tail() {
        let (truncator, _dir) = setup_truncator();
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let options = TruncateOptions {
            max_lines: 10,
            max_bytes: 1024 * 1024,
            direction: "tail".to_string(),
            save_full_output: false,
            ..Default::default()
        };

        let result = truncator.truncate(&text, Some(&options)).unwrap();
        assert!(result.truncated);
        assert!(result.content.contains("Line 100"));
        assert!(!result.content.contains("Line 1\n"));
    }

    #[test]
    fn test_byte_truncation() {
        let (truncator, _dir) = setup_truncator();
        let text = "a".repeat(10000);

        let options = TruncateOptions {
            max_lines: 10000,
            max_bytes: 100,
            save_full_output: false,
            ..Default::default()
        };

        let result = truncator.truncate(&text, Some(&options)).unwrap();
        assert!(result.truncated);
        assert!(result.bytes_removed > 0);
    }

    #[test]
    fn test_save_full_output() {
        let (truncator, _dir) = setup_truncator();
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let options = TruncateOptions {
            max_lines: 10,
            save_full_output: true,
            ..Default::default()
        };

        let result = truncator.truncate(&text, Some(&options)).unwrap();
        assert!(result.truncated);
        assert!(result.output_path.is_some());

        // Verify file was written
        let saved_content = fs::read_to_string(result.output_path.unwrap()).unwrap();
        assert_eq!(saved_content, text);
    }

    #[test]
    fn test_cleanup() {
        let (truncator, _dir) = setup_truncator();

        // Create some output files
        let text = "test content";
        let options = TruncateOptions {
            max_lines: 0,
            save_full_output: true,
            ..Default::default()
        };

        truncator.truncate(text, Some(&options)).unwrap();
        truncator.truncate(text, Some(&options)).unwrap();

        // Cleanup with 0 retention should remove all files
        let removed = truncator.cleanup(0).unwrap();
        assert!(removed >= 2);
    }
}
