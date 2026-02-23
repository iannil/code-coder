//! HTTP routes for Zero Workflow management.
//!
//! Provides REST API endpoints for:
//! - Workflow CRUD operations
//! - Cron task management
//! - Execution monitoring
//! - Competitive intelligence monitoring

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::monitor_bridge::{MonitorBridge, MonitorReport, MonitorRunResult};
use crate::scheduler::{Scheduler, TaskInfo};
use crate::workflow::{ExecutionStatus, Workflow, WorkflowExecutor, WorkflowResult};
use std::collections::HashMap;
use zero_common::config::{CronTask, MonitorTask};

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
    /// Monitor bridge
    pub monitor_bridge: Arc<MonitorBridge>,
    /// Monitor tasks configuration
    pub monitor_tasks: Arc<RwLock<HashMap<String, MonitorTask>>>,
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
// Monitor Routes
// ============================================================================

/// Query parameters for listing reports.
#[derive(Debug, Deserialize)]
struct ListReportsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

/// Response for monitor status.
#[derive(Debug, Serialize, Deserialize)]
struct MonitorStatus {
    enabled: bool,
    tasks_count: usize,
    tasks: Vec<MonitorTaskSummary>,
}

/// Summary of a monitor task.
#[derive(Debug, Serialize, Deserialize)]
struct MonitorTaskSummary {
    id: String,
    name: String,
    schedule: String,
    sources_count: usize,
    channel_type: String,
}

/// Get monitor status.
async fn get_monitor_status(
    State(state): State<Arc<WorkflowState>>,
) -> Json<ApiResponse<MonitorStatus>> {
    let tasks = state.monitor_tasks.read().await;

    let task_summaries: Vec<MonitorTaskSummary> = tasks
        .values()
        .map(|t| MonitorTaskSummary {
            id: t.id.clone(),
            name: t.name.clone(),
            schedule: t.schedule.clone(),
            sources_count: t.sources.len(),
            channel_type: t.notification.channel_type.clone(),
        })
        .collect();

    Json(ApiResponse::success(MonitorStatus {
        enabled: !tasks.is_empty(),
        tasks_count: tasks.len(),
        tasks: task_summaries,
    }))
}

/// List reports for a specific task.
async fn list_task_reports(
    State(state): State<Arc<WorkflowState>>,
    Path(task_id): Path<String>,
    Query(query): Query<ListReportsQuery>,
) -> Json<ApiResponse<Vec<MonitorReport>>> {
    let reports = state.monitor_bridge.get_reports(&task_id, query.limit).await;
    Json(ApiResponse::success(reports))
}

/// List all recent reports.
async fn list_all_reports(
    State(state): State<Arc<WorkflowState>>,
    Query(query): Query<ListReportsQuery>,
) -> Json<ApiResponse<Vec<MonitorReport>>> {
    let reports = state.monitor_bridge.get_all_reports(query.limit).await;
    Json(ApiResponse::success(reports))
}

/// Response for running a monitor task.
#[derive(Debug, Serialize)]
struct RunMonitorResponse {
    task_id: String,
    status: String,
    report_id: Option<String>,
    message: String,
}

/// Manually trigger a monitor task.
async fn run_monitor_task(
    State(state): State<Arc<WorkflowState>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<RunMonitorResponse>> {
    let tasks = state.monitor_tasks.read().await;

    let task = match tasks.get(&task_id) {
        Some(t) => t.clone(),
        None => {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Monitor task '{}' not found", task_id)),
            });
        }
    };
    drop(tasks);

    tracing::info!(task_id = %task_id, "Manually triggering monitor task");

    match state.monitor_bridge.run_monitor(&task).await {
        Ok(result) => {
            let (status, report_id, message) = match result {
                MonitorRunResult::Success {
                    report,
                    notification_sent,
                } => (
                    "success".to_string(),
                    Some(report.id),
                    format!(
                        "Report generated successfully. Notification sent: {}",
                        notification_sent
                    ),
                ),
                MonitorRunResult::Partial {
                    report,
                    failed_sources,
                    notification_sent,
                } => (
                    "partial".to_string(),
                    Some(report.id),
                    format!(
                        "Report generated with {} failed sources: {}. Notification sent: {}",
                        failed_sources.len(),
                        failed_sources.join(", "),
                        notification_sent
                    ),
                ),
                MonitorRunResult::Failed { reason } => {
                    ("failed".to_string(), None, format!("Monitor failed: {}", reason))
                }
            };

            Json(ApiResponse::success(RunMonitorResponse {
                task_id,
                status,
                report_id,
                message,
            }))
        }
        Err(e) => {
            tracing::error!(task_id = %task_id, error = %e, "Monitor task failed");
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Request to create/update a monitor task.
#[derive(Debug, Deserialize)]
struct CreateMonitorTaskRequest {
    id: String,
    name: String,
    schedule: String,
    sources: Vec<zero_common::config::MonitorSourceConfig>,
    notification: zero_common::config::MonitorNotificationConfig,
}

/// Create a new monitor task.
async fn create_monitor_task(
    State(state): State<Arc<WorkflowState>>,
    Json(req): Json<CreateMonitorTaskRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let task = MonitorTask {
        id: req.id.clone(),
        name: req.name,
        schedule: req.schedule,
        sources: req.sources,
        notification: req.notification,
    };

    let mut tasks = state.monitor_tasks.write().await;
    tasks.insert(req.id.clone(), task);

    tracing::info!(task_id = %req.id, "Created monitor task");
    Ok(Json(ApiResponse::success(req.id)))
}

/// Delete a monitor task.
async fn delete_monitor_task(
    State(state): State<Arc<WorkflowState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let mut tasks = state.monitor_tasks.write().await;
    let removed = tasks.remove(&task_id).is_some();

    if removed {
        tracing::info!(task_id = %task_id, "Deleted monitor task");
    }

    Ok(Json(ApiResponse::success(removed)))
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
        // Monitor endpoints
        .route("/api/v1/monitor/status", get(get_monitor_status))
        .route("/api/v1/monitor/reports", get(list_all_reports))
        .route("/api/v1/monitor/tasks", post(create_monitor_task))
        .route("/api/v1/monitor/:task_id/run", post(run_monitor_task))
        .route("/api/v1/monitor/:task_id/reports", get(list_task_reports))
        .route("/api/v1/monitor/:task_id", delete(delete_monitor_task))
        // Add state
        .with_state(state)
}

/// Create the workflow state.
pub fn create_state(codecoder_endpoint: String) -> Arc<WorkflowState> {
    let monitor_bridge = MonitorBridge::new(&codecoder_endpoint);

    Arc::new(WorkflowState {
        scheduler: Arc::new(Scheduler::new()),
        executor: Arc::new(WorkflowExecutor::new()),
        workflows: Arc::new(RwLock::new(HashMap::new())),
        executions: Arc::new(RwLock::new(Vec::new())),
        codecoder_endpoint,
        monitor_bridge: Arc::new(monitor_bridge),
        monitor_tasks: Arc::new(RwLock::new(HashMap::new())),
    })
}

/// Create workflow state with channels endpoint for IM notifications.
pub fn create_state_with_channels(
    codecoder_endpoint: String,
    channels_endpoint: String,
) -> Arc<WorkflowState> {
    let monitor_bridge = MonitorBridge::new(&codecoder_endpoint)
        .with_channels_endpoint(channels_endpoint);

    Arc::new(WorkflowState {
        scheduler: Arc::new(Scheduler::new()),
        executor: Arc::new(WorkflowExecutor::new()),
        workflows: Arc::new(RwLock::new(HashMap::new())),
        executions: Arc::new(RwLock::new(Vec::new())),
        codecoder_endpoint,
        monitor_bridge: Arc::new(monitor_bridge),
        monitor_tasks: Arc::new(RwLock::new(HashMap::new())),
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
        // Use a unique temp directory for each test
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let scheduler = crate::scheduler::Scheduler::with_data_dir(tmp.path().to_path_buf());
        let monitor_bridge = MonitorBridge::new("http://localhost:4400");

        // Keep tmp alive by leaking it (tests are short-lived)
        std::mem::forget(tmp);

        Arc::new(WorkflowState {
            scheduler: Arc::new(scheduler),
            executor: Arc::new(WorkflowExecutor::new()),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            codecoder_endpoint: "http://localhost:4400".to_string(),
            monitor_bridge: Arc::new(monitor_bridge),
            monitor_tasks: Arc::new(RwLock::new(HashMap::new())),
        })
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

    #[tokio::test]
    async fn test_monitor_status_empty() {
        let state = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/monitor/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp: ApiResponse<MonitorStatus> = serde_json::from_slice(&body).unwrap();
        assert!(resp.success);
        let status = resp.data.unwrap();
        assert!(!status.enabled);
        assert_eq!(status.tasks_count, 0);
    }

    #[tokio::test]
    async fn test_create_monitor_task() {
        let state = create_test_state();
        let app = build_router(state);

        let payload = serde_json::json!({
            "id": "test-monitor",
            "name": "Test Monitor",
            "schedule": "0 0 9 * * *",
            "sources": [
                {
                    "id": "src-1",
                    "name": "Test Source",
                    "url": "https://example.com",
                    "source_type": "website"
                }
            ],
            "notification": {
                "channel_type": "feishu",
                "channel_id": "test-group"
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/monitor/tasks")
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
        assert_eq!(resp.data.unwrap(), "test-monitor");
    }

    #[tokio::test]
    async fn test_list_monitor_reports_empty() {
        let state = create_test_state();
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/monitor/reports")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let resp: ApiResponse<Vec<MonitorReport>> = serde_json::from_slice(&body).unwrap();
        assert!(resp.success);
        assert!(resp.data.unwrap().is_empty());
    }
}
