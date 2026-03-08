//! Session API endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::state::AppState;
use crate::session::types::MessageRole;

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub message_count: usize,
}

/// Create session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub parent_id: Option<String>,
}

/// Message information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Add message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub role: String,
    pub content: String,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Convert Unix timestamp to RFC3339 string
fn timestamp_to_rfc3339(timestamp: i64) -> String {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

/// Get current session list
pub async fn get_session(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<SessionInfo>>> {
    match state.session_store.list_sessions() {
        Ok(sessions) => {
            let session_infos: Vec<SessionInfo> = sessions
                .into_iter()
                .map(|(session_key, msg_count, last_active)| SessionInfo {
                    id: session_key,
                    title: None, // No title storage in current schema
                    created_at: timestamp_to_rfc3339(last_active),
                    message_count: msg_count,
                })
                .collect();
            Json(ApiResponse::ok(session_infos))
        }
        Err(e) => Json(ApiResponse::err(format!("Failed to list sessions: {e}"))),
    }
}

/// Create a new session
pub async fn create_session(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<SessionInfo>>, (StatusCode, Json<ApiResponse<SessionInfo>>)> {
    // Generate a new session ID (UUID format)
    // The session is lazily created in the database when the first message is added
    let session_id = Uuid::new_v4().to_string();

    let session = SessionInfo {
        id: session_id,
        title: request.title,
        created_at: chrono::Utc::now().to_rfc3339(),
        message_count: 0,
    };

    Ok(Json(ApiResponse::ok(session)))
}

/// Get a session by ID
pub async fn get_session_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SessionInfo>>, (StatusCode, Json<ApiResponse<SessionInfo>>)> {
    // Check if session exists by getting message count
    match state.session_store.get_message_count(&id) {
        Ok(count) => {
            if count == 0 {
                // Session doesn't exist (no messages)
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::err(format!("Session not found: {id}"))),
                ));
            }

            // Get creation timestamp
            let created_at = state
                .session_store
                .get_session_created_at(&id)
                .ok()
                .flatten()
                .map(timestamp_to_rfc3339)
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            let session = SessionInfo {
                id,
                title: None,
                created_at,
                message_count: count,
            };

            Ok(Json(ApiResponse::ok(session)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to get session: {e}"))),
        )),
    }
}

/// Get messages for a session
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<MessageInfo>>> {
    match state.session_store.get_messages(&id) {
        Ok(messages) => {
            let message_infos: Vec<MessageInfo> = messages
                .into_iter()
                .map(|msg| MessageInfo {
                    id: msg.id.to_string(),
                    role: msg.role.as_str().to_string(),
                    content: msg.content,
                    created_at: timestamp_to_rfc3339(msg.timestamp),
                })
                .collect();
            Json(ApiResponse::ok(message_infos))
        }
        Err(e) => Json(ApiResponse::err(format!("Failed to get messages: {e}"))),
    }
}

/// Add a message to a session
pub async fn add_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<AddMessageRequest>,
) -> Result<Json<ApiResponse<MessageInfo>>, (StatusCode, Json<ApiResponse<MessageInfo>>)> {
    let role = MessageRole::parse(&request.role);

    match state.session_store.add_message(&id, role, &request.content) {
        Ok(msg_id) => {
            let message = MessageInfo {
                id: msg_id.to_string(),
                role: role.as_str().to_string(),
                content: request.content,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            Ok(Json(ApiResponse::ok(message)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::err(format!("Failed to add message: {e}"))),
        )),
    }
}
