//! Integration tests for the Capture API endpoints.
//!
//! Tests the asset capture functionality including:
//! - `/api/v1/capture` - Capture content from URL
//! - `/api/v1/capture/history` - Get capture history
//! - `/api/v1/capture/:asset_id` - Get specific asset
//! - `/api/v1/capture/:asset_id/save` - Save to new destination

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use zero_channels::{
    build_router,
    capture_bridge::{CapturedAsset, CaptureBridge},
    create_state_with_capture,
    ChannelMessage, ChannelType, MessageContent, OutboundRouter,
};
use zero_common::config::{AutoCaptureConfig, CaptureConfig};

// ─────────────────────────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn create_test_capture_config() -> CaptureConfig {
    CaptureConfig {
        enabled: true,
        feishu_docs: None,
        notion: None,
        auto_capture: AutoCaptureConfig {
            capture_forwarded: true,
            capture_links: true,
            trigger_prefixes: vec!["#收藏".to_string(), "#save".to_string()],
        },
    }
}

fn create_test_capture_state() -> (axum::Router, tokio::sync::mpsc::Receiver<ChannelMessage>) {
    let config = create_test_capture_config();
    let capture = Arc::new(CaptureBridge::new(
        config,
        "http://localhost:4400".to_string(),
    ));
    let outbound = Arc::new(OutboundRouter::new());

    let (state, rx) = create_state_with_capture(
        None,
        None,
        None,
        outbound,
        capture,
        "http://localhost:4400".to_string(),
    );

    (build_router(state), rx)
}

fn create_state_without_capture() -> (axum::Router, tokio::sync::mpsc::Receiver<ChannelMessage>) {
    let (state, rx) = zero_channels::create_state(None, "http://localhost:4400".to_string());
    (build_router(state), rx)
}

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
// Capture API Endpoint Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_capture_not_configured() {
    let (app, _rx) = create_state_without_capture();

    let payload = json!({
        "url": "https://example.com/article"
    });

    let (status, json) = request_json(&app, Method::POST, "/api/v1/capture", Some(payload)).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["success"], false);
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("not configured"));
}

#[tokio::test]
async fn test_capture_history_not_configured() {
    let (app, _rx) = create_state_without_capture();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/capture/history", None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["success"], false);
}

#[tokio::test]
async fn test_capture_history_empty() {
    let (app, _rx) = create_test_capture_state();

    let (status, json) = request_json(&app, Method::GET, "/api/v1/capture/history", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
    assert_eq!(json["total"], 0);
}

#[tokio::test]
async fn test_capture_history_with_pagination() {
    let (app, _rx) = create_test_capture_state();

    // Test with limit and offset
    let (status, json) = request_json(
        &app,
        Method::GET,
        "/api/v1/capture/history?limit=10&offset=0",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_get_asset_not_found() {
    let (app, _rx) = create_test_capture_state();

    let (status, json) = request_json(
        &app,
        Method::GET,
        "/api/v1/capture/non-existent-id",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_get_asset_not_configured() {
    let (app, _rx) = create_state_without_capture();

    let (status, json) =
        request_json(&app, Method::GET, "/api/v1/capture/some-id", None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["success"], false);
}

#[tokio::test]
async fn test_save_asset_not_configured() {
    let (app, _rx) = create_state_without_capture();

    let payload = json!({
        "destination": "notion"
    });

    let (status, json) = request_json(
        &app,
        Method::POST,
        "/api/v1/capture/some-id/save",
        Some(payload),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["success"], false);
}

#[tokio::test]
async fn test_save_asset_not_found() {
    let (app, _rx) = create_test_capture_state();

    let payload = json!({
        "destination": "notion"
    });

    let (status, json) = request_json(
        &app,
        Method::POST,
        "/api/v1/capture/non-existent-id/save",
        Some(payload),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// CaptureBridge Unit Tests (re-exported from lib)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_capture_bridge_is_enabled() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    assert!(bridge.is_enabled());
}

#[test]
fn test_capture_bridge_disabled() {
    let mut config = create_test_capture_config();
    config.enabled = false;
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    assert!(!bridge.is_enabled());
}

#[test]
fn test_capture_bridge_is_capturable_with_link() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    let message = ChannelMessage {
        id: "test-1".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Check this: https://example.com/article".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    assert!(bridge.is_capturable(&message));
}

#[test]
fn test_capture_bridge_is_capturable_with_forward() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("forward_from".into(), "other_user".into());

    let message = ChannelMessage {
        id: "test-2".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Forwarded message".into(),
        },
        attachments: vec![],
        metadata,
        timestamp: 0,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    assert!(bridge.is_capturable(&message));
}

#[test]
fn test_capture_bridge_is_capturable_with_trigger_prefix() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    let message = ChannelMessage {
        id: "test-3".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "#收藏 这篇文章很好".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    assert!(bridge.is_capturable(&message));

    let message2 = ChannelMessage {
        id: "test-4".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "#save this article".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    assert!(bridge.is_capturable(&message2));
}

#[test]
fn test_capture_bridge_not_capturable_plain_text() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    let message = ChannelMessage {
        id: "test-5".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Hello, how are you?".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    assert!(!bridge.is_capturable(&message));
}

#[test]
fn test_capture_bridge_is_capture_request() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    let test_cases = vec![
        ("请收藏这篇文章", true),
        ("保存这个链接", true),
        ("#save", true),
        ("@save this", true),
        ("#收藏 好文", true),
        ("capture this", true),
        ("Hello world", false),
        ("Let me check", false),
    ];

    for (text, expected) in test_cases {
        let message = ChannelMessage {
            id: "test".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "123".into(),
            user_id: "user1".into(),
            content: MessageContent::Text { text: text.into() },
            attachments: vec![],
            metadata: std::collections::HashMap::new(),
            timestamp: 0,
            trace_id: "test-trace".into(),
            span_id: "test-span".into(),
            parent_span_id: None,
        };

        assert_eq!(
            bridge.is_capture_request(&message),
            expected,
            "Text '{}' should return {}",
            text,
            expected
        );
    }
}

#[tokio::test]
async fn test_capture_bridge_get_history() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    // Initially empty
    let history = bridge.get_history(10, 0).await;
    assert_eq!(history.len(), 0);

    // Add directly to internal state for testing
    // (In real usage, capture() would populate this)
}

#[tokio::test]
async fn test_capture_bridge_get_asset() {
    let config = create_test_capture_config();
    let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

    // Asset not found
    let asset = bridge.get_asset("non-existent").await;
    assert!(asset.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Response Format Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_captured_asset_serialization() {
    use chrono::Utc;
    use zero_channels::capture_bridge::{AssetContentType, SavedLocation};

    let asset = CapturedAsset {
        id: "test-id".into(),
        source_channel: ChannelType::Telegram,
        source_user: "user1".into(),
        original_url: Some("https://example.com".into()),
        content_type: AssetContentType::Article,
        raw_content: "Test content".into(),
        summary: "Test summary".into(),
        tags: vec!["tag1".into(), "tag2".into()],
        category: Some("技术".into()),
        key_points: vec!["Point 1".into()],
        captured_at: Utc::now(),
        saved_to: vec![SavedLocation {
            platform: "feishu_docs".into(),
            url: "https://feishu.cn/doc/xxx".into(),
            title: "Test Doc".into(),
        }],
    };

    let json = serde_json::to_string(&asset).unwrap();
    assert!(json.contains("\"id\":\"test-id\""));
    assert!(json.contains("\"source_channel\":\"telegram\""));
    assert!(json.contains("\"content_type\":\"article\""));
    assert!(json.contains("\"tags\":[\"tag1\",\"tag2\"]"));
}

#[test]
fn test_asset_content_type_variants() {
    use zero_channels::capture_bridge::AssetContentType;

    let test_cases = vec![
        (AssetContentType::Article, "article"),
        (AssetContentType::Tweet, "tweet"),
        (AssetContentType::Image, "image"),
        (AssetContentType::Document, "document"),
        (AssetContentType::Link, "link"),
        (AssetContentType::RawText, "raw_text"),
    ];

    for (content_type, expected_str) in test_cases {
        assert_eq!(content_type.as_str(), expected_str);
    }
}

#[test]
fn test_asset_content_type_from_url() {
    use zero_channels::capture_bridge::AssetContentType;

    let test_cases = vec![
        ("https://twitter.com/user/status/123", AssetContentType::Tweet),
        ("https://x.com/user/status/456", AssetContentType::Tweet),
        ("https://weibo.com/detail/789", AssetContentType::Tweet),
        ("https://example.com/doc.pdf", AssetContentType::Document),
        ("https://example.com/image.png", AssetContentType::Image),
        ("https://example.com/photo.jpg", AssetContentType::Image),
        ("https://example.com/photo.jpeg", AssetContentType::Image),
        ("https://example.com/animation.gif", AssetContentType::Image),
        ("https://example.com/image.webp", AssetContentType::Image),
        ("https://example.com/article", AssetContentType::Article),
        ("https://medium.com/story/abc", AssetContentType::Article),
    ];

    for (url, expected) in test_cases {
        assert_eq!(
            AssetContentType::from_url(url),
            expected,
            "URL '{}' should be {:?}",
            url,
            expected
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Handling Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_capture_invalid_request() {
    let (app, _rx) = create_test_capture_state();

    // Missing required field
    let payload = json!({
        "tags": ["test"]
    });

    let (status, _json) = request_json(&app, Method::POST, "/api/v1/capture", Some(payload)).await;

    // Axum returns 422 Unprocessable Entity for JSON deserialization errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_save_invalid_request() {
    let (app, _rx) = create_test_capture_state();

    // Missing required field
    let payload = json!({});

    let (status, _json) = request_json(
        &app,
        Method::POST,
        "/api/v1/capture/some-id/save",
        Some(payload),
    )
    .await;

    // Axum returns 422 Unprocessable Entity for JSON deserialization errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
