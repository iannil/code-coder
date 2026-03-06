//! Read tool - efficient file reading with mmap support
//!
//! This module provides file reading with:
//! - Memory-mapped I/O for large files
//! - Line range extraction
//! - Encoding detection
//! - Binary file handling

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};

/// Options for reading files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOptions {
    /// Starting line number (1-indexed, default: 1)
    #[serde(default = "default_one")]
    pub offset: usize,

    /// Number of lines to read (default: entire file)
    pub limit: Option<usize>,

    /// Maximum line length before truncation
    #[serde(default = "default_max_line_length")]
    pub max_line_length: usize,

    /// Whether to include line numbers in output
    #[serde(default = "default_true")]
    pub line_numbers: bool,

    /// Use memory mapping for large files (threshold in bytes)
    #[serde(default = "default_mmap_threshold")]
    pub mmap_threshold: u64,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            offset: 1,
            limit: None,
            max_line_length: 2000,
            line_numbers: true,
            mmap_threshold: 10 * 1024 * 1024, // 10MB
        }
    }
}

fn default_one() -> usize {
    1
}

fn default_max_line_length() -> usize {
    2000
}

fn default_mmap_threshold() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_true() -> bool {
    true
}

/// Result of reading a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    /// The file content (with optional line numbers)
    pub content: String,

    /// Lines read as a vector
    pub lines: Vec<String>,

    /// Total number of lines in the file
    pub total_lines: usize,

    /// Lines actually returned
    pub lines_returned: usize,

    /// Whether content was truncated
    pub truncated: bool,

    /// File size in bytes
    pub size: u64,

    /// Whether the file appears to be binary
    pub is_binary: bool,

    /// Detected encoding (if applicable)
    pub encoding: Option<String>,
}

impl Default for ReadResult {
    fn default() -> Self {
        Self {
            content: String::new(),
            lines: Vec::new(),
            total_lines: 0,
            lines_returned: 0,
            truncated: false,
            size: 0,
            is_binary: false,
            encoding: None,
        }
    }
}

/// File reader with mmap support
pub struct Reader {
    /// Default options
    default_options: ReadOptions,
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    /// Create a new Reader with default options
    pub fn new() -> Self {
        Self {
            default_options: ReadOptions::default(),
        }
    }

    /// Create a new Reader with custom default options
    pub fn with_defaults(options: ReadOptions) -> Self {
        Self {
            default_options: options,
        }
    }

    /// Read a file with the given options
    pub fn read(&self, path: &Path, options: Option<&ReadOptions>) -> Result<ReadResult> {
        let options = options.unwrap_or(&self.default_options);

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to read file metadata: {}", path.display()))?;

        let size = metadata.len();

        // Check if file is likely binary
        if Self::is_binary_file(path)? {
            return Ok(ReadResult {
                content: "[Binary file content not shown]".to_string(),
                is_binary: true,
                size,
                ..Default::default()
            });
        }

        // Use mmap for large files
        if size > options.mmap_threshold {
            self.read_with_mmap(path, options, size)
        } else {
            self.read_buffered(path, options, size)
        }
    }

    /// Read file using buffered I/O (for smaller files)
    fn read_buffered(&self, path: &Path, options: &ReadOptions, size: u64) -> Result<ReadResult> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut total_lines = 0;
        let mut truncated = false;

        let start_line = options.offset.saturating_sub(1); // Convert to 0-indexed
        let max_lines = options.limit.unwrap_or(usize::MAX);

        for (i, line_result) in reader.lines().enumerate() {
            total_lines = i + 1;

            // Skip lines before offset
            if i < start_line {
                continue;
            }

            // Check limit
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }

            let mut line = line_result
                .with_context(|| format!("Failed to read line {} from file", i + 1))?;

            // Truncate long lines
            if line.len() > options.max_line_length {
                line.truncate(options.max_line_length);
                line.push_str("...");
            }

            lines.push(line);
        }

        // Format content with optional line numbers
        let content = if options.line_numbers {
            Self::format_with_line_numbers(&lines, options.offset)
        } else {
            lines.join("\n")
        };

        Ok(ReadResult {
            content,
            lines_returned: lines.len(),
            lines,
            total_lines,
            truncated,
            size,
            is_binary: false,
            encoding: Some("UTF-8".to_string()),
        })
    }

    /// Read file using memory mapping (for larger files)
    fn read_with_mmap(&self, path: &Path, options: &ReadOptions, size: u64) -> Result<ReadResult> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let mmap = unsafe {
            Mmap::map(&file)
                .with_context(|| format!("Failed to mmap file: {}", path.display()))?
        };

        // Convert to string (this may fail for non-UTF8 files)
        let content = std::str::from_utf8(&mmap)
            .with_context(|| format!("File is not valid UTF-8: {}", path.display()))?;

        let all_lines: Vec<&str> = content.lines().collect();
        let total_lines = all_lines.len();

        let start_line = options.offset.saturating_sub(1);
        let max_lines = options.limit.unwrap_or(usize::MAX);
        let end_line = (start_line + max_lines).min(total_lines);

        let truncated = end_line < total_lines;

        let lines: Vec<String> = all_lines[start_line..end_line]
            .iter()
            .map(|line| {
                if line.len() > options.max_line_length {
                    format!("{}...", &line[..options.max_line_length])
                } else {
                    line.to_string()
                }
            })
            .collect();

        let content = if options.line_numbers {
            Self::format_with_line_numbers(&lines, options.offset)
        } else {
            lines.join("\n")
        };

        Ok(ReadResult {
            content,
            lines_returned: lines.len(),
            lines,
            total_lines,
            truncated,
            size,
            is_binary: false,
            encoding: Some("UTF-8".to_string()),
        })
    }

    /// Format lines with line numbers
    fn format_with_line_numbers(lines: &[String], start_line: usize) -> String {
        let max_line_num = start_line + lines.len();
        let width = max_line_num.to_string().len().max(4);

        lines
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>width$}\t{}", start_line + i, line, width = width))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if a file appears to be binary
    fn is_binary_file(path: &Path) -> Result<bool> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8000];

        let bytes_read = reader.read(&mut buffer)?;

        // Check for null bytes (common indicator of binary content)
        Ok(buffer[..bytes_read].contains(&0))
    }

    /// Read the entire file as a string
    pub fn read_to_string(&self, path: &Path) -> Result<String> {
        let result = self.read(path, None)?;
        if result.is_binary {
            anyhow::bail!("Cannot read binary file as string");
        }
        Ok(result.lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test file
        let content = (1..=100)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(dir.path().join("test.txt"), content).unwrap();

        // Create binary file
        fs::write(dir.path().join("binary.bin"), [0u8, 1, 2, 0, 3, 4]).unwrap();

        dir
    }

    #[test]
    fn test_read_entire_file() {
        let dir = setup_test_dir();
        let reader = Reader::new();

        let result = reader.read(&dir.path().join("test.txt"), None).unwrap();
        assert_eq!(result.total_lines, 100);
        assert_eq!(result.lines_returned, 100);
        assert!(!result.truncated);
    }

    #[test]
    fn test_read_with_offset_and_limit() {
        let dir = setup_test_dir();
        let reader = Reader::new();

        let options = ReadOptions {
            offset: 10,
            limit: Some(5),
            ..Default::default()
        };

        let result = reader
            .read(&dir.path().join("test.txt"), Some(&options))
            .unwrap();
        assert_eq!(result.lines_returned, 5);
        assert!(result.lines[0].contains("Line 10"));
        assert!(result.truncated);
    }

    #[test]
    fn test_read_binary_file() {
        let dir = setup_test_dir();
        let reader = Reader::new();

        let result = reader.read(&dir.path().join("binary.bin"), None).unwrap();
        assert!(result.is_binary);
        assert!(result.content.contains("Binary"));
    }

    #[test]
    fn test_read_with_line_numbers() {
        let dir = setup_test_dir();
        let reader = Reader::new();

        let options = ReadOptions {
            offset: 1,
            limit: Some(3),
            line_numbers: true,
            ..Default::default()
        };

        let result = reader
            .read(&dir.path().join("test.txt"), Some(&options))
            .unwrap();
        assert!(result.content.contains("   1\t"));
    }
}
