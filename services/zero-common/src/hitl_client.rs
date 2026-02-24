//! Human-in-the-Loop (HitL) client library for workflow integration.
//!
//! This module provides a client for interacting with the HitL API in zero-gateway,
//! allowing other services to create approval requests, check status, and wait for decisions.
//!
//! # Example
//!
//! ```rust,ignore
//! use zero_common::hitl_client::{HitLClient, CreateApprovalRequest, ApprovalType};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = HitLClient::new("http://localhost:4430");
//!
//!     // Create an approval request
//!     let request = CreateApprovalRequest {
//!         approval_type: ApprovalType::RiskOperation {
//!             description: "Delete production data".to_string(),
//!             risk_level: RiskLevel::Critical,
//!         },
//!         requester: "workflow-123".to_string(),
//!         approvers: vec!["admin@example.com".to_string()],
//!         title: "Delete old data".to_string(),
//!         description: Some("Cleanup operation".to_string()),
//!         channel: "telegram".to_string(),
//!         metadata: serde_json::json!({}),
//!         ttl_seconds: Some(3600),
//!     };
//!
//!     let response = client.create_request(request).await?;
//!     let approval_id = response.approval.unwrap().id;
//!
//!     // Wait for decision with polling
//!     let status = client.wait_for_decision(&approval_id, Duration::from_secs(5)).await?;
//!
//!     Ok(())
//! }
//! ```

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for HitL client operations.
#[derive(Error, Debug)]
pub enum HitLClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Request(String),

    /// Server returned an error response
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    Parse(String),

    /// Approval request not found
    #[error("Approval request not found: {0}")]
    NotFound(String),

    /// Operation timed out
    #[error("Operation timed out waiting for decision")]
    Timeout,

    /// Request was cancelled
    #[error("Request was cancelled: {0}")]
    Cancelled(String),
}

/// Result type for HitL client operations.
pub type Result<T> = std::result::Result<T, HitLClientError>;

/// Risk level for operations requiring approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RiskLevel {
    /// Low risk - minimal impact if operation fails
    Low = 1,
    /// Medium risk - moderate impact, reversible
    Medium = 2,
    /// High risk - significant impact, may be difficult to reverse
    High = 3,
    /// Critical risk - severe impact, irreversible
    Critical = 4,
}

/// Type of operation requiring approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalType {
    /// Merge request approval for code changes
    MergeRequest {
        platform: String,
        repo: String,
        mr_id: i64,
    },
    /// Trading command approval
    TradingCommand {
        asset: String,
        action: String,
        amount: f64,
    },
    /// Configuration change approval
    ConfigChange {
        key: String,
        old_value: String,
        new_value: String,
    },
    /// High-cost operation approval
    HighCostOperation {
        operation: String,
        estimated_cost: f64,
    },
    /// Risk operation approval
    RiskOperation {
        description: String,
        risk_level: RiskLevel,
    },
}

/// Status of an approval request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Request is pending review
    Pending,
    /// Request was approved
    Approved {
        by: String,
        at: DateTime<Utc>,
    },
    /// Request was rejected
    Rejected {
        by: String,
        reason: Option<String>,
        at: DateTime<Utc>,
    },
    /// Request was cancelled
    Cancelled {
        reason: String,
    },
}

impl ApprovalStatus {
    /// Returns true if the status is terminal (no further state changes possible).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, ApprovalStatus::Pending)
    }

    /// Returns true if the request was approved.
    pub fn is_approved(&self) -> bool {
        matches!(self, ApprovalStatus::Approved { .. })
    }

    /// Returns true if the request was rejected.
    pub fn is_rejected(&self) -> bool {
        matches!(self, ApprovalStatus::Rejected { .. })
    }

    /// Returns true if the request was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, ApprovalStatus::Cancelled { .. })
    }
}

/// An approval request in the HitL system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique identifier for the request
    pub id: String,
    /// Type of operation requiring approval
    pub approval_type: ApprovalType,
    /// Current status of the request
    pub status: ApprovalStatus,
    /// User or system that initiated the request
    pub requester: String,
    /// Users who can approve this request
    pub approvers: Vec<String>,
    /// Human-readable title for the request
    pub title: String,
    /// Detailed description of the operation
    pub description: Option<String>,
    /// IM channel where the request was sent
    pub channel: String,
    /// Message ID in the IM channel for reference
    pub message_id: Option<String>,
    /// Additional metadata as key-value pairs
    pub metadata: serde_json::Value,
    /// Request creation timestamp
    pub created_at: DateTime<Utc>,
    /// Request last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Request expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request payload to create a new approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApprovalRequest {
    /// Type of operation requiring approval
    pub approval_type: ApprovalType,
    /// User or system that initiated the request
    pub requester: String,
    /// Users who can approve this request
    pub approvers: Vec<String>,
    /// Human-readable title for the request
    pub title: String,
    /// Detailed description of the operation
    #[serde(default)]
    pub description: Option<String>,
    /// IM channel to send the request to
    pub channel: String,
    /// Additional metadata as key-value pairs
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
    /// Time-to-live in seconds
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

fn default_metadata() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Response after creating or retrieving an approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// The approval request (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<ApprovalRequest>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request to cancel an approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CancelRequest {
    decided_by: String,
    approved: bool,
    reason: Option<String>,
}

/// HTTP client for interacting with the HitL API.
#[derive(Clone)]
pub struct HitLClient {
    endpoint: String,
    client: reqwest::Client,
}

impl HitLClient {
    /// Create a new HitL client pointing to the given gateway endpoint.
    ///
    /// # Arguments
    ///
    /// * `gateway_endpoint` - The base URL of the zero-gateway, e.g., "http://localhost:4430"
    pub fn new(gateway_endpoint: &str) -> Self {
        let endpoint = gateway_endpoint.trim_end_matches('/').to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { endpoint, client }
    }

    /// Create a new HitL client with a custom reqwest client.
    ///
    /// Useful for testing or when custom client configuration is needed.
    pub fn with_client(gateway_endpoint: &str, client: reqwest::Client) -> Self {
        let endpoint = gateway_endpoint.trim_end_matches('/').to_string();
        Self { endpoint, client }
    }

    /// Create a new approval request.
    ///
    /// # Arguments
    ///
    /// * `req` - The approval request to create
    ///
    /// # Returns
    ///
    /// The created approval response containing the full ApprovalRequest with ID and timestamps.
    pub async fn create_request(&self, req: CreateApprovalRequest) -> Result<ApprovalResponse> {
        let url = format!("{}/api/v1/hitl/request", self.endpoint);

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| HitLClientError::Request(e.to_string()))?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HitLClientError::Server {
                status,
                message: body,
            });
        }

        response
            .json::<ApprovalResponse>()
            .await
            .map_err(|e| HitLClientError::Parse(e.to_string()))
    }

    /// Check the status of an approval request.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the approval request
    ///
    /// # Returns
    ///
    /// The current status of the approval request.
    pub async fn check_status(&self, request_id: &str) -> Result<ApprovalStatus> {
        let url = format!("{}/api/v1/hitl/{}", self.endpoint, request_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| HitLClientError::Request(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 404 {
            return Err(HitLClientError::NotFound(request_id.to_string()));
        }

        if !response.status().is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HitLClientError::Server {
                status,
                message: body,
            });
        }

        let approval_response: ApprovalResponse = response
            .json()
            .await
            .map_err(|e| HitLClientError::Parse(e.to_string()))?;

        approval_response
            .approval
            .map(|a| a.status)
            .ok_or_else(|| HitLClientError::NotFound(request_id.to_string()))
    }

    /// Get the full approval request details.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the approval request
    ///
    /// # Returns
    ///
    /// The full approval request with all details.
    pub async fn get_request(&self, request_id: &str) -> Result<ApprovalRequest> {
        let url = format!("{}/api/v1/hitl/{}", self.endpoint, request_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| HitLClientError::Request(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 404 {
            return Err(HitLClientError::NotFound(request_id.to_string()));
        }

        if !response.status().is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HitLClientError::Server {
                status,
                message: body,
            });
        }

        let approval_response: ApprovalResponse = response
            .json()
            .await
            .map_err(|e| HitLClientError::Parse(e.to_string()))?;

        approval_response
            .approval
            .ok_or_else(|| HitLClientError::NotFound(request_id.to_string()))
    }

    /// Cancel an approval request.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the approval request to cancel
    /// * `reason` - The reason for cancellation
    ///
    /// # Returns
    ///
    /// Ok(()) if the cancellation was successful.
    pub async fn cancel(&self, request_id: &str, reason: &str) -> Result<()> {
        let url = format!("{}/api/v1/hitl/{}/decide", self.endpoint, request_id);

        let cancel_request = CancelRequest {
            decided_by: "system".to_string(),
            approved: false,
            reason: Some(format!("Cancelled: {}", reason)),
        };

        let response = self
            .client
            .post(&url)
            .json(&cancel_request)
            .send()
            .await
            .map_err(|e| HitLClientError::Request(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 404 {
            return Err(HitLClientError::NotFound(request_id.to_string()));
        }

        if !response.status().is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HitLClientError::Server {
                status,
                message: body,
            });
        }

        Ok(())
    }

    /// Wait for a decision on an approval request with polling.
    ///
    /// This method polls the API at the specified interval until a terminal
    /// status is reached (approved, rejected, or cancelled).
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the approval request
    /// * `poll_interval` - How often to poll for status updates
    ///
    /// # Returns
    ///
    /// The final status when a decision is made, or an error if cancelled.
    pub async fn wait_for_decision(
        &self,
        request_id: &str,
        poll_interval: Duration,
    ) -> Result<ApprovalStatus> {
        loop {
            let status = self.check_status(request_id).await?;

            if status.is_terminal() {
                if let ApprovalStatus::Cancelled { reason } = &status {
                    return Err(HitLClientError::Cancelled(reason.clone()));
                }
                return Ok(status);
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Wait for a decision with a maximum timeout.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the approval request
    /// * `poll_interval` - How often to poll for status updates
    /// * `timeout` - Maximum time to wait for a decision
    ///
    /// # Returns
    ///
    /// The final status when a decision is made, or a timeout error.
    pub async fn wait_for_decision_with_timeout(
        &self,
        request_id: &str,
        poll_interval: Duration,
        timeout: Duration,
    ) -> Result<ApprovalStatus> {
        tokio::time::timeout(timeout, self.wait_for_decision(request_id, poll_interval))
            .await
            .map_err(|_| HitLClientError::Timeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_status_helpers() {
        // Test pending status
        let pending = ApprovalStatus::Pending;
        assert!(!pending.is_terminal());
        assert!(!pending.is_approved());
        assert!(!pending.is_rejected());
        assert!(!pending.is_cancelled());

        // Test approved status
        let approved = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: Utc::now(),
        };
        assert!(approved.is_terminal());
        assert!(approved.is_approved());
        assert!(!approved.is_rejected());
        assert!(!approved.is_cancelled());

        // Test rejected status
        let rejected = ApprovalStatus::Rejected {
            by: "admin".to_string(),
            reason: Some("Invalid request".to_string()),
            at: Utc::now(),
        };
        assert!(rejected.is_terminal());
        assert!(!rejected.is_approved());
        assert!(rejected.is_rejected());
        assert!(!rejected.is_cancelled());

        // Test cancelled status
        let cancelled = ApprovalStatus::Cancelled {
            reason: "Timeout".to_string(),
        };
        assert!(cancelled.is_terminal());
        assert!(!cancelled.is_approved());
        assert!(!cancelled.is_rejected());
        assert!(cancelled.is_cancelled());
    }

    #[test]
    fn test_create_approval_request_serialization() {
        let request = CreateApprovalRequest {
            approval_type: ApprovalType::RiskOperation {
                description: "Delete data".to_string(),
                risk_level: RiskLevel::Critical,
            },
            requester: "workflow-123".to_string(),
            approvers: vec!["admin@example.com".to_string()],
            title: "Delete old data".to_string(),
            description: Some("Cleanup operation".to_string()),
            channel: "telegram".to_string(),
            metadata: serde_json::json!({"workflow_id": "wf-123"}),
            ttl_seconds: Some(3600),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"risk_operation\""));
        assert!(json.contains("\"risk_level\":\"Critical\""));
        assert!(json.contains("\"requester\":\"workflow-123\""));

        // Test deserialization roundtrip
        let deserialized: CreateApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.requester, "workflow-123");
        assert_eq!(deserialized.ttl_seconds, Some(3600));
    }

    #[test]
    fn test_approval_type_variants() {
        // Test MergeRequest
        let merge = ApprovalType::MergeRequest {
            platform: "github".to_string(),
            repo: "org/repo".to_string(),
            mr_id: 42,
        };
        let json = serde_json::to_string(&merge).unwrap();
        assert!(json.contains("\"type\":\"merge_request\""));

        // Test TradingCommand
        let trade = ApprovalType::TradingCommand {
            asset: "BTC".to_string(),
            action: "buy".to_string(),
            amount: 1.5,
        };
        let json = serde_json::to_string(&trade).unwrap();
        assert!(json.contains("\"type\":\"trading_command\""));

        // Test ConfigChange
        let config = ApprovalType::ConfigChange {
            key: "max_tokens".to_string(),
            old_value: "1000".to_string(),
            new_value: "2000".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"config_change\""));

        // Test HighCostOperation
        let cost = ApprovalType::HighCostOperation {
            operation: "deploy_cluster".to_string(),
            estimated_cost: 1500.0,
        };
        let json = serde_json::to_string(&cost).unwrap();
        assert!(json.contains("\"type\":\"high_cost_operation\""));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_hitl_client_endpoint_normalization() {
        let client1 = HitLClient::new("http://localhost:4430");
        assert_eq!(client1.endpoint, "http://localhost:4430");

        let client2 = HitLClient::new("http://localhost:4430/");
        assert_eq!(client2.endpoint, "http://localhost:4430");

        let client3 = HitLClient::new("http://localhost:4430///");
        assert_eq!(client3.endpoint, "http://localhost:4430");
    }

    #[test]
    fn test_approval_response_serialization() {
        // Test success response
        let approval = ApprovalRequest {
            id: "test-123".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "test".to_string(),
                risk_level: RiskLevel::Low,
            },
            status: ApprovalStatus::Pending,
            requester: "user".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Test".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        };

        let response = ApprovalResponse {
            success: true,
            approval: Some(approval),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"id\":\"test-123\""));
        assert!(!json.contains("\"error\"")); // Should be skipped

        // Test error response
        let error_response = ApprovalResponse {
            success: false,
            approval: None,
            error: Some("Request failed".to_string()),
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Request failed\""));
        assert!(!json.contains("\"approval\"")); // Should be skipped
    }

    #[test]
    fn test_error_display() {
        let request_err = HitLClientError::Request("connection refused".to_string());
        assert!(request_err.to_string().contains("connection refused"));

        let server_err = HitLClientError::Server {
            status: 500,
            message: "Internal server error".to_string(),
        };
        assert!(server_err.to_string().contains("500"));
        assert!(server_err.to_string().contains("Internal server error"));

        let not_found = HitLClientError::NotFound("req-123".to_string());
        assert!(not_found.to_string().contains("req-123"));

        let timeout = HitLClientError::Timeout;
        assert!(timeout.to_string().contains("timed out"));

        let cancelled = HitLClientError::Cancelled("User requested".to_string());
        assert!(cancelled.to_string().contains("User requested"));
    }

    // Integration-style tests (would need a mock server in practice)
    // These tests verify the URL construction and request structure

    #[tokio::test]
    async fn test_client_url_construction() {
        let client = HitLClient::new("http://localhost:4430");

        // We can't actually make requests without a server, but we can verify
        // the client is constructed correctly
        assert_eq!(client.endpoint, "http://localhost:4430");
    }
}
