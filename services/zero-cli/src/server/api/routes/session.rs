//! Session API endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::state::AppState;

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

/// Get current session list
pub async fn get_session(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<SessionInfo>>> {
    // TODO: Implement actual session storage
    Json(ApiResponse::ok(vec![]))
}

/// Create a new session
pub async fn create_session(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<SessionInfo>>, (StatusCode, Json<ApiResponse<SessionInfo>>)> {
    let session = SessionInfo {
        id: Uuid::new_v4().to_string(),
        title: request.title,
        created_at: chrono::Utc::now().to_rfc3339(),
        message_count: 0,
    };

    // TODO: Store session in database

    Ok(Json(ApiResponse::ok(session)))
}

/// Get a session by ID
pub async fn get_session_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SessionInfo>>, (StatusCode, Json<ApiResponse<SessionInfo>>)> {
    // TODO: Implement actual session retrieval
    let session = SessionInfo {
        id,
        title: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        message_count: 0,
    };

    Ok(Json(ApiResponse::ok(session)))
}

/// Get messages for a session
pub async fn get_messages(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<MessageInfo>>> {
    // TODO: Implement actual message retrieval
    Json(ApiResponse::ok(vec![]))
}

/// Add a message to a session
pub async fn add_message(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<AddMessageRequest>,
) -> Result<Json<ApiResponse<MessageInfo>>, (StatusCode, Json<ApiResponse<MessageInfo>>)> {
    let message = MessageInfo {
        id: Uuid::new_v4().to_string(),
        role: request.role,
        content: request.content,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // TODO: Store message in database

    Ok(Json(ApiResponse::ok(message)))
}
