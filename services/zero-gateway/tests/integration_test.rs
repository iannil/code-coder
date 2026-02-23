//! Integration tests for Zero Gateway.
//!
//! Tests the full HTTP API including authentication, user management, and proxying.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;
use zero_common::config::{Config, GatewayConfig};
use zero_gateway::{
    routes::{build_all_routes_with_db, ErrorResponse, ListUsersResponse, LoginResponse, UserResponse},
    sandbox::{Sandbox, SandboxConfig},
};

/// Test helper to create a test config and router with an isolated database.
fn create_test_app(temp_dir: &TempDir) -> axum::Router {
    // Set up environment for the test
    std::env::set_var("JWT_SECRET", "test-secret-key-for-integration-tests!");

    let config = Config {
        gateway: GatewayConfig {
            port: 4410,
            host: "127.0.0.1".to_string(),
            jwt_secret: Some("test-secret-key-for-integration-tests!".to_string()),
            token_expiry_secs: 3600,
            rate_limiting: false,
            rate_limit_rpm: 60,
            codecoder_endpoint: "http://127.0.0.1:4400".to_string(),
        },
        ..Default::default()
    };

    // Use a unique database path for each test
    let db_path = temp_dir.path().join("test-gateway.db");
    build_all_routes_with_db(&config, Some(db_path))
}

/// Helper to make a request and get JSON response.
async fn request_json<T: serde::de::DeserializeOwned>(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, T) {
    let mut request = Request::builder().method(method).uri(uri);

    if let Some(t) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }

    let request = if let Some(b) = body {
        request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&b).unwrap()))
            .unwrap()
    } else {
        request.body(Body::empty()).unwrap()
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: T = serde_json::from_slice(&body).unwrap();

    (status, json)
}

// ─────────────────────────────────────────────────────────────────────────────
// Health Check Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "zero-gateway");
}

#[tokio::test]
async fn test_health_check_api_path() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ─────────────────────────────────────────────────────────────────────────────
// Authentication Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_success() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login with default admin credentials
    let (status, response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(!response.token.is_empty());
    assert_eq!(response.user.username, "admin");
    assert!(response.user.roles.contains(&"admin".to_string()));
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let (status, response): (_, ErrorResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "wrongpassword"
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.code, "AUTH_INVALID_CREDENTIALS");
}

#[tokio::test]
async fn test_login_empty_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let (status, response): (_, ErrorResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "",
            "password": ""
        })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.code, "AUTH_INVALID_CREDENTIALS");
}

#[tokio::test]
async fn test_me_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // First login
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Then get current user info
    let (status, user_response): (_, UserResponse) = request_json(
        &app,
        Method::GET,
        "/api/v1/auth/me",
        None,
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(user_response.username, "admin");
}

#[tokio::test]
async fn test_me_endpoint_unauthenticated() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/me")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ─────────────────────────────────────────────────────────────────────────────
// User Management Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_users_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // List users
    let (status, list_response): (_, ListUsersResponse) = request_json(
        &app,
        Method::GET,
        "/api/v1/users",
        None,
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(list_response.total >= 1);
    assert!(!list_response.users.is_empty());
}

#[tokio::test]
async fn test_create_user_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a new user
    let (status, user_response): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "newuser",
            "password": "newpassword123",
            "roles": ["user"],
            "email": "newuser@example.com"
        })),
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(user_response.username, "newuser");
    assert_eq!(user_response.email, Some("newuser@example.com".to_string()));
}

#[tokio::test]
async fn test_get_user_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Get admin's own profile
    let user_id = &login_response.user.id;
    let (status, user_response): (_, UserResponse) = request_json(
        &app,
        Method::GET,
        &format!("/api/v1/users/{}", user_id),
        None,
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(user_response.id, *user_id);
}

#[tokio::test]
async fn test_update_user_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a user first
    let (_, created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "updatetest",
            "password": "password123",
            "roles": ["user"]
        })),
        Some(&login_response.token),
    )
    .await;

    // Update the user
    let (status, updated_user): (_, UserResponse) = request_json(
        &app,
        Method::PUT,
        &format!("/api/v1/users/{}", created_user.id),
        Some(json!({
            "display_name": "Updated Name",
            "email": "updated@example.com"
        })),
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_user.display_name, Some("Updated Name".to_string()));
    assert_eq!(updated_user.email, Some("updated@example.com".to_string()));
}

#[tokio::test]
async fn test_delete_user_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a user to delete
    let (_, created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "deletetest",
            "password": "password123",
            "roles": ["user"]
        })),
        Some(&login_response.token),
    )
    .await;

    // Delete the user
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/users/{}", created_user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", login_response.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify user is gone
    let (status, _): (_, ErrorResponse) = request_json(
        &app,
        Method::GET,
        &format!("/api/v1/users/{}", created_user.id),
        None,
        Some(&login_response.token),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cannot_delete_self() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Try to delete self
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/users/{}", login_response.user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", login_response.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_regular_user_cannot_list_users() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin first
    let (_, admin_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a regular user
    let (_, _): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "regularuser",
            "password": "password123",
            "roles": ["api_consumer"]
        })),
        Some(&admin_login.token),
    )
    .await;

    // Login as regular user
    let (_, user_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "regularuser",
            "password": "password123"
        })),
        None,
    )
    .await;

    // Try to list users - should be forbidden
    let (status, _): (_, ErrorResponse) = request_json(
        &app,
        Method::GET,
        "/api/v1/users",
        None,
        Some(&user_login.token),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_regular_user_can_read_own_profile() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin first
    let (_, admin_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a regular user
    let (_, created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "ownprofile",
            "password": "password123",
            "roles": ["api_consumer"]
        })),
        Some(&admin_login.token),
    )
    .await;

    // Login as regular user
    let (_, user_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "ownprofile",
            "password": "password123"
        })),
        None,
    )
    .await;

    // Read own profile - should succeed
    let (status, user_response): (_, UserResponse) = request_json(
        &app,
        Method::GET,
        &format!("/api/v1/users/{}", created_user.id),
        None,
        Some(&user_login.token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(user_response.username, "ownprofile");
}

// ─────────────────────────────────────────────────────────────────────────────
// Sandbox Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sandbox_sensitive_data_detection() {
    let sandbox = Sandbox::new(SandboxConfig::default());

    // Test API key detection
    let content = "Here is my API key: sk-1234567890abcdefghijABCDEF";
    let result = sandbox.sanitize_response(content);
    assert!(!result.content.contains("sk-1234"));
}

#[test]
fn test_sandbox_request_filtering() {
    let sandbox = Sandbox::new(SandboxConfig::default());
    use std::collections::HashMap;

    // Test path traversal blocking
    let result = sandbox.filter_request("GET", "/../../../etc/passwd", None, &HashMap::new());
    assert!(matches!(
        result,
        zero_gateway::sandbox::FilterResult::Blocked { .. }
    ));

    // Test normal path allowed
    let result = sandbox.filter_request("GET", "/api/v1/users", None, &HashMap::new());
    assert!(matches!(
        result,
        zero_gateway::sandbox::FilterResult::Allowed
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Quota and Metering Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_my_quota() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Get quota
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/quota")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", login_response.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Check response structure
    assert!(json.get("user_id").is_some());
    assert!(json.get("usage").is_some());
    assert!(json.get("limits").is_some());

    // Check limits have reasonable defaults
    let limits = &json["limits"];
    assert!(limits["daily_input_tokens"].as_i64().unwrap() > 0);
    assert!(limits["daily_output_tokens"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_get_quota_unauthenticated() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Try to get quota without auth
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/quota")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_set_user_quota_as_admin() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin
    let (_, login_response): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a test user
    let (_, created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "quotatest",
            "password": "password123",
            "roles": ["user"]
        })),
        Some(&login_response.token),
    )
    .await;

    // Set custom quota limits
    let request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/quota/{}", created_user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", login_response.token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "daily_input_tokens": 50000,
                "daily_output_tokens": 25000,
                "daily_requests": 100
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["daily_input_tokens"], 50000);
    assert_eq!(json["daily_output_tokens"], 25000);
    assert_eq!(json["daily_requests"], 100);

    // Verify by getting the quota
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/quota/{}", created_user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", login_response.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["limits"]["daily_input_tokens"], 50000);
}

#[tokio::test]
async fn test_regular_user_cannot_set_quota() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin first to create a regular user
    let (_, admin_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a regular user
    let (_, _created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "regular_user",
            "password": "password123",
            "roles": ["user"]
        })),
        Some(&admin_login.token),
    )
    .await;

    // Login as regular user
    let (_, user_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "regular_user",
            "password": "password123"
        })),
        None,
    )
    .await;

    // Try to set quota (should fail - no admin permission)
    let request = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/quota/{}", user_login.user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", user_login.token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "daily_input_tokens": 999999
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_user_can_check_own_quota() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Login as admin first to create a regular user
    let (_, admin_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "admin",
            "password": "admin123"
        })),
        None,
    )
    .await;

    // Create a regular user
    let (_, created_user): (_, UserResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(json!({
            "username": "quota_checker",
            "password": "password123",
            "roles": ["user"]
        })),
        Some(&admin_login.token),
    )
    .await;

    // Login as regular user
    let (_, user_login): (_, LoginResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "username": "quota_checker",
            "password": "password123"
        })),
        None,
    )
    .await;

    // User can check their own quota via /api/v1/quota
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/quota")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", user_login.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // User can also check via /api/v1/quota/:id with their own ID
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/quota/{}", created_user.id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", user_login.token),
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
