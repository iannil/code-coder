//! WebSocket handler for real-time communication

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::state::AppState;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Ping/pong for keepalive
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,

    /// Tool execution request
    #[serde(rename = "tool_request")]
    ToolRequest {
        id: String,
        tool: String,
        params: Value,
    },

    /// Tool execution response
    #[serde(rename = "tool_response")]
    ToolResponse {
        id: String,
        success: bool,
        result: Option<Value>,
        error: Option<String>,
    },

    /// Streaming content
    #[serde(rename = "stream")]
    Stream {
        id: String,
        content: String,
        done: bool,
    },

    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}

/// Handle WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    tracing::info!("WebSocket connection established");

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed");
                break;
            }
            Ok(Message::Ping(data)) => {
                if sender.send(Message::Pong(data)).await.is_err() {
                    break;
                }
                continue;
            }
            _ => continue,
        };

        // Parse incoming message
        let ws_msg: WsMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                let error = WsMessage::Error {
                    message: format!("Failed to parse message: {}", e),
                };
                let _ = sender
                    .send(Message::Text(serde_json::to_string(&error).unwrap().into()))
                    .await;
                continue;
            }
        };

        // Handle message
        let response = match ws_msg {
            WsMessage::Ping => WsMessage::Pong,
            WsMessage::ToolRequest { id, tool, params } => {
                handle_tool_request(&state, id, tool, params).await
            }
            _ => WsMessage::Error {
                message: "Unsupported message type".to_string(),
            },
        };

        // Send response
        if let Ok(json) = serde_json::to_string(&response) {
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }

    tracing::info!("WebSocket connection ended");
}

/// Handle tool request via WebSocket
async fn handle_tool_request(
    state: &AppState,
    id: String,
    tool: String,
    params: Value,
) -> WsMessage {
    let result = match tool.as_str() {
        "grep" => {
            let options: Result<zero_core::GrepOptions, _> = serde_json::from_value(params);
            match options {
                Ok(opts) => match state.grep.search(&opts).await {
                    Ok(r) => Ok(serde_json::to_value(r).ok()),
                    Err(e) => Err(e.to_string()),
                },
                Err(e) => Err(format!("Invalid params: {}", e)),
            }
        }
        "read" => {
            let file_path = params.get("file_path").and_then(|v| v.as_str());
            match file_path {
                Some(path) => match state.reader.read(std::path::Path::new(path), None) {
                    Ok(r) => Ok(serde_json::to_value(r).ok()),
                    Err(e) => Err(e.to_string()),
                },
                None => Err("Missing file_path".to_string()),
            }
        }
        _ => Err(format!("Unknown tool: {}", tool)),
    };

    match result {
        Ok(value) => WsMessage::ToolResponse {
            id,
            success: true,
            result: value,
            error: None,
        },
        Err(error) => WsMessage::ToolResponse {
            id,
            success: false,
            result: None,
            error: Some(error),
        },
    }
}
