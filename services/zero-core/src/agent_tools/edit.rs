//! Edit tool.
//!
//! File editing with exact string replacement and fuzzy matching support.
//! Wraps zero-core's Editor for use with the Tool trait.

use super::security::SecurityPolicy;
use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use crate::tools::edit::{replace_with_fuzzy_match, EditOperation, Editor};

/// Edit tool for file modifications.
pub struct EditTool {
    security: Arc<SecurityPolicy>,
    editor: Editor,
}

impl EditTool {
    /// Create a new edit tool.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self {
            security,
            editor: Editor::new(),
        }
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
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing text. Performs exact string replacement with \
        optional fuzzy matching for whitespace and indentation differences. \
        Use replace_all=true to replace all occurrences."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace it with (must be different from old_string)"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)",
                    "default": false
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string' parameter"))?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string' parameter"))?;

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Validate that old_string and new_string are different
        if old_string == new_string {
            return Ok(ToolResult::failure(
                "old_string and new_string must be different",
            ));
        }

        // Resolve and validate path
        let resolved = match self.resolve_path(file_path).await {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::failure(e.to_string())),
        };

        // Read the file
        let content = match fs::read_to_string(&resolved).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::failure(format!("Cannot read file: {e}"))),
        };

        // Try exact match first
        if content.contains(old_string) {
            // Check uniqueness (unless replace_all is true)
            if !replace_all {
                let count = content.matches(old_string).count();
                if count > 1 {
                    return Ok(ToolResult::failure(format!(
                        "old_string is not unique in file (found {} occurrences). \
                        Use replace_all=true or provide more context.",
                        count
                    )));
                }
            }

            // Perform the edit
            let operation = EditOperation {
                old_string: old_string.to_string(),
                new_string: new_string.to_string(),
                replace_all,
            };

            match self.editor.edit(&resolved, &operation) {
                Ok(result) => {
                    if result.success {
                        let output = format!(
                            "Replaced {} occurrence(s) in {}\n{}",
                            result.replacements,
                            file_path,
                            result.diff
                        );
                        Ok(ToolResult::success(output))
                    } else {
                        Ok(ToolResult::failure(
                            result.error.unwrap_or_else(|| "Edit failed".to_string()),
                        ))
                    }
                }
                Err(e) => Ok(ToolResult::failure(e.to_string())),
            }
        } else {
            // Try fuzzy matching
            let fuzzy_result = replace_with_fuzzy_match(&content, old_string, new_string, replace_all);

            if fuzzy_result.found {
                // Write the fuzzy-matched result
                if let Err(e) = fs::write(&resolved, &fuzzy_result.content).await {
                    return Ok(ToolResult::failure(format!("Failed to write file: {e}")));
                }

                let matched = fuzzy_result
                    .matched_string
                    .as_deref()
                    .unwrap_or("unknown");
                let strategy = fuzzy_result.strategy.as_deref().unwrap_or("unknown");

                let output = format!(
                    "Fuzzy match succeeded (strategy: {}) in {}\n\
                    Matched: {:?}",
                    strategy, file_path, matched
                );
                Ok(ToolResult::success(output))
            } else {
                let error = fuzzy_result
                    .error
                    .unwrap_or_else(|| "old_string not found in file".to_string());
                Ok(ToolResult::failure(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn setup() -> (TempDir, EditTool) {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            workspace_only: false, // Allow absolute paths for tests
            forbidden_paths: vec![], // Clear forbidden paths for tests
            ..SecurityPolicy::default()
        });
        (tmp, EditTool::new(security))
    }

    fn setup_restricted() -> EditTool {
        let security = Arc::new(SecurityPolicy {
            workspace_dir: std::path::PathBuf::from("/nonexistent"),
            workspace_only: true, // Restrict to workspace for security tests
            ..SecurityPolicy::default()
        });
        EditTool::new(security)
    }

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = EditTool::new(security);
        assert_eq!(tool.name(), "edit");
        assert!(tool.parameters_schema()["properties"]["file_path"].is_object());
    }

    #[tokio::test]
    async fn simple_replacement() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "world",
                "new_string": "Rust"
            }))
            .await
            .unwrap();

        assert!(result.success, "Expected success, got: {:?}", result.error);
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hello, Rust!");
    }

    #[tokio::test]
    async fn replace_all() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"foo bar foo baz foo").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "foo",
                "new_string": "qux",
                "replace_all": true
            }))
            .await
            .unwrap();

        assert!(result.success);
        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "qux bar qux baz qux");
    }

    #[tokio::test]
    async fn non_unique_fails() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"foo bar foo baz foo").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "foo",
                "new_string": "qux",
                "replace_all": false
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not unique"));
    }

    #[tokio::test]
    async fn same_strings_fails() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "world",
                "new_string": "world"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("must be different"));
    }

    #[tokio::test]
    async fn fuzzy_whitespace_match() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        // Content has extra spaces
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"const   x   =   1;").await.unwrap();
        drop(file);

        // Search for normalized version
        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "const x = 1",
                "new_string": "const y = 2"
            }))
            .await
            .unwrap();

        // Fuzzy matching should succeed
        assert!(result.success, "Expected success, got: {:?}", result.error);
    }

    #[tokio::test]
    async fn not_found() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "file_path": file_path.to_str().unwrap(),
                "old_string": "nonexistent",
                "new_string": "replacement"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let tool = setup_restricted();

        let result = tool
            .execute(json!({
                "file_path": "/etc/passwd",
                "old_string": "root",
                "new_string": "admin"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_parameters() {
        let (_tmp, tool) = setup().await;

        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
