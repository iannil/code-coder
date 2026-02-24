//! HTTP routes for the HitL approval system.
//!
//! Provides REST API endpoints for creating, querying, and managing approvals.

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::cards::{CallbackAction, CardRenderer};
use super::store::HitLStore;
use super::{ApprovalRequest, ApprovalResponse, ApprovalStatus, CreateApprovalRequest};

/// HitL service that manages approval requests and card renderers.
pub struct HitLService {
    store: Arc<HitLStore>,
    renderers: HashMap<String, Arc<dyn CardRenderer>>,
}

impl HitLService {
    /// Create a new HitL service with the given store and renderers.
    pub fn new(
        store: Arc<HitLStore>,
        renderers: HashMap<String, Arc<dyn CardRenderer>>,
    ) -> Self {
        Self { store, renderers }
    }

    /// Get the store reference.
    pub fn store(&self) -> &Arc<HitLStore> {
        &self.store
    }

    /// Get a renderer by channel type.
    pub fn get_renderer(&self, channel: &str) -> Option<Arc<dyn CardRenderer>> {
        self.renderers.get(channel).cloned()
    }
}

/// Query parameters for listing pending approvals.
#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    /// Filter by approver ID (optional)
    #[serde(default)]
    pub approver_id: Option<String>,
}

/// Response for list pending approvals endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListPendingResponse {
    pub requests: Vec<ApprovalRequest>,
    pub total: usize,
}

/// Request body for the decide endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct DecideRequest {
    /// User making the decision
    pub decided_by: String,
    /// Whether to approve (true) or reject (false)
    pub approved: bool,
    /// Optional reason for rejection
    #[serde(default)]
    pub reason: Option<String>,
}

/// Build the HitL API routes.
pub fn hitl_routes(service: Arc<HitLService>) -> Router {
    Router::new()
        .route("/request", post(create_request))
        .route("/pending", get(list_pending))
        .route("/:id", get(get_request))
        .route("/:id/decide", post(decide))
        .route("/callback/:channel", post(handle_callback))
        .with_state(service)
}

/// Create a new approval request.
///
/// POST /api/v1/hitl/request
async fn create_request(
    State(service): State<Arc<HitLService>>,
    Json(request): Json<CreateApprovalRequest>,
) -> Result<(StatusCode, Json<ApprovalResponse>), (StatusCode, Json<ApprovalResponse>)> {
    // Validate required fields
    if request.title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApprovalResponse::error("Title is required")),
        ));
    }

    if request.approvers.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApprovalResponse::error("At least one approver is required")),
        ));
    }

    if request.channel.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApprovalResponse::error("Channel is required")),
        ));
    }

    // Check if renderer exists for the channel
    let renderer = service.get_renderer(&request.channel);
    if renderer.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApprovalResponse::error(format!(
                "Unsupported channel: {}",
                request.channel
            ))),
        ));
    }

    // Generate request ID and timestamps
    let now = Utc::now();
    let expires_at = request.ttl_seconds.map(|ttl| now + chrono::Duration::seconds(ttl as i64));

    let approval = ApprovalRequest {
        id: Uuid::new_v4().to_string(),
        approval_type: request.approval_type,
        status: ApprovalStatus::Pending,
        requester: request.requester,
        approvers: request.approvers,
        title: request.title,
        description: request.description,
        channel: request.channel.clone(),
        message_id: None,
        metadata: request.metadata,
        created_at: now,
        updated_at: now,
        expires_at,
    };

    // Store the request
    if let Err(e) = service.store.create(&approval) {
        tracing::error!(error = %e, "Failed to create approval request");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApprovalResponse::error("Failed to create approval request")),
        ));
    }

    // Send approval card to IM channel
    let renderer = renderer.unwrap();
    match renderer.send_approval_card(&approval, &request.channel).await {
        Ok(message_id) => {
            // Update the stored request with message ID
            if let Err(e) = service.store.update_message_id(&approval.id, &message_id) {
                tracing::warn!(error = %e, "Failed to update message ID");
            }

            // Fetch the updated approval
            match service.store.get(&approval.id) {
                Ok(Some(updated_approval)) => {
                    Ok((StatusCode::CREATED, Json(ApprovalResponse::success(updated_approval))))
                }
                _ => {
                    Ok((StatusCode::CREATED, Json(ApprovalResponse::success(approval))))
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to send approval card");
            // Still return success since the request was created
            // The card sending can be retried later
            Ok((StatusCode::CREATED, Json(ApprovalResponse::success(approval))))
        }
    }
}

/// List pending approval requests.
///
/// GET /api/v1/hitl/pending
async fn list_pending(
    State(service): State<Arc<HitLService>>,
    Query(query): Query<PendingQuery>,
) -> Result<Json<ListPendingResponse>, (StatusCode, Json<ApprovalResponse>)> {
    match service.store.list_pending(query.approver_id.as_deref()) {
        Ok(requests) => {
            let total = requests.len();
            Ok(Json(ListPendingResponse { requests, total }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list pending requests");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApprovalResponse::error("Failed to list pending requests")),
            ))
        }
    }
}

/// Get a specific approval request by ID.
///
/// GET /api/v1/hitl/:id
async fn get_request(
    State(service): State<Arc<HitLService>>,
    Path(id): Path<String>,
) -> Result<Json<ApprovalResponse>, (StatusCode, Json<ApprovalResponse>)> {
    match service.store.get(&id) {
        Ok(Some(approval)) => Ok(Json(ApprovalResponse::success(approval))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApprovalResponse::error(format!("Approval request '{}' not found", id))),
        )),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get approval request");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApprovalResponse::error("Failed to get approval request")),
            ))
        }
    }
}

/// Process a decision on an approval request.
///
/// POST /api/v1/hitl/:id/decide
async fn decide(
    State(service): State<Arc<HitLService>>,
    Path(id): Path<String>,
    Json(request): Json<DecideRequest>,
) -> Result<Json<ApprovalResponse>, (StatusCode, Json<ApprovalResponse>)> {
    // Get the current request
    let approval = match service.store.get(&id) {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApprovalResponse::error(format!("Approval request '{}' not found", id))),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get approval request");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApprovalResponse::error("Failed to get approval request")),
            ));
        }
    };

    // Check if already decided
    if approval.status.is_terminal() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApprovalResponse::error(format!(
                "Request already has terminal status: {}",
                approval.status.status_name()
            ))),
        ));
    }

    // Check if the user is an authorized approver
    if !approval.approvers.contains(&request.decided_by) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApprovalResponse::error(format!(
                "User '{}' is not authorized to approve this request",
                request.decided_by
            ))),
        ));
    }

    // Build the new status
    let now = Utc::now();
    let new_status = if request.approved {
        ApprovalStatus::Approved {
            by: request.decided_by.clone(),
            at: now,
        }
    } else {
        ApprovalStatus::Rejected {
            by: request.decided_by.clone(),
            reason: request.reason,
            at: now,
        }
    };

    // Update the status in store
    if let Err(e) = service.store.update_status(&id, &new_status) {
        tracing::error!(error = %e, "Failed to update approval status");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApprovalResponse::error("Failed to update approval status")),
        ));
    }

    // Fetch updated approval
    let updated_approval = match service.store.get(&id) {
        Ok(Some(a)) => a,
        _ => {
            // Construct the updated approval manually if fetch fails
            ApprovalRequest {
                status: new_status,
                updated_at: now,
                ..approval
            }
        }
    };

    // Update the card in the IM channel
    if let Some(message_id) = &updated_approval.message_id {
        if let Some(renderer) = service.get_renderer(&updated_approval.channel) {
            if let Err(e) = renderer.update_card(&updated_approval, message_id).await {
                tracing::warn!(error = %e, "Failed to update approval card");
            }
        }
    }

    Ok(Json(ApprovalResponse::success(updated_approval)))
}

/// Handle platform-specific callbacks from IM channels.
///
/// POST /api/v1/hitl/callback/:channel
async fn handle_callback(
    State(service): State<Arc<HitLService>>,
    Path(channel): Path<String>,
    body: Bytes,
) -> Result<Json<ApprovalResponse>, (StatusCode, Json<ApprovalResponse>)> {
    // Get the renderer for this channel
    let renderer = match service.get_renderer(&channel) {
        Some(r) => r,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApprovalResponse::error(format!("Unsupported channel: {}", channel))),
            ));
        }
    };

    // Parse the callback
    let callback_data = match renderer.parse_callback(&body) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse callback");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApprovalResponse::error("Failed to parse callback payload")),
            ));
        }
    };

    // Convert callback action to decide request
    let (approved, reason) = match callback_data.action {
        CallbackAction::Approve => (true, None),
        CallbackAction::Reject { reason } => (false, reason),
    };

    let decide_request = DecideRequest {
        decided_by: callback_data.user_id,
        approved,
        reason,
    };

    // Delegate to the decide handler logic
    decide(
        State(service),
        Path(callback_data.request_id),
        Json(decide_request),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitl::cards::CallbackData;
    use crate::hitl::{ApprovalType, RiskLevel};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tower::ServiceExt;

    /// Mock card renderer for testing.
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
            _request: &ApprovalRequest,
            _channel_id: &str,
        ) -> anyhow::Result<String> {
            Ok("mock-message-id".to_string())
        }

        async fn update_card(
            &self,
            _request: &ApprovalRequest,
            _message_id: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn parse_callback(&self, payload: &[u8]) -> anyhow::Result<CallbackData> {
            // Simple mock parsing - expect JSON
            let data: CallbackData = serde_json::from_slice(payload)?;
            Ok(data)
        }
    }

    fn create_test_service() -> (Arc<HitLService>, PathBuf, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_hitl.db");
        let store = Arc::new(HitLStore::new(&db_path).unwrap());

        let mut renderers: HashMap<String, Arc<dyn CardRenderer>> = HashMap::new();
        renderers.insert("telegram".to_string(), Arc::new(MockCardRenderer::new("telegram")));
        renderers.insert("slack".to_string(), Arc::new(MockCardRenderer::new("slack")));

        let service = Arc::new(HitLService::new(store, renderers));
        (service, db_path, dir)
    }

    fn create_test_approval_request() -> CreateApprovalRequest {
        CreateApprovalRequest {
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 42,
            },
            requester: "developer".to_string(),
            approvers: vec!["admin".to_string(), "reviewer".to_string()],
            title: "Test PR".to_string(),
            description: Some("Test description".to_string()),
            channel: "telegram".to_string(),
            metadata: serde_json::json!({}),
            ttl_seconds: None,
        }
    }

    #[tokio::test]
    async fn test_create_request_success() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let request_body = create_test_approval_request();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        assert!(response.approval.is_some());

        let approval = response.approval.unwrap();
        assert_eq!(approval.title, "Test PR");
        assert_eq!(approval.requester, "developer");
        assert!(matches!(approval.status, ApprovalStatus::Pending));
    }

    #[tokio::test]
    async fn test_create_request_empty_title() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let mut request_body = create_test_approval_request();
        request_body.title = "".to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_request_no_approvers() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let mut request_body = create_test_approval_request();
        request_body.approvers = vec![];

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_request_unsupported_channel() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let mut request_body = create_test_approval_request();
        request_body.channel = "unsupported".to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_pending_empty() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/pending")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ListPendingResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.total, 0);
        assert!(response.requests.is_empty());
    }

    #[tokio::test]
    async fn test_list_pending_with_requests() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-pending-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Pending Request".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/pending")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ListPendingResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.total, 1);
        assert_eq!(response.requests[0].id, "test-pending-1");
    }

    #[tokio::test]
    async fn test_list_pending_with_approver_filter() {
        let (service, _db_path, _dir) = create_test_service();

        // Create requests with different approvers
        let now = Utc::now();
        let approval1 = ApprovalRequest {
            id: "test-filter-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Request 1".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let approval2 = ApprovalRequest {
            id: "test-filter-2".to_string(),
            approvers: vec!["other-user".to_string()],
            title: "Request 2".to_string(),
            ..approval1.clone()
        };

        service.store.create(&approval1).unwrap();
        service.store.create(&approval2).unwrap();

        let app = hitl_routes(service.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/pending?approver_id=admin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ListPendingResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.total, 1);
        assert_eq!(response.requests[0].id, "test-filter-1");
    }

    #[tokio::test]
    async fn test_get_request_success() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-get-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Get Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/test-get-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        assert_eq!(approval.id, "test-get-1");
        assert_eq!(approval.title, "Get Test");
    }

    #[tokio::test]
    async fn test_get_request_not_found() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_decide_approve_success() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-decide-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Decide Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: Some("msg-123".to_string()),
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let decide_request = DecideRequest {
            decided_by: "admin".to_string(),
            approved: true,
            reason: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test-decide-1/decide")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&decide_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        match approval.status {
            ApprovalStatus::Approved { by, .. } => {
                assert_eq!(by, "admin");
            }
            _ => panic!("Expected Approved status"),
        }
    }

    #[tokio::test]
    async fn test_decide_reject_success() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-decide-2".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["reviewer".to_string()],
            title: "Reject Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let decide_request = DecideRequest {
            decided_by: "reviewer".to_string(),
            approved: false,
            reason: Some("Code quality issues".to_string()),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test-decide-2/decide")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&decide_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        match approval.status {
            ApprovalStatus::Rejected { by, reason, .. } => {
                assert_eq!(by, "reviewer");
                assert_eq!(reason, Some("Code quality issues".to_string()));
            }
            _ => panic!("Expected Rejected status"),
        }
    }

    #[tokio::test]
    async fn test_decide_not_found() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let decide_request = DecideRequest {
            decided_by: "admin".to_string(),
            approved: true,
            reason: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/nonexistent/decide")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&decide_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_decide_already_decided() {
        let (service, _db_path, _dir) = create_test_service();

        // Create an already approved request
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-decide-3".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Approved {
                by: "admin".to_string(),
                at: now,
            },
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Already Approved".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let decide_request = DecideRequest {
            decided_by: "admin".to_string(),
            approved: false,
            reason: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test-decide-3/decide")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&decide_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_decide_unauthorized_user() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request with specific approvers
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-decide-4".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Unauthorized Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        let decide_request = DecideRequest {
            decided_by: "unauthorized-user".to_string(),
            approved: true,
            reason: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test-decide-4/decide")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&decide_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_callback_approve() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-callback-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Callback Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        // Mock callback payload
        let callback_data = CallbackData {
            request_id: "test-callback-1".to_string(),
            action: CallbackAction::Approve,
            user_id: "admin".to_string(),
            platform_callback_id: "cb-123".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/callback/telegram")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        assert!(matches!(approval.status, ApprovalStatus::Approved { .. }));
    }

    #[tokio::test]
    async fn test_callback_reject() {
        let (service, _db_path, _dir) = create_test_service();

        // Create a request first
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-callback-2".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["reviewer".to_string()],
            title: "Callback Reject Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };
        service.store.create(&approval).unwrap();

        let app = hitl_routes(service.clone());

        // Mock callback payload with rejection
        let callback_data = CallbackData {
            request_id: "test-callback-2".to_string(),
            action: CallbackAction::Reject {
                reason: Some("Not ready".to_string()),
            },
            user_id: "reviewer".to_string(),
            platform_callback_id: "cb-456".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/callback/telegram")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        match approval.status {
            ApprovalStatus::Rejected { reason, .. } => {
                assert_eq!(reason, Some("Not ready".to_string()));
            }
            _ => panic!("Expected Rejected status"),
        }
    }

    #[tokio::test]
    async fn test_callback_unsupported_channel() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let callback_data = CallbackData {
            request_id: "test-1".to_string(),
            action: CallbackAction::Approve,
            user_id: "admin".to_string(),
            platform_callback_id: "cb-789".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/callback/unsupported")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&callback_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_request_with_ttl() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let mut request_body = create_test_approval_request();
        request_body.ttl_seconds = Some(3600); // 1 hour

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        assert!(approval.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_create_trading_command_request() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let request_body = CreateApprovalRequest {
            approval_type: ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 1.5,
            },
            requester: "trader".to_string(),
            approvers: vec!["risk-manager".to_string()],
            title: "Buy BTC".to_string(),
            description: Some("Market order".to_string()),
            channel: "slack".to_string(),
            metadata: serde_json::json!({"exchange": "binance"}),
            ttl_seconds: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        match approval.approval_type {
            ApprovalType::TradingCommand { asset, action, amount } => {
                assert_eq!(asset, "BTC");
                assert_eq!(action, "buy");
                assert!((amount - 1.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected TradingCommand type"),
        }
    }

    #[tokio::test]
    async fn test_create_risk_operation_request() {
        let (service, _db_path, _dir) = create_test_service();
        let app = hitl_routes(service.clone());

        let request_body = CreateApprovalRequest {
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production data".to_string(),
                risk_level: RiskLevel::Critical,
            },
            requester: "dba".to_string(),
            approvers: vec!["cto".to_string()],
            title: "Production Data Cleanup".to_string(),
            description: Some("Routine maintenance".to_string()),
            channel: "telegram".to_string(),
            metadata: serde_json::json!({}),
            ttl_seconds: Some(1800),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ApprovalResponse = serde_json::from_slice(&body).unwrap();

        assert!(response.success);
        let approval = response.approval.unwrap();
        match approval.approval_type {
            ApprovalType::RiskOperation { description, risk_level } => {
                assert_eq!(description, "Delete production data");
                assert_eq!(risk_level, RiskLevel::Critical);
            }
            _ => panic!("Expected RiskOperation type"),
        }
    }
}
