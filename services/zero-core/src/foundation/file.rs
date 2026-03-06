//! File type detection and utilities

use std::path::Path;

use serde::{Deserialize, Serialize};

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// File type
    pub file_type: FileType,
    /// MIME type
    pub mime_type: Option<String>,
    /// Whether the file is binary
    pub is_binary: bool,
    /// Last modified timestamp
    pub modified: Option<i64>,
}

/// File type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    /// Source code
    Source,
    /// Configuration file
    Config,
    /// Documentation
    Documentation,
    /// Data file
    Data,
    /// Image
    Image,
    /// Binary executable
    Binary,
    /// Archive
    Archive,
    /// Unknown
    Unknown,
}

impl FileType {
    /// Detect file type from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Source code
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "rb" | "php" | "swift" | "kt" | "scala" | "cs" | "fs" | "vue" | "svelte" => {
                FileType::Source
            }
            // Config
            "json" | "yaml" | "yml" | "toml" | "ini" | "conf" | "env" | "properties" => {
                FileType::Config
            }
            // Documentation
            "md" | "rst" | "txt" | "adoc" | "org" => FileType::Documentation,
            // Data
            "csv" | "xml" | "sql" | "graphql" | "prisma" => FileType::Data,
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp" => FileType::Image,
            // Binary
            "exe" | "dll" | "so" | "dylib" | "bin" | "o" | "a" => FileType::Binary,
            // Archives
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => FileType::Archive,
            _ => FileType::Unknown,
        }
    }

    /// Detect file type from path
    pub fn from_path(path: &Path) -> Self {
        // Check for special files first
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match filename {
            "Cargo.toml" | "package.json" | "tsconfig.json" | "Makefile" | "Dockerfile"
            | "docker-compose.yml" | ".gitignore" | ".env" | ".env.local" => FileType::Config,
            "README.md" | "LICENSE" | "CHANGELOG.md" | "CONTRIBUTING.md" => {
                FileType::Documentation
            }
            _ => {
                // Fall back to extension detection
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(Self::from_extension)
                    .unwrap_or(FileType::Unknown)
            }
        }
    }

    /// Check if this file type is text-based
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            FileType::Source | FileType::Config | FileType::Documentation | FileType::Data
        )
    }

    /// Get the typical MIME type for this file type
    pub fn mime_type(&self) -> &'static str {
        match self {
            FileType::Source => "text/plain",
            FileType::Config => "application/json",
            FileType::Documentation => "text/markdown",
            FileType::Data => "application/octet-stream",
            FileType::Image => "image/png",
            FileType::Binary => "application/octet-stream",
            FileType::Archive => "application/zip",
            FileType::Unknown => "application/octet-stream",
        }
    }
}

/// Detect MIME type from file extension
pub fn mime_from_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

/// Check if a file appears to be binary
pub fn is_binary(content: &[u8]) -> bool {
    // Check first 8KB for null bytes
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("rs"), FileType::Source);
        assert_eq!(FileType::from_extension("json"), FileType::Config);
        assert_eq!(FileType::from_extension("md"), FileType::Documentation);
        assert_eq!(FileType::from_extension("png"), FileType::Image);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
    }

    #[test]
    fn test_file_type_from_path() {
        assert_eq!(
            FileType::from_path(Path::new("Cargo.toml")),
            FileType::Config
        );
        assert_eq!(
            FileType::from_path(Path::new("README.md")),
            FileType::Documentation
        );
        assert_eq!(
            FileType::from_path(Path::new("src/main.rs")),
            FileType::Source
        );
    }

    #[test]
    fn test_is_text() {
        assert!(FileType::Source.is_text());
        assert!(FileType::Config.is_text());
        assert!(!FileType::Binary.is_text());
        assert!(!FileType::Image.is_text());
    }

    #[test]
    fn test_is_binary() {
        assert!(!is_binary(b"Hello, world!"));
        assert!(is_binary(&[0, 1, 2, 0, 3, 4]));
    }
}
