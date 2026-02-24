//! Feishu (Lark) card renderer for approval requests.
//!
//! Implements the CardRenderer trait for Feishu interactive message cards.
//! Reference: https://open.feishu.cn/document/uAjLw4CM/ukTMukTMukTM/feishu-cards/card-components

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{CallbackAction, CallbackData, CardRenderer};
use crate::hitl::{ApprovalRequest, ApprovalStatus};

/// Feishu card renderer for approval requests.
///
/// Sends interactive message cards to Feishu channels through
/// the zero-channels service.
pub struct FeishuCardRenderer {
    /// Endpoint for the zero-channels service
    channels_endpoint: String,
    /// HTTP client for making requests
    client: reqwest::Client,
}

impl FeishuCardRenderer {
    /// Create a new Feishu card renderer.
    ///
    /// # Arguments
    /// * `channels_endpoint` - Base URL of the zero-channels service
    pub fn new(channels_endpoint: impl Into<String>) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Create with a custom HTTP client.
    pub fn with_client(channels_endpoint: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            client,
        }
    }

    /// Build the approval card JSON for Feishu interactive message.
    pub fn build_approval_card(request: &ApprovalRequest) -> Value {
        let summary = super::format_approval_summary(request);
        let header_template = Self::get_header_template(&request.status);
        let header_title = Self::get_header_title(&request.status);

        let mut elements: Vec<Value> = vec![
            // Content section with approval details
            json!({
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": summary
                }
            }),
            // Divider
            json!({
                "tag": "hr"
            }),
            // Request ID note
            json!({
                "tag": "note",
                "elements": [
                    {
                        "tag": "plain_text",
                        "content": format!("Request ID: {}", request.id)
                    }
                ]
            }),
        ];

        // Add action buttons only for pending requests
        if matches!(request.status, ApprovalStatus::Pending) {
            elements.push(json!({
                "tag": "action",
                "actions": [
                    {
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": "✅ 批准"
                        },
                        "type": "primary",
                        "value": {
                            "action": "approve",
                            "request_id": request.id
                        }
                    },
                    {
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": "❌ 拒绝"
                        },
                        "type": "danger",
                        "value": {
                            "action": "reject",
                            "request_id": request.id
                        }
                    }
                ]
            }));
        }

        json!({
            "header": {
                "title": {
                    "tag": "plain_text",
                    "content": header_title
                },
                "template": header_template
            },
            "elements": elements
        })
    }

    /// Build a result card showing the approval decision.
    pub fn build_result_card(request: &ApprovalRequest) -> Value {
        let (status_text, template) = match &request.status {
            ApprovalStatus::Approved { by, at } => {
                let text = format!(
                    "✅ 已批准\n\n审批人: {}\n时间: {}",
                    by,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                (text, "green")
            }
            ApprovalStatus::Rejected { by, reason, at } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                let text = format!(
                    "❌ 已拒绝\n\n审批人: {}\n原因: {}\n时间: {}",
                    by,
                    reason_text,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                (text, "red")
            }
            ApprovalStatus::Cancelled { reason } => {
                let text = format!("⚠️ 已取消\n\n原因: {}", reason);
                (text, "grey")
            }
            ApprovalStatus::Pending => {
                let text = "⏳ 等待审批".to_string();
                (text, "orange")
            }
        };

        json!({
            "header": {
                "title": {
                    "tag": "plain_text",
                    "content": format!("审批结果: {}", request.title)
                },
                "template": template
            },
            "elements": [
                {
                    "tag": "div",
                    "text": {
                        "tag": "lark_md",
                        "content": status_text
                    }
                },
                {
                    "tag": "hr"
                },
                {
                    "tag": "note",
                    "elements": [
                        {
                            "tag": "plain_text",
                            "content": format!("Request ID: {}", request.id)
                        }
                    ]
                }
            ]
        })
    }

    /// Get header template color based on status.
    fn get_header_template(status: &ApprovalStatus) -> &'static str {
        match status {
            ApprovalStatus::Pending => "orange",
            ApprovalStatus::Approved { .. } => "green",
            ApprovalStatus::Rejected { .. } => "red",
            ApprovalStatus::Cancelled { .. } => "grey",
        }
    }

    /// Get header title based on status.
    fn get_header_title(status: &ApprovalStatus) -> &'static str {
        match status {
            ApprovalStatus::Pending => "🔐 审批请求",
            ApprovalStatus::Approved { .. } => "✅ 审批已通过",
            ApprovalStatus::Rejected { .. } => "❌ 审批已拒绝",
            ApprovalStatus::Cancelled { .. } => "⚠️ 审批已取消",
        }
    }
}

/// Feishu callback payload from interactive message actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuCallbackPayload {
    /// User who clicked the button
    pub user_id: String,
    /// Open ID of the user
    #[serde(default)]
    pub open_id: Option<String>,
    /// Action details
    pub action: FeishuAction,
    /// Challenge for URL verification (optional)
    #[serde(default)]
    pub challenge: Option<String>,
}

/// Feishu action value from button click.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuAction {
    /// The value attached to the button
    pub value: FeishuActionValue,
    /// Tag of the action element
    #[serde(default)]
    pub tag: Option<String>,
}

/// Value embedded in the Feishu button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuActionValue {
    /// Action type: "approve" or "reject"
    pub action: String,
    /// Request ID this action applies to
    pub request_id: String,
}

#[async_trait]
impl CardRenderer for FeishuCardRenderer {
    fn channel_type(&self) -> &'static str {
        "feishu"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let card = Self::build_approval_card(request);

        let payload = json!({
            "channel": "feishu",
            "chat_id": channel_id,
            "msg_type": "interactive",
            "card": card
        });

        let response = self
            .client
            .post(format!("{}/send", self.channels_endpoint))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to send Feishu card: {} - {}",
                status,
                body
            ));
        }

        let result: Value = response.json().await?;
        let message_id = result["message_id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing message_id in response"))?
            .to_string();

        Ok(message_id)
    }

    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()> {
        let card = Self::build_result_card(request);

        let payload = json!({
            "channel": "feishu",
            "message_id": message_id,
            "msg_type": "interactive",
            "card": card
        });

        let response = self
            .client
            .post(format!("{}/update", self.channels_endpoint))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to update Feishu card: {} - {}",
                status,
                body
            ));
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let feishu_callback: FeishuCallbackPayload = serde_json::from_slice(payload)
            .map_err(|e| anyhow!("Failed to parse Feishu callback: {}", e))?;

        let action = match feishu_callback.action.value.action.as_str() {
            "approve" => CallbackAction::Approve,
            "reject" => CallbackAction::Reject { reason: None },
            other => {
                return Err(anyhow!("Unknown Feishu action: {}", other));
            }
        };

        Ok(CallbackData {
            request_id: feishu_callback.action.value.request_id,
            action,
            user_id: feishu_callback
                .open_id
                .unwrap_or(feishu_callback.user_id),
            platform_callback_id: String::new(), // Feishu doesn't provide a callback ID
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitl::{ApprovalStatus, ApprovalType, RiskLevel};
    use chrono::Utc;

    fn create_test_request() -> ApprovalRequest {
        let now = Utc::now();
        ApprovalRequest {
            id: "test-req-123".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/awesome-project".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Pending,
            requester: "developer".to_string(),
            approvers: vec!["lead".to_string(), "admin".to_string()],
            title: "Add new feature".to_string(),
            description: Some("Implements the new dashboard".to_string()),
            channel: "feishu".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_parse_feishu_callback() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431");

        // Test approve callback
        let approve_payload = r#"{
            "user_id": "ou_xxx123",
            "open_id": "ou_open_xxx456",
            "action": {
                "value": {
                    "action": "approve",
                    "request_id": "req-abc-123"
                },
                "tag": "button"
            }
        }"#;

        let result = renderer.parse_callback(approve_payload.as_bytes());
        assert!(result.is_ok());
        let callback = result.unwrap();
        assert_eq!(callback.request_id, "req-abc-123");
        assert_eq!(callback.action, CallbackAction::Approve);
        assert_eq!(callback.user_id, "ou_open_xxx456");

        // Test reject callback
        let reject_payload = r#"{
            "user_id": "ou_xxx789",
            "action": {
                "value": {
                    "action": "reject",
                    "request_id": "req-def-456"
                }
            }
        }"#;

        let result = renderer.parse_callback(reject_payload.as_bytes());
        assert!(result.is_ok());
        let callback = result.unwrap();
        assert_eq!(callback.request_id, "req-def-456");
        assert_eq!(callback.action, CallbackAction::Reject { reason: None });
        assert_eq!(callback.user_id, "ou_xxx789"); // Falls back to user_id when open_id is missing
    }

    #[test]
    fn test_parse_feishu_callback_invalid_action() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431");

        let invalid_payload = r#"{
            "user_id": "ou_xxx",
            "action": {
                "value": {
                    "action": "unknown_action",
                    "request_id": "req-123"
                }
            }
        }"#;

        let result = renderer.parse_callback(invalid_payload.as_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown Feishu action"));
    }

    #[test]
    fn test_parse_feishu_callback_malformed() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431");

        let malformed_payload = r#"{ "not": "valid" }"#;
        let result = renderer.parse_callback(malformed_payload.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_build_feishu_card() {
        let request = create_test_request();
        let card = FeishuCardRenderer::build_approval_card(&request);

        // Verify header
        assert_eq!(card["header"]["title"]["tag"], "plain_text");
        assert_eq!(card["header"]["title"]["content"], "🔐 审批请求");
        assert_eq!(card["header"]["template"], "orange");

        // Verify elements structure
        let elements = card["elements"].as_array().unwrap();
        assert!(elements.len() >= 3);

        // First element should be div with content
        assert_eq!(elements[0]["tag"], "div");
        assert_eq!(elements[0]["text"]["tag"], "lark_md");
        let content = elements[0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("审批请求: Add new feature"));
        assert!(content.contains("【合并请求】"));

        // Should have action buttons for pending status
        let action_element = elements
            .iter()
            .find(|e| e["tag"] == "action")
            .expect("Should have action element");
        let actions = action_element["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2);

        // Verify approve button
        assert_eq!(actions[0]["tag"], "button");
        assert_eq!(actions[0]["text"]["content"], "✅ 批准");
        assert_eq!(actions[0]["type"], "primary");
        assert_eq!(actions[0]["value"]["action"], "approve");
        assert_eq!(actions[0]["value"]["request_id"], "test-req-123");

        // Verify reject button
        assert_eq!(actions[1]["tag"], "button");
        assert_eq!(actions[1]["text"]["content"], "❌ 拒绝");
        assert_eq!(actions[1]["type"], "danger");
        assert_eq!(actions[1]["value"]["action"], "reject");
    }

    #[test]
    fn test_build_feishu_card_approved_status() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: now,
        };

        let card = FeishuCardRenderer::build_approval_card(&request);

        // Verify header reflects approved status
        assert_eq!(card["header"]["title"]["content"], "✅ 审批已通过");
        assert_eq!(card["header"]["template"], "green");

        // Should NOT have action buttons for approved status
        let elements = card["elements"].as_array().unwrap();
        let action_element = elements.iter().find(|e| e["tag"] == "action");
        assert!(action_element.is_none());
    }

    #[test]
    fn test_build_feishu_card_rejected_status() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Rejected {
            by: "reviewer".to_string(),
            reason: Some("Code quality issues".to_string()),
            at: now,
        };

        let card = FeishuCardRenderer::build_approval_card(&request);

        assert_eq!(card["header"]["title"]["content"], "❌ 审批已拒绝");
        assert_eq!(card["header"]["template"], "red");
    }

    #[test]
    fn test_build_result_card_approved() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: now,
        };

        let card = FeishuCardRenderer::build_result_card(&request);

        assert_eq!(card["header"]["template"], "green");
        let content = card["elements"][0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("✅ 已批准"));
        assert!(content.contains("审批人: admin"));
    }

    #[test]
    fn test_build_result_card_rejected() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Rejected {
            by: "reviewer".to_string(),
            reason: Some("Not ready".to_string()),
            at: now,
        };

        let card = FeishuCardRenderer::build_result_card(&request);

        assert_eq!(card["header"]["template"], "red");
        let content = card["elements"][0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("❌ 已拒绝"));
        assert!(content.contains("原因: Not ready"));
    }

    #[test]
    fn test_build_result_card_cancelled() {
        let mut request = create_test_request();
        request.status = ApprovalStatus::Cancelled {
            reason: "Timeout".to_string(),
        };

        let card = FeishuCardRenderer::build_result_card(&request);

        assert_eq!(card["header"]["template"], "grey");
        let content = card["elements"][0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("⚠️ 已取消"));
        assert!(content.contains("原因: Timeout"));
    }

    #[test]
    fn test_build_feishu_card_trading_command() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "trade-req-456".to_string(),
            approval_type: ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 0.5,
            },
            status: ApprovalStatus::Pending,
            requester: "trader".to_string(),
            approvers: vec!["risk-manager".to_string()],
            title: "Buy BTC".to_string(),
            description: None,
            channel: "feishu".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = FeishuCardRenderer::build_approval_card(&request);

        let content = card["elements"][0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("【交易指令】"));
        assert!(content.contains("资产: BTC"));
        assert!(content.contains("操作: buy"));
        assert!(content.contains("数量: 0.5"));
    }

    #[test]
    fn test_build_feishu_card_risk_operation() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "risk-req-789".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production database".to_string(),
                risk_level: RiskLevel::Critical,
            },
            status: ApprovalStatus::Pending,
            requester: "dba".to_string(),
            approvers: vec!["cto".to_string()],
            title: "Database cleanup".to_string(),
            description: None,
            channel: "feishu".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = FeishuCardRenderer::build_approval_card(&request);

        let content = card["elements"][0]["text"]["content"].as_str().unwrap();
        assert!(content.contains("【风险操作】"));
        assert!(content.contains("风险等级: 危急"));
    }

    #[test]
    fn test_feishu_renderer_channel_type() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431");
        assert_eq!(renderer.channel_type(), "feishu");
    }

    #[test]
    fn test_feishu_callback_payload_serialization() {
        let payload = FeishuCallbackPayload {
            user_id: "ou_user123".to_string(),
            open_id: Some("ou_open456".to_string()),
            action: FeishuAction {
                value: FeishuActionValue {
                    action: "approve".to_string(),
                    request_id: "req-123".to_string(),
                },
                tag: Some("button".to_string()),
            },
            challenge: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: FeishuCallbackPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, "ou_user123");
        assert_eq!(deserialized.open_id, Some("ou_open456".to_string()));
        assert_eq!(deserialized.action.value.action, "approve");
        assert_eq!(deserialized.action.value.request_id, "req-123");
    }

    #[test]
    fn test_card_contains_request_id_note() {
        let request = create_test_request();
        let card = FeishuCardRenderer::build_approval_card(&request);

        let elements = card["elements"].as_array().unwrap();
        let note_element = elements
            .iter()
            .find(|e| e["tag"] == "note")
            .expect("Should have note element");

        let note_content = note_element["elements"][0]["content"].as_str().unwrap();
        assert!(note_content.contains("test-req-123"));
    }
}
