//! HTTP routes for Zero Workflow management.
//!
//! Provides REST API endpoints for:
//! - Workflow CRUD operations
//! - Cron task management
//! - Execution monitoring

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::scheduler::{Scheduler, TaskInfo};
use crate::workflow::{ExecutionStatus, Workflow, WorkflowExecutor, WorkflowResult};
use std::collections::HashMap;
use zero_common::config::CronTask;

// ============================================================================
// State
// ============================================================================

/// Shared state for the workflow HTTP server.
pub struct WorkflowState {
    /// Cron scheduler
    pub scheduler: Arc<Scheduler>,
    /// Workflow executor
    pub executor: Arc<WorkflowExecutor>,
    /// Registered workflows
    pub workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    /// Execution history
    pub executions: Arc<RwLock<Vec<WorkflowResult>>>,
    /// CodeCoder API endpoint
    pub codecoder_endpoint: String,
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    #[allow(dead_code)]
    fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ============================================================================
// Health Routes
// ============================================================================

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy",
        service: "zero-workflow",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ready() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ready",
            service: "zero-workflow",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

// ============================================================================
// Cron Task Routes
// ============================================================================

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    id: String,
    expression: String,
    command: String,
    description: Option<String>,
}

async fn list_tasks(
    State(state): State<Arc<WorkflowState>>,
) -> Json<ApiResponse<Vec<TaskInfo>>> {
    match state.scheduler.list_tasks() {
        Ok(tasks) => Json(ApiResponse::success(tasks)),
        Err(e) => {
            tracing::error!("Failed to list tasks: {}", e);
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })
        }
    }
}

async fn create_task(
    State(state): State<Arc<WorkflowState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let task = CronTask {
        id: req.id.clone(),
        expression: req.expression,
        command: req.command,
        description: req.description,
    };

    match state.scheduler.add_task(task) {
        Ok(()) => {
            tracing::info!(task_id = %req.id, "Created cron task");
            Ok(Json(ApiResponse::success(req.id)))
        }
        Err(e) => {
            tracing::error!("Failed to create task: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn delete_task(
    State(state): State<Arc<WorkflowState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    match state.scheduler.remove_task(&id) {
        Ok(removed) => {
            if removed {
                tracing::info!(task_id = %id, "Deleted cron task");
            }
            Ok(Json(ApiResponse::success(removed)))
        }
        Err(e) => {
            tracing::error!("Failed to delete task: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

// ============================================================================
// Workflow Routes
// ============================================================================

async fn list_workflows(
    State(state): State<Arc<WorkflowState>>,
) -> Json<ApiResponse<Vec<WorkflowSummary>>> {
    let workflows = state.workflows.read().await;
    let summaries: Vec<WorkflowSummary> = workflows
        .values()
        .map(|w| WorkflowSummary {
            name: w.name.clone(),
            description: w.description.clone(),
            trigger_type: match &w.trigger {
                crate::workflow::Trigger::Webhook { .. } => "webhook".into(),
                crate::workflow::Trigger::Cron { .. } => "cron".into(),
                crate::workflow::Trigger::Manual => "manual".into(),
            },
            steps_count: w.steps.len(),
        })
        .collect();
    Json(ApiResponse::success(summaries))
}

#[derive(Debug, Serialize)]
struct WorkflowSummary {
    name: String,
    description: Option<String>,
    trigger_type: String,
    steps_count: usize,
}

async fn get_workflow(
    State(state): State<Arc<WorkflowState>>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Workflow>>, StatusCode> {
    let workflows = state.workflows.read().await;
    match workflows.get(&name) {
        Some(workflow) => Ok(Json(ApiResponse::success(workflow.clone()))),
        None => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Workflow '{}' not found", name)),
        })),
    }
}

async fn create_workflow(
    State(state): State<Arc<WorkflowState>>,
    Json(workflow): Json<Workflow>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let name = workflow.name.clone();
    let mut workflows = state.workflows.write().await;

    if workflows.contains_key(&name) {
        return Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Workflow '{}' already exists", name)),
        }));
    }

    workflows.insert(name.clone(), workflow);
    tracing::info!(workflow = %name, "Created workflow");
    Ok(Json(ApiResponse::success(name)))
}

async fn update_workflow(
    State(state): State<Arc<WorkflowState>>,
    Path(name): Path<String>,
    Json(workflow): Json<Workflow>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let mut workflows = state.workflows.write().await;

    if !workflows.contains_key(&name) {
        return Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Workflow '{}' not found", name)),
        }));
    }

    workflows.insert(name.clone(), workflow);
    tracing::info!(workflow = %name, "Updated workflow");
    Ok(Json(ApiResponse::success(name)))
}

async fn delete_workflow(
    State(state): State<Arc<WorkflowState>>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let mut workflows = state.workflows.write().await;
    let removed = workflows.remove(&name).is_some();
    if removed {
        tracing::info!(workflow = %name, "Deleted workflow");
    }
    Ok(Json(ApiResponse::success(removed)))
}

// ============================================================================
// Execution Routes
// ============================================================================

#[derive(Debug, Deserialize)]
struct ExecuteWorkflowRequest {
    #[serde(default)]
    context: serde_json::Value,
}

async fn execute_workflow(
    State(state): State<Arc<WorkflowState>>,
    Path(name): Path<String>,
    Json(req): Json<ExecuteWorkflowRequest>,
) -> Result<Json<ApiResponse<WorkflowResult>>, StatusCode> {
    let workflows = state.workflows.read().await;
    let workflow = match workflows.get(&name) {
        Some(w) => w.clone(),
        None => {
            return Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Workflow '{}' not found", name)),
            }));
        }
    };
    drop(workflows);

    match state.executor.execute(&workflow, req.context).await {
        Ok(result) => {
            // Store execution result
            let mut executions = state.executions.write().await;
            executions.push(result.clone());
            // Keep only last 100 executions
            if executions.len() > 100 {
                executions.remove(0);
            }

            Ok(Json(ApiResponse::success(result)))
        }
        Err(e) => {
            tracing::error!(workflow = %name, error = %e, "Workflow execution failed");
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn list_executions(
    State(state): State<Arc<WorkflowState>>,
) -> Json<ApiResponse<Vec<ExecutionSummary>>> {
    let executions = state.executions.read().await;
    let summaries: Vec<ExecutionSummary> = executions
        .iter()
        .rev()
        .take(50)
        .map(|e| ExecutionSummary {
            execution_id: e.execution_id.clone(),
            workflow: e.workflow.clone(),
            status: e.status,
            started_at: e.started_at,
            ended_at: e.ended_at,
        })
        .collect();
    Json(ApiResponse::success(summaries))
}

#[derive(Debug, Serialize)]
struct ExecutionSummary {
    execution_id: String,
    workflow: String,
    status: ExecutionStatus,
    started_at: i64,
    ended_at: Option<i64>,
}

async fn get_execution(
    State(state): State<Arc<WorkflowState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<WorkflowResult>>, StatusCode> {
    let executions = state.executions.read().await;
    match executions.iter().find(|e| e.execution_id == id) {
        Some(execution) => Ok(Json(ApiResponse::success(execution.clone()))),
        None => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Execution '{}' not found", id)),
        })),
    }
}

// ============================================================================
// Router Builder
// ============================================================================

/// Build the workflow HTTP router.
pub fn build_router(state: Arc<WorkflowState>) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health))
        .route("/ready", get(ready))
        // Cron task endpoints
        .route("/api/v1/tasks", get(list_tasks))
        .route("/api/v1/tasks", post(create_task))
        .route("/api/v1/tasks/:id", delete(delete_task))
        // Workflow endpoints
        .route("/api/v1/workflows", get(list_workflows))
        .route("/api/v1/workflows", post(create_workflow))
        .route("/api/v1/workflows/:name", get(get_workflow))
        .route("/api/v1/workflows/:name", put(update_workflow))
        .route("/api/v1/workflows/:name", delete(delete_workflow))
        // Execution endpoints
        .route("/api/v1/workflows/:name/execute", post(execute_workflow))
        .route("/api/v1/executions", get(list_executions))
        .route("/api/v1/executions/:id", get(get_execution))
        // Add state
        .with_state(state)
}

/// Create the workflow state.
pub fn create_state(codecoder_endpoint: String) -> Arc<WorkflowState> {
    Arc::new(WorkflowState {
        scheduler: Arc::new(Scheduler::new()),
        executor: Arc::new(WorkflowExecutor::new()),
        workflows: Arc::new(RwLock::new(HashMap::new())),
        executions: Arc::new(RwLock::new(Vec::new())),
        codecoder_endpoint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn create_test_state() -> Arc<WorkflowState> {
        create_state("http://localhost:4400".to_string())
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let state = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp: ApiResponse<Vec<TaskInfo>> = serde_json::from_slice(&body).unwrap();
        assert!(resp.success);
        assert!(resp.data.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_task() {
        let state = create_test_state();
        let app = build_router(state);

        let payload = serde_json::json!({
            "id": "test-task",
            "expression": "0 0 * * * *",
            "command": "echo hello",
            "description": "Test task"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp: ApiResponse<String> = serde_json::from_slice(&body).unwrap();
        assert!(resp.success);
        assert_eq!(resp.data.unwrap(), "test-task");
    }

    #[tokio::test]
    async fn test_list_workflows_empty() {
        let state = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/workflows")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_workflow() {
        let state = create_test_state();
        let app = build_router(state);

        let payload = serde_json::json!({
            "name": "test-workflow",
            "description": "A test workflow",
            "trigger": {
                "type": "manual"
            },
            "steps": [
                {
                    "name": "step1",
                    "type": "shell",
                    "command": "echo hello"
                }
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/workflows")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp: ApiResponse<String> = serde_json::from_slice(&body).unwrap();
        assert!(resp.success);
        assert_eq!(resp.data.unwrap(), "test-workflow");
    }
}
