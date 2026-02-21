//! File read tool.
//!
//! Reads file contents with path sandboxing and symlink escape prevention.

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

/// Maximum file size to read (1MB).
const MAX_FILE_SIZE: u64 = 1024 * 1024;

/// File read tool.
pub struct FileReadTool {
    security: Arc<SecurityPolicy>,
}

impl FileReadTool {
    /// Create a new file read tool.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    /// Resolve and validate a path.
    async fn resolve_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        // Check path allowlist first
        if !self.security.is_path_allowed(path) {
            anyhow::bail!("Path not allowed: {}", path);
        }

        // Resolve relative to workspace
        let full_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.security.workspace_dir.join(path)
        };

        // Canonicalize to detect symlink escapes
        let resolved = fs::canonicalize(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Path does not exist: {} ({})", path, e))?;

        // Verify still within workspace after symlink resolution
        if !self.security.is_resolved_path_allowed(&resolved) {
            anyhow::bail!("Path escapes workspace boundary: {}", path);
        }

        Ok(resolved)
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file within the workspace. Paths are validated \
        against security policies and symlink escapes are prevented. Use for \
        reading source code, configuration files, and other text files."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read (relative to workspace)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Byte offset to start reading from (optional)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum bytes to read (optional, default: 1MB)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        #[allow(clippy::cast_possible_truncation)]
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(MAX_FILE_SIZE) as usize;

        // Resolve and validate path
        let resolved = match self.resolve_path(path).await {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::failure(e.to_string())),
        };

        // Check file size
        let metadata = match fs::metadata(&resolved).await {
            Ok(m) => m,
            Err(e) => return Ok(ToolResult::failure(format!("Cannot read file metadata: {e}"))),
        };

        if !metadata.is_file() {
            return Ok(ToolResult::failure("Path is not a file"));
        }

        if metadata.len() > MAX_FILE_SIZE && limit >= MAX_FILE_SIZE as usize {
            return Ok(ToolResult::failure(format!(
                "File too large: {} bytes (max {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        // Read file
        let content = match fs::read(&resolved).await {
            Ok(bytes) => bytes,
            Err(e) => return Ok(ToolResult::failure(format!("Failed to read file: {e}"))),
        };

        // Apply offset and limit
        let end = (offset + limit).min(content.len());
        let slice = if offset < content.len() {
            &content[offset..end]
        } else {
            &[]
        };

        // Convert to string
        match String::from_utf8(slice.to_vec()) {
            Ok(text) => Ok(ToolResult::success(text)),
            Err(_) => {
                // Return as base64 for binary files
                let b64 = base64_encode(slice);
                Ok(ToolResult::success(format!("[binary: base64]\n{}", b64)))
            }
        }
    }
}

/// Simple base64 encoding without external dependency.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn setup() -> (TempDir, FileReadTool) {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            workspace_only: false, // Allow absolute paths for tests
            forbidden_paths: vec![], // Clear forbidden paths for tests
            ..SecurityPolicy::default()
        });
        (tmp, FileReadTool::new(security))
    }

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = FileReadTool::new(security);
        assert_eq!(tool.name(), "file_read");
        assert!(tool.parameters_schema()["properties"]["path"].is_object());
    }

    #[tokio::test]
    async fn read_file() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({"path": file_path.to_str().unwrap()}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "Hello, World!");
    }

    #[tokio::test]
    async fn read_with_offset() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({"path": file_path.to_str().unwrap(), "offset": 7}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "World!");
    }

    #[tokio::test]
    async fn read_with_limit() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({"path": file_path.to_str().unwrap(), "limit": 5}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "Hello");
    }

    #[tokio::test]
    async fn read_nonexistent() {
        let (_tmp, tool) = setup().await;
        let result = tool
            .execute(json!({"path": "nonexistent.txt"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("does not exist"));
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let (_tmp, tool) = setup().await;
        let result = tool
            .execute(json!({"path": "../../../etc/passwd"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_path_parameter() {
        let (_tmp, tool) = setup().await;
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn base64_encode_works() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }
}
