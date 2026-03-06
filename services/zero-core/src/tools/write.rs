//! Write tool - atomic file writing with backup support
//!
//! This module provides safe file writing with:
//! - Atomic writes using temporary files
//! - Automatic backup creation
//! - Parent directory creation
//! - Newline normalization

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Options for writing files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOptions {
    /// Whether to create parent directories if they don't exist
    #[serde(default = "default_true")]
    pub create_parents: bool,

    /// Whether to create a backup of the existing file
    #[serde(default)]
    pub backup: bool,

    /// Backup suffix (default: ".bak")
    #[serde(default = "default_backup_suffix")]
    pub backup_suffix: String,

    /// Whether to append instead of overwrite
    #[serde(default)]
    pub append: bool,

    /// Normalize line endings to LF
    #[serde(default = "default_true")]
    pub normalize_newlines: bool,

    /// File permissions (Unix mode, e.g., 0o644)
    pub mode: Option<u32>,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            create_parents: true,
            backup: false,
            backup_suffix: ".bak".to_string(),
            append: false,
            normalize_newlines: true,
            mode: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_backup_suffix() -> String {
    ".bak".to_string()
}

/// Result of writing a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Path to the written file
    pub path: String,

    /// Number of bytes written
    pub bytes_written: usize,

    /// Path to the backup file (if created)
    pub backup_path: Option<String>,

    /// Whether this was a new file
    pub created: bool,

    /// Whether parent directories were created
    pub parents_created: bool,
}

/// File writer with atomic write support
pub struct Writer {
    /// Default options
    default_options: WriteOptions,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer {
    /// Create a new Writer with default options
    pub fn new() -> Self {
        Self {
            default_options: WriteOptions::default(),
        }
    }

    /// Create a new Writer with custom default options
    pub fn with_defaults(options: WriteOptions) -> Self {
        Self {
            default_options: options,
        }
    }

    /// Write content to a file atomically
    pub fn write(&self, path: &Path, content: &str, options: Option<&WriteOptions>) -> Result<WriteResult> {
        let options = options.unwrap_or(&self.default_options);
        let path = path.to_path_buf();

        let exists = path.exists();
        let mut parents_created = false;
        let mut backup_path = None;

        // Create parent directories if needed
        if options.create_parents {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create parent directories for: {}", path.display()))?;
                    parents_created = true;
                }
            }
        }

        // Create backup if requested and file exists
        if options.backup && exists {
            let backup = PathBuf::from(format!("{}{}", path.display(), options.backup_suffix));
            fs::copy(&path, &backup)
                .with_context(|| format!("Failed to create backup: {}", backup.display()))?;
            backup_path = Some(backup.to_string_lossy().to_string());
        }

        // Normalize content
        let content = if options.normalize_newlines {
            content.replace("\r\n", "\n")
        } else {
            content.to_string()
        };

        // Write atomically using a temporary file
        let bytes_written = if options.append {
            self.append_to_file(&path, &content)?
        } else {
            self.atomic_write(&path, &content)?
        };

        // Set file permissions if specified
        #[cfg(unix)]
        if let Some(mode) = options.mode {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(mode);
            fs::set_permissions(&path, permissions)
                .with_context(|| format!("Failed to set permissions on: {}", path.display()))?;
        }

        Ok(WriteResult {
            path: path.to_string_lossy().to_string(),
            bytes_written,
            backup_path,
            created: !exists,
            parents_created,
        })
    }

    /// Perform atomic write using a temporary file
    fn atomic_write(&self, path: &Path, content: &str) -> Result<usize> {
        let parent = path.parent().unwrap_or(Path::new("."));

        // Ensure parent directory exists
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
        }

        // Create a temporary file in the same directory
        let mut temp_path = parent.to_path_buf();
        temp_path.push(format!(".{}.tmp", uuid::Uuid::new_v4()));

        // Write to temporary file
        let mut file = File::create(&temp_path)
            .with_context(|| format!("Failed to create temporary file: {}", temp_path.display()))?;

        file.write_all(content.as_bytes())
            .with_context(|| format!("Failed to write to temporary file: {}", temp_path.display()))?;

        file.sync_all()
            .with_context(|| "Failed to sync temporary file")?;

        // Rename temporary file to target (atomic on most filesystems)
        fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename temporary file to: {}", path.display()))?;

        Ok(content.len())
    }

    /// Append content to a file
    fn append_to_file(&self, path: &Path, content: &str) -> Result<usize> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("Failed to open file for appending: {}", path.display()))?;

        file.write_all(content.as_bytes())
            .with_context(|| format!("Failed to append to file: {}", path.display()))?;

        Ok(content.len())
    }

    /// Write content and return the path
    pub fn write_to_string(&self, path: &Path, content: &str) -> Result<String> {
        self.write(path, content, None)?;
        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_write_new_file() {
        let dir = TempDir::new().unwrap();
        let writer = Writer::new();

        let path = dir.path().join("new_file.txt");
        let result = writer.write(&path, "Hello, world!", None).unwrap();

        assert!(result.created);
        assert_eq!(result.bytes_written, 13);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_write_creates_parents() {
        let dir = TempDir::new().unwrap();
        let writer = Writer::new();

        let path = dir.path().join("nested/dir/file.txt");
        let result = writer.write(&path, "Content", None).unwrap();

        assert!(result.created);
        assert!(result.parents_created);
        assert!(path.exists());
    }

    #[test]
    fn test_write_with_backup() {
        let dir = TempDir::new().unwrap();
        let writer = Writer::new();

        let path = dir.path().join("existing.txt");
        fs::write(&path, "Original content").unwrap();

        let options = WriteOptions {
            backup: true,
            ..Default::default()
        };

        let result = writer.write(&path, "New content", Some(&options)).unwrap();

        assert!(result.backup_path.is_some());
        let backup = PathBuf::from(result.backup_path.unwrap());
        assert_eq!(fs::read_to_string(backup).unwrap(), "Original content");
        assert_eq!(fs::read_to_string(&path).unwrap(), "New content");
    }

    #[test]
    fn test_write_append() {
        let dir = TempDir::new().unwrap();
        let writer = Writer::new();

        let path = dir.path().join("append.txt");
        fs::write(&path, "First line\n").unwrap();

        let options = WriteOptions {
            append: true,
            ..Default::default()
        };

        writer.write(&path, "Second line\n", Some(&options)).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "First line\nSecond line\n");
    }

    #[test]
    fn test_write_normalizes_newlines() {
        let dir = TempDir::new().unwrap();
        let writer = Writer::new();

        let path = dir.path().join("normalized.txt");
        writer.write(&path, "Line 1\r\nLine 2\r\n", None).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "Line 1\nLine 2\n");
    }
}
