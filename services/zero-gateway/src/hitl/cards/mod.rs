//! IM channel card templates for approval requests.
//!
//! This module generates rich card messages for various IM platforms
//! (Telegram, Feishu, Slack, DingTalk) to display approval requests.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ApprovalRequest, ApprovalType, RiskLevel};

// Stub modules for platform-specific implementations
pub mod dingtalk;
pub mod feishu;
pub mod slack;
pub mod telegram;

/// Callback action from user interaction with approval card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CallbackAction {
    /// User approved the request
    Approve,
    /// User rejected the request
    Reject {
        /// Optional reason for rejection
        #[serde(default)]
        reason: Option<String>,
    },
}

impl CallbackAction {
    /// Parse a callback action from a string.
    ///
    /// Supported formats:
    /// - "approve" -> Approve
    /// - "reject" -> Reject { reason: None }
    /// - "reject:reason text" -> Reject { reason: Some("reason text") }
    pub fn from_str(s: &str) -> Self {
        let s = s.trim().to_lowercase();
        if s == "approve" {
            CallbackAction::Approve
        } else if s == "reject" {
            CallbackAction::Reject { reason: None }
        } else if let Some(reason) = s.strip_prefix("reject:") {
            let reason = reason.trim();
            CallbackAction::Reject {
                reason: if reason.is_empty() {
                    None
                } else {
                    Some(reason.to_string())
                },
            }
        } else {
            // Default to reject with the entire string as reason if unrecognized
            CallbackAction::Reject {
                reason: Some(s.to_string()),
            }
        }
    }
}

/// Data extracted from a platform callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackData {
    /// ID of the approval request
    pub request_id: String,
    /// Action taken by the user
    pub action: CallbackAction,
    /// User ID who performed the action
    pub user_id: String,
    /// Platform-specific callback identifier
    pub platform_callback_id: String,
}

/// Trait for rendering approval cards to different IM platforms.
#[async_trait]
pub trait CardRenderer: Send + Sync {
    /// Returns the channel type identifier (e.g., "telegram", "slack").
    fn channel_type(&self) -> &'static str;

    /// Send an approval card to the specified channel.
    ///
    /// Returns the message ID in the platform for future updates.
    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String>;

    /// Update an existing approval card with new status.
    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()>;

    /// Parse a callback payload from the platform.
    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData>;
}

/// Format an approval summary in Chinese for display in cards.
pub fn format_approval_summary(request: &ApprovalRequest) -> String {
    let type_summary = match &request.approval_type {
        ApprovalType::MergeRequest {
            platform,
            repo,
            mr_id,
        } => {
            format!(
                "【合并请求】\n平台: {}\n仓库: {}\nMR编号: #{}",
                platform, repo, mr_id
            )
        }
        ApprovalType::TradingCommand {
            asset,
            action,
            amount,
        } => {
            format!(
                "【交易指令】\n资产: {}\n操作: {}\n数量: {}",
                asset, action, amount
            )
        }
        ApprovalType::ConfigChange {
            key,
            old_value,
            new_value,
        } => {
            format!(
                "【配置变更】\n配置项: {}\n原值: {}\n新值: {}",
                key, old_value, new_value
            )
        }
        ApprovalType::HighCostOperation {
            operation,
            estimated_cost,
        } => {
            format!(
                "【高成本操作】\n操作: {}\n预估成本: ${:.2}",
                operation, estimated_cost
            )
        }
        ApprovalType::RiskOperation {
            description,
            risk_level,
        } => {
            let risk_label = match risk_level {
                RiskLevel::Low => "低",
                RiskLevel::Medium => "中",
                RiskLevel::High => "高",
                RiskLevel::Critical => "危急",
            };
            format!(
                "【风险操作】\n描述: {}\n风险等级: {}",
                description, risk_label
            )
        }
        ApprovalType::ToolExecution {
            tool,
            risk_level,
            hand_id,
            execution_id,
            ..
        } => {
            let risk_label = match risk_level {
                RiskLevel::Low => "低",
                RiskLevel::Medium => "中",
                RiskLevel::High => "高",
                RiskLevel::Critical => "危急",
            };
            format!(
                "【工具执行】\n工具: {}\nHand: {}\n执行ID: {}\n风险等级: {}",
                tool, hand_id, execution_id, risk_label
            )
        }
    };

    let mut summary = format!(
        "审批请求: {}\n\n{}\n\n请求人: {}\n审批人: {}",
        request.title,
        type_summary,
        request.requester,
        request.approvers.join(", ")
    );

    if let Some(desc) = &request.description {
        summary.push_str(&format!("\n\n备注: {}", desc));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_callback_action_from_string() {
        // Test approve
        let action = CallbackAction::from_str("approve");
        assert_eq!(action, CallbackAction::Approve);

        // Test approve with different case
        let action = CallbackAction::from_str("APPROVE");
        assert_eq!(action, CallbackAction::Approve);

        // Test approve with whitespace
        let action = CallbackAction::from_str("  approve  ");
        assert_eq!(action, CallbackAction::Approve);

        // Test reject without reason
        let action = CallbackAction::from_str("reject");
        assert_eq!(action, CallbackAction::Reject { reason: None });

        // Test reject with reason
        let action = CallbackAction::from_str("reject:code quality issues");
        assert_eq!(
            action,
            CallbackAction::Reject {
                reason: Some("code quality issues".to_string())
            }
        );

        // Test reject with empty reason
        let action = CallbackAction::from_str("reject:");
        assert_eq!(action, CallbackAction::Reject { reason: None });

        // Test unknown string becomes reject with reason
        let action = CallbackAction::from_str("invalid_action");
        assert_eq!(
            action,
            CallbackAction::Reject {
                reason: Some("invalid_action".to_string())
            }
        );
    }

    #[test]
    fn test_callback_data_construction() {
        let callback = CallbackData {
            request_id: "req-123".to_string(),
            action: CallbackAction::Approve,
            user_id: "user-456".to_string(),
            platform_callback_id: "cb-789".to_string(),
        };

        assert_eq!(callback.request_id, "req-123");
        assert_eq!(callback.action, CallbackAction::Approve);
        assert_eq!(callback.user_id, "user-456");
        assert_eq!(callback.platform_callback_id, "cb-789");

        // Test serialization roundtrip
        let json = serde_json::to_string(&callback).unwrap();
        let deserialized: CallbackData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.request_id, callback.request_id);
        assert_eq!(deserialized.user_id, callback.user_id);
        assert_eq!(deserialized.platform_callback_id, callback.platform_callback_id);

        // Test with reject action
        let reject_callback = CallbackData {
            request_id: "req-456".to_string(),
            action: CallbackAction::Reject {
                reason: Some("Not ready".to_string()),
            },
            user_id: "reviewer-1".to_string(),
            platform_callback_id: "telegram-callback-123".to_string(),
        };

        let json = serde_json::to_string(&reject_callback).unwrap();
        let deserialized: CallbackData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.request_id, "req-456");
        match deserialized.action {
            CallbackAction::Reject { reason } => {
                assert_eq!(reason, Some("Not ready".to_string()));
            }
            _ => panic!("Expected Reject action"),
        }
    }

    #[test]
    fn test_format_approval_summary_mr() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "test-id".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/awesome-project".to_string(),
                mr_id: 42,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "developer".to_string(),
            approvers: vec!["lead".to_string(), "admin".to_string()],
            title: "Add new feature".to_string(),
            description: Some("Implements the new dashboard".to_string()),
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let summary = format_approval_summary(&request);

        // Verify key content is present
        assert!(summary.contains("审批请求: Add new feature"));
        assert!(summary.contains("【合并请求】"));
        assert!(summary.contains("平台: github"));
        assert!(summary.contains("仓库: org/awesome-project"));
        assert!(summary.contains("MR编号: #42"));
        assert!(summary.contains("请求人: developer"));
        assert!(summary.contains("审批人: lead, admin"));
        assert!(summary.contains("备注: Implements the new dashboard"));
    }

    #[test]
    fn test_format_approval_summary_trading() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "trade-id".to_string(),
            approval_type: ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 0.5,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "trader".to_string(),
            approvers: vec!["risk-manager".to_string()],
            title: "Buy BTC".to_string(),
            description: None,
            channel: "slack".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let summary = format_approval_summary(&request);

        assert!(summary.contains("【交易指令】"));
        assert!(summary.contains("资产: BTC"));
        assert!(summary.contains("操作: buy"));
        assert!(summary.contains("数量: 0.5"));
        assert!(!summary.contains("备注:"));
    }

    #[test]
    fn test_format_approval_summary_config_change() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "config-id".to_string(),
            approval_type: ApprovalType::ConfigChange {
                key: "max_concurrent_requests".to_string(),
                old_value: "100".to_string(),
                new_value: "200".to_string(),
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "devops".to_string(),
            approvers: vec!["sre".to_string()],
            title: "Increase request limit".to_string(),
            description: None,
            channel: "dingtalk".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let summary = format_approval_summary(&request);

        assert!(summary.contains("【配置变更】"));
        assert!(summary.contains("配置项: max_concurrent_requests"));
        assert!(summary.contains("原值: 100"));
        assert!(summary.contains("新值: 200"));
    }

    #[test]
    fn test_format_approval_summary_high_cost() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "cost-id".to_string(),
            approval_type: ApprovalType::HighCostOperation {
                operation: "Deploy K8s cluster".to_string(),
                estimated_cost: 2500.50,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "platform-team".to_string(),
            approvers: vec!["finance".to_string()],
            title: "Cloud deployment".to_string(),
            description: None,
            channel: "feishu".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let summary = format_approval_summary(&request);

        assert!(summary.contains("【高成本操作】"));
        assert!(summary.contains("操作: Deploy K8s cluster"));
        assert!(summary.contains("预估成本: $2500.50"));
    }

    #[test]
    fn test_format_approval_summary_risk_operation() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "risk-id".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production database".to_string(),
                risk_level: RiskLevel::Critical,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "dba".to_string(),
            approvers: vec!["cto".to_string()],
            title: "Database cleanup".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let summary = format_approval_summary(&request);

        assert!(summary.contains("【风险操作】"));
        assert!(summary.contains("描述: Delete production database"));
        assert!(summary.contains("风险等级: 危急"));
    }

    #[test]
    fn test_format_approval_summary_risk_levels() {
        let now = Utc::now();

        let risk_levels = [
            (RiskLevel::Low, "低"),
            (RiskLevel::Medium, "中"),
            (RiskLevel::High, "高"),
            (RiskLevel::Critical, "危急"),
        ];

        for (level, expected_label) in risk_levels {
            let request = ApprovalRequest {
                id: "id".to_string(),
                approval_type: ApprovalType::RiskOperation {
                    description: "Test".to_string(),
                    risk_level: level,
                },
                status: super::super::ApprovalStatus::Pending,
                requester: "user".to_string(),
                approvers: vec!["admin".to_string()],
                title: "Test".to_string(),
                description: None,
                channel: "telegram".to_string(),
                message_id: None,
                metadata: serde_json::json!({}),
                created_at: now,
                updated_at: now,
                expires_at: None,
            };

            let summary = format_approval_summary(&request);
            assert!(
                summary.contains(&format!("风险等级: {}", expected_label)),
                "Expected risk label '{}' for level {:?}",
                expected_label,
                level
            );
        }
    }

    #[test]
    fn test_callback_action_serialization() {
        // Test Approve serialization
        let approve = CallbackAction::Approve;
        let json = serde_json::to_string(&approve).unwrap();
        assert!(json.contains("\"action\":\"approve\""));

        let deserialized: CallbackAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CallbackAction::Approve);

        // Test Reject with reason serialization
        let reject = CallbackAction::Reject {
            reason: Some("Not compliant".to_string()),
        };
        let json = serde_json::to_string(&reject).unwrap();
        assert!(json.contains("\"action\":\"reject\""));
        assert!(json.contains("\"reason\":\"Not compliant\""));

        let deserialized: CallbackAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reject);

        // Test Reject without reason
        let reject_no_reason = CallbackAction::Reject { reason: None };
        let json = serde_json::to_string(&reject_no_reason).unwrap();
        let deserialized: CallbackAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reject_no_reason);
    }
}
