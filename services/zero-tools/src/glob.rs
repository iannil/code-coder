//! Glob tool.
//!
//! Fast file pattern matching using the ignore crate.
//! Wraps zero-core's Glob for use with the Tool trait.

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use zero_core::tools::glob::{Glob, GlobOptions};

/// Glob tool for file pattern matching.
pub struct GlobTool {
    security: Arc<SecurityPolicy>,
    glob: Glob,
}

impl GlobTool {
    /// Create a new glob tool.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self {
            security,
            glob: Glob::new(),
        }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Supports patterns like \"**/*.rs\", \
        \"src/**/*.ts\". Respects .gitignore by default. Returns file paths sorted \
        by modification time."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., \"**/*.rs\", \"src/**/*.ts\")"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in (defaults to workspace)"
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Whether to include hidden files (starting with .)"
                },
                "respect_gitignore": {
                    "type": "boolean",
                    "description": "Whether to respect .gitignore files (default: true)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth to traverse (unlimited if not specified)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Limit the number of results"
                },
                "sort_by_mtime": {
                    "type": "boolean",
                    "description": "Sort results by modification time (newest first)"
                },
                "files_only": {
                    "type": "boolean",
                    "description": "Only include files (no directories), default: true"
                },
                "follow_symlinks": {
                    "type": "boolean",
                    "description": "Follow symbolic links"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?
            .to_string();

        // Resolve path to absolute, defaulting to workspace
        let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let search_path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            self.security.workspace_dir.join(path_str)
        };

        // Validate path is within allowed boundaries
        if !self.security.is_path_allowed(search_path.to_string_lossy().as_ref()) {
            return Ok(ToolResult::failure(format!(
                "Path not allowed: {}",
                search_path.display()
            )));
        }

        // Build glob options from args
        let options = GlobOptions {
            pattern,
            path: Some(search_path.to_string_lossy().to_string()),
            include_hidden: args
                .get("include_hidden")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            respect_gitignore: args
                .get("respect_gitignore")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            max_depth: args.get("max_depth").and_then(|v| v.as_u64()).map(|v| v as usize),
            limit: args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize),
            sort_by_mtime: args
                .get("sort_by_mtime")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            files_only: args
                .get("files_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            follow_symlinks: args
                .get("follow_symlinks")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        // Execute the search
        match self.glob.find(&options).await {
            Ok(result) => {
                let mut output = result
                    .files
                    .iter()
                    .map(|f| f.path.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                if result.truncated {
                    output.push_str(&format!(
                        "\n... (truncated, {} total matches)",
                        result.total_matches
                    ));
                }

                Ok(ToolResult::success(output))
            }
            Err(e) => Ok(ToolResult::failure(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, GlobTool) {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            workspace_only: false, // Allow absolute paths for tests
            forbidden_paths: vec![], // Clear forbidden paths for tests
            ..SecurityPolicy::default()
        });
        (tmp, GlobTool::new(security))
    }

    fn setup_restricted() -> GlobTool {
        let security = Arc::new(SecurityPolicy {
            workspace_dir: std::path::PathBuf::from("/nonexistent"),
            workspace_only: true, // Restrict to workspace for security tests
            ..SecurityPolicy::default()
        });
        GlobTool::new(security)
    }

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = GlobTool::new(security);
        assert_eq!(tool.name(), "glob");
        assert!(tool.parameters_schema()["properties"]["pattern"].is_object());
    }

    #[tokio::test]
    async fn find_rust_files() {
        let (tmp, tool) = setup();

        // Create test files
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(tmp.path().join("README.md"), "# Test").unwrap();

        let result = tool
            .execute(json!({
                "pattern": "**/*.rs",
                "path": tmp.path().to_str().unwrap()
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("main.rs"));
        assert!(result.output.contains("lib.rs"));
        assert!(!result.output.contains("README"));
    }

    #[tokio::test]
    async fn find_with_limit() {
        let (tmp, tool) = setup();

        // Create multiple files
        for i in 0..5 {
            fs::write(tmp.path().join(format!("file{}.txt", i)), "content").unwrap();
        }

        let result = tool
            .execute(json!({
                "pattern": "**/*.txt",
                "path": tmp.path().to_str().unwrap(),
                "limit": 2
            }))
            .await
            .unwrap();

        assert!(result.success);
        // Should be truncated
        let lines: Vec<_> = result.output.lines().collect();
        assert!(lines.len() <= 3); // 2 files + truncation message
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let tool = setup_restricted();

        // Try to search outside workspace
        let result = tool
            .execute(json!({
                "pattern": "*",
                "path": "/etc"
            }))
            .await
            .unwrap();

        // Should be blocked by security policy
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_pattern() {
        let (_tmp, tool) = setup();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
