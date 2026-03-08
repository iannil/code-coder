//! MCP (Model Context Protocol) API endpoints

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::state::AppState;

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallRequest {
    pub name: String,
    pub arguments: Value,
}

/// MCP tool call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallResponse {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

/// MCP content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String, text: Option<String> },
}

/// List available MCP tools
pub async fn list_tools(
    State(_state): State<Arc<AppState>>,
) -> Json<Vec<McpToolInfo>> {
    let tools = vec![
        McpToolInfo {
            name: "grep".to_string(),
            description: "Search file contents with regex patterns".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Glob pattern to filter files"
                    }
                },
                "required": ["pattern"]
            }),
        },
        McpToolInfo {
            name: "read".to_string(),
            description: "Read file contents".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of lines to read"
                    }
                },
                "required": ["file_path"]
            }),
        },
        McpToolInfo {
            name: "write".to_string(),
            description: "Write content to a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to write to"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["file_path", "content"]
            }),
        },
        McpToolInfo {
            name: "ls".to_string(),
            description: "List directory contents".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list"
                    }
                },
                "required": ["path"]
            }),
        },
        McpToolInfo {
            name: "webfetch".to_string(),
            description: "Fetch content from a URL".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch"
                    }
                },
                "required": ["url"]
            }),
        },
    ];

    Json(tools)
}

/// Call an MCP tool
pub async fn call_tool(
    State(state): State<Arc<AppState>>,
    Json(request): Json<McpCallRequest>,
) -> Result<Json<McpCallResponse>, (StatusCode, Json<McpCallResponse>)> {
    let make_error = |msg: String| -> (StatusCode, Json<McpCallResponse>) {
        (StatusCode::BAD_REQUEST, Json(McpCallResponse {
            content: vec![McpContent::Text { text: msg }],
            is_error: true,
        }))
    };

    let result = match request.name.as_str() {
        "grep" => {
            let options: zero_core::GrepOptions = serde_json::from_value(request.arguments)
                .map_err(|e| make_error(format!("Invalid arguments: {}", e)))?;

            let result = state.grep.search(&options).await
                .map_err(|e| make_error(format!("Grep failed: {}", e)))?;

            serde_json::to_string_pretty(&result).unwrap_or_default()
        }
        "read" => {
            let file_path = request.arguments.get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| make_error("Missing file_path".to_string()))?;

            let result = state.reader.read(std::path::Path::new(file_path), None)
                .map_err(|e| make_error(format!("Read failed: {}", e)))?;

            result.content
        }
        "write" => {
            let file_path = request.arguments.get("file_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| make_error("Missing file_path".to_string()))?;
            let content = request.arguments.get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| make_error("Missing content".to_string()))?;

            state.writer.write(std::path::Path::new(file_path), content, None)
                .map_err(|e| make_error(format!("Write failed: {}", e)))?;

            format!("Successfully wrote to {}", file_path)
        }
        "ls" => {
            let path = request.arguments.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");

            let result = state.ls.list(std::path::Path::new(path), None)
                .map_err(|e| make_error(format!("Ls failed: {}", e)))?;

            result.output
        }
        "webfetch" => {
            let url = request.arguments.get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| make_error("Missing url".to_string()))?;

            let options = zero_core::WebFetchOptions {
                url: url.to_string(),
                ..Default::default()
            };

            let result = state.web_fetcher.fetch(&options).await
                .map_err(|e| make_error(format!("Fetch failed: {}", e)))?;

            result.content
        }
        _ => return Err((
            StatusCode::NOT_FOUND,
            Json(McpCallResponse {
                content: vec![McpContent::Text {
                    text: format!("Unknown tool: {}", request.name),
                }],
                is_error: true,
            }),
        )),
    };

    Ok(Json(McpCallResponse {
        content: vec![McpContent::Text { text: result }],
        is_error: false,
    }))
}
