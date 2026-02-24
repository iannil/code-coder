//! Integration tests for the Human-in-the-Loop (HitL) approval system.
//!
//! These tests verify the full approval workflow including:
//! - Request creation and storage
//! - Approval and rejection flows
//! - Authorization checks
//! - Callback handling from multiple IM channels
//! - Concurrent request management

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

use zero_gateway::hitl::{
    cards::{CallbackAction, CallbackData, CardRenderer},
    routes::{DecideRequest, HitLService, ListPendingResponse, hitl_routes},
    store::HitLStore,
    ApprovalRequest, ApprovalResponse, ApprovalStatus, ApprovalType, CreateApprovalRequest,
    RiskLevel,
};

// ─────────────────────────────────────────────────────────────────────────────
// Test Setup Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Mock card renderer for testing purposes.
struct MockCardRenderer {
    channel_type: &'static str,
}

impl MockCardRenderer {
    fn new(channel_type: &'static str) -> Self {
        Self { channel_type }
    }
}

#[async_trait::async_trait]
impl CardRenderer for MockCardRenderer {
    fn channel_type(&self) -> &'static str {
        self.channel_type
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        _channel_id: &str,
    ) -> anyhow::Result<String> {
        // Return a mock message ID based on request ID
        Ok(format!("mock-msg-{}", request.id))
    }

    async fn update_card(
        &self,
        _request: &ApprovalRequest,
        _message_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> anyhow::Result<CallbackData> {
        // Simple JSON parsing for tests
        let data: CallbackData = serde_json::from_slice(payload)?;
        Ok(data)
    }
}

/// Setup a test service with a temporary database and mock renderers.
fn setup_test_service() -> (Arc<HitLService>, TempDir) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test_hitl.db");
    let store = Arc::new(HitLStore::new(&db_path).unwrap());

    // Register mock renderers for all supported channels
    let mut renderers: HashMap<String, Arc<dyn CardRenderer>> = HashMap::new();
    renderers.insert("telegram".to_string(), Arc::new(MockCardRenderer::new("telegram")));
    renderers.insert("slack".to_string(), Arc::new(MockCardRenderer::new("slack")));
    renderers.insert("feishu".to_string(), Arc::new(MockCardRenderer::new("feishu")));
    renderers.insert("dingtalk".to_string(), Arc::new(MockCardRenderer::new("dingtalk")));

    let service = Arc::new(HitLService::new(store, renderers));
    (service, dir)
}

/// Create a test approval request directly in the store.
fn create_stored_request(
    store: &HitLStore,
    id: &str,
    approvers: Vec<String>,
    channel: &str,
) -> ApprovalRequest {
    let now = Utc::now();
    let request = ApprovalRequest {
        id: id.to_string(),
        approval_type: ApprovalType::MergeRequest {
            platform: "github".to_string(),
            repo: "test/repo".to_string(),
            mr_id: 42,
        },
        status: ApprovalStatus::Pending,
        requester: "developer".to_string(),
        approvers,
        title: format!("Test Request {}", id),
        description: Some("Test description".to_string()),
        channel: channel.to_string(),
        message_id: Some(format!("msg-{}", id)),
        metadata: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        expires_at: None,
    };
    store.create(&request).unwrap();
    request
}

/// Helper to make a request and get JSON response.
async fn request_json<T: serde::de::DeserializeOwned>(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, T) {
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
    let json: T = serde_json::from_slice(&body).unwrap();

    (status, json)
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Full Approval Flow
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_full_approval_flow() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Step 1: Create approval request via API
    let create_request = CreateApprovalRequest {
        approval_type: ApprovalType::MergeRequest {
            platform: "github".to_string(),
            repo: "org/repo".to_string(),
            mr_id: 123,
        },
        requester: "developer".to_string(),
        approvers: vec!["admin".to_string(), "reviewer".to_string()],
        title: "Add new feature".to_string(),
        description: Some("Implements the dashboard feature".to_string()),
        channel: "telegram".to_string(),
        metadata: serde_json::json!({"priority": "high"}),
        ttl_seconds: Some(3600),
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&create_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(response.success);
    let approval = response.approval.unwrap();
    let approval_id = approval.id.clone();

    // Step 2: Verify request is stored
    let stored = service.store().get(&approval_id).unwrap();
    assert!(stored.is_some());
    let stored = stored.unwrap();
    assert_eq!(stored.title, "Add new feature");
    assert!(matches!(stored.status, ApprovalStatus::Pending));

    // Step 3: Simulate approval via decide endpoint
    let decide_request = DecideRequest {
        decided_by: "admin".to_string(),
        approved: true,
        reason: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        &format!("/{}/decide", approval_id),
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.success);
    let approval = response.approval.unwrap();

    // Step 4: Verify status updated to Approved
    match approval.status {
        ApprovalStatus::Approved { by, at: _ } => {
            assert_eq!(by, "admin");
        }
        _ => panic!("Expected Approved status, got {:?}", approval.status),
    }

    // Step 5: Verify in store
    let final_stored = service.store().get(&approval_id).unwrap().unwrap();
    assert!(matches!(final_stored.status, ApprovalStatus::Approved { .. }));

    // Verify audit log
    let audit_log = service.store().get_audit_log(&approval_id).unwrap();
    assert!(audit_log.len() >= 2); // created + approved
    assert!(audit_log.iter().any(|e| e.action == "created"));
    assert!(audit_log.iter().any(|e| e.action == "approved"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Rejection Flow
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_rejection_flow() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Create a request
    let create_request = CreateApprovalRequest {
        approval_type: ApprovalType::TradingCommand {
            asset: "BTC".to_string(),
            action: "buy".to_string(),
            amount: 10.0,
        },
        requester: "trader".to_string(),
        approvers: vec!["risk-manager".to_string()],
        title: "Buy 10 BTC".to_string(),
        description: None,
        channel: "slack".to_string(),
        metadata: serde_json::json!({}),
        ttl_seconds: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&create_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let approval_id = response.approval.unwrap().id;

    // Simulate rejection with reason
    let decide_request = DecideRequest {
        decided_by: "risk-manager".to_string(),
        approved: false,
        reason: Some("Exceeds daily trading limit".to_string()),
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        &format!("/{}/decide", approval_id),
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.success);
    let approval = response.approval.unwrap();

    // Verify status is Rejected with reason
    match approval.status {
        ApprovalStatus::Rejected { by, reason, at: _ } => {
            assert_eq!(by, "risk-manager");
            assert_eq!(reason, Some("Exceeds daily trading limit".to_string()));
        }
        _ => panic!("Expected Rejected status, got {:?}", approval.status),
    }

    // Verify audit log has rejection
    let audit_log = service.store().get_audit_log(&approval_id).unwrap();
    assert!(audit_log.iter().any(|e| e.action == "rejected"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Multiple Pending Requests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_multiple_pending_requests() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    // Create multiple requests with different approvers
    create_stored_request(store, "req-1", vec!["admin".to_string()], "telegram");
    create_stored_request(
        store,
        "req-2",
        vec!["admin".to_string(), "reviewer".to_string()],
        "slack",
    );
    create_stored_request(store, "req-3", vec!["reviewer".to_string()], "telegram");
    create_stored_request(store, "req-4", vec!["other-user".to_string()], "slack");

    let app = hitl_routes(service.clone());

    // List all pending requests
    let (status, response): (_, ListPendingResponse) =
        request_json(&app, Method::GET, "/pending", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.total, 4);
    assert_eq!(response.requests.len(), 4);

    // List pending filtered by admin approver
    let (status, response): (_, ListPendingResponse) =
        request_json(&app, Method::GET, "/pending?approver_id=admin", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.total, 2); // req-1 and req-2
    assert!(response.requests.iter().all(|r| r.approvers.contains(&"admin".to_string())));

    // List pending filtered by reviewer approver
    let (status, response): (_, ListPendingResponse) =
        request_json(&app, Method::GET, "/pending?approver_id=reviewer", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.total, 2); // req-2 and req-3

    // Approve req-1, reject req-2
    let decide_approve = DecideRequest {
        decided_by: "admin".to_string(),
        approved: true,
        reason: None,
    };

    let (status, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/req-1/decide",
        Some(serde_json::to_value(&decide_approve).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let decide_reject = DecideRequest {
        decided_by: "admin".to_string(),
        approved: false,
        reason: Some("Not needed".to_string()),
    };

    let (status, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/req-2/decide",
        Some(serde_json::to_value(&decide_reject).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify final states
    let req1 = store.get("req-1").unwrap().unwrap();
    assert!(matches!(req1.status, ApprovalStatus::Approved { .. }));

    let req2 = store.get("req-2").unwrap().unwrap();
    assert!(matches!(req2.status, ApprovalStatus::Rejected { .. }));

    let req3 = store.get("req-3").unwrap().unwrap();
    assert!(matches!(req3.status, ApprovalStatus::Pending));

    let req4 = store.get("req-4").unwrap().unwrap();
    assert!(matches!(req4.status, ApprovalStatus::Pending));

    // List pending should now only show 2
    let (status, response): (_, ListPendingResponse) =
        request_json(&app, Method::GET, "/pending", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.total, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Channel Callback Routing
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_channel_callback_routing() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    // Create requests for different channels
    let channels = ["telegram", "feishu", "slack", "dingtalk"];
    for (i, channel) in channels.iter().enumerate() {
        create_stored_request(
            store,
            &format!("callback-{}", i),
            vec!["approver".to_string()],
            channel,
        );
    }

    let app = hitl_routes(service.clone());

    // Send callbacks for each channel
    for (i, channel) in channels.iter().enumerate() {
        let callback_data = CallbackData {
            request_id: format!("callback-{}", i),
            action: CallbackAction::Approve,
            user_id: "approver".to_string(),
            platform_callback_id: format!("{}-cb-{}", channel, i),
        };

        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/callback/{}", channel))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Callback failed for channel: {}",
            channel
        );
    }

    // Verify all requests were approved
    for i in 0..channels.len() {
        let req = store.get(&format!("callback-{}", i)).unwrap().unwrap();
        assert!(
            matches!(req.status, ApprovalStatus::Approved { .. }),
            "Request callback-{} was not approved",
            i
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Unauthorized Approver
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_unauthorized_approver() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    // Create request with specific approvers
    create_stored_request(
        store,
        "auth-test",
        vec!["admin".to_string(), "security-team".to_string()],
        "telegram",
    );

    let app = hitl_routes(service.clone());

    // Attempt decision from non-approver
    let decide_request = DecideRequest {
        decided_by: "unauthorized-user".to_string(),
        approved: true,
        reason: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/auth-test/decide",
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;

    // Should return FORBIDDEN
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("not authorized"));

    // Verify request is still pending
    let req = store.get("auth-test").unwrap().unwrap();
    assert!(matches!(req.status, ApprovalStatus::Pending));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: Already Decided Request (Idempotency)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_already_decided_request() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    // Create and approve a request
    create_stored_request(store, "already-decided", vec!["admin".to_string()], "slack");

    let app = hitl_routes(service.clone());

    // First approval
    let decide_request = DecideRequest {
        decided_by: "admin".to_string(),
        approved: true,
        reason: None,
    };

    let (status, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/already-decided/decide",
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Try to reject the same request
    let reject_request = DecideRequest {
        decided_by: "admin".to_string(),
        approved: false,
        reason: Some("Changed my mind".to_string()),
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/already-decided/decide",
        Some(serde_json::to_value(&reject_request).unwrap()),
    )
    .await;

    // Should return BAD_REQUEST with "already has terminal status"
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("terminal status"));

    // Verify request is still approved (not changed to rejected)
    let req = store.get("already-decided").unwrap().unwrap();
    assert!(matches!(req.status, ApprovalStatus::Approved { .. }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: Callback with Rejection and Reason
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_callback_rejection_with_reason() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    create_stored_request(
        store,
        "callback-reject",
        vec!["reviewer".to_string()],
        "feishu",
    );

    let app = hitl_routes(service.clone());

    // Send rejection callback with reason
    let callback_data = CallbackData {
        request_id: "callback-reject".to_string(),
        action: CallbackAction::Reject {
            reason: Some("Code quality does not meet standards".to_string()),
        },
        user_id: "reviewer".to_string(),
        platform_callback_id: "feishu-cb-123".to_string(),
    };

    let request = Request::builder()
        .method(Method::POST)
        .uri("/callback/feishu")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify rejection with reason
    let req = store.get("callback-reject").unwrap().unwrap();
    match req.status {
        ApprovalStatus::Rejected { by, reason, .. } => {
            assert_eq!(by, "reviewer");
            assert_eq!(
                reason,
                Some("Code quality does not meet standards".to_string())
            );
        }
        _ => panic!("Expected Rejected status"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 8: Unsupported Channel Callback
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_unsupported_channel_callback() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    let callback_data = CallbackData {
        request_id: "test".to_string(),
        action: CallbackAction::Approve,
        user_id: "user".to_string(),
        platform_callback_id: "cb".to_string(),
    };

    let request = Request::builder()
        .method(Method::POST)
        .uri("/callback/unsupported_channel")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 9: Create Request with Various Approval Types
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_various_approval_types() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Test ConfigChange
    let config_request = CreateApprovalRequest {
        approval_type: ApprovalType::ConfigChange {
            key: "max_connections".to_string(),
            old_value: "100".to_string(),
            new_value: "200".to_string(),
        },
        requester: "devops".to_string(),
        approvers: vec!["sre".to_string()],
        title: "Increase connection limit".to_string(),
        description: None,
        channel: "slack".to_string(),
        metadata: json!({}),
        ttl_seconds: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&config_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(response.success);

    // Test HighCostOperation
    let cost_request = CreateApprovalRequest {
        approval_type: ApprovalType::HighCostOperation {
            operation: "Deploy GPU cluster".to_string(),
            estimated_cost: 5000.0,
        },
        requester: "ml-team".to_string(),
        approvers: vec!["finance".to_string()],
        title: "Deploy GPU cluster for training".to_string(),
        description: Some("Required for model training".to_string()),
        channel: "telegram".to_string(),
        metadata: json!({"department": "ML"}),
        ttl_seconds: Some(7200),
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&cost_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(response.success);
    let approval = response.approval.unwrap();
    match approval.approval_type {
        ApprovalType::HighCostOperation {
            operation,
            estimated_cost,
        } => {
            assert_eq!(operation, "Deploy GPU cluster");
            assert!((estimated_cost - 5000.0).abs() < f64::EPSILON);
        }
        _ => panic!("Expected HighCostOperation type"),
    }

    // Test RiskOperation
    let risk_request = CreateApprovalRequest {
        approval_type: ApprovalType::RiskOperation {
            description: "Delete production database".to_string(),
            risk_level: RiskLevel::Critical,
        },
        requester: "dba".to_string(),
        approvers: vec!["cto".to_string(), "security".to_string()],
        title: "Database cleanup".to_string(),
        description: None,
        channel: "dingtalk".to_string(),
        metadata: json!({}),
        ttl_seconds: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&risk_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(response.success);
    let approval = response.approval.unwrap();
    match approval.approval_type {
        ApprovalType::RiskOperation {
            description,
            risk_level,
        } => {
            assert_eq!(description, "Delete production database");
            assert_eq!(risk_level, RiskLevel::Critical);
        }
        _ => panic!("Expected RiskOperation type"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 10: Request Not Found
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_request_not_found() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Get nonexistent request
    let (status, response): (_, ApprovalResponse) =
        request_json(&app, Method::GET, "/nonexistent-id", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("not found"));

    // Decide on nonexistent request
    let decide_request = DecideRequest {
        decided_by: "admin".to_string(),
        approved: true,
        reason: None,
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/nonexistent-id/decide",
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!response.success);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 11: Validation Errors
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_validation_errors() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Empty title
    let invalid_request = json!({
        "approval_type": {
            "type": "merge_request",
            "platform": "github",
            "repo": "test/repo",
            "mr_id": 1
        },
        "requester": "user",
        "approvers": ["admin"],
        "title": "",
        "channel": "telegram"
    });

    let (status, response): (_, ApprovalResponse) =
        request_json(&app, Method::POST, "/request", Some(invalid_request)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("Title"));

    // Empty approvers
    let invalid_request = json!({
        "approval_type": {
            "type": "merge_request",
            "platform": "github",
            "repo": "test/repo",
            "mr_id": 1
        },
        "requester": "user",
        "approvers": [],
        "title": "Test",
        "channel": "telegram"
    });

    let (status, response): (_, ApprovalResponse) =
        request_json(&app, Method::POST, "/request", Some(invalid_request)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("approver"));

    // Unsupported channel
    let invalid_request = json!({
        "approval_type": {
            "type": "merge_request",
            "platform": "github",
            "repo": "test/repo",
            "mr_id": 1
        },
        "requester": "user",
        "approvers": ["admin"],
        "title": "Test",
        "channel": "unsupported"
    });

    let (status, response): (_, ApprovalResponse) =
        request_json(&app, Method::POST, "/request", Some(invalid_request)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!response.success);
    assert!(response.error.unwrap().contains("Unsupported channel"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 12: Concurrent Decisions by Multiple Approvers
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_decisions() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    // Create request with multiple approvers
    create_stored_request(
        store,
        "concurrent-test",
        vec![
            "approver1".to_string(),
            "approver2".to_string(),
            "approver3".to_string(),
        ],
        "telegram",
    );

    let app = hitl_routes(service.clone());

    // First approver approves
    let decide1 = DecideRequest {
        decided_by: "approver1".to_string(),
        approved: true,
        reason: None,
    };

    let (status, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/concurrent-test/decide",
        Some(serde_json::to_value(&decide1).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Second approver tries to reject - should fail (already decided)
    let decide2 = DecideRequest {
        decided_by: "approver2".to_string(),
        approved: false,
        reason: Some("Too late".to_string()),
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/concurrent-test/decide",
        Some(serde_json::to_value(&decide2).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(response.error.unwrap().contains("terminal status"));

    // Third approver also fails
    let decide3 = DecideRequest {
        decided_by: "approver3".to_string(),
        approved: true,
        reason: None,
    };

    let (status, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/concurrent-test/decide",
        Some(serde_json::to_value(&decide3).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Verify only first approval is recorded
    let req = store.get("concurrent-test").unwrap().unwrap();
    match req.status {
        ApprovalStatus::Approved { by, .. } => {
            assert_eq!(by, "approver1");
        }
        _ => panic!("Expected Approved by approver1"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 13: TTL and Expiration
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_request_with_ttl() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Create request with TTL
    let create_request = CreateApprovalRequest {
        approval_type: ApprovalType::MergeRequest {
            platform: "github".to_string(),
            repo: "test/repo".to_string(),
            mr_id: 999,
        },
        requester: "developer".to_string(),
        approvers: vec!["admin".to_string()],
        title: "Time-sensitive request".to_string(),
        description: None,
        channel: "telegram".to_string(),
        metadata: json!({}),
        ttl_seconds: Some(3600), // 1 hour
    };

    let (status, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&create_request).unwrap()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let approval = response.approval.unwrap();

    // Verify expires_at is set
    assert!(approval.expires_at.is_some());

    // Verify expiration is approximately 1 hour from now
    let expires_at = approval.expires_at.unwrap();
    let now = Utc::now();
    let diff = expires_at.signed_duration_since(now);
    assert!(diff.num_seconds() > 3500 && diff.num_seconds() <= 3600);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 14: Get Request Details
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_request_details() {
    let (service, _dir) = setup_test_service();
    let store = service.store();

    create_stored_request(store, "detail-test", vec!["admin".to_string()], "slack");

    let app = hitl_routes(service.clone());

    let (status, response): (_, ApprovalResponse) =
        request_json(&app, Method::GET, "/detail-test", None).await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.success);

    let approval = response.approval.unwrap();
    assert_eq!(approval.id, "detail-test");
    assert_eq!(approval.channel, "slack");
    assert!(approval.approvers.contains(&"admin".to_string()));
    assert!(matches!(approval.status, ApprovalStatus::Pending));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 15: Audit Trail Completeness
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_trail_completeness() {
    let (service, _dir) = setup_test_service();
    let app = hitl_routes(service.clone());

    // Create request
    let create_request = CreateApprovalRequest {
        approval_type: ApprovalType::ConfigChange {
            key: "feature_flag".to_string(),
            old_value: "false".to_string(),
            new_value: "true".to_string(),
        },
        requester: "pm".to_string(),
        approvers: vec!["tech-lead".to_string()],
        title: "Enable feature flag".to_string(),
        description: Some("Rolling out new feature".to_string()),
        channel: "feishu".to_string(),
        metadata: json!({}),
        ttl_seconds: None,
    };

    let (_, response): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        "/request",
        Some(serde_json::to_value(&create_request).unwrap()),
    )
    .await;

    let approval_id = response.approval.unwrap().id;

    // Decide on request
    let decide_request = DecideRequest {
        decided_by: "tech-lead".to_string(),
        approved: true,
        reason: None,
    };

    let (_, _): (_, ApprovalResponse) = request_json(
        &app,
        Method::POST,
        &format!("/{}/decide", approval_id),
        Some(serde_json::to_value(&decide_request).unwrap()),
    )
    .await;

    // Check audit trail
    let audit_log = service.store().get_audit_log(&approval_id).unwrap();

    // Should have at least: created, message_sent, approved
    assert!(audit_log.len() >= 2);

    // Verify created entry
    let created_entry = audit_log.iter().find(|e| e.action == "created").unwrap();
    assert_eq!(created_entry.actor_id, "pm");
    assert_eq!(created_entry.request_id, approval_id);

    // Verify approved entry
    let approved_entry = audit_log.iter().find(|e| e.action == "approved").unwrap();
    assert_eq!(approved_entry.actor_id, "tech-lead");
}
