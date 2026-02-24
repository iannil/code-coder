//! DingTalk card renderer for approval requests.
//!
//! Implements the CardRenderer trait for DingTalk ActionCard messages.
//! DingTalk uses URL-based callbacks instead of webhook JSON payloads.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{format_approval_summary, CallbackAction, CallbackData, CardRenderer};
use crate::hitl::{ApprovalRequest, ApprovalStatus};

/// DingTalk ActionCard renderer for HitL approval requests.
///
/// DingTalk ActionCard format uses buttons with actionURL for callbacks.
/// When a user clicks a button, DingTalk opens the URL in a browser,
/// triggering our callback endpoint.
pub struct DingTalkCardRenderer {
    /// Endpoint for the channels service (e.g., "http://localhost:4431")
    channels_endpoint: String,
    /// Base URL for callback endpoints (e.g., "https://gateway.example.com")
    callback_base_url: String,
    /// HTTP client for sending requests
    client: reqwest::Client,
}

impl DingTalkCardRenderer {
    /// Create a new DingTalk card renderer.
    ///
    /// # Arguments
    /// * `channels_endpoint` - URL of the channels service for sending messages
    /// * `callback_base_url` - Base URL for callback endpoints (must be publicly accessible)
    pub fn new(channels_endpoint: impl Into<String>, callback_base_url: impl Into<String>) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            callback_base_url: callback_base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Build a DingTalk ActionCard JSON for an approval request.
    pub fn build_card(&self, request: &ApprovalRequest) -> DingTalkActionCard {
        let summary = format_approval_summary(request);
        let status_emoji = match &request.status {
            ApprovalStatus::Pending => "🔐",
            ApprovalStatus::Approved { .. } => "✅",
            ApprovalStatus::Rejected { .. } => "❌",
            ApprovalStatus::Cancelled { .. } => "⚠️",
        };

        let title = format!("{} 审批请求", status_emoji);
        let text = format!("### {}\n\n{}", request.title, summary);

        let approve_url = format!(
            "{}/hitl/callback/dingtalk?action=approve&id={}",
            self.callback_base_url, request.id
        );
        let reject_url = format!(
            "{}/hitl/callback/dingtalk?action=reject&id={}",
            self.callback_base_url, request.id
        );

        let btns = if matches!(request.status, ApprovalStatus::Pending) {
            vec![
                ActionCardButton {
                    title: "✅ 批准".to_string(),
                    action_url: approve_url,
                },
                ActionCardButton {
                    title: "❌ 拒绝".to_string(),
                    action_url: reject_url,
                },
            ]
        } else {
            vec![]
        };

        DingTalkActionCard {
            msgtype: "actionCard".to_string(),
            action_card: ActionCardContent {
                title,
                text,
                btn_orientation: "1".to_string(),
                btns,
            },
        }
    }

    /// Parse callback data from DingTalk URL query parameters.
    ///
    /// DingTalk callbacks come as URL query parameters:
    /// `?action=approve&id=request_id` or `?action=reject&id=request_id`
    pub fn parse_callback_params(&self, query: &str) -> Result<CallbackData> {
        let params = parse_query_string(query);

        let action_str = params
            .get("action")
            .context("Missing 'action' parameter in callback")?;

        let request_id = params
            .get("id")
            .context("Missing 'id' parameter in callback")?
            .clone();

        let action = CallbackAction::from_str(action_str);

        // DingTalk doesn't provide user info in URL callbacks directly
        // The user_id would typically come from a separate authentication mechanism
        let user_id = params
            .get("user_id")
            .cloned()
            .unwrap_or_else(|| "dingtalk_user".to_string());

        Ok(CallbackData {
            request_id,
            action,
            user_id,
            platform_callback_id: format!("dingtalk-{}", uuid::Uuid::new_v4()),
        })
    }
}

/// Parse a URL query string into key-value pairs.
/// Handles URL decoding for percent-encoded characters.
fn parse_query_string(query: &str) -> std::collections::HashMap<String, String> {
    let query = query.strip_prefix('?').unwrap_or(query);

    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");

            // URL decode the key and value
            let key = url_decode(key);
            let value = url_decode(value);

            Some((key, value))
        })
        .collect()
}

/// Simple URL decoder for percent-encoded strings.
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to parse the next two characters as hex
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If parsing fails, keep the original %XX
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            // + represents space in query strings
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// DingTalk ActionCard message structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkActionCard {
    /// Message type - always "actionCard"
    pub msgtype: String,
    /// ActionCard content
    #[serde(rename = "actionCard")]
    pub action_card: ActionCardContent,
}

/// Content of a DingTalk ActionCard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCardContent {
    /// Card title
    pub title: String,
    /// Card content in Markdown format
    pub text: String,
    /// Button orientation: "0" for vertical, "1" for horizontal
    #[serde(rename = "btnOrientation")]
    pub btn_orientation: String,
    /// Action buttons
    pub btns: Vec<ActionCardButton>,
}

/// A button in a DingTalk ActionCard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCardButton {
    /// Button display text
    pub title: String,
    /// URL to open when button is clicked
    #[serde(rename = "actionURL")]
    pub action_url: String,
}

/// Request payload for sending a DingTalk message via channels service.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendMessageRequest {
    /// Target channel/group ID
    channel_id: String,
    /// Message payload
    message: DingTalkActionCard,
}

/// Response from the channels service.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendMessageResponse {
    /// Whether the message was sent successfully
    success: bool,
    /// Message ID in DingTalk
    message_id: Option<String>,
    /// Error message if failed
    error: Option<String>,
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
        let card = self.build_card(request);

        let payload = SendMessageRequest {
            channel_id: channel_id.to_string(),
            message: card,
        };

        let response = self
            .client
            .post(format!("{}/dingtalk/send", self.channels_endpoint))
            .json(&payload)
            .send()
            .await
            .context("Failed to send DingTalk message")?;

        let result: SendMessageResponse = response
            .json()
            .await
            .context("Failed to parse channels service response")?;

        if result.success {
            result
                .message_id
                .context("Message sent but no message_id returned")
        } else {
            anyhow::bail!(
                "Failed to send DingTalk message: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()> {
        let card = self.build_card(request);

        let payload = serde_json::json!({
            "message_id": message_id,
            "message": card,
        });

        let response = self
            .client
            .put(format!("{}/dingtalk/update", self.channels_endpoint))
            .json(&payload)
            .send()
            .await
            .context("Failed to update DingTalk message")?;

        let result: SendMessageResponse = response
            .json()
            .await
            .context("Failed to parse channels service response")?;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!(
                "Failed to update DingTalk message: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        // DingTalk callbacks come as URL query string in the payload
        let query = std::str::from_utf8(payload)
            .context("Invalid UTF-8 in callback payload")?;
        self.parse_callback_params(query)
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
            id: "req-dingtalk-001".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/awesome-project".to_string(),
                mr_id: 123,
            },
            status: ApprovalStatus::Pending,
            requester: "developer".to_string(),
            approvers: vec!["lead".to_string(), "admin".to_string()],
            title: "Add DingTalk integration".to_string(),
            description: Some("Implements DingTalk card rendering".to_string()),
            channel: "dingtalk".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_build_dingtalk_card() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let request = create_test_request();
        let card = renderer.build_card(&request);

        // Verify card structure
        assert_eq!(card.msgtype, "actionCard");
        assert!(card.action_card.title.contains("审批请求"));
        assert!(card.action_card.title.contains("🔐"));
        assert_eq!(card.action_card.btn_orientation, "1");

        // Verify content includes key information
        assert!(card.action_card.text.contains("Add DingTalk integration"));
        assert!(card.action_card.text.contains("【合并请求】"));
        assert!(card.action_card.text.contains("github"));
        assert!(card.action_card.text.contains("org/awesome-project"));
        assert!(card.action_card.text.contains("#123"));

        // Verify buttons for pending status
        assert_eq!(card.action_card.btns.len(), 2);

        let approve_btn = &card.action_card.btns[0];
        assert_eq!(approve_btn.title, "✅ 批准");
        assert!(approve_btn.action_url.contains("/hitl/callback/dingtalk"));
        assert!(approve_btn.action_url.contains("action=approve"));
        assert!(approve_btn.action_url.contains("id=req-dingtalk-001"));

        let reject_btn = &card.action_card.btns[1];
        assert_eq!(reject_btn.title, "❌ 拒绝");
        assert!(reject_btn.action_url.contains("action=reject"));
        assert!(reject_btn.action_url.contains("id=req-dingtalk-001"));
    }

    #[test]
    fn test_build_dingtalk_card_approved_status() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let mut request = create_test_request();
        request.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: Utc::now(),
        };

        let card = renderer.build_card(&request);

        // Approved cards should have no buttons
        assert!(card.action_card.btns.is_empty());
        assert!(card.action_card.title.contains("✅"));
    }

    #[test]
    fn test_build_dingtalk_card_rejected_status() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let mut request = create_test_request();
        request.status = ApprovalStatus::Rejected {
            by: "lead".to_string(),
            reason: Some("Code quality issues".to_string()),
            at: Utc::now(),
        };

        let card = renderer.build_card(&request);

        // Rejected cards should have no buttons
        assert!(card.action_card.btns.is_empty());
        assert!(card.action_card.title.contains("❌"));
    }

    #[test]
    fn test_build_dingtalk_card_cancelled_status() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let mut request = create_test_request();
        request.status = ApprovalStatus::Cancelled {
            reason: "Timeout".to_string(),
        };

        let card = renderer.build_card(&request);

        assert!(card.action_card.btns.is_empty());
        assert!(card.action_card.title.contains("⚠️"));
    }

    #[test]
    fn test_build_dingtalk_card_trading_type() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "trade-001".to_string(),
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
            channel: "dingtalk".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = renderer.build_card(&request);

        assert!(card.action_card.text.contains("【交易指令】"));
        assert!(card.action_card.text.contains("BTC"));
        assert!(card.action_card.text.contains("buy"));
        assert!(card.action_card.text.contains("0.5"));
    }

    #[test]
    fn test_build_dingtalk_card_risk_operation() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "risk-001".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production database".to_string(),
                risk_level: RiskLevel::Critical,
            },
            status: ApprovalStatus::Pending,
            requester: "dba".to_string(),
            approvers: vec!["cto".to_string()],
            title: "Database cleanup".to_string(),
            description: None,
            channel: "dingtalk".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let card = renderer.build_card(&request);

        assert!(card.action_card.text.contains("【风险操作】"));
        assert!(card.action_card.text.contains("Delete production database"));
        assert!(card.action_card.text.contains("危急"));
    }

    #[test]
    fn test_parse_dingtalk_callback_approve() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let query = "action=approve&id=req-123";
        let callback = renderer.parse_callback_params(query).unwrap();

        assert_eq!(callback.request_id, "req-123");
        assert_eq!(callback.action, CallbackAction::Approve);
        assert!(callback.platform_callback_id.starts_with("dingtalk-"));
    }

    #[test]
    fn test_parse_dingtalk_callback_reject() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let query = "action=reject&id=req-456";
        let callback = renderer.parse_callback_params(query).unwrap();

        assert_eq!(callback.request_id, "req-456");
        assert_eq!(callback.action, CallbackAction::Reject { reason: None });
    }

    #[test]
    fn test_parse_dingtalk_callback_with_user_id() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let query = "action=approve&id=req-789&user_id=admin@company.com";
        let callback = renderer.parse_callback_params(query).unwrap();

        assert_eq!(callback.request_id, "req-789");
        assert_eq!(callback.user_id, "admin@company.com");
        assert_eq!(callback.action, CallbackAction::Approve);
    }

    #[test]
    fn test_parse_dingtalk_callback_missing_action() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let query = "id=req-123";
        let result = renderer.parse_callback_params(query);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("action"));
    }

    #[test]
    fn test_parse_dingtalk_callback_missing_id() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let query = "action=approve";
        let result = renderer.parse_callback_params(query);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("id"));
    }

    #[test]
    fn test_parse_callback_trait_method() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        let payload = b"action=approve&id=req-trait-test";
        let callback = renderer.parse_callback(payload).unwrap();

        assert_eq!(callback.request_id, "req-trait-test");
        assert_eq!(callback.action, CallbackAction::Approve);
    }

    #[test]
    fn test_channel_type() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        assert_eq!(renderer.channel_type(), "dingtalk");
    }

    #[test]
    fn test_action_card_serialization() {
        let card = DingTalkActionCard {
            msgtype: "actionCard".to_string(),
            action_card: ActionCardContent {
                title: "🔐 审批请求".to_string(),
                text: "### Test\n\nContent".to_string(),
                btn_orientation: "1".to_string(),
                btns: vec![
                    ActionCardButton {
                        title: "✅ 批准".to_string(),
                        action_url: "https://example.com/approve".to_string(),
                    },
                    ActionCardButton {
                        title: "❌ 拒绝".to_string(),
                        action_url: "https://example.com/reject".to_string(),
                    },
                ],
            },
        };

        let json = serde_json::to_string(&card).unwrap();

        // Verify JSON structure matches DingTalk API expectations
        assert!(json.contains("\"msgtype\":\"actionCard\""));
        assert!(json.contains("\"actionCard\""));
        assert!(json.contains("\"btnOrientation\":\"1\""));
        assert!(json.contains("\"actionURL\""));

        // Verify roundtrip
        let deserialized: DingTalkActionCard = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.msgtype, "actionCard");
        assert_eq!(deserialized.action_card.btns.len(), 2);
    }

    #[test]
    fn test_url_encoded_callback_params() {
        let renderer = DingTalkCardRenderer::new(
            "http://localhost:4431",
            "https://gateway.example.com",
        );

        // Test with URL-encoded characters
        let query = "action=reject&id=req-with%20spaces&user_id=user%40example.com";
        let callback = renderer.parse_callback_params(query).unwrap();

        assert_eq!(callback.request_id, "req-with spaces");
        assert_eq!(callback.user_id, "user@example.com");
        assert_eq!(callback.action, CallbackAction::Reject { reason: None });
    }
}
