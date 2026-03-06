//! Grep tool.
//!
//! High-performance content search using ripgrep-based implementation.
//! Wraps zero-core's Grep for use with the Tool trait.

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use zero_core::tools::grep::{Grep, GrepOptions};

/// Grep tool for content search.
pub struct GrepTool {
    security: Arc<SecurityPolicy>,
    grep: Grep,
}

impl GrepTool {
    /// Create a new grep tool.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self {
            security,
            grep: Grep::new(),
        }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for content in files using regex patterns. Supports glob filtering, \
        file type filtering, context lines, and multiple output modes. Uses ripgrep \
        under the hood for high performance."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "The path to search in (file or directory, defaults to workspace)"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., \"*.rs\", \"*.{ts,tsx}\")"
                },
                "type": {
                    "type": "string",
                    "description": "File type to search (e.g., \"rust\", \"typescript\")"
                },
                "-i": {
                    "type": "boolean",
                    "description": "Case insensitive search"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: content (show matches), files_with_matches (file paths only), count (match counts)"
                },
                "-B": {
                    "type": "integer",
                    "description": "Number of lines to show before each match"
                },
                "-A": {
                    "type": "integer",
                    "description": "Number of lines to show after each match"
                },
                "-C": {
                    "type": "integer",
                    "description": "Number of context lines (before and after)"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of context lines (alias for -C)"
                },
                "head_limit": {
                    "type": "integer",
                    "description": "Limit output to first N results"
                },
                "offset": {
                    "type": "integer",
                    "description": "Skip first N results"
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline matching"
                },
                "-n": {
                    "type": "boolean",
                    "description": "Show line numbers (default: true)"
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

        // Build grep options from args
        let glob = args.get("glob").and_then(|v| v.as_str()).map(String::from);
        let file_type = args.get("type").and_then(|v| v.as_str()).map(String::from);
        let case_insensitive = args.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_mode = args
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches")
            .to_string();

        // Context lines
        let context = args.get("-C").or(args.get("context")).and_then(|v| v.as_u64());
        let context_before = args
            .get("-B")
            .and_then(|v| v.as_u64())
            .or(context)
            .unwrap_or(0) as usize;
        let context_after = args
            .get("-A")
            .and_then(|v| v.as_u64())
            .or(context)
            .unwrap_or(0) as usize;

        // Limits
        let limit = args.get("head_limit").and_then(|v| v.as_u64()).map(|v| v as usize);
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let multiline = args.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);
        let line_numbers = args.get("-n").and_then(|v| v.as_bool()).unwrap_or(true);

        let options = GrepOptions {
            pattern,
            path: Some(search_path.to_string_lossy().to_string()),
            glob,
            file_type,
            case_insensitive,
            output_mode: output_mode.clone(),
            context_before,
            context_after,
            limit,
            offset,
            multiline,
            line_numbers,
        };

        // Execute the search
        match self.grep.search(&options).await {
            Ok(result) => {
                let output = match output_mode.as_str() {
                    "content" => {
                        let mut lines = Vec::new();
                        for m in &result.matches {
                            if line_numbers {
                                lines.push(format!("{}:{}: {}", m.path, m.line_number, m.line_content));
                            } else {
                                lines.push(format!("{}: {}", m.path, m.line_content));
                            }
                        }
                        if result.truncated {
                            lines.push(format!("\n... (truncated, {} total matches)", result.total_matches));
                        }
                        lines.join("\n")
                    }
                    "files_with_matches" => {
                        let mut output = result.files.join("\n");
                        if result.truncated {
                            output.push_str(&format!(
                                "\n... (truncated, {} total files)",
                                result.total_matches
                            ));
                        }
                        output
                    }
                    "count" => result
                        .counts
                        .iter()
                        .map(|(path, count)| format!("{}:{}", path, count))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => "Invalid output mode".to_string(),
                };

                Ok(ToolResult::success(output))
            }
            Err(e) => Ok(ToolResult::failure(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn setup() -> (TempDir, GrepTool) {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy {
            workspace_dir: tmp.path().to_path_buf(),
            workspace_only: false, // Allow absolute paths for tests
            forbidden_paths: vec![], // Clear forbidden paths for tests
            ..SecurityPolicy::default()
        });
        (tmp, GrepTool::new(security))
    }

    fn setup_restricted() -> GrepTool {
        let security = Arc::new(SecurityPolicy {
            workspace_dir: std::path::PathBuf::from("/nonexistent"),
            workspace_only: true, // Restrict to workspace for security tests
            ..SecurityPolicy::default()
        });
        GrepTool::new(security)
    }

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = GrepTool::new(security);
        assert_eq!(tool.name(), "grep");
        assert!(tool.parameters_schema()["properties"]["pattern"].is_object());
    }

    #[tokio::test]
    async fn search_pattern() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.rs");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"fn main() {\n    println!(\"Hello, world!\");\n}\n")
            .await
            .unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "pattern": "fn main",
                "path": tmp.path().to_str().unwrap(),
                "output_mode": "content"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("fn main"));
    }

    #[tokio::test]
    async fn search_files_only() {
        let (tmp, tool) = setup().await;
        let file_path = tmp.path().join("test.rs");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"fn hello() {}\n").await.unwrap();
        drop(file);

        let result = tool
            .execute(json!({
                "pattern": "fn",
                "path": tmp.path().to_str().unwrap(),
                "output_mode": "files_with_matches"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("test.rs"));
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let tool = setup_restricted();

        // Try to search outside workspace
        let result = tool
            .execute(json!({
                "pattern": "test",
                "path": "/etc/passwd"
            }))
            .await
            .unwrap();

        // Should be blocked by security policy
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_pattern() {
        let (_tmp, tool) = setup().await;
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
