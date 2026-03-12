//! Tool Execution API
//!
//! Provides endpoints for listing and executing tools directly.
//!
//! ## Endpoints
//!
//! - GET /api/v1/tools - List all available tools
//! - POST /api/v1/tools/:name - Execute a specific tool

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    /// Risk level: safe, low, medium, high, critical
    pub risk_level: String,
}

#[derive(Debug, Serialize)]
pub struct ListToolsResponse {
    pub success: bool,
    pub tools: Vec<ToolInfo>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteToolRequest {
    /// Tool parameters
    #[serde(default)]
    pub params: serde_json::Value,
    /// Execution timeout in seconds (default: 30)
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteToolResponse {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/tools - List all available tools
pub async fn list_tools(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    let registry = state.tools.read().await;
    let native_tools = registry.native_tools();

    let tools: Vec<ToolInfo> = native_tools
        .iter()
        .map(|t| {
            let risk_level = match t.name().to_lowercase().as_str() {
                "read" | "glob" | "grep" | "ls" | "list" | "webfetch" | "websearch" => "safe",
                "codesearch" | "notebook_read" | "task_list" | "task_get" => "low",
                "edit" | "write" | "notebook_edit" | "task_create" | "task_update" => "medium",
                "bash" | "shell" | "mcp_call" | "browser" => "high",
                _ => "high",
            };

            ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
                risk_level: risk_level.to_string(),
            }
        })
        .collect();

    let total = tools.len();
    Json(ListToolsResponse {
        success: true,
        tools,
        total,
    })
}

/// POST /api/v1/tools/:name - Execute a specific tool
pub async fn execute_tool(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
    Json(request): Json<ExecuteToolRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let timeout_secs = request.timeout.unwrap_or(30).min(300); // Max 5 minutes

    let registry = state.tools.read().await;

    let tool = match registry.get_tool(&name).await {
        Some(t) => t,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Tool not found: {}", name),
                }),
            )
                .into_response();
        }
    };

    // Execute with timeout
    let result = match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tool.execute(request.params),
    )
    .await
    {
        Ok(Ok(result)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            if result.success {
                Json(ExecuteToolResponse {
                    success: true,
                    output: Some(result.output),
                    error: None,
                    duration_ms,
                })
                .into_response()
            } else {
                Json(ExecuteToolResponse {
                    success: false,
                    output: None,
                    error: result.error,
                    duration_ms,
                })
                .into_response()
            }
        }
        Ok(Err(e)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Json(ExecuteToolResponse {
                success: false,
                output: None,
                error: Some(e.to_string()),
                duration_ms,
            })
            .into_response()
        }
        Err(_) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(ExecuteToolResponse {
                    success: false,
                    output: None,
                    error: Some(format!("Tool execution timed out after {}s", timeout_secs)),
                    duration_ms,
                }),
            )
                .into_response()
        }
    };

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_info_serialization() {
        let info = ToolInfo {
            name: "read".to_string(),
            description: "Read file contents".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
            risk_level: "safe".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"read\""));
        assert!(json.contains("\"risk_level\":\"safe\""));
    }

    #[test]
    fn test_execute_response_serialization() {
        let response = ExecuteToolResponse {
            success: true,
            output: Some("Hello, World!".to_string()),
            error: None,
            duration_ms: 42,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"duration_ms\":42"));
    }
}
