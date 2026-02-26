//! Integration tests for Zero Workflow.
//!
//! Tests the HTTP API endpoints and webhook handlers.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::Arc;
use tower::ServiceExt;
use zero_workflow::{
    build_router, create_isolated_test_state, webhook_routes, WebhookState,
};

type HmacSha256 = Hmac<Sha256>;

/// Create a test app with workflow routes using isolated state.
fn create_test_app() -> axum::Router {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    build_router(state)
}

/// Create a test app with webhook routes.
fn create_webhook_app(secret: Option<&str>, github_secret: Option<&str>, gitlab_token: Option<&str>) -> axum::Router {
    let state = WebhookState::new(
        secret.map(|s| Arc::new(s.to_string())),
        github_secret.map(|s| Arc::new(s.to_string())),
        gitlab_token.map(|s| Arc::new(s.to_string())),
    );
    webhook_routes(state)
}

/// Helper to make a JSON request.
async fn request_json(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = if let Some(b) = body {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&b).unwrap()))
            .unwrap()
    } else {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

// ─────────────────────────────────────────────────────────────────────────────
// Health Check Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "zero-workflow");
}

#[tokio::test]
async fn test_ready_check() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/ready", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ready");
}

// ─────────────────────────────────────────────────────────────────────────────
// Cron Task Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_tasks_initially_empty() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/tasks", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert!(json["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_and_list_task() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    // Create a task
    let payload = json!({
        "id": "test-task-1",
        "expression": "0 0 * * * *",
        "command": "echo hello",
        "description": "Test cron task"
    });

    let (status, json) = request_json(&app, Method::POST, "/api/v1/tasks", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], "test-task-1");

    // List tasks
    let (status, json) = request_json(&app, Method::GET, "/api/v1/tasks", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    let tasks = json["data"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["id"], "test-task-1");
}

#[tokio::test]
async fn test_delete_task() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    // Create a task
    let payload = json!({
        "id": "task-to-delete",
        "expression": "0 0 * * * *",
        "command": "echo hello"
    });
    let (status, _) = request_json(&app, Method::POST, "/api/v1/tasks", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);

    // Delete the task
    let (status, json) = request_json(&app, Method::DELETE, "/api/v1/tasks/task-to-delete", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], true);

    // Verify it's deleted
    let (status, json) = request_json(&app, Method::GET, "/api/v1/tasks", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_delete_nonexistent_task() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::DELETE, "/api/v1/tasks/nonexistent", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], false); // Not found returns false
}

// ─────────────────────────────────────────────────────────────────────────────
// Workflow Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_workflows_initially_empty() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/workflows", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert!(json["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_and_get_workflow() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    // Create workflow
    let payload = json!({
        "name": "test-workflow",
        "description": "A test workflow",
        "trigger": { "type": "manual" },
        "steps": [
            {
                "name": "step1",
                "type": "shell",
                "command": "echo hello"
            }
        ]
    });

    let (status, json) = request_json(&app, Method::POST, "/api/v1/workflows", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], "test-workflow");

    // Get workflow
    let (status, json) = request_json(&app, Method::GET, "/api/v1/workflows/test-workflow", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["name"], "test-workflow");
}

#[tokio::test]
async fn test_create_duplicate_workflow() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    let payload = json!({
        "name": "duplicate-workflow",
        "trigger": { "type": "manual" },
        "steps": []
    });

    // First create
    let (status, _) = request_json(&app, Method::POST, "/api/v1/workflows", Some(payload.clone())).await;
    assert_eq!(status, StatusCode::OK);

    // Second create (should fail)
    let (status, json) = request_json(&app, Method::POST, "/api/v1/workflows", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("already exists"));
}

#[tokio::test]
async fn test_update_workflow() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    // Create workflow
    let payload = json!({
        "name": "updatable-workflow",
        "description": "Original description",
        "trigger": { "type": "manual" },
        "steps": []
    });
    let (status, _) = request_json(&app, Method::POST, "/api/v1/workflows", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);

    // Update workflow
    let payload = json!({
        "name": "updatable-workflow",
        "description": "Updated description",
        "trigger": { "type": "manual" },
        "steps": [{ "name": "new-step", "type": "shell", "command": "echo updated" }]
    });
    let (status, json) = request_json(&app, Method::PUT, "/api/v1/workflows/updatable-workflow", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);

    // Verify update
    let (status, json) = request_json(&app, Method::GET, "/api/v1/workflows/updatable-workflow", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["data"]["description"], "Updated description");
}

#[tokio::test]
async fn test_delete_workflow() {
    let state = create_isolated_test_state("http://localhost:4400".to_string());
    let app = build_router(state);

    // Create workflow
    let payload = json!({
        "name": "deletable-workflow",
        "trigger": { "type": "manual" },
        "steps": []
    });
    let (status, _) = request_json(&app, Method::POST, "/api/v1/workflows", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);

    // Delete workflow
    let (status, json) = request_json(&app, Method::DELETE, "/api/v1/workflows/deletable-workflow", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], true);

    // Verify deletion
    let (status, json) = request_json(&app, Method::GET, "/api/v1/workflows/deletable-workflow", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], false);
}

#[tokio::test]
async fn test_get_nonexistent_workflow() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/workflows/nonexistent", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Webhook Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_generic_webhook_no_auth() {
    let app = create_webhook_app(None, None, None);

    let payload = json!({
        "event": "test",
        "data": { "key": "value" }
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_generic_webhook_with_signature() {
    let secret = "test-secret";
    let app = create_webhook_app(Some(secret), None, None);

    let payload = json!({ "event": "test" });
    let body = serde_json::to_string(&payload).unwrap();

    // Generate signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Webhook-Signature", signature)
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_generic_webhook_invalid_signature() {
    let app = create_webhook_app(Some("test-secret"), None, None);

    let payload = json!({ "event": "test" });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Webhook-Signature", "invalid-signature")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_generic_webhook_missing_signature() {
    let app = create_webhook_app(Some("test-secret"), None, None);

    let payload = json!({ "event": "test" });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_github_webhook_no_auth() {
    let app = create_webhook_app(None, None, None);

    let payload = json!({
        "action": "opened",
        "pull_request": { "number": 1 }
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook/github")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-GitHub-Event", "pull_request")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["message"].as_str().unwrap().contains("pull_request"));
}

#[tokio::test]
async fn test_github_webhook_with_signature() {
    let secret = "github-secret";
    let app = create_webhook_app(None, Some(secret), None);

    let payload = json!({ "action": "push" });
    let body = serde_json::to_string(&payload).unwrap();

    // Generate GitHub-style signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook/github")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-GitHub-Event", "push")
        .header("X-Hub-Signature-256", signature)
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_gitlab_webhook_with_token() {
    let token = "gitlab-token";
    let app = create_webhook_app(None, None, Some(token));

    let payload = json!({
        "object_kind": "merge_request"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook/gitlab")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Gitlab-Event", "Merge Request Hook")
        .header("X-Gitlab-Token", token)
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_gitlab_webhook_invalid_token() {
    let app = create_webhook_app(None, None, Some("correct-token"));

    let payload = json!({});

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook/gitlab")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Gitlab-Token", "wrong-token")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ─────────────────────────────────────────────────────────────────────────────
// Execution Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_executions_initially_empty() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/executions", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert!(json["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_execute_nonexistent_workflow() {
    let app = create_test_app();

    let payload = json!({ "context": {} });
    let (status, json) = request_json(&app, Method::POST, "/api/v1/workflows/nonexistent/execute", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_get_nonexistent_execution() {
    let app = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/executions/nonexistent-id", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// GitHub PR Event Parsing Tests
// ─────────────────────────────────────────────────────────────────────────────

use zero_workflow::PullRequestEvent;

#[test]
fn test_github_pr_event_parsing() {
    let json = r#"{
        "action": "opened",
        "number": 42,
        "pull_request": {
            "id": 12345,
            "number": 42,
            "title": "Add new feature",
            "body": "This PR adds an important feature",
            "state": "open",
            "draft": false,
            "head": {
                "ref": "feature-branch",
                "sha": "abc123def456"
            },
            "base": {
                "ref": "main",
                "sha": "789xyz000"
            },
            "html_url": "https://github.com/owner/repo/pull/42",
            "diff_url": "https://github.com/owner/repo/pull/42.diff",
            "changed_files": 5,
            "additions": 100,
            "deletions": 20
        },
        "repository": {
            "id": 98765,
            "name": "repo",
            "full_name": "owner/repo",
            "clone_url": "https://github.com/owner/repo.git",
            "default_branch": "main"
        },
        "sender": {
            "id": 11111,
            "login": "developer"
        }
    }"#;

    let event: PullRequestEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event.action, "opened");
    assert_eq!(event.number, 42);
    assert_eq!(event.pull_request.title, "Add new feature");
    assert!(event.should_review());
    assert_eq!(event.repo_full_name(), "owner/repo");
    assert_eq!(event.pr_number(), 42);
}

#[test]
fn test_github_pr_draft_should_not_review() {
    let json = r#"{
        "action": "opened",
        "number": 1,
        "pull_request": {
            "id": 1,
            "number": 1,
            "title": "Draft PR",
            "state": "open",
            "draft": true,
            "head": { "ref": "feature", "sha": "abc" },
            "base": { "ref": "main", "sha": "def" },
            "html_url": "https://github.com/o/r/pull/1",
            "diff_url": "https://github.com/o/r/pull/1.diff"
        },
        "repository": {
            "id": 1,
            "name": "repo",
            "full_name": "owner/repo",
            "clone_url": "https://github.com/owner/repo.git"
        },
        "sender": { "id": 1, "login": "user" }
    }"#;

    let event: PullRequestEvent = serde_json::from_str(json).unwrap();
    assert!(!event.should_review());
}

#[test]
fn test_github_pr_closed_should_not_review() {
    let json = r#"{
        "action": "closed",
        "number": 1,
        "pull_request": {
            "id": 1,
            "number": 1,
            "title": "Closed PR",
            "state": "closed",
            "draft": false,
            "head": { "ref": "feature", "sha": "abc" },
            "base": { "ref": "main", "sha": "def" },
            "html_url": "https://github.com/o/r/pull/1",
            "diff_url": "https://github.com/o/r/pull/1.diff"
        },
        "repository": {
            "id": 1,
            "name": "repo",
            "full_name": "owner/repo",
            "clone_url": "https://github.com/owner/repo.git"
        },
        "sender": { "id": 1, "login": "user" }
    }"#;

    let event: PullRequestEvent = serde_json::from_str(json).unwrap();
    assert!(!event.should_review());
}

// ─────────────────────────────────────────────────────────────────────────────
// GitLab MR Event Parsing Tests
// ─────────────────────────────────────────────────────────────────────────────

use zero_workflow::MergeRequestEvent;

#[test]
fn test_gitlab_mr_event_parsing() {
    let json = r#"{
        "object_kind": "merge_request",
        "user": {
            "id": 1,
            "username": "developer",
            "name": "Developer Name"
        },
        "project": {
            "id": 123,
            "name": "project",
            "path_with_namespace": "group/project",
            "web_url": "https://gitlab.com/group/project",
            "git_http_url": "https://gitlab.com/group/project.git"
        },
        "object_attributes": {
            "id": 456,
            "iid": 7,
            "title": "New feature MR",
            "description": "This MR adds new functionality",
            "state": "opened",
            "draft": false,
            "source_branch": "feature",
            "target_branch": "main",
            "source_project_id": 123,
            "target_project_id": 123,
            "action": "open",
            "url": "https://gitlab.com/group/project/-/merge_requests/7"
        }
    }"#;

    let event: MergeRequestEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event.object_kind, "merge_request");
    assert_eq!(event.object_attributes.iid, 7);
    assert!(event.should_review());
    assert_eq!(event.project_path(), "group/project");
    assert_eq!(event.mr_iid(), 7);
}

#[test]
fn test_gitlab_mr_draft_should_not_review() {
    let json = r#"{
        "object_kind": "merge_request",
        "user": { "id": 1, "username": "user", "name": "User" },
        "project": {
            "id": 1,
            "name": "project",
            "path_with_namespace": "group/project",
            "web_url": "https://gitlab.com/group/project",
            "git_http_url": "https://gitlab.com/group/project.git"
        },
        "object_attributes": {
            "id": 1,
            "iid": 1,
            "title": "Draft: Work in progress",
            "state": "opened",
            "draft": true,
            "source_branch": "feature",
            "target_branch": "main",
            "source_project_id": 1,
            "target_project_id": 1,
            "action": "open",
            "url": "https://gitlab.com/group/project/-/merge_requests/1"
        }
    }"#;

    let event: MergeRequestEvent = serde_json::from_str(json).unwrap();
    assert!(!event.should_review());
}

// ─────────────────────────────────────────────────────────────────────────────
// Review Bridge Tests
// ─────────────────────────────────────────────────────────────────────────────

use zero_workflow::ReviewBridge;

#[test]
fn test_review_bridge_creation() {
    let _bridge = ReviewBridge::new("http://localhost:4400");
    // Bridge should be created without errors
}
