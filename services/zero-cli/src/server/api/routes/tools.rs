//! Tools API endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::state::AppState;
use zero_core::{
    GrepOptions, GlobOptions, ReadOptions, WriteOptions, LsOptions,
    CodeSearchOptions, WebFetchOptions, TruncateOptions,
};

/// Tool execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub params: Value,
}

/// Tool execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResponse {
    pub fn ok(result: Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            duration_ms,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(error.into()),
            duration_ms: 0,
        }
    }
}

/// Available tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// List all available tools
pub async fn list_tools() -> Json<Vec<ToolInfo>> {
    let tools = vec![
        ToolInfo {
            name: "grep".to_string(),
            description: "Search file contents with regex".to_string(),
        },
        ToolInfo {
            name: "glob".to_string(),
            description: "Find files by pattern".to_string(),
        },
        ToolInfo {
            name: "read".to_string(),
            description: "Read file contents".to_string(),
        },
        ToolInfo {
            name: "write".to_string(),
            description: "Write file contents".to_string(),
        },
        ToolInfo {
            name: "edit".to_string(),
            description: "Edit file with string replacement".to_string(),
        },
        ToolInfo {
            name: "ls".to_string(),
            description: "List directory contents".to_string(),
        },
        ToolInfo {
            name: "codesearch".to_string(),
            description: "Semantic code search".to_string(),
        },
        ToolInfo {
            name: "webfetch".to_string(),
            description: "Fetch URL content".to_string(),
        },
        ToolInfo {
            name: "truncate".to_string(),
            description: "Truncate large output".to_string(),
        },
    ];

    Json(tools)
}

/// Execute a tool
pub async fn execute_tool(
    State(state): State<Arc<AppState>>,
    Path(tool): Path<String>,
    Json(request): Json<ToolRequest>,
) -> Result<Json<ToolResponse>, (StatusCode, Json<ToolResponse>)> {
    let start = std::time::Instant::now();

    let result = match tool.as_str() {
        "grep" => execute_grep(&state, request.params).await,
        "glob" => execute_glob(&state, request.params).await,
        "read" => execute_read(&state, request.params).await,
        "write" => execute_write(&state, request.params).await,
        "ls" => execute_ls(&state, request.params).await,
        "codesearch" => execute_codesearch(&state, request.params).await,
        "webfetch" => execute_webfetch(&state, request.params).await,
        "truncate" => execute_truncate(&state, request.params).await,
        _ => Err(format!("Unknown tool: {}", tool)),
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(value) => Ok(Json(ToolResponse::ok(value, duration_ms))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(ToolResponse::err(e)))),
    }
}

async fn execute_grep(state: &AppState, params: Value) -> Result<Value, String> {
    let options: GrepOptions = serde_json::from_value(params)
        .map_err(|e| format!("Invalid grep params: {}", e))?;

    let result = state.grep.search(&options).await
        .map_err(|e| format!("Grep failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_glob(state: &AppState, params: Value) -> Result<Value, String> {
    let options: GlobOptions = serde_json::from_value(params)
        .map_err(|e| format!("Invalid glob params: {}", e))?;

    let glob = zero_core::tools::glob::Glob::new();
    let result = glob.find(&options).await
        .map_err(|e| format!("Glob failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_read(state: &AppState, params: Value) -> Result<Value, String> {
    let file_path: String = params.get("file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing file_path".to_string())?;

    let options: Option<ReadOptions> = params.get("options")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("Invalid read options: {}", e))?;

    let path = std::path::Path::new(&file_path);
    let result = state.reader.read(path, options.as_ref())
        .map_err(|e| format!("Read failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_write(state: &AppState, params: Value) -> Result<Value, String> {
    let file_path: String = params.get("file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing file_path".to_string())?;

    let content: String = params.get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing content".to_string())?;

    let options: Option<WriteOptions> = params.get("options")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("Invalid write options: {}", e))?;

    let path = std::path::Path::new(&file_path);
    let result = state.writer.write(path, &content, options.as_ref())
        .map_err(|e| format!("Write failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_ls(state: &AppState, params: Value) -> Result<Value, String> {
    let path: String = params.get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| ".".to_string());

    let options: Option<LsOptions> = params.get("options")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("Invalid ls options: {}", e))?;

    let path = std::path::Path::new(&path);
    let result = state.ls.list(path, options.as_ref())
        .map_err(|e| format!("Ls failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_codesearch(state: &AppState, params: Value) -> Result<Value, String> {
    let options: CodeSearchOptions = serde_json::from_value(params)
        .map_err(|e| format!("Invalid codesearch params: {}", e))?;

    let result = state.code_search.search(&options).await
        .map_err(|e| format!("CodeSearch failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_webfetch(state: &AppState, params: Value) -> Result<Value, String> {
    let options: WebFetchOptions = serde_json::from_value(params)
        .map_err(|e| format!("Invalid webfetch params: {}", e))?;

    let result = state.web_fetcher.fetch(&options).await
        .map_err(|e| format!("WebFetch failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

async fn execute_truncate(state: &AppState, params: Value) -> Result<Value, String> {
    let text: String = params.get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing text".to_string())?;

    let options: Option<TruncateOptions> = params.get("options")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("Invalid truncate options: {}", e))?;

    let result = state.truncator.truncate(&text, options.as_ref())
        .map_err(|e| format!("Truncate failed: {}", e))?;

    serde_json::to_value(result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}
