//! Integration tests for Zero Channels.
//!
//! Tests the webhook endpoints and message handling.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use zero_channels::{build_router, create_state, ChannelMessage};

/// Test helper to create a test router.
fn create_test_app() -> (axum::Router, tokio::sync::mpsc::Receiver<ChannelMessage>) {
    let (state, rx) = create_state(None, "http://localhost:4400".to_string());
    (build_router(state), rx)
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
    let (app, _rx) = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "zero-channels");
}

#[tokio::test]
async fn test_ready_check() {
    let (app, _rx) = create_test_app();

    let (status, json) = request_json(&app, Method::GET, "/ready", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ready");
}

#[tokio::test]
async fn test_ready_check_closed_channel() {
    let (state, rx) = create_state(None, "http://localhost:4400".to_string());
    let app = build_router(state);

    // Drop the receiver to close the channel
    drop(rx);

    let (status, json) = request_json(&app, Method::GET, "/ready", None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["status"], "not_ready");
}

// ─────────────────────────────────────────────────────────────────────────────
// Generic Webhook Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_generic_webhook_success() {
    let (app, mut rx) = create_test_app();

    let payload = json!({
        "channel": "test",
        "user_id": "user123",
        "message": "Hello, World!"
    });

    let (status, json) = request_json(&app, Method::POST, "/webhook/generic", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);

    // Verify message was received
    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.user_id, "user123");
}

#[tokio::test]
async fn test_generic_webhook_with_metadata() {
    let (app, mut rx) = create_test_app();

    let payload = json!({
        "channel": "telegram",
        "user_id": "user456",
        "message": "Test message",
        "metadata": {
            "source": "test",
            "version": "1.0"
        }
    });

    let (status, _) = request_json(&app, Method::POST, "/webhook/generic", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);

    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.user_id, "user456");
    assert!(msg.metadata.contains_key("source"));
}

#[tokio::test]
async fn test_generic_webhook_invalid_json() {
    let (app, _rx) = create_test_app();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/webhook/generic")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("not valid json"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─────────────────────────────────────────────────────────────────────────────
// Telegram Webhook Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_telegram_webhook_text_message() {
    let (app, mut rx) = create_test_app();

    let payload = json!({
        "update_id": 12345,
        "message": {
            "message_id": 1,
            "chat": { "id": 123456789 },
            "from": { "id": 987654321, "username": "testuser" },
            "text": "Hello from Telegram!"
        }
    });

    let (status, json) =
        request_json(&app, Method::POST, "/webhook/telegram/test-token", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);

    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.user_id, "testuser");
    assert_eq!(msg.channel_id, "123456789");
}

#[tokio::test]
async fn test_telegram_webhook_empty_token() {
    let (app, _rx) = create_test_app();

    let payload = json!({
        "update_id": 12345
    });

    let (status, _json) =
        request_json(&app, Method::POST, "/webhook/telegram/", Some(payload)).await;

    // Empty token path results in 404
    assert!(status == StatusCode::NOT_FOUND || status == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_telegram_webhook_no_message() {
    let (app, _rx) = create_test_app();

    // Update without a message (e.g., edited_message, callback_query)
    let payload = json!({
        "update_id": 12345
    });

    let (status, json) =
        request_json(&app, Method::POST, "/webhook/telegram/valid-token", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn test_telegram_webhook_user_fallback() {
    let (app, mut rx) = create_test_app();

    // Message from user without username
    let payload = json!({
        "update_id": 12345,
        "message": {
            "message_id": 1,
            "chat": { "id": 111222333 },
            "from": { "id": 444555666 },
            "text": "No username here"
        }
    });

    let (status, _) =
        request_json(&app, Method::POST, "/webhook/telegram/test-token", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);

    let msg = rx.try_recv().unwrap();
    // Should fallback to user ID
    assert_eq!(msg.user_id, "444555666");
}

// ─────────────────────────────────────────────────────────────────────────────
// Feishu Webhook Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_feishu_webhook_not_configured() {
    let (app, _rx) = create_test_app();

    let (status, json) = request_json(&app, Method::POST, "/webhook/feishu", Some(json!({}))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["success"], false);
}

// ─────────────────────────────────────────────────────────────────────────────
// Channel Type Mapping Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_channel_type_mapping() {
    let test_cases = vec![
        ("telegram", "telegram"),
        ("discord", "discord"),
        ("slack", "slack"),
        ("feishu", "feishu"),
        ("whatsapp", "whatsapp"),
        ("matrix", "matrix"),
        ("imessage", "imessage"),
        ("email", "email"),
        ("unknown", "cli"), // Unknown channels map to CLI
    ];

    for (input, expected) in test_cases {
        let (app, mut rx) = create_test_app();

        let payload = json!({
            "channel": input,
            "user_id": "test",
            "message": "test"
        });

        let (status, _) = request_json(&app, Method::POST, "/webhook/generic", Some(payload)).await;
        assert_eq!(status, StatusCode::OK);

        let msg = rx.try_recv().unwrap();
        assert_eq!(
            msg.channel_type.as_str(),
            expected,
            "Channel {} should map to {}",
            input,
            expected
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Message Content Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_message_content_text() {
    let (app, mut rx) = create_test_app();

    let payload = json!({
        "channel": "test",
        "user_id": "user",
        "message": "This is a text message with special chars: <>&\""
    });

    let (status, _) = request_json(&app, Method::POST, "/webhook/generic", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);

    let msg = rx.try_recv().unwrap();
    if let zero_channels::MessageContent::Text { text } = msg.content {
        assert!(text.contains("<>&\""));
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
async fn test_message_timestamp() {
    let (app, mut rx) = create_test_app();

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let payload = json!({
        "channel": "test",
        "user_id": "user",
        "message": "test"
    });

    let (status, _) = request_json(&app, Method::POST, "/webhook/generic", Some(payload)).await;
    assert_eq!(status, StatusCode::OK);

    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let msg = rx.try_recv().unwrap();
    assert!(msg.timestamp >= before && msg.timestamp <= after);
}

// ─────────────────────────────────────────────────────────────────────────────
// Outbound Router Tests
// ─────────────────────────────────────────────────────────────────────────────

use zero_channels::{ChannelType, MessageContent, OutboundRouter, OutgoingContent};

#[tokio::test]
async fn test_outbound_router_pending_registration() {
    let router = OutboundRouter::new();

    // Create a test message
    let message = ChannelMessage {
        id: "test-msg-1".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123456".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Hello".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 1234567890000,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    // Register pending
    router.register_pending(message).await;

    // Should have 1 pending
    assert_eq!(router.pending_count().await, 1);
}

#[tokio::test]
async fn test_outbound_router_take_pending() {
    let router = OutboundRouter::new();

    let message = ChannelMessage {
        id: "test-msg-2".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123456".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Hello".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 1234567890000,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    router.register_pending(message).await;

    // Take the pending message
    let taken = router.take_pending("test-msg-2").await;
    assert!(taken.is_some());

    // Should be empty now
    assert_eq!(router.pending_count().await, 0);

    // Taking again should return None
    let taken_again = router.take_pending("test-msg-2").await;
    assert!(taken_again.is_none());
}

#[tokio::test]
async fn test_outbound_router_cleanup_stale() {
    let router = OutboundRouter::new();

    let message = ChannelMessage {
        id: "stale-msg".into(),
        channel_type: ChannelType::Telegram,
        channel_id: "123456".into(),
        user_id: "user1".into(),
        content: MessageContent::Text {
            text: "Hello".into(),
        },
        attachments: vec![],
        metadata: std::collections::HashMap::new(),
        timestamp: 1234567890000,
        trace_id: "test-trace".into(),
        span_id: "test-span".into(),
        parent_span_id: None,
    };

    router.register_pending(message).await;
    assert_eq!(router.pending_count().await, 1);

    // Cleanup with TTL of 0 should remove all
    router.cleanup_stale(0).await;
    assert_eq!(router.pending_count().await, 0);
}

#[tokio::test]
async fn test_outbound_router_send_without_channel() {
    let router = OutboundRouter::new();

    // Try to send without any channels configured
    let result = router
        .send_direct(
            ChannelType::Telegram,
            "123456".into(),
            OutgoingContent::Text {
                text: "Test".into(),
            },
        )
        .await;

    // Should fail because no Telegram channel is configured
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("not configured"));
}

#[tokio::test]
async fn test_outbound_router_respond_without_pending() {
    let router = OutboundRouter::new();

    // Try to respond to a non-existent message
    let result = router
        .respond(
            "nonexistent-msg",
            OutgoingContent::Text {
                text: "Response".into(),
            },
        )
        .await;

    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("No pending response"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Bridge Tests
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use zero_channels::CodeCoderBridge;

#[tokio::test]
async fn test_bridge_creation() {
    let router = Arc::new(OutboundRouter::new());
    let _bridge = CodeCoderBridge::new("http://localhost:4400", router);

    // Bridge should be created successfully
    // We can't test actual API calls without a running server
    // Verify bridge is a valid instance by checking type compilation succeeds
}

#[test]
fn test_chat_request_serialization() {
    use zero_channels::ChatRequest;

    let request = ChatRequest {
        message: "Hello, CodeCoder!".into(),
        conversation_id: Some("conv-123".into()),
        agent: None,
        user_id: "user1".into(),
        channel: "telegram".into(),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"message\":\"Hello, CodeCoder!\""));
    assert!(json.contains("\"conversation_id\":\"conv-123\""));
    assert!(!json.contains("\"agent\"")); // Should be skipped when None
}

#[test]
fn test_chat_response_deserialization() {
    use zero_channels::ChatApiResponse;

    // Test the wrapped API response format
    let json = r#"{
        "success": true,
        "data": {
            "message": "Hello! How can I help you?",
            "conversation_id": "conv-123",
            "agent": "general",
            "usage": {
                "input_tokens": 15,
                "output_tokens": 25,
                "total_tokens": 40
            }
        }
    }"#;

    let api_response: ChatApiResponse = serde_json::from_str(json).unwrap();
    assert!(api_response.success);
    assert!(api_response.data.is_some());

    let response = api_response.data.unwrap();
    assert_eq!(response.message, "Hello! How can I help you?");
    assert_eq!(response.conversation_id, Some("conv-123".into()));
    assert_eq!(response.agent, Some("general".into()));

    let usage = response.usage.unwrap();
    assert_eq!(usage.input_tokens, 15);
    assert_eq!(usage.output_tokens, 25);
}

#[test]
fn test_chat_response_error() {
    use zero_channels::ChatApiResponse;

    // Test error response format
    let json = r#"{
        "success": false,
        "error": "message is required"
    }"#;

    let api_response: ChatApiResponse = serde_json::from_str(json).unwrap();
    assert!(!api_response.success);
    assert!(api_response.data.is_none());
    assert_eq!(api_response.error, Some("message is required".into()));
}

#[test]
fn test_chat_response_data_without_usage() {
    use zero_channels::ChatResponseData;

    let json = r#"{
        "message": "Simple response"
    }"#;

    let response: ChatResponseData = serde_json::from_str(json).unwrap();
    assert_eq!(response.message, "Simple response");
    assert!(response.usage.is_none());
}
