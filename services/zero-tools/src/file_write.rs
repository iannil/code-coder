//! File write tool.
//!
//! Writes file contents with path sandboxing and symlink escape prevention.

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

/// Maximum file size to write (10MB).
const MAX_WRITE_SIZE: usize = 10 * 1024 * 1024;

/// File write tool.
pub struct FileWriteTool {
    security: Arc<SecurityPolicy>,
}

impl FileWriteTool {
    /// Create a new file write tool.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    /// Resolve and validate a path for writing.
    async fn resolve_write_path(&self, path: &str) -> anyhow::Result<PathBuf> {
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

        // For new files, check the parent directory exists and is in workspace
        if let Some(parent) = full_path.parent() {
            if parent.exists() {
                let resolved_parent = fs::canonicalize(parent).await.map_err(|e| {
                    anyhow::anyhow!("Parent directory does not exist: {} ({})", path, e)
                })?;

                // Verify parent is within workspace
                if !self.security.is_resolved_path_allowed(&resolved_parent) {
                    anyhow::bail!("Path escapes workspace boundary: {}", path);
                }
            }
        }

        // For existing files, also check symlink escapes
        if full_path.exists() {
            let resolved = fs::canonicalize(&full_path)
                .await
                .map_err(|e| anyhow::anyhow!("Cannot resolve path: {} ({})", path, e))?;

            if !self.security.is_resolved_path_allowed(&resolved) {
                anyhow::bail!("Path escapes workspace boundary: {}", path);
            }
        }

        Ok(full_path)
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file within the workspace. Paths are validated \
        against security policies and symlink escapes are prevented. Can create \
        new files or overwrite existing ones. Parent directories must exist."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write (relative to workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "append": {
                    "type": "boolean",
                    "description": "Append to file instead of overwriting (default: false)"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist (default: false)"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Check autonomy level
        if !self.security.can_act() {
            return Ok(ToolResult::failure("Action blocked: autonomy is read-only"));
        }

        // Check rate limit
        if !self.security.record_action() {
            return Ok(ToolResult::failure("Action blocked: rate limit exceeded"));
        }

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let append = args
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let create_dirs = args
            .get("create_dirs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check content size
        if content.len() > MAX_WRITE_SIZE {
            return Ok(ToolResult::failure(format!(
                "Content too large: {} bytes (max {} bytes)",
                content.len(),
                MAX_WRITE_SIZE
            )));
        }

        // Resolve and validate path
        let full_path = match self.resolve_write_path(path).await {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::failure(e.to_string())),
        };

        // Create parent directories if requested
        if create_dirs {
            if let Some(parent) = full_path.parent() {
                if !parent.exists() {
                    // Validate the parent path before creating
                    let parent_str = parent.to_string_lossy();
                    if !self.security.is_path_allowed(&parent_str) {
                        return Ok(ToolResult::failure(format!(
                            "Cannot create directory: {}",
                            parent_str
                        )));
                    }

                    if let Err(e) = fs::create_dir_all(parent).await {
                        return Ok(ToolResult::failure(format!(
                            "Failed to create directories: {e}"
                        )));
                    }
                }
            }
        }

        // Write file
        let result = if append {
            let mut existing = fs::read_to_string(&full_path).await.unwrap_or_default();
            existing.push_str(content);
            fs::write(&full_path, existing).await
        } else {
            fs::write(&full_path, content).await
        };

        match result {
            Ok(()) => Ok(ToolResult::success(format!(
                "Successfully {} {} ({} bytes)",
                if append { "appended to" } else { "wrote" },
                path,
                content.len()
            ))),
            Err(e) => Ok(ToolResult::failure(format!("Failed to write file: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, FileWriteTool) {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            workspace_only: false,
            forbidden_paths: vec![], // Clear forbidden paths for tests
            ..SecurityPolicy::default()
        });
        (tmp, FileWriteTool::new(security))
    }

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = FileWriteTool::new(security);
        assert_eq!(tool.name(), "file_write");
        assert!(tool.parameters_schema()["properties"]["path"].is_object());
        assert!(tool.parameters_schema()["properties"]["content"].is_object());
    }

    #[tokio::test]
    async fn write_new_file() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("new.txt");

        let result = tool
            .execute(json!({
                "path": file_path.to_str().unwrap(),
                "content": "Hello, World!"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).await.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn overwrite_file() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("existing.txt");

        fs::write(&file_path, "original").await.unwrap();

        let result = tool
            .execute(json!({
                "path": file_path.to_str().unwrap(),
                "content": "updated"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(fs::read_to_string(&file_path).await.unwrap(), "updated");
    }

    #[tokio::test]
    async fn append_to_file() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("append.txt");

        fs::write(&file_path, "Hello").await.unwrap();

        let result = tool
            .execute(json!({
                "path": file_path.to_str().unwrap(),
                "content": ", World!",
                "append": true
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(fs::read_to_string(&file_path).await.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn create_directories() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("deep/nested/dir/file.txt");

        let result = tool
            .execute(json!({
                "path": file_path.to_str().unwrap(),
                "content": "content",
                "create_dirs": true
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let (_tmp, tool) = setup().await;
        let result = tool
            .execute(json!({
                "path": "../../../etc/passwd",
                "content": "malicious"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_path_parameter() {
        let (_tmp, tool) = setup().await;
        let result = tool.execute(json!({"content": "hello"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_content_parameter() {
        let (_tmp, tool) = setup().await;
        let result = tool.execute(json!({"path": "test.txt"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn readonly_blocks_write() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            autonomy: crate::security::AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = FileWriteTool::new(security);

        let result = tool
            .execute(json!({
                "path": "test.txt",
                "content": "hello"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("read-only"));
    }
}
