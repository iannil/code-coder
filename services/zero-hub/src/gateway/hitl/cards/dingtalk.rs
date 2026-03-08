//! DingTalk (钉钉) card renderer for approval requests.
//!
//! Implements the CardRenderer trait for DingTalk ActionCard messages.
//! Uses DingTalk's interactive card format with action buttons.
//!
//! Reference: https://open.dingtalk.com/document/orgapp/robot-message-types-and-data-format

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{format_approval_summary, CallbackAction, CallbackData, CardRenderer};
use crate::gateway::hitl::{ApprovalRequest, ApprovalStatus};

/// DingTalk card renderer for approval requests.
///
/// Sends ActionCard messages to DingTalk channels through
/// the zero-channels service.
pub struct DingTalkCardRenderer {
    /// Endpoint for the zero-channels service
    channels_endpoint: String,
    /// HTTP client for making requests
    client: reqwest::Client,
    /// Callback URL base for button actions
    callback_base_url: String,
}

impl DingTalkCardRenderer {
    /// Create a new DingTalk card renderer.
    ///
    /// # Arguments
    /// * `channels_endpoint` - Base URL of the zero-channels service
    /// * `callback_base_url` - Base URL for approval callbacks
    pub fn new(
        channels_endpoint: impl Into<String>,
        callback_base_url: impl Into<String>,
    ) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            client: reqwest::Client::new(),
            callback_base_url: callback_base_url.into(),
        }
    }

    /// Create with a custom HTTP client.
    pub fn with_client(
        channels_endpoint: impl Into<String>,
        callback_base_url: impl Into<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            client,
            callback_base_url: callback_base_url.into(),
        }
    }

    /// Build an ActionCard for an approval request.
    pub fn build_action_card(&self, request: &ApprovalRequest) -> DingTalkActionCard {
        let summary = format_approval_summary(request);
        let text = format!(
            "### 🔐 审批请求\n\n{}\n\n---\n\n*Request ID: {}*",
            escape_markdown(&summary),
            request.id
        );

        // Build action URLs for approve/reject
        let approve_url = format!(
            "{}/hitl/callback?action=approve&request_id={}",
            self.callback_base_url, request.id
        );
        let reject_url = format!(
            "{}/hitl/callback?action=reject&request_id={}",
            self.callback_base_url, request.id
        );

        DingTalkActionCard {
            title: format!("审批请求: {}", request.title),
            text,
            btn_orientation: "1".to_string(), // Horizontal layout
            btns: vec![
                ActionButton {
                    title: "✅ 批准".to_string(),
                    action_url: approve_url,
                },
                ActionButton {
                    title: "❌ 拒绝".to_string(),
                    action_url: reject_url,
                },
            ],
        }
    }

    /// Build a result card showing the approval decision.
    pub fn build_result_card(&self, request: &ApprovalRequest) -> DingTalkActionCard {
        let summary = format_approval_summary(request);

        let (status_text, emoji) = match &request.status {
            ApprovalStatus::Pending => ("⏳ 等待审批".to_string(), "⏳"),
            ApprovalStatus::Approved { by, at } => {
                let text = format!(
                    "✅ 已批准\n\n**审批人:** {}\n**时间:** {}",
                    by,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                (text, "✅")
            }
            ApprovalStatus::Rejected { by, reason, at } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                let text = format!(
                    "❌ 已拒绝\n\n**审批人:** {}\n**原因:** {}\n**时间:** {}",
                    by,
                    reason_text,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                (text, "❌")
            }
            ApprovalStatus::Cancelled { reason } => {
                let text = format!("⚠️ 已取消\n\n**原因:** {}", reason);
                (text, "⚠️")
            }
        };

        let text = format!(
            "### {} 审批结果\n\n{}\n\n---\n\n{}\n\n*Request ID: {}*",
            emoji,
            escape_markdown(&summary),
            status_text,
            request.id
        );

        DingTalkActionCard {
            title: format!("审批结果: {}", request.title),
            text,
            btn_orientation: "1".to_string(),
            btns: vec![], // No buttons for result card
        }
    }
}

/// Escape special Markdown characters for DingTalk.
fn escape_markdown(text: &str) -> String {
    // DingTalk Markdown is similar to standard Markdown
    // We need to be careful not to break intentional formatting
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// DingTalk ActionCard message structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkActionCard {
    /// Card title
    pub title: String,
    /// Card body text (Markdown supported)
    pub text: String,
    /// Button orientation: "0" for vertical, "1" for horizontal
    #[serde(rename = "btnOrientation")]
    pub btn_orientation: String,
    /// Action buttons
    pub btns: Vec<ActionButton>,
}

/// Action button in DingTalk ActionCard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionButton {
    /// Button display text
    pub title: String,
    /// Action URL when button is clicked
    #[serde(rename = "actionURL")]
    pub action_url: String,
}

/// DingTalk callback payload from action button click.
#[derive(Debug, Clone, Deserialize)]
pub struct DingTalkCallback {
    /// User who clicked the button
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    /// Sender staff ID (for enterprise internal users)
    #[serde(rename = "senderStaffId")]
    pub sender_staff_id: Option<String>,
    /// Conversation ID
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    /// Action taken
    pub action: Option<String>,
    /// Request ID
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
}

/// Alternative callback format using query parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct DingTalkQueryCallback {
    /// Action from query param
    pub action: String,
    /// Request ID from query param
    pub request_id: String,
    /// User ID (may come from header or body)
    #[serde(default)]
    pub user_id: Option<String>,
}

/// Request payload for sending DingTalk ActionCard.
#[derive(Debug, Serialize)]
struct SendActionCardRequest {
    /// Target channel (webhook URL or conversation ID)
    channel: String,
    /// Message type
    msgtype: String,
    /// ActionCard content
    #[serde(rename = "actionCard")]
    action_card: DingTalkActionCard,
}

/// Request payload for sending simple text (for result updates).
#[derive(Debug, Serialize)]
struct SendMarkdownRequest {
    channel: String,
    msgtype: String,
    markdown: MarkdownContent,
}

#[derive(Debug, Serialize)]
struct MarkdownContent {
    title: String,
    text: String,
}

/// Response from DingTalk send message.
#[derive(Debug, Deserialize)]
struct DingTalkResponse {
    #[serde(default)]
    errcode: i32,
    #[serde(default)]
    errmsg: String,
    #[serde(default)]
    message_id: Option<String>,
}

#[async_trait]
impl CardRenderer for DingTalkCardRenderer {
    fn channel_type(&self) -> &'static str {
        "dingtalk"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let action_card = self.build_action_card(request);

        let payload = SendActionCardRequest {
            channel: channel_id.to_string(),
            msgtype: "actionCard".to_string(),
            action_card,
        };

        let url = format!("{}/dingtalk/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send DingTalk message: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to send DingTalk card ({}): {}",
                status,
                body
            ));
        }

        let result: DingTalkResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse DingTalk response: {}", e))?;

        if result.errcode != 0 {
            return Err(anyhow!(
                "DingTalk API error ({}): {}",
                result.errcode,
                result.errmsg
            ));
        }

        // DingTalk may not return a message_id for ActionCards sent via webhook
        // Generate a pseudo-ID based on request ID for tracking
        let message_id = result
            .message_id
            .unwrap_or_else(|| format!("dingtalk-{}", request.id));

        Ok(message_id)
    }

    async fn update_card(&self, request: &ApprovalRequest, _message_id: &str) -> Result<()> {
        // DingTalk ActionCards cannot be directly updated once sent
        // Instead, we send a new Markdown message with the result
        let result_card = self.build_result_card(request);

        let payload = SendMarkdownRequest {
            channel: request.channel.clone(),
            msgtype: "markdown".to_string(),
            markdown: MarkdownContent {
                title: result_card.title,
                text: result_card.text,
            },
        };

        let url = format!("{}/dingtalk/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send DingTalk update: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to update DingTalk card ({}): {}",
                status,
                body
            ));
        }

        let result: DingTalkResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse DingTalk response: {}", e))?;

        if result.errcode != 0 {
            return Err(anyhow!(
                "DingTalk API error ({}): {}",
                result.errcode,
                result.errmsg
            ));
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        // Try parsing as JSON body with camelCase field names first (DingTalk native format)
        if let Ok(callback) = serde_json::from_slice::<DingTalkCallback>(payload) {
            // Only proceed if we have the required request_id field (camelCase: requestId)
            if let Some(request_id) = callback.request_id {
                let action = callback.action.as_deref().unwrap_or("");
                let user_id = callback
                    .sender_staff_id
                    .or(callback.user_id)
                    .unwrap_or_else(|| "unknown".to_string());

                let callback_action = match action {
                    "approve" => CallbackAction::Approve,
                    "reject" => CallbackAction::Reject { reason: None },
                    other => {
                        return Err(anyhow!("Unknown DingTalk action: {}", other));
                    }
                };

                let platform_callback_id = callback
                    .conversation_id
                    .unwrap_or_else(|| format!("dingtalk-cb-{}", request_id));

                return Ok(CallbackData {
                    request_id,
                    action: callback_action,
                    user_id,
                    platform_callback_id,
                });
            }
        }

        // Try parsing as query parameters (from URL callback)
        if let Ok(query_callback) = serde_json::from_slice::<DingTalkQueryCallback>(payload) {
            let callback_action = match query_callback.action.as_str() {
                "approve" => CallbackAction::Approve,
                "reject" => CallbackAction::Reject { reason: None },
                other => {
                    return Err(anyhow!("Unknown DingTalk action: {}", other));
                }
            };

            return Ok(CallbackData {
                request_id: query_callback.request_id.clone(),
                action: callback_action,
                user_id: query_callback
                    .user_id
                    .unwrap_or_else(|| "unknown".to_string()),
                platform_callback_id: format!("dingtalk-cb-{}", query_callback.request_id),
            });
        }

        Err(anyhow!("Failed to parse DingTalk callback payload"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::hitl::{ApprovalStatus, ApprovalType, RiskLevel};
    use chrono::Utc;

    fn create_test_request() -> ApprovalRequest {
        let now = Utc::now();
        ApprovalRequest {
            id: "req-12345".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Pending,
            requester: "developer".to_string(),
            approvers: vec!["reviewer".to_string()],
            title: "Add new feature".to_string(),
            description: Some("This adds a cool feature".to_string()),
            channel: "webhook://dingtalk.example.com/xxx".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_build_action_card() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        let request = create_test_request();

        let card = renderer.build_action_card(&request);

        assert!(card.title.contains("审批请求"));
        assert!(card.title.contains("Add new feature"));
        assert!(card.text.contains("🔐"));
        assert_eq!(card.btn_orientation, "1");
        assert_eq!(card.btns.len(), 2);

        // Check approve button
        assert_eq!(card.btns[0].title, "✅ 批准");
        assert!(card.btns[0].action_url.contains("action=approve"));
        assert!(card.btns[0].action_url.contains("request_id=req-12345"));

        // Check reject button
        assert_eq!(card.btns[1].title, "❌ 拒绝");
        assert!(card.btns[1].action_url.contains("action=reject"));
    }

    #[test]
    fn test_build_result_card_approved() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: now,
        };

        let card = renderer.build_result_card(&request);

        assert!(card.title.contains("审批结果"));
        assert!(card.text.contains("✅ 已批准"));
        assert!(card.text.contains("审批人"));
        assert!(card.text.contains("admin"));
        assert!(card.btns.is_empty()); // No buttons for result
    }

    #[test]
    fn test_build_result_card_rejected() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Rejected {
            by: "reviewer".to_string(),
            reason: Some("Code quality issues".to_string()),
            at: now,
        };

        let card = renderer.build_result_card(&request);

        assert!(card.text.contains("❌ 已拒绝"));
        assert!(card.text.contains("Code quality issues"));
    }

    #[test]
    fn test_parse_callback_json_body() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );

        let callback_json = r#"{
            "userId": "user123",
            "senderStaffId": "staff456",
            "conversationId": "conv789",
            "action": "approve",
            "requestId": "req-abc-123"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-abc-123");
        assert_eq!(callback_data.action, CallbackAction::Approve);
        assert_eq!(callback_data.user_id, "staff456"); // Prefers senderStaffId
        assert_eq!(callback_data.platform_callback_id, "conv789");
    }

    #[test]
    fn test_parse_callback_query_params() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );

        let callback_json = r#"{
            "action": "reject",
            "request_id": "req-xyz-789"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-xyz-789");
        assert_eq!(callback_data.action, CallbackAction::Reject { reason: None });
    }

    #[test]
    fn test_parse_callback_unknown_action() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );

        let callback_json = r#"{
            "action": "unknown",
            "requestId": "req-123"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown DingTalk action"));
    }

    #[test]
    fn test_parse_callback_invalid() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );

        let invalid_json = b"not valid json";
        let result = renderer.parse_callback(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
        assert_eq!(escape_markdown("_italic_"), "\\_italic\\_");
        assert_eq!(escape_markdown("[link]"), "\\[link\\]");
        assert_eq!(escape_markdown("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_channel_type() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        assert_eq!(renderer.channel_type(), "dingtalk");
    }

    #[test]
    fn test_action_card_serialization() {
        let card = DingTalkActionCard {
            title: "Test Title".to_string(),
            text: "### Test Text".to_string(),
            btn_orientation: "1".to_string(),
            btns: vec![ActionButton {
                title: "Button".to_string(),
                action_url: "http://example.com".to_string(),
            }],
        };

        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("\"title\":\"Test Title\""));
        assert!(json.contains("\"btnOrientation\":\"1\""));
        assert!(json.contains("\"actionURL\":\"http://example.com\""));
    }

    #[test]
    fn test_build_trading_command_card() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "trade-123".to_string(),
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
            channel: "webhook://dingtalk.example.com".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = renderer.build_action_card(&request);

        assert!(card.text.contains("BTC"));
        assert!(card.text.contains("buy"));
        assert!(card.text.contains("0.5"));
    }

    #[test]
    fn test_build_risk_operation_card() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "http://localhost:4402",
        );
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "risk-123".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production data".to_string(),
                risk_level: RiskLevel::Critical,
            },
            status: ApprovalStatus::Pending,
            requester: "dba".to_string(),
            approvers: vec!["cto".to_string()],
            title: "Database cleanup".to_string(),
            description: None,
            channel: "webhook://dingtalk.example.com".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = renderer.build_action_card(&request);

        assert!(card.text.contains("Delete production data"));
        assert!(card.text.contains("危急")); // Critical in Chinese
    }
}
