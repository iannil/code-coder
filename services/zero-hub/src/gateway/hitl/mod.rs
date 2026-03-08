//! Human-in-the-Loop (HitL) approval system for Zero Gateway.
//!
//! This module provides centralized approval workflow for critical operations
//! across multiple IM channels (Telegram, Feishu, Slack, DingTalk).
//!
//! ## Architecture
//!
//! ```text
//! Operation Request → HitL System → IM Channel → User Approval → Action Execution
//!                          ↓
//!                    Approval Store (persistence)
//! ```
//!
//! ## Approval Types
//!
//! - Merge Requests: Code review and merge approvals
//! - Trading Commands: High-value trading operations
//! - Config Changes: System configuration modifications
//! - High-Cost Operations: Operations with significant cost implications
//! - Risk Operations: Operations with elevated risk levels

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Stub modules for future implementation
pub mod actions;
pub mod cards;
pub mod routes;
pub mod store;

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

impl RiskLevel {
    /// Returns the numeric value of the risk level.
    pub fn value(self) -> u8 {
        self as u8
    }

    /// Returns the display name of the risk level.
    pub fn name(self) -> &'static str {
        match self {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        }
    }
}

/// Type of operation requiring approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalType {
    /// Merge request approval for code changes
    MergeRequest {
        /// Platform hosting the repository (e.g., "github", "gitlab")
        platform: String,
        /// Repository identifier
        repo: String,
        /// Merge request ID
        mr_id: i64,
    },
    /// Trading command approval
    TradingCommand {
        /// Asset being traded (e.g., "BTC", "ETH")
        asset: String,
        /// Trading action (e.g., "buy", "sell")
        action: String,
        /// Amount to trade
        amount: f64,
    },
    /// Configuration change approval
    ConfigChange {
        /// Configuration key being changed
        key: String,
        /// Previous value
        old_value: String,
        /// New value
        new_value: String,
    },
    /// High-cost operation approval
    HighCostOperation {
        /// Description of the operation
        operation: String,
        /// Estimated cost in base currency
        estimated_cost: f64,
    },
    /// Risk operation approval
    RiskOperation {
        /// Description of the risky operation
        description: String,
        /// Risk level assessment
        risk_level: RiskLevel,
    },
    /// Tool execution approval (for Hands autonomous execution)
    ToolExecution {
        /// Tool name being executed
        tool: String,
        /// Tool arguments (JSON)
        args: serde_json::Value,
        /// Risk level assessment
        risk_level: RiskLevel,
        /// Hand ID that requested this execution
        hand_id: String,
        /// Execution ID for tracking
        execution_id: String,
    },
}

impl ApprovalType {
    /// Returns the type name as a string for display and logging.
    pub fn type_name(&self) -> &'static str {
        match self {
            ApprovalType::MergeRequest { .. } => "merge_request",
            ApprovalType::TradingCommand { .. } => "trading_command",
            ApprovalType::ConfigChange { .. } => "config_change",
            ApprovalType::HighCostOperation { .. } => "high_cost_operation",
            ApprovalType::RiskOperation { .. } => "risk_operation",
            ApprovalType::ToolExecution { .. } => "tool_execution",
        }
    }
}

/// Status of an approval request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Request is pending review
    Pending,
    /// Request was approved
    Approved {
        /// User who approved the request
        by: String,
        /// Timestamp of approval
        at: DateTime<Utc>,
    },
    /// Request was rejected
    Rejected {
        /// User who rejected the request
        by: String,
        /// Optional reason for rejection
        reason: Option<String>,
        /// Timestamp of rejection
        at: DateTime<Utc>,
    },
    /// Request was cancelled
    Cancelled {
        /// Reason for cancellation
        reason: String,
    },
}

impl ApprovalStatus {
    /// Returns true if the status is terminal (no further state changes possible).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, ApprovalStatus::Pending)
    }

    /// Returns the status name as a string for display and logging.
    pub fn status_name(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved { .. } => "approved",
            ApprovalStatus::Rejected { .. } => "rejected",
            ApprovalStatus::Cancelled { .. } => "cancelled",
        }
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
    /// IM channel where the request was sent (e.g., "telegram", "slack")
    pub channel: String,
    /// Message ID in the IM channel for reference
    pub message_id: Option<String>,
    /// Additional metadata as key-value pairs
    pub metadata: serde_json::Value,
    /// Request creation timestamp
    pub created_at: DateTime<Utc>,
    /// Request last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Request expiration timestamp (after which it auto-cancels)
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
    /// Time-to-live in seconds (how long until the request expires)
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

impl ApprovalResponse {
    /// Create a successful response with the given approval.
    pub fn success(approval: ApprovalRequest) -> Self {
        Self {
            success: true,
            approval: Some(approval),
            error: None,
        }
    }

    /// Create an error response with the given message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            approval: None,
            error: Some(message.into()),
        }
    }
}

/// A decision made on an approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// ID of the approval request being decided
    pub approval_id: String,
    /// User making the decision
    pub decided_by: String,
    /// Whether to approve (true) or reject (false)
    pub approved: bool,
    /// Optional reason for rejection (required if approved is false)
    #[serde(default)]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_type_serialization() {
        // Test MergeRequest serialization
        let merge_request = ApprovalType::MergeRequest {
            platform: "github".to_string(),
            repo: "org/repo".to_string(),
            mr_id: 123,
        };
        let json = serde_json::to_string(&merge_request).unwrap();
        assert!(json.contains("\"type\":\"merge_request\""));
        assert!(json.contains("\"platform\":\"github\""));
        assert!(json.contains("\"mr_id\":123"));

        // Test deserialization roundtrip
        let deserialized: ApprovalType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, merge_request);

        // Test TradingCommand
        let trading = ApprovalType::TradingCommand {
            asset: "BTC".to_string(),
            action: "buy".to_string(),
            amount: 1.5,
        };
        let json = serde_json::to_string(&trading).unwrap();
        assert!(json.contains("\"type\":\"trading_command\""));
        let deserialized: ApprovalType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, trading);

        // Test ConfigChange
        let config = ApprovalType::ConfigChange {
            key: "max_tokens".to_string(),
            old_value: "1000".to_string(),
            new_value: "2000".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"config_change\""));
        let deserialized: ApprovalType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, config);

        // Test HighCostOperation
        let high_cost = ApprovalType::HighCostOperation {
            operation: "deploy_cluster".to_string(),
            estimated_cost: 1500.0,
        };
        let json = serde_json::to_string(&high_cost).unwrap();
        assert!(json.contains("\"type\":\"high_cost_operation\""));
        let deserialized: ApprovalType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, high_cost);

        // Test RiskOperation
        let risk = ApprovalType::RiskOperation {
            description: "Delete production data".to_string(),
            risk_level: RiskLevel::Critical,
        };
        let json = serde_json::to_string(&risk).unwrap();
        assert!(json.contains("\"type\":\"risk_operation\""));
        assert!(json.contains("\"risk_level\":\"Critical\""));
        let deserialized: ApprovalType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, risk);
    }

    #[test]
    fn test_risk_level_ordering() {
        // Test that risk levels are properly ordered
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);

        // Test numeric values
        assert_eq!(RiskLevel::Low.value(), 1);
        assert_eq!(RiskLevel::Medium.value(), 2);
        assert_eq!(RiskLevel::High.value(), 3);
        assert_eq!(RiskLevel::Critical.value(), 4);

        // Test names
        assert_eq!(RiskLevel::Low.name(), "Low");
        assert_eq!(RiskLevel::Medium.name(), "Medium");
        assert_eq!(RiskLevel::High.name(), "High");
        assert_eq!(RiskLevel::Critical.name(), "Critical");

        // Test sorting
        let mut levels = vec![
            RiskLevel::High,
            RiskLevel::Low,
            RiskLevel::Critical,
            RiskLevel::Medium,
        ];
        levels.sort();
        assert_eq!(
            levels,
            vec![
                RiskLevel::Low,
                RiskLevel::Medium,
                RiskLevel::High,
                RiskLevel::Critical
            ]
        );
    }

    #[test]
    fn test_approval_status_is_terminal() {
        // Pending is not terminal
        assert!(!ApprovalStatus::Pending.is_terminal());

        // Approved is terminal
        assert!(ApprovalStatus::Approved {
            by: "user".to_string(),
            at: Utc::now(),
        }
        .is_terminal());

        // Rejected is terminal
        assert!(ApprovalStatus::Rejected {
            by: "user".to_string(),
            reason: Some("Invalid".to_string()),
            at: Utc::now(),
        }
        .is_terminal());

        // Cancelled is terminal
        assert!(ApprovalStatus::Cancelled {
            reason: "Timeout".to_string(),
        }
        .is_terminal());
    }

    #[test]
    fn test_approval_type_name() {
        assert_eq!(
            ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 1,
            }
            .type_name(),
            "merge_request"
        );

        assert_eq!(
            ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 1.0,
            }
            .type_name(),
            "trading_command"
        );

        assert_eq!(
            ApprovalType::ConfigChange {
                key: "k".to_string(),
                old_value: "a".to_string(),
                new_value: "b".to_string(),
            }
            .type_name(),
            "config_change"
        );

        assert_eq!(
            ApprovalType::HighCostOperation {
                operation: "op".to_string(),
                estimated_cost: 100.0,
            }
            .type_name(),
            "high_cost_operation"
        );

        assert_eq!(
            ApprovalType::RiskOperation {
                description: "risky".to_string(),
                risk_level: RiskLevel::High,
            }
            .type_name(),
            "risk_operation"
        );
    }

    #[test]
    fn test_approval_status_name() {
        assert_eq!(ApprovalStatus::Pending.status_name(), "pending");

        assert_eq!(
            ApprovalStatus::Approved {
                by: "user".to_string(),
                at: Utc::now(),
            }
            .status_name(),
            "approved"
        );

        assert_eq!(
            ApprovalStatus::Rejected {
                by: "user".to_string(),
                reason: None,
                at: Utc::now(),
            }
            .status_name(),
            "rejected"
        );

        assert_eq!(
            ApprovalStatus::Cancelled {
                reason: "test".to_string(),
            }
            .status_name(),
            "cancelled"
        );
    }

    #[test]
    fn test_approval_response_constructors() {
        let now = Utc::now();
        let approval = ApprovalRequest {
            id: "test-id".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Test PR".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        // Test success constructor
        let success_response = ApprovalResponse::success(approval.clone());
        assert!(success_response.success);
        assert!(success_response.approval.is_some());
        assert!(success_response.error.is_none());

        // Test error constructor
        let error_response = ApprovalResponse::error("Something went wrong");
        assert!(!error_response.success);
        assert!(error_response.approval.is_none());
        assert_eq!(error_response.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_approval_decision_serialization() {
        let decision = ApprovalDecision {
            approval_id: "approval-123".to_string(),
            decided_by: "admin@example.com".to_string(),
            approved: true,
            reason: None,
        };

        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: ApprovalDecision = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.approval_id, "approval-123");
        assert_eq!(deserialized.decided_by, "admin@example.com");
        assert!(deserialized.approved);
        assert!(deserialized.reason.is_none());

        // Test rejection with reason
        let rejection = ApprovalDecision {
            approval_id: "approval-456".to_string(),
            decided_by: "reviewer".to_string(),
            approved: false,
            reason: Some("Does not meet requirements".to_string()),
        };

        let json = serde_json::to_string(&rejection).unwrap();
        let deserialized: ApprovalDecision = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.approved);
        assert_eq!(
            deserialized.reason,
            Some("Does not meet requirements".to_string())
        );
    }

    #[test]
    fn test_create_approval_request_defaults() {
        let json = r#"{
            "approval_type": {
                "type": "merge_request",
                "platform": "github",
                "repo": "test/repo",
                "mr_id": 1
            },
            "requester": "user",
            "approvers": ["admin"],
            "title": "Test",
            "channel": "slack"
        }"#;

        let request: CreateApprovalRequest = serde_json::from_str(json).unwrap();

        assert!(request.description.is_none());
        assert!(request.metadata.is_object());
        assert!(request.ttl_seconds.is_none());
    }
}
