//! Task Management Routes
//!
//! Handles async task management for long-running operations.
//! Tasks can be autonomous agent runs, scheduled jobs, or external integrations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Json,
};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Task Types
// ══════════════════════════════════════════════════════════════════════════════

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    AgentRun,
    Scheduled,
    External,
    Webhook,
}

/// Task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub agent: Option<String>,
    pub session_id: Option<String>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub progress: Option<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub success: bool,
    pub tasks: Vec<TaskSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub agent: Option<String>,
    pub created_at: i64,
    pub progress: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub task_type: TaskType,
    pub agent: Option<String>,
    pub session_id: Option<String>,
    pub input: serde_json::Value,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub success: bool,
    pub task: Task,
}

#[derive(Debug, Serialize)]
pub struct TaskDetailResponse {
    pub success: bool,
    pub task: Task,
}

#[derive(Debug, Deserialize)]
pub struct InteractTaskRequest {
    pub action: String,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct InteractTaskResponse {
    pub success: bool,
    pub action: String,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// In-Memory Task Store (would be replaced with persistent store)
// ══════════════════════════════════════════════════════════════════════════════

lazy_static::lazy_static! {
    static ref TASKS: RwLock<HashMap<String, Task>> = RwLock::new(HashMap::new());
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/tasks - List tasks
pub async fn list_tasks(
    State(_state): State<Arc<UnifiedApiState>>,
    Query(query): Query<ListTasksQuery>,
) -> impl IntoResponse {
    let tasks = TASKS.read().await;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let filtered: Vec<TaskSummary> = tasks
        .values()
        .filter(|t| {
            // Filter by status if specified
            if let Some(ref status) = query.status {
                let status_match = match status.as_str() {
                    "pending" => t.status == TaskStatus::Pending,
                    "running" => t.status == TaskStatus::Running,
                    "completed" => t.status == TaskStatus::Completed,
                    "failed" => t.status == TaskStatus::Failed,
                    "cancelled" => t.status == TaskStatus::Cancelled,
                    _ => true,
                };
                if !status_match {
                    return false;
                }
            }
            // Filter by agent if specified
            if let Some(ref agent) = query.agent {
                if t.agent.as_ref() != Some(agent) {
                    return false;
                }
            }
            true
        })
        .skip(offset)
        .take(limit)
        .map(|t| TaskSummary {
            id: t.id.clone(),
            task_type: t.task_type.clone(),
            status: t.status,
            agent: t.agent.clone(),
            created_at: t.created_at,
            progress: t.progress,
        })
        .collect();

    let total = filtered.len();

    Json(TaskListResponse {
        success: true,
        tasks: filtered,
        total,
    })
}

/// POST /api/v1/tasks - Create a new task
pub async fn create_task(
    State(_state): State<Arc<UnifiedApiState>>,
    Json(request): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let task_id = format!("task-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp();

    let task = Task {
        id: task_id.clone(),
        task_type: request.task_type,
        status: TaskStatus::Pending,
        agent: request.agent,
        session_id: request.session_id,
        input: request.input,
        output: None,
        error: None,
        created_at: now,
        started_at: None,
        completed_at: None,
        progress: None,
        metadata: request.metadata,
    };

    {
        let mut tasks = TASKS.write().await;
        tasks.insert(task_id.clone(), task.clone());
    }

    (
        StatusCode::CREATED,
        Json(CreateTaskResponse {
            success: true,
            task,
        }),
    )
        .into_response()
}

/// GET /api/v1/tasks/:id - Get task details
pub async fn get_task(
    State(_state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let tasks = TASKS.read().await;

    match tasks.get(&id) {
        Some(task) => Json(TaskDetailResponse {
            success: true,
            task: task.clone(),
        })
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Task not found: {}", id),
            }),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/tasks/:id - Delete/cancel a task
pub async fn delete_task(
    State(_state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut tasks = TASKS.write().await;

    match tasks.remove(&id) {
        Some(_) => Json(serde_json::json!({
            "success": true,
            "id": id
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Task not found: {}", id),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/tasks/:id/events - Stream task events (SSE)
pub async fn stream_task_events(
    State(_state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        let mut last_status = None;

        loop {
            let task = {
                let tasks = TASKS.read().await;
                tasks.get(&id).cloned()
            };

            match task {
                Some(t) => {
                    // Send update if status changed
                    if last_status != Some(t.status) {
                        last_status = Some(t.status);

                        let event_data = serde_json::json!({
                            "id": t.id,
                            "status": t.status,
                            "progress": t.progress,
                            "error": t.error,
                        });

                        yield Ok::<_, std::convert::Infallible>(Event::default()
                            .data(serde_json::to_string(&event_data).unwrap_or_default())
                            .event("task"));
                    }

                    // Stop if task is done
                    if matches!(t.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled) {
                        break;
                    }
                }
                None => {
                    yield Ok::<_, std::convert::Infallible>(Event::default()
                        .data(r#"{"error":"Task not found"}"#)
                        .event("error"));
                    break;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    };

    Sse::new(stream)
}

/// POST /api/v1/tasks/:id/interact - Interact with a running task
pub async fn interact_task(
    State(_state): State<Arc<UnifiedApiState>>,
    Path(id): Path<String>,
    Json(request): Json<InteractTaskRequest>,
) -> impl IntoResponse {
    let mut tasks = TASKS.write().await;

    match tasks.get_mut(&id) {
        Some(task) => {
            match request.action.as_str() {
                "cancel" => {
                    task.status = TaskStatus::Cancelled;
                    task.completed_at = Some(chrono::Utc::now().timestamp());
                    Json(InteractTaskResponse {
                        success: true,
                        action: request.action,
                        result: Some(serde_json::json!({"cancelled": true})),
                    })
                    .into_response()
                }
                "pause" | "resume" | "input" => {
                    // These would be implemented for interactive tasks
                    Json(InteractTaskResponse {
                        success: true,
                        action: request.action,
                        result: request.input,
                    })
                    .into_response()
                }
                _ => (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        success: false,
                        error: format!("Unknown action: {}", request.action),
                    }),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Task not found: {}", id),
            }),
        )
            .into_response(),
    }
}
