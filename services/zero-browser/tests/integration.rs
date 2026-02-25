//! Integration tests for zero-browser.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use zero_browser::{build_router, AppState};

fn test_app() -> axum::Router {
    build_router(AppState::new(1800))
}

#[tokio::test]
async fn test_full_learning_flow() {
    let app = test_app();

    // 1. Create session
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/browser/sessions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"headless": true}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let session_id = json["data"]["session_id"].as_str().unwrap();

    // 2. Start learning
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/browser/sessions/{}/learn/start", session_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"filter": {"hosts": ["api.example.com"]}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 3. Stop learning (no requests captured in test)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/browser/sessions/{}/learn/stop", session_id))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["request_count"], 0);
    assert_eq!(json["data"]["unique_endpoints"], 0);

    // 4. Close session
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/browser/sessions/{}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_pattern_crud() {
    let state = AppState::new(1800);

    // Manually insert a pattern
    {
        let mut patterns = state.patterns.write().await;
        let pattern = zero_browser::ApiPattern::new("api.example.com", "GET", "/users");
        patterns.insert(pattern.id.clone(), pattern);
    }

    let app = build_router(state);

    // List patterns
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/patterns").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["count"], 1);

    // Get single pattern
    let pattern_id = "api.example.com:GET:/users";
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/patterns/{}", urlencoding::encode(pattern_id)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Delete pattern
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/patterns/{}", urlencoding::encode(pattern_id)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
