//! Session Management Routes
//!
//! Handles CRUD operations for conversation sessions.
//! This replaces the TypeScript session handlers in packages/ccode/src/api/server/handlers/session.ts
//!
//! Note: The underlying SessionStore uses session_key as the identifier.
//! Sessions are identified by their keys (e.g., "cli:uuid", "telegram:user123").

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::state::UnifiedApiState;
use crate::session::types::MessageRole;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub success: bool,
    pub sessions: Vec<SessionSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub message_count: usize,
    pub token_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub project_id: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub success: bool,
    pub session: SessionDetail,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub id: String,
    pub title: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub token_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
    pub project_id: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub role: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub message: SessionMessage,
}

#[derive(Debug, Deserialize)]
pub struct ForkSessionRequest {
    pub from_message_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ForkSessionResponse {
    pub success: bool,
    pub session: SessionDetail,
}

#[derive(Debug, Serialize)]
pub struct CompactSessionResponse {
    pub success: bool,
    pub deleted_count: usize,
    pub new_token_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/sessions - List sessions
pub async fn list_sessions(
    State(state): State<Arc<UnifiedApiState>>,
    Query(query): Query<ListSessionsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    match state.sessions.list_sessions_with_metadata() {
        Ok(sessions) => {
            let summaries: Vec<SessionSummary> = sessions
                .into_iter()
                .filter(|(_, _, _, meta)| {
                    // Filter by project_id if specified
                    if let Some(ref filter_project) = query.project_id {
                        meta.as_ref()
                            .and_then(|m| m.project_id.as_ref())
                            .map_or(false, |p| p == filter_project)
                    } else {
                        true
                    }
                })
                .skip(offset)
                .take(limit)
                .map(|(session_key, message_count, last_active, metadata)| {
                    let token_count = state
                        .sessions
                        .get_token_count(&session_key)
                        .unwrap_or(0);
                    let created_at = state
                        .sessions
                        .get_session_created_at(&session_key)
                        .ok()
                        .flatten()
                        .unwrap_or(last_active);

                    SessionSummary {
                        id: session_key,
                        title: metadata.as_ref().and_then(|m| m.title.clone()),
                        message_count,
                        token_count,
                        created_at,
                        updated_at: last_active,
                        project_id: metadata.and_then(|m| m.project_id),
                    }
                })
                .collect();

            let total = summaries.len();
            Json(SessionListResponse {
                success: true,
                sessions: summaries,
                total,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions - Create a new session
pub async fn create_session(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    // Generate session key
    let session_id = format!("cli:{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp();

    // Save metadata if any fields are provided
    if request.title.is_some() || request.project_id.is_some() || request.agent.is_some() {
        if let Err(e) = state.sessions.set_metadata(
            &session_id,
            request.title.as_deref(),
            request.project_id.as_deref(),
            request.agent.as_deref(),
        ) {
            tracing::warn!("Failed to save session metadata: {}", e);
        }
    }

    // Sessions are created implicitly when first message is added
    // Return the session ID for the client to use
    let session = SessionDetail {
        id: session_id,
        title: request.title,
        messages: vec![],
        token_count: 0,
        created_at: now,
        updated_at: now,
        project_id: request.project_id,
        agent: request.agent,
    };

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse {
            success: true,
            session,
        }),
    )
        .into_response()
}

/// GET /api/v1/sessions/:id - Get session details
pub async fn get_session(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.sessions.get_messages(&id) {
        Ok(messages) => {
            let token_count = state.sessions.get_token_count(&id).unwrap_or(0);
            let created_at = state
                .sessions
                .get_session_created_at(&id)
                .ok()
                .flatten()
                .unwrap_or_else(|| chrono::Utc::now().timestamp());
            let updated_at = messages
                .last()
                .map(|m| m.timestamp)
                .unwrap_or(created_at);

            // Get metadata
            let metadata = state.sessions.get_metadata(&id).ok().flatten();

            let converted_messages: Vec<SessionMessage> = messages
                .into_iter()
                .map(|m| SessionMessage {
                    id: m.id,
                    role: m.role.as_str().to_string(),
                    content: m.content,
                    timestamp: m.timestamp,
                    tool_calls: None,
                    tool_call_id: None,
                })
                .collect();

            let session = SessionDetail {
                id: id.clone(),
                title: metadata.as_ref().and_then(|m| m.title.clone()),
                messages: converted_messages,
                token_count,
                created_at,
                updated_at,
                project_id: metadata.as_ref().and_then(|m| m.project_id.clone()),
                agent: metadata.and_then(|m| m.agent),
            };

            Json(serde_json::json!({
                "success": true,
                "session": session
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// PATCH /api/v1/sessions/:id - Update session
pub async fn update_session(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateSessionRequest>,
) -> impl IntoResponse {
    // Update metadata
    if let Err(e) = state.sessions.set_metadata(
        &id,
        request.title.as_deref(),
        request.project_id.as_deref(),
        None,
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "success": true,
        "id": id,
        "message": "Session metadata updated"
    }))
    .into_response()
}

/// DELETE /api/v1/sessions/:id - Delete session
pub async fn delete_session(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Delete metadata first
    if let Err(e) = state.sessions.delete_metadata(&id) {
        tracing::warn!("Failed to delete session metadata: {}", e);
    }

    // Delete messages
    match state.sessions.clear_session(&id) {
        Ok(deleted) => Json(serde_json::json!({
            "success": true,
            "id": id,
            "deleted_messages": deleted
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/:id/messages - Get session messages
pub async fn get_session_messages(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.sessions.get_messages(&id) {
        Ok(messages) => {
            let converted: Vec<SessionMessage> = messages
                .into_iter()
                .map(|m| SessionMessage {
                    id: m.id,
                    role: m.role.as_str().to_string(),
                    content: m.content,
                    timestamp: m.timestamp,
                    tool_calls: None,
                    tool_call_id: None,
                })
                .collect();

            Json(serde_json::json!({
                "success": true,
                "messages": converted
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/:id/messages - Send a message
pub async fn send_session_message(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let role = match request.role.as_deref() {
        Some("assistant") => MessageRole::Assistant,
        Some("system") => MessageRole::System,
        _ => MessageRole::User,
    };
    let timestamp = chrono::Utc::now().timestamp();

    match state.sessions.add_message(&id, role, &request.content) {
        Ok(message_id) => {
            let message = SessionMessage {
                id: message_id,
                role: role.as_str().to_string(),
                content: request.content,
                timestamp,
                tool_calls: None,
                tool_call_id: None,
            };

            (
                StatusCode::CREATED,
                Json(SendMessageResponse {
                    success: true,
                    message,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/:id/fork - Fork a session
pub async fn fork_session(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
    Json(request): Json<ForkSessionRequest>,
) -> impl IntoResponse {
    let new_session_id = format!("cli:{}", uuid::Uuid::new_v4());

    // Get messages from original session
    match state.sessions.get_messages(&id) {
        Ok(messages) => {
            let now = chrono::Utc::now().timestamp();

            // Copy messages to new session (optionally up to a specific message)
            let messages_to_copy = if let Some(ref from_id) = request.from_message_id {
                let from_id_parsed: i64 = from_id.parse().unwrap_or(0);
                messages
                    .into_iter()
                    .take_while(|m| m.id <= from_id_parsed)
                    .collect::<Vec<_>>()
            } else {
                messages
            };

            // Add messages to new session
            for msg in &messages_to_copy {
                if let Err(e) = state
                    .sessions
                    .add_message(&new_session_id, msg.role.clone(), &msg.content)
                {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            success: false,
                            error: format!("Failed to copy message: {}", e),
                        }),
                    )
                        .into_response();
                }
            }

            let token_count = state.sessions.get_token_count(&new_session_id).unwrap_or(0);
            let converted_messages: Vec<SessionMessage> = messages_to_copy
                .into_iter()
                .map(|m| SessionMessage {
                    id: m.id,
                    role: m.role.as_str().to_string(),
                    content: m.content,
                    timestamp: m.timestamp,
                    tool_calls: None,
                    tool_call_id: None,
                })
                .collect();

            (
                StatusCode::CREATED,
                Json(ForkSessionResponse {
                    success: true,
                    session: SessionDetail {
                        id: new_session_id,
                        title: None,
                        messages: converted_messages,
                        token_count,
                        created_at: now,
                        updated_at: now,
                        project_id: None,
                        agent: None,
                    },
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/:id/compact - Compact session history
pub async fn compact_session(
    State(state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Keep 5 most recent messages by default
    match state.sessions.compact_session(&id, "", 5) {
        Ok(deleted_count) => {
            let new_token_count = state.sessions.get_token_count(&id).unwrap_or(0);
            Json(CompactSessionResponse {
                success: true,
                deleted_count,
                new_token_count,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
