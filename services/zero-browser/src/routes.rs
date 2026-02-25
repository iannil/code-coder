//! HTTP API routes.

use crate::browser::{LearnFilter, SessionConfig, SessionManager};
use crate::error::BrowserError;
use crate::pattern::{extract_patterns, ApiPattern};
use crate::replay::{ReplayExecutor, ReplayParams};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub session_manager: SessionManager,
    pub patterns: Arc<RwLock<HashMap<String, ApiPattern>>>,
    pub replay_executor: Arc<ReplayExecutor>,
}

impl AppState {
    pub fn new(session_timeout_secs: i64) -> Self {
        Self {
            session_manager: SessionManager::new(session_timeout_secs),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            replay_executor: Arc::new(ReplayExecutor::new()),
        }
    }
}

/// Build the application router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Session management
        .route("/browser/sessions", post(create_session))
        .route("/browser/sessions/:id", delete(close_session))
        // Learning
        .route("/browser/sessions/:id/learn/start", post(start_learning))
        .route("/browser/sessions/:id/learn/stop", post(stop_learning))
        // Pattern management
        .route("/patterns", get(list_patterns))
        .route("/patterns/:id", get(get_pattern))
        .route("/patterns/:id", delete(delete_pattern))
        // Replay
        .route("/replay", post(replay_api))
        .with_state(state)
}

// ============ Health Check ============

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "zero-browser",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ============ Session Management ============

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    session_id: String,
}

async fn create_session(
    State(state): State<AppState>,
    Json(config): Json<SessionConfig>,
) -> impl IntoResponse {
    let session_id = state.session_manager.create(config).await;
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": CreateSessionResponse { session_id }
        })),
    )
}

async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    state.session_manager.close(&id).await?;
    Ok(Json(serde_json::json!({
        "success": true
    })))
}

// ============ Learning ============

#[derive(Debug, Deserialize)]
struct StartLearningRequest {
    filter: LearnFilter,
}

async fn start_learning(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<StartLearningRequest>,
) -> Result<impl IntoResponse, BrowserError> {
    state
        .session_manager
        .update(&id, |session| {
            session.start_learning(request.filter);
        })
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "status": "learning"
        }
    })))
}

#[derive(Debug, Serialize)]
struct StopLearningResponse {
    patterns: Vec<ApiPattern>,
    request_count: usize,
    unique_endpoints: usize,
}

async fn stop_learning(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    // Get captured requests
    let captured = {
        let mut captured = Vec::new();
        state
            .session_manager
            .update(&id, |session| {
                captured = session.stop_learning();
            })
            .await?;
        captured
    };

    let request_count = captured.len();

    // Extract patterns
    let patterns = extract_patterns(&captured);
    let unique_endpoints = patterns.len();

    // Store patterns
    {
        let mut stored = state.patterns.write().await;
        for pattern in &patterns {
            stored.insert(pattern.id.clone(), pattern.clone());
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": StopLearningResponse {
            patterns,
            request_count,
            unique_endpoints,
        }
    })))
}

// ============ Pattern Management ============

#[derive(Debug, Deserialize)]
struct ListPatternsQuery {
    host: Option<String>,
    method: Option<String>,
}

async fn list_patterns(
    State(state): State<AppState>,
    Query(query): Query<ListPatternsQuery>,
) -> impl IntoResponse {
    let patterns = state.patterns.read().await;

    let filtered: Vec<&ApiPattern> = patterns
        .values()
        .filter(|p| {
            query.host.as_ref().map_or(true, |h| &p.host == h)
                && query.method.as_ref().map_or(true, |m| &p.method == m)
        })
        .collect();

    Json(serde_json::json!({
        "success": true,
        "data": {
            "patterns": filtered,
            "count": filtered.len()
        }
    }))
}

async fn get_pattern(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    let patterns = state.patterns.read().await;
    let pattern = patterns
        .get(&id)
        .ok_or_else(|| BrowserError::PatternNotFound(id))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": pattern
    })))
}

async fn delete_pattern(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    let mut patterns = state.patterns.write().await;
    patterns
        .remove(&id)
        .ok_or_else(|| BrowserError::PatternNotFound(id.clone()))?;

    Ok(Json(serde_json::json!({
        "success": true
    })))
}

// ============ Replay ============

async fn replay_api(
    State(state): State<AppState>,
    Json(params): Json<ReplayParams>,
) -> Result<impl IntoResponse, BrowserError> {
    let patterns = state.patterns.read().await;
    let pattern = patterns
        .get(&params.pattern_id)
        .ok_or_else(|| BrowserError::PatternNotFound(params.pattern_id.clone()))?
        .clone();
    drop(patterns);

    let response = state.replay_executor.execute(&pattern, &params).await?;

    // Update usage count
    {
        let mut patterns = state.patterns.write().await;
        if let Some(p) = patterns.get_mut(&params.pattern_id) {
            p.record_success();
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> Router {
        build_router(AppState::new(1800))
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = test_app();

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_session() {
        let app = test_app();

        let response = app
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
    }

    #[tokio::test]
    async fn test_list_patterns_empty() {
        let app = test_app();

        let response = app
            .oneshot(Request::builder().uri("/patterns").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
