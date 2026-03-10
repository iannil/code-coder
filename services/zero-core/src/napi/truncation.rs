//! NAPI bindings for truncation module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - TruncatorHandle: Output truncation with file saving
//! - truncateOutput: Standalone truncation function

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::tools::truncation::{
    TruncateOptions as RustTruncateOptions, TruncateResult as RustTruncateResult,
    Truncator as RustTruncator, DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES,
};

// ============================================================================
// Truncation Types for NAPI
// ============================================================================

/// Options for truncation
#[napi(object)]
pub struct NapiTruncateOptions {
    /// Maximum number of lines before truncation (default: 2000)
    pub max_lines: Option<u32>,

    /// Maximum number of bytes before truncation (default: 50KB)
    pub max_bytes: Option<u32>,

    /// Direction to truncate from: "head" or "tail" (default: "head")
    pub direction: Option<String>,

    /// Whether to save the full output to a file (default: true)
    pub save_full_output: Option<bool>,
}

impl From<NapiTruncateOptions> for RustTruncateOptions {
    fn from(options: NapiTruncateOptions) -> Self {
        Self {
            max_lines: options.max_lines.map(|v| v as usize).unwrap_or(DEFAULT_MAX_LINES),
            max_bytes: options.max_bytes.map(|v| v as usize).unwrap_or(DEFAULT_MAX_BYTES),
            direction: options.direction.unwrap_or_else(|| "head".to_string()),
            save_full_output: options.save_full_output.unwrap_or(true),
        }
    }
}

/// Result of a truncation operation
#[napi(object)]
pub struct NapiTruncateResult {
    /// The (possibly truncated) content
    pub content: String,

    /// Whether the content was truncated
    pub truncated: bool,

    /// Path to the full output file (if saved)
    pub output_path: Option<String>,

    /// Number of lines removed
    pub lines_removed: u32,

    /// Number of bytes removed
    pub bytes_removed: u32,

    /// Original line count
    pub original_lines: u32,

    /// Original byte count
    pub original_bytes: u32,
}

impl From<RustTruncateResult> for NapiTruncateResult {
    fn from(result: RustTruncateResult) -> Self {
        Self {
            content: result.content,
            truncated: result.truncated,
            output_path: result.output_path,
            lines_removed: result.lines_removed as u32,
            bytes_removed: result.bytes_removed as u32,
            original_lines: result.original_lines as u32,
            original_bytes: result.original_bytes as u32,
        }
    }
}

// ============================================================================
// Truncator NAPI Handle
// ============================================================================

/// Thread-safe wrapper for Truncator
#[napi]
pub struct TruncatorHandle {
    inner: Arc<Mutex<RustTruncator>>,
}

#[napi]
impl TruncatorHandle {
    /// Create a new Truncator with the given output directory
    #[napi(constructor)]
    pub fn new(output_dir: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustTruncator::new(PathBuf::from(output_dir)))),
        }
    }

    /// Create with custom default options
    #[napi(factory)]
    pub fn with_defaults(output_dir: String, options: NapiTruncateOptions) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustTruncator::with_defaults(
                PathBuf::from(output_dir),
                options.into(),
            ))),
        }
    }

    /// Truncate text if it exceeds limits
    #[napi]
    pub fn truncate(&self, text: String, options: Option<NapiTruncateOptions>) -> Result<NapiTruncateResult> {
        let truncator = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_options = options.map(|o| o.into());
        let result = truncator
            .truncate(&text, rust_options.as_ref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Clean up old output files
    /// Returns the number of files removed
    #[napi]
    pub fn cleanup(&self, retention_secs: Option<f64>) -> Result<u32> {
        let truncator = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let secs = retention_secs.unwrap_or(7.0 * 24.0 * 60.0 * 60.0) as u64;
        let removed = truncator
            .cleanup(secs)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(removed as u32)
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Truncate output text with default options
///
/// This is a convenience function for one-off truncation.
/// For repeated truncation operations, use TruncatorHandle.
#[napi]
pub fn truncate_output(text: String, output_dir: String, options: Option<NapiTruncateOptions>) -> Result<NapiTruncateResult> {
    let truncator = RustTruncator::new(PathBuf::from(output_dir));
    let rust_options = options.map(|o| o.into());
    let result = truncator
        .truncate(&text, rust_options.as_ref())
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(result.into())
}

/// Truncate output without saving to file
///
/// Returns only the truncated content, useful for preview purposes.
#[napi]
pub fn truncate_preview(
    text: String,
    max_lines: Option<u32>,
    max_bytes: Option<u32>,
    direction: Option<String>,
) -> NapiTruncateResult {
    let options = RustTruncateOptions {
        max_lines: max_lines.map(|v| v as usize).unwrap_or(DEFAULT_MAX_LINES),
        max_bytes: max_bytes.map(|v| v as usize).unwrap_or(DEFAULT_MAX_BYTES),
        direction: direction.unwrap_or_else(|| "head".to_string()),
        save_full_output: false,
    };

    // Create a temporary truncator (no file saving)
    let truncator = RustTruncator::new(PathBuf::from("/tmp"));

    match truncator.truncate(&text, Some(&options)) {
        Ok(result) => result.into(),
        Err(_) => {
            // On error, return original content as non-truncated
            NapiTruncateResult {
                content: text.clone(),
                truncated: false,
                output_path: None,
                lines_removed: 0,
                bytes_removed: 0,
                original_lines: text.lines().count() as u32,
                original_bytes: text.len() as u32,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_truncator_handle() {
        let dir = TempDir::new().unwrap();
        let handle = TruncatorHandle::new(dir.path().to_string_lossy().to_string());

        // Test non-truncated content
        let result = handle.truncate("short text".to_string(), None).unwrap();
        assert!(!result.truncated);
        assert_eq!(result.content, "short text");
    }

    #[test]
    fn test_truncator_with_options() {
        let dir = TempDir::new().unwrap();
        let handle = TruncatorHandle::new(dir.path().to_string_lossy().to_string());

        // Create content that exceeds limits
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let options = NapiTruncateOptions {
            max_lines: Some(10),
            max_bytes: None,
            direction: Some("head".to_string()),
            save_full_output: Some(false),
        };

        let result = handle.truncate(text, Some(options)).unwrap();
        assert!(result.truncated);
        assert!(result.lines_removed > 0);
    }

    #[test]
    fn test_truncate_preview() {
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let result = truncate_preview(text, Some(10), None, None);
        assert!(result.truncated);
        assert!(result.output_path.is_none()); // No file saved
    }

    #[test]
    fn test_truncate_output_standalone() {
        let dir = TempDir::new().unwrap();
        let text = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let options = NapiTruncateOptions {
            max_lines: Some(10),
            max_bytes: None,
            direction: None,
            save_full_output: Some(true),
        };

        let result = truncate_output(
            text,
            dir.path().to_string_lossy().to_string(),
            Some(options),
        ).unwrap();

        assert!(result.truncated);
        assert!(result.output_path.is_some());
    }
}
