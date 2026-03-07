//! NAPI bindings for Tool Registry
//!
//! Provides JavaScript/TypeScript bindings for unified tool discovery and execution.
//! This module exposes all Rust tools through a single registry interface.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures_util::future::join_all;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::json;
use walkdir;

use crate::tools::{
    apply_patch::PatchApplicator,
    edit::Editor,
    glob::{Glob, GlobOptions},
    grep::{Grep, GrepOptions},
    multiedit::MultiEditor,
    read::Reader,
};

// ============================================================================
// NAPI Types
// ============================================================================

/// Tool specification for LLM function calling
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiToolSpec {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description for the LLM
    pub description: String,
    /// JSON Schema for the tool's parameters (as JSON string)
    pub parameters_schema: String,
    /// Whether this tool is implemented natively in Rust
    pub native: bool,
}

/// Result of tool execution
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiToolExecuteResult {
    /// Whether the tool succeeded
    pub success: bool,
    /// Tool output (stdout, result text, etc.)
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u32,
}

/// Tool call specification for batch execution
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiToolCall {
    /// Tool name
    pub tool: String,
    /// Tool arguments as JSON string
    pub args_json: String,
    /// Call ID for tracking
    pub call_id: String,
}

/// Batch execution result
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiBatchResult {
    /// Results in same order as input calls
    pub results: Vec<NapiToolExecuteResult>,
    /// Total execution duration in milliseconds
    pub total_duration_ms: u32,
    /// Number of calls that were native (vs fallback)
    pub native_count: u32,
}

/// Result of argument validation
#[napi(object)]
#[derive(Debug, Clone)]
pub struct NapiValidationResult {
    /// Whether the arguments are valid
    pub valid: bool,
    /// Validation error messages
    pub errors: Vec<String>,
}

// ============================================================================
// Internal Tool Registry
// ============================================================================

/// Internal tool specification (not exposed to NAPI)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Build the list of all available Rust tool specifications
fn build_tool_specs() -> Vec<ToolDef> {
    vec![
        // Grep tool
        ToolDef {
            name: "grep".to_string(),
            description: "Search for content in files using regex patterns. Supports glob filtering, \
                file type filtering, context lines, and multiple output modes. Uses ripgrep \
                under the hood for high performance.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "The path to search in (file or directory)"
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
                        "description": "Output mode"
                    },
                    "-B": {
                        "type": "integer",
                        "description": "Lines before match"
                    },
                    "-A": {
                        "type": "integer",
                        "description": "Lines after match"
                    },
                    "-C": {
                        "type": "integer",
                        "description": "Context lines (before and after)"
                    },
                    "head_limit": {
                        "type": "integer",
                        "description": "Limit output to first N results"
                    },
                    "multiline": {
                        "type": "boolean",
                        "description": "Enable multiline matching"
                    }
                },
                "required": ["pattern"]
            }),
        },
        // Glob tool
        ToolDef {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern. Returns file paths sorted by \
                modification time. Uses the ignore crate for high performance and respects \
                .gitignore patterns.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match (e.g., \"**/*.rs\", \"src/**/*.ts\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory for search"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results"
                    },
                    "include_hidden": {
                        "type": "boolean",
                        "description": "Include hidden files/directories"
                    }
                },
                "required": ["pattern"]
            }),
        },
        // Read tool
        ToolDef {
            name: "read".to_string(),
            description: "Read a file from the filesystem. Supports line range selection and \
                uses memory-mapped I/O for large files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from (1-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to read"
                    }
                },
                "required": ["file_path"]
            }),
        },
        // Write tool
        ToolDef {
            name: "write".to_string(),
            description: "Write content to a file. Creates parent directories if needed and \
                can optionally create backups.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    },
                    "create_backup": {
                        "type": "boolean",
                        "description": "Create a backup before overwriting"
                    }
                },
                "required": ["file_path", "content"]
            }),
        },
        // Edit tool
        ToolDef {
            name: "edit".to_string(),
            description: "Edit a file by replacing text. Uses fuzzy matching when exact \
                match fails. Returns a unified diff of changes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Text to replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        },
        // Ls tool
        ToolDef {
            name: "ls".to_string(),
            description: "List directory contents. Returns file metadata including size, \
                type, and modification time.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Include hidden files"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "List recursively"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum recursion depth"
                    }
                },
                "required": ["path"]
            }),
        },
        // Apply patch tool
        ToolDef {
            name: "apply_patch".to_string(),
            description: "Apply a unified diff patch to files. Supports fuzzy matching for \
                context lines that have minor differences.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "patch": {
                        "type": "string",
                        "description": "The patch content in unified diff format"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory for relative paths"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Preview changes without applying"
                    },
                    "fuzz": {
                        "type": "integer",
                        "description": "Fuzz factor for context matching (0-3)"
                    }
                },
                "required": ["patch"]
            }),
        },
        // Multiedit tool
        ToolDef {
            name: "multiedit".to_string(),
            description: "Apply multiple edits to one or more files in a single operation. \
                More efficient than multiple separate edit calls.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" },
                                "old_string": { "type": "string" },
                                "new_string": { "type": "string" }
                            },
                            "required": ["file_path", "old_string", "new_string"]
                        },
                        "description": "Array of edit operations"
                    }
                },
                "required": ["edits"]
            }),
        },
        // Todo tool
        ToolDef {
            name: "todo".to_string(),
            description: "Manage a task list for tracking work items. Supports creating, \
                updating, and querying tasks.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "update", "get", "list", "delete"],
                        "description": "Action to perform"
                    },
                    "id": {
                        "type": "string",
                        "description": "Task ID (for update/get/delete)"
                    },
                    "subject": {
                        "type": "string",
                        "description": "Task subject (for create)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Task description (for create)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed"],
                        "description": "Task status (for update)"
                    }
                },
                "required": ["action"]
            }),
        },
    ]
}

// ============================================================================
// ToolRegistryHandle
// ============================================================================

/// Thread-safe handle to the tool registry
#[napi]
pub struct ToolRegistryHandle {
    tools: Arc<Mutex<HashMap<String, ToolDef>>>,
    grep: Arc<Grep>,
    glob: Arc<Glob>,
    reader: Arc<Reader>,
    editor: Arc<Mutex<Editor>>,
    multi_editor: Arc<Mutex<MultiEditor>>,
    patch_applicator: Arc<Mutex<PatchApplicator>>,
}

#[napi]
impl ToolRegistryHandle {
    /// Create a new tool registry with all built-in tools
    #[napi(constructor)]
    pub fn new() -> Self {
        let specs = build_tool_specs();
        let mut tools = HashMap::new();
        for spec in specs {
            tools.insert(spec.name.clone(), spec);
        }

        Self {
            tools: Arc::new(Mutex::new(tools)),
            grep: Arc::new(Grep::new()),
            glob: Arc::new(Glob::new()),
            reader: Arc::new(Reader::new()),
            editor: Arc::new(Mutex::new(Editor::new())),
            multi_editor: Arc::new(Mutex::new(MultiEditor::new())),
            patch_applicator: Arc::new(Mutex::new(PatchApplicator::new())),
        }
    }

    /// List all available tool specifications
    #[napi]
    pub fn list_tools(&self) -> Result<Vec<NapiToolSpec>> {
        let tools = self
            .tools
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let specs: Vec<NapiToolSpec> = tools
            .values()
            .map(|t| NapiToolSpec {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters_schema: serde_json::to_string(&t.parameters).unwrap_or_default(),
                native: true,
            })
            .collect();

        Ok(specs)
    }

    /// Get a specific tool's specification by name
    #[napi]
    pub fn get_spec(&self, name: String) -> Result<Option<NapiToolSpec>> {
        let tools = self
            .tools
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(tools.get(&name).map(|t| NapiToolSpec {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters_schema: serde_json::to_string(&t.parameters).unwrap_or_default(),
            native: true,
        }))
    }

    /// Check if a tool exists in the registry
    #[napi]
    pub fn has_tool(&self, name: String) -> Result<bool> {
        let tools = self
            .tools
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(tools.contains_key(&name))
    }

    /// Get the number of registered tools
    #[napi]
    pub fn tool_count(&self) -> Result<u32> {
        let tools = self
            .tools
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(tools.len() as u32)
    }

    /// Validate tool arguments against its schema
    #[napi]
    pub fn validate_args(&self, name: String, args_json: String) -> Result<NapiValidationResult> {
        let tools = self
            .tools
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let tool = match tools.get(&name) {
            Some(t) => t,
            None => {
                return Ok(NapiValidationResult {
                    valid: false,
                    errors: vec![format!("Unknown tool: {}", name)],
                });
            }
        };

        // Parse the args
        let args: serde_json::Value = match serde_json::from_str(&args_json) {
            Ok(v) => v,
            Err(e) => {
                return Ok(NapiValidationResult {
                    valid: false,
                    errors: vec![format!("Invalid JSON: {}", e)],
                });
            }
        };

        // Check required fields
        let mut errors = Vec::new();
        if let Some(required) = tool.parameters.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(field) = req.as_str() {
                    if args.get(field).is_none() {
                        errors.push(format!("Missing required field: {}", field));
                    }
                }
            }
        }

        Ok(NapiValidationResult {
            valid: errors.is_empty(),
            errors,
        })
    }

    /// Execute a tool with the given arguments
    ///
    /// Note: This is an async function that dispatches to the appropriate
    /// native implementation based on the tool name.
    #[napi]
    pub async fn execute(&self, name: String, args_json: String) -> Result<NapiToolExecuteResult> {
        let start = std::time::Instant::now();

        // Parse arguments
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON: {}", e)))?;

        // Dispatch to the appropriate implementation
        let result = match name.as_str() {
            "grep" => self.execute_grep(args).await,
            "glob" => self.execute_glob(args).await,
            "read" => self.execute_read(args),
            "edit" => self.execute_edit(args),
            "write" => self.execute_write(args),
            "ls" => self.execute_ls(args),
            "apply_patch" => self.execute_apply_patch(args),
            "multiedit" => self.execute_multiedit(args),
            _ => Err(Error::from_reason(format!("Tool not implemented: {}", name))),
        };

        let duration_ms = start.elapsed().as_millis() as u32;

        match result {
            Ok(output) => Ok(NapiToolExecuteResult {
                success: true,
                output,
                error: None,
                duration_ms,
            }),
            Err(e) => Ok(NapiToolExecuteResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_ms,
            }),
        }
    }

    /// Execute multiple tools in parallel (native batch execution)
    ///
    /// Native tools (grep, glob, read, edit, write, ls, apply_patch, multiedit)
    /// are executed in parallel using futures::future::join_all. Non-native tools
    /// return an error with 'not_native' flag.
    #[napi]
    pub async fn execute_batch(&self, calls: Vec<NapiToolCall>) -> Result<NapiBatchResult> {
        let start = std::time::Instant::now();

        // List of native tool names
        const NATIVE_TOOLS: &[&str] = &[
            "grep",
            "glob",
            "read",
            "edit",
            "write",
            "ls",
            "apply_patch",
            "multiedit",
        ];

        // Clone Arc references for parallel execution
        let grep = self.grep.clone();
        let glob_tool = self.glob.clone();
        let reader = self.reader.clone();
        let editor = self.editor.clone();
        let multi_editor = self.multi_editor.clone();
        let patch_applicator = self.patch_applicator.clone();

        // Execute all calls in parallel
        let futures: Vec<_> = calls
            .into_iter()
            .map(|call| {
                let name = call.tool.clone();
                let args_json = call.args_json.clone();
                let is_native = NATIVE_TOOLS.contains(&name.as_str());

                // Clone Arcs for this future
                let grep = grep.clone();
                let glob_tool = glob_tool.clone();
                let reader = reader.clone();
                let editor = editor.clone();
                let multi_editor = multi_editor.clone();
                let patch_applicator = patch_applicator.clone();

                async move {
                    if !is_native {
                        return NapiToolExecuteResult {
                            success: false,
                            output: String::new(),
                            error: Some(format!("Tool '{}' is not_native", name)),
                            duration_ms: 0,
                        };
                    }

                    let call_start = std::time::Instant::now();

                    // Parse arguments
                    let args: serde_json::Value = match serde_json::from_str(&args_json) {
                        Ok(v) => v,
                        Err(e) => {
                            return NapiToolExecuteResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Invalid JSON: {}", e)),
                                duration_ms: call_start.elapsed().as_millis() as u32,
                            };
                        }
                    };

                    // Dispatch to the appropriate implementation
                    let result: std::result::Result<String, Error> = match name.as_str() {
                        "grep" => Self::execute_grep_batch(&grep, args).await,
                        "glob" => Self::execute_glob_batch(&glob_tool, args).await,
                        "read" => Self::execute_read_batch(&reader, args),
                        "edit" => Self::execute_edit_batch(&editor, args),
                        "write" => Self::execute_write_batch(args),
                        "ls" => Self::execute_ls_batch(args),
                        "apply_patch" => Self::execute_apply_patch_batch(&patch_applicator, args),
                        "multiedit" => Self::execute_multiedit_batch(&multi_editor, args),
                        _ => Err(Error::from_reason(format!(
                            "Tool not implemented: {}",
                            name
                        ))),
                    };

                    let duration_ms = call_start.elapsed().as_millis() as u32;

                    match result {
                        Ok(output) => NapiToolExecuteResult {
                            success: true,
                            output,
                            error: None,
                            duration_ms,
                        },
                        Err(e) => NapiToolExecuteResult {
                            success: false,
                            output: String::new(),
                            error: Some(e.to_string()),
                            duration_ms,
                        },
                    }
                }
            })
            .collect();

        let results = join_all(futures).await;

        // Count native calls (those that didn't return "not_native" error)
        let native_count = results
            .iter()
            .filter(|r| {
                r.success
                    || !r
                        .error
                        .as_ref()
                        .map(|e| e.contains("not_native"))
                        .unwrap_or(false)
            })
            .count() as u32;

        Ok(NapiBatchResult {
            results,
            total_duration_ms: start.elapsed().as_millis() as u32,
            native_count,
        })
    }

    // Static execution methods for batch processing (take Arc references)

    async fn execute_grep_batch(grep: &Arc<Grep>, args: serde_json::Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'pattern' parameter"))?
            .to_string();

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from);

        let glob = args.get("glob").and_then(|v| v.as_str()).map(String::from);
        let file_type = args.get("type").and_then(|v| v.as_str()).map(String::from);
        let case_insensitive = args.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_mode = args
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches")
            .to_string();

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

        let limit = args
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let multiline = args.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);
        let line_numbers = args.get("-n").and_then(|v| v.as_bool()).unwrap_or(true);

        let options = GrepOptions {
            pattern,
            path,
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

        let result = grep
            .search(&options)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        // Format output based on mode
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
                    lines.push(format!(
                        "\n... (truncated, {} total matches)",
                        result.total_matches
                    ));
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

        Ok(output)
    }

    async fn execute_glob_batch(glob_tool: &Arc<Glob>, args: serde_json::Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'pattern' parameter"))?
            .to_string();

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from);

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let include_hidden = args
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let options = GlobOptions {
            pattern,
            path,
            limit,
            include_hidden,
            ..Default::default()
        };

        let results = glob_tool
            .find(&options)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let files: Vec<&str> = results.files.iter().map(|f| f.path.as_str()).collect();

        Ok(files.join("\n"))
    }

    fn execute_read_batch(reader: &Arc<Reader>, args: serde_json::Value) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        let options = crate::tools::read::ReadOptions {
            offset,
            limit,
            ..Default::default()
        };

        let path = std::path::Path::new(file_path);
        let result = reader
            .read(path, Some(&options))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(result.content)
    }

    fn execute_edit_batch(
        editor: &Arc<Mutex<Editor>>,
        args: serde_json::Value,
    ) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'old_string' parameter"))?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'new_string' parameter"))?;

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let editor_guard = editor
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let path = std::path::Path::new(file_path);
        let operation = crate::tools::edit::EditOperation {
            old_string: old_string.to_string(),
            new_string: new_string.to_string(),
            replace_all,
        };

        let result = editor_guard
            .edit(path, &operation)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            Ok(format!(
                "Replaced {} occurrence(s)\n\n{}",
                result.replacements, result.diff
            ))
        } else {
            Err(Error::from_reason(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    fn execute_write_batch(args: serde_json::Value) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'content' parameter"))?;

        let create_backup = args
            .get("create_backup")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = std::path::Path::new(file_path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::from_reason(format!("Failed to create directories: {}", e)))?;
        }

        // Create backup if requested and file exists
        if create_backup && path.exists() {
            let backup_path = format!("{}.bak", file_path);
            std::fs::copy(path, &backup_path)
                .map_err(|e| Error::from_reason(format!("Failed to create backup: {}", e)))?;
        }

        // Write the file
        std::fs::write(path, content)
            .map_err(|e| Error::from_reason(format!("Failed to write file: {}", e)))?;

        Ok(format!("Wrote {} bytes to {}", content.len(), file_path))
    }

    fn execute_ls_batch(args: serde_json::Value) -> Result<String> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'path' parameter"))?;

        let show_all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = args
            .get("depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let path = std::path::Path::new(dir_path);
        if !path.exists() {
            return Err(Error::from_reason(format!("Path does not exist: {}", dir_path)));
        }

        let mut entries = Vec::new();

        if recursive {
            // Use walkdir for recursive listing
            let walker = walkdir::WalkDir::new(path)
                .max_depth(max_depth.unwrap_or(10))
                .follow_links(false);

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy();
                if !show_all && name.starts_with('.') {
                    continue;
                }
                if entry.path() == path {
                    continue;
                }

                let relative = entry
                    .path()
                    .strip_prefix(path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());

                if entry.file_type().is_dir() {
                    entries.push(format!("{}/ (dir)", relative));
                } else {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    entries.push(format!("{} ({} bytes)", relative, size));
                }
            }
        } else {
            // Simple directory listing
            let read_dir = std::fs::read_dir(path)
                .map_err(|e| Error::from_reason(format!("Failed to read directory: {}", e)))?;

            for entry in read_dir.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !show_all && name.starts_with('.') {
                    continue;
                }

                let file_type = entry.file_type().ok();
                let metadata = entry.metadata().ok();

                if file_type.map(|ft| ft.is_dir()).unwrap_or(false) {
                    entries.push(format!("{}/ (dir)", name));
                } else {
                    let size = metadata.map(|m| m.len()).unwrap_or(0);
                    entries.push(format!("{} ({} bytes)", name, size));
                }
            }

            // Sort entries
            entries.sort();
        }

        Ok(entries.join("\n"))
    }

    fn execute_apply_patch_batch(
        patch_applicator: &Arc<Mutex<PatchApplicator>>,
        args: serde_json::Value,
    ) -> Result<String> {
        let patch_content = args
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'patch' parameter"))?;

        let working_dir = args
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(String::from);

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let fuzz = args
            .get("fuzz")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2);

        let applicator = patch_applicator
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let options = crate::tools::apply_patch::ApplyPatchOptions {
            dry_run,
            fuzz,
            working_dir,
            ..Default::default()
        };

        let result = applicator
            .apply(patch_content, Some(&options))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            let mode = if dry_run { "(dry run) " } else { "" };
            Ok(format!(
                "{}Applied patch: {} file(s) modified\n\n{}",
                mode, result.files_changed, result.combined_diff
            ))
        } else {
            Err(Error::from_reason(
                result
                    .error
                    .unwrap_or_else(|| "Patch application failed".to_string()),
            ))
        }
    }

    fn execute_multiedit_batch(
        multi_editor: &Arc<Mutex<MultiEditor>>,
        args: serde_json::Value,
    ) -> Result<String> {
        let edits = args
            .get("edits")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::from_reason("Missing 'edits' parameter"))?;

        let editor = multi_editor
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let operations: Vec<crate::tools::multiedit::FileEdit> = edits
            .iter()
            .filter_map(|edit| {
                let file_path = edit.get("file_path")?.as_str()?.to_string();
                let old_string = edit.get("old_string")?.as_str()?.to_string();
                let new_string = edit.get("new_string")?.as_str()?.to_string();
                Some(crate::tools::multiedit::FileEdit {
                    file_path,
                    old_string,
                    new_string,
                    replace_all: false,
                })
            })
            .collect();

        if operations.is_empty() {
            return Err(Error::from_reason("No valid edits provided"));
        }

        let result = editor
            .edit_multiple(&operations, None)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            Ok(format!(
                "Applied {} edit(s) across {} file(s)\n\n{}",
                result.total_replacements, result.files_edited, result.combined_diff
            ))
        } else {
            Err(Error::from_reason(
                result
                    .error
                    .unwrap_or_else(|| "Multiedit failed".to_string()),
            ))
        }
    }

    // Internal execution methods (instance methods for single execute())

    async fn execute_grep(&self, args: serde_json::Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'pattern' parameter"))?
            .to_string();

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from);

        let glob = args.get("glob").and_then(|v| v.as_str()).map(String::from);
        let file_type = args.get("type").and_then(|v| v.as_str()).map(String::from);
        let case_insensitive = args.get("-i").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_mode = args
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches")
            .to_string();

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

        let limit = args
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let multiline = args.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);
        let line_numbers = args.get("-n").and_then(|v| v.as_bool()).unwrap_or(true);

        let options = GrepOptions {
            pattern,
            path,
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

        let result = self
            .grep
            .search(&options)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        // Format output based on mode
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
                    lines.push(format!(
                        "\n... (truncated, {} total matches)",
                        result.total_matches
                    ));
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

        Ok(output)
    }

    async fn execute_glob(&self, args: serde_json::Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'pattern' parameter"))?
            .to_string();

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from);

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let include_hidden = args
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let options = GlobOptions {
            pattern,
            path,
            limit,
            include_hidden,
            ..Default::default()
        };

        let results = self
            .glob
            .find(&options)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let files: Vec<&str> = results.files.iter().map(|f| f.path.as_str()).collect();

        Ok(files.join("\n"))
    }

    fn execute_read(&self, args: serde_json::Value) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        let options = crate::tools::read::ReadOptions {
            offset,
            limit,
            ..Default::default()
        };

        let path = std::path::Path::new(file_path);
        let result = self
            .reader
            .read(path, Some(&options))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(result.content)
    }

    fn execute_edit(&self, args: serde_json::Value) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'old_string' parameter"))?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'new_string' parameter"))?;

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let editor = self
            .editor
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let path = std::path::Path::new(file_path);
        let operation = crate::tools::edit::EditOperation {
            old_string: old_string.to_string(),
            new_string: new_string.to_string(),
            replace_all,
        };

        let result = editor
            .edit(path, &operation)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            Ok(format!(
                "Replaced {} occurrence(s)\n\n{}",
                result.replacements, result.diff
            ))
        } else {
            Err(Error::from_reason(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    fn execute_write(&self, args: serde_json::Value) -> Result<String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'file_path' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'content' parameter"))?;

        let create_backup = args
            .get("create_backup")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = std::path::Path::new(file_path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::from_reason(format!("Failed to create directories: {}", e)))?;
        }

        // Create backup if requested and file exists
        if create_backup && path.exists() {
            let backup_path = format!("{}.bak", file_path);
            std::fs::copy(path, &backup_path)
                .map_err(|e| Error::from_reason(format!("Failed to create backup: {}", e)))?;
        }

        // Write the file
        std::fs::write(path, content)
            .map_err(|e| Error::from_reason(format!("Failed to write file: {}", e)))?;

        Ok(format!("Wrote {} bytes to {}", content.len(), file_path))
    }

    fn execute_ls(&self, args: serde_json::Value) -> Result<String> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'path' parameter"))?;

        let show_all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = args
            .get("depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let path = std::path::Path::new(dir_path);
        if !path.exists() {
            return Err(Error::from_reason(format!("Path does not exist: {}", dir_path)));
        }

        let mut entries = Vec::new();

        if recursive {
            // Use walkdir for recursive listing
            let walker = walkdir::WalkDir::new(path)
                .max_depth(max_depth.unwrap_or(10))
                .follow_links(false);

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy();
                if !show_all && name.starts_with('.') {
                    continue;
                }
                if entry.path() == path {
                    continue;
                }

                let relative = entry
                    .path()
                    .strip_prefix(path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());

                if entry.file_type().is_dir() {
                    entries.push(format!("{}/ (dir)", relative));
                } else {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    entries.push(format!("{} ({} bytes)", relative, size));
                }
            }
        } else {
            // Simple directory listing
            let read_dir = std::fs::read_dir(path)
                .map_err(|e| Error::from_reason(format!("Failed to read directory: {}", e)))?;

            for entry in read_dir.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !show_all && name.starts_with('.') {
                    continue;
                }

                let file_type = entry.file_type().ok();
                let metadata = entry.metadata().ok();

                if file_type.map(|ft| ft.is_dir()).unwrap_or(false) {
                    entries.push(format!("{}/ (dir)", name));
                } else {
                    let size = metadata.map(|m| m.len()).unwrap_or(0);
                    entries.push(format!("{} ({} bytes)", name, size));
                }
            }

            // Sort entries
            entries.sort();
        }

        Ok(entries.join("\n"))
    }

    fn execute_apply_patch(&self, args: serde_json::Value) -> Result<String> {
        let patch_content = args
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from_reason("Missing 'patch' parameter"))?;

        let working_dir = args
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(String::from);

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let fuzz = args
            .get("fuzz")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2);

        let applicator = self
            .patch_applicator
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let options = crate::tools::apply_patch::ApplyPatchOptions {
            dry_run,
            fuzz,
            working_dir,
            ..Default::default()
        };

        let result = applicator
            .apply(patch_content, Some(&options))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            let mode = if dry_run { "(dry run) " } else { "" };
            Ok(format!(
                "{}Applied patch: {} file(s) modified\n\n{}",
                mode,
                result.files_changed,
                result.combined_diff
            ))
        } else {
            Err(Error::from_reason(
                result.error.unwrap_or_else(|| "Patch application failed".to_string()),
            ))
        }
    }

    fn execute_multiedit(&self, args: serde_json::Value) -> Result<String> {
        let edits = args
            .get("edits")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::from_reason("Missing 'edits' parameter"))?;

        let multi_editor = self
            .multi_editor
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let operations: Vec<crate::tools::multiedit::FileEdit> = edits
            .iter()
            .filter_map(|edit| {
                let file_path = edit.get("file_path")?.as_str()?.to_string();
                let old_string = edit.get("old_string")?.as_str()?.to_string();
                let new_string = edit.get("new_string")?.as_str()?.to_string();
                Some(crate::tools::multiedit::FileEdit {
                    file_path,
                    old_string,
                    new_string,
                    replace_all: false,
                })
            })
            .collect();

        if operations.is_empty() {
            return Err(Error::from_reason("No valid edits provided"));
        }

        let result = multi_editor
            .edit_multiple(&operations, None)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        if result.success {
            Ok(format!(
                "Applied {} edit(s) across {} file(s)\n\n{}",
                result.total_replacements,
                result.files_edited,
                result.combined_diff
            ))
        } else {
            Err(Error::from_reason(
                result.error.unwrap_or_else(|| "Multiedit failed".to_string()),
            ))
        }
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create a new tool registry with all built-in tools
#[napi]
pub fn create_tool_registry() -> ToolRegistryHandle {
    ToolRegistryHandle::new()
}

/// Get specifications for all built-in Rust tools (without creating a registry)
#[napi]
pub fn get_builtin_tool_specs() -> Vec<NapiToolSpec> {
    build_tool_specs()
        .into_iter()
        .map(|t| NapiToolSpec {
            name: t.name,
            description: t.description,
            parameters_schema: serde_json::to_string(&t.parameters).unwrap_or_default(),
            native: true,
        })
        .collect()
}

/// Get the list of tool names that have native Rust implementations
#[napi]
pub fn get_native_tool_names() -> Vec<String> {
    // These are tools that have native execute() implementations
    vec![
        "grep".to_string(),
        "glob".to_string(),
        "read".to_string(),
        "edit".to_string(),
        "write".to_string(),
        "ls".to_string(),
        "apply_patch".to_string(),
        "multiedit".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tool_specs() {
        let specs = build_tool_specs();
        assert!(!specs.is_empty());

        // Check that grep exists
        let grep = specs.iter().find(|s| s.name == "grep");
        assert!(grep.is_some());

        let grep = grep.unwrap();
        assert!(grep.description.contains("regex"));
        assert!(grep.parameters.get("required").is_some());
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistryHandle::new();
        let count = registry.tool_count().unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_list_tools() {
        let registry = ToolRegistryHandle::new();
        let tools = registry.list_tools().unwrap();
        assert!(!tools.is_empty());

        // All tools should be native
        for tool in &tools {
            assert!(tool.native);
        }
    }

    #[test]
    fn test_get_spec() {
        let registry = ToolRegistryHandle::new();

        let grep = registry.get_spec("grep".to_string()).unwrap();
        assert!(grep.is_some());
        assert_eq!(grep.unwrap().name, "grep");

        let unknown = registry.get_spec("unknown_tool".to_string()).unwrap();
        assert!(unknown.is_none());
    }

    #[test]
    fn test_has_tool() {
        let registry = ToolRegistryHandle::new();

        assert!(registry.has_tool("grep".to_string()).unwrap());
        assert!(registry.has_tool("glob".to_string()).unwrap());
        assert!(!registry.has_tool("unknown".to_string()).unwrap());
    }

    #[test]
    fn test_validate_args() {
        let registry = ToolRegistryHandle::new();

        // Valid args for grep
        let result = registry
            .validate_args(
                "grep".to_string(),
                r#"{"pattern": "test"}"#.to_string(),
            )
            .unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());

        // Missing required field
        let result = registry
            .validate_args("grep".to_string(), r#"{}"#.to_string())
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("pattern")));

        // Unknown tool
        let result = registry
            .validate_args("unknown".to_string(), r#"{}"#.to_string())
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Unknown tool")));

        // Invalid JSON
        let result = registry
            .validate_args("grep".to_string(), "not json".to_string())
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Invalid JSON")));
    }

    #[test]
    fn test_get_builtin_tool_specs() {
        let specs = get_builtin_tool_specs();
        assert!(!specs.is_empty());

        // Verify specs have valid JSON schemas
        for spec in specs {
            let schema: serde_json::Value = serde_json::from_str(&spec.parameters_schema).unwrap();
            assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
        }
    }

    #[test]
    fn test_get_native_tool_names() {
        let names = get_native_tool_names();
        assert!(names.contains(&"grep".to_string()));
        assert!(names.contains(&"glob".to_string()));
        assert!(names.contains(&"read".to_string()));
        assert!(names.contains(&"edit".to_string()));
    }
}
