//! NAPI bindings for read module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - ReaderHandle: File reading with mmap support
//! - readFileWithLines: Standalone read function with line numbers

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::tools::read::{
    ReadOptions as RustReadOptions, ReadResult as RustReadResult, Reader as RustReader,
};

// ============================================================================
// Read Types for NAPI
// ============================================================================

/// Options for reading files
#[napi(object)]
pub struct NapiReadOptions {
    /// Starting line number (1-indexed, default: 1)
    pub offset: Option<u32>,

    /// Number of lines to read (default: entire file)
    pub limit: Option<u32>,

    /// Maximum line length before truncation (default: 2000)
    pub max_line_length: Option<u32>,

    /// Whether to include line numbers in output (default: true)
    pub line_numbers: Option<bool>,

    /// Use memory mapping for large files (threshold in bytes, default: 10MB)
    pub mmap_threshold: Option<f64>,
}

impl From<NapiReadOptions> for RustReadOptions {
    fn from(options: NapiReadOptions) -> Self {
        Self {
            offset: options.offset.map(|v| v as usize).unwrap_or(1),
            limit: options.limit.map(|v| v as usize),
            max_line_length: options.max_line_length.map(|v| v as usize).unwrap_or(2000),
            line_numbers: options.line_numbers.unwrap_or(true),
            mmap_threshold: options.mmap_threshold.map(|v| v as u64).unwrap_or(10 * 1024 * 1024),
        }
    }
}

/// Result of reading a file
#[napi(object)]
pub struct NapiReadResult {
    /// The file content (with optional line numbers)
    pub content: String,

    /// Lines read as a vector
    pub lines: Vec<String>,

    /// Total number of lines in the file
    pub total_lines: u32,

    /// Lines actually returned
    pub lines_returned: u32,

    /// Whether content was truncated
    pub truncated: bool,

    /// File size in bytes
    pub size: f64,

    /// Whether the file appears to be binary
    pub is_binary: bool,

    /// Detected encoding (if applicable)
    pub encoding: Option<String>,
}

impl From<RustReadResult> for NapiReadResult {
    fn from(result: RustReadResult) -> Self {
        Self {
            content: result.content,
            lines: result.lines,
            total_lines: result.total_lines as u32,
            lines_returned: result.lines_returned as u32,
            truncated: result.truncated,
            size: result.size as f64,
            is_binary: result.is_binary,
            encoding: result.encoding,
        }
    }
}

// ============================================================================
// Reader NAPI Handle
// ============================================================================

/// Thread-safe wrapper for Reader
#[napi]
pub struct ReaderHandle {
    inner: Arc<Mutex<RustReader>>,
}

#[napi]
impl ReaderHandle {
    /// Create a new Reader with default options
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustReader::new())),
        }
    }

    /// Create with custom default options
    #[napi(factory)]
    pub fn with_defaults(options: NapiReadOptions) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustReader::with_defaults(options.into()))),
        }
    }

    /// Read a file with the given options
    #[napi]
    pub fn read(&self, file_path: String, options: Option<NapiReadOptions>) -> Result<NapiReadResult> {
        let reader = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let path = PathBuf::from(&file_path);
        let rust_options = options.map(|o| o.into());
        let result = reader
            .read(&path, rust_options.as_ref())
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Read the entire file as a string
    #[napi]
    pub fn read_to_string(&self, file_path: String) -> Result<String> {
        let reader = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let path = PathBuf::from(&file_path);
        reader
            .read_to_string(&path)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Read a file with line numbers (convenience function)
///
/// This is a convenience function for one-off file reads.
/// For repeated reads, use ReaderHandle.
#[napi]
pub fn read_file_with_lines(
    file_path: String,
    offset: Option<u32>,
    limit: Option<u32>,
    max_line_length: Option<u32>,
    line_numbers: Option<bool>,
) -> Result<NapiReadResult> {
    let reader = RustReader::new();
    let path = PathBuf::from(&file_path);

    let options = RustReadOptions {
        offset: offset.map(|v| v as usize).unwrap_or(1),
        limit: limit.map(|v| v as usize),
        max_line_length: max_line_length.map(|v| v as usize).unwrap_or(2000),
        line_numbers: line_numbers.unwrap_or(true),
        mmap_threshold: 10 * 1024 * 1024,
    };

    let result = reader
        .read(&path, Some(&options))
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(result.into())
}

/// Read a file range (for pagination)
///
/// Returns content from line `offset` to `offset + limit`.
/// Line numbers are 1-indexed.
#[napi]
pub fn read_file_range(
    file_path: String,
    offset: u32,
    limit: u32,
) -> Result<NapiReadResult> {
    read_file_with_lines(
        file_path,
        Some(offset),
        Some(limit),
        None,
        Some(true),
    )
}

/// Check if a file appears to be binary
#[napi]
pub fn is_binary_file(file_path: String) -> Result<bool> {
    use std::fs::File;
    use std::io::{BufReader, Read};

    let path = PathBuf::from(&file_path);
    let file = File::open(&path)
        .map_err(|e| Error::from_reason(format!("Failed to open file: {}", e)))?;

    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 8000];
    let bytes_read = reader.read(&mut buffer)
        .map_err(|e| Error::from_reason(format!("Failed to read file: {}", e)))?;

    // Check for null bytes (common indicator of binary content)
    Ok(buffer[..bytes_read].contains(&0))
}

/// Get file line count without reading entire content
#[napi]
pub fn count_file_lines(file_path: String) -> Result<u32> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let path = PathBuf::from(&file_path);
    let file = File::open(&path)
        .map_err(|e| Error::from_reason(format!("Failed to open file: {}", e)))?;

    let reader = BufReader::new(file);
    let count = reader.lines().count();

    Ok(count as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_reader_handle() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "Line 1\nLine 2\nLine 3");

        let handle = ReaderHandle::new();
        let result = handle.read(path.to_string_lossy().to_string(), None).unwrap();

        assert_eq!(result.total_lines, 3);
        assert_eq!(result.lines_returned, 3);
        assert!(!result.truncated);
    }

    #[test]
    fn test_reader_with_options() {
        let dir = TempDir::new().unwrap();
        let content = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        let path = setup_test_file(&dir, "test.txt", &content);

        let options = NapiReadOptions {
            offset: Some(10),
            limit: Some(5),
            max_line_length: None,
            line_numbers: Some(true),
            mmap_threshold: None,
        };

        let handle = ReaderHandle::new();
        let result = handle.read(path.to_string_lossy().to_string(), Some(options)).unwrap();

        assert_eq!(result.lines_returned, 5);
        assert!(result.truncated);
    }

    #[test]
    fn test_read_file_with_lines() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "Line 1\nLine 2\nLine 3");

        let result = read_file_with_lines(
            path.to_string_lossy().to_string(),
            Some(1),
            Some(2),
            None,
            Some(true),
        ).unwrap();

        assert_eq!(result.lines_returned, 2);
    }

    #[test]
    fn test_is_binary_file() {
        let dir = TempDir::new().unwrap();

        // Text file
        let text_path = setup_test_file(&dir, "text.txt", "Hello, world!");
        assert!(!is_binary_file(text_path.to_string_lossy().to_string()).unwrap());

        // Binary file (with null bytes)
        let binary_path = dir.path().join("binary.bin");
        fs::write(&binary_path, &[0u8, 1, 2, 0, 3, 4]).unwrap();
        assert!(is_binary_file(binary_path.to_string_lossy().to_string()).unwrap());
    }

    #[test]
    fn test_count_file_lines() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "Line 1\nLine 2\nLine 3\n");

        let count = count_file_lines(path.to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 4); // Including empty line at end
    }
}
