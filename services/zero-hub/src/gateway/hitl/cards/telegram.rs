//! Telegram card renderer for approval requests.
//!
//! Implements the CardRenderer trait for Telegram Inline Keyboard messages.
//! Uses Telegram's native inline keyboard buttons for interactive approval.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{format_approval_summary, CallbackAction, CallbackData, CardRenderer};
use crate::gateway::hitl::{ApprovalRequest, ApprovalStatus};

/// Telegram card renderer for approval requests.
///
/// Sends messages with inline keyboard buttons to Telegram channels
/// through the zero-channels service.
pub struct TelegramCardRenderer {
    /// Endpoint for the zero-channels service
    channels_endpoint: String,
    /// HTTP client for making requests
    client: reqwest::Client,
}

impl TelegramCardRenderer {
    /// Create a new Telegram card renderer.
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

    /// Build the inline keyboard for an approval request.
    ///
    /// Returns a 2D array of buttons where each inner array is a row.
    pub fn build_inline_keyboard(request_id: &str) -> Vec<Vec<InlineKeyboardButton>> {
        vec![vec![
            InlineKeyboardButton {
                text: "✅ 批准".to_string(),
                callback_data: format!("approve:{}", request_id),
            },
            InlineKeyboardButton {
                text: "❌ 拒绝".to_string(),
                callback_data: format!("reject:{}", request_id),
            },
        ]]
    }

    /// Build the message text for an approval request.
    pub fn build_message_text(request: &ApprovalRequest) -> String {
        let summary = format_approval_summary(request);
        format!("🔐 <b>审批请求</b>\n\n{}", escape_html(&summary))
    }

    /// Build the status text for a decided request.
    pub fn build_status_text(request: &ApprovalRequest) -> String {
        let summary = format_approval_summary(request);
        let status_line = match &request.status {
            ApprovalStatus::Pending => "⏳ 等待审批".to_string(),
            ApprovalStatus::Approved { by, at } => {
                format!(
                    "✅ 已批准\n审批人: {}\n时间: {}",
                    by,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            ApprovalStatus::Rejected { by, reason, at } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                format!(
                    "❌ 已拒绝\n审批人: {}\n原因: {}\n时间: {}",
                    by,
                    reason_text,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            ApprovalStatus::Cancelled { reason } => {
                format!("⚠️ 已取消\n原因: {}", reason)
            }
        };

        format!(
            "<b>审批结果</b>\n\n{}\n\n<b>状态</b>\n{}",
            escape_html(&summary),
            escape_html(&status_line)
        )
    }
}

/// Escape HTML special characters for Telegram HTML parse mode.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A single inline keyboard button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboardButton {
    /// Button display text
    pub text: String,
    /// Callback data sent when button is pressed (max 64 bytes)
    pub callback_data: String,
}

/// Telegram callback query payload from button press.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramCallback {
    /// Unique identifier for this query
    pub id: String,
    /// User who pressed the button
    pub from: TelegramUser,
    /// Message with the callback button
    #[serde(default)]
    pub message: Option<TelegramMessage>,
    /// Data from the callback button
    pub data: String,
}

/// Telegram user info in callback.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUser {
    /// User ID
    pub id: i64,
    /// Username (optional)
    #[serde(default)]
    pub username: Option<String>,
    /// First name
    #[serde(default)]
    pub first_name: Option<String>,
}

/// Telegram message info in callback.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessage {
    /// Message ID
    pub message_id: i64,
    /// Chat info
    pub chat: TelegramChat,
}

/// Telegram chat info.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    /// Chat ID
    pub id: i64,
}

/// Request payload for sending Telegram message with inline keyboard.
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    chat_id: String,
    text: String,
    parse_mode: String,
    reply_markup: ReplyMarkup,
}

/// Reply markup with inline keyboard.
#[derive(Debug, Serialize)]
struct ReplyMarkup {
    inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

/// Request payload for editing Telegram message.
#[derive(Debug, Serialize)]
struct EditMessageRequest {
    chat_id: String,
    message_id: i64,
    text: String,
    parse_mode: String,
}

/// Response from Telegram send/edit message.
#[derive(Debug, Deserialize)]
struct TelegramResponse {
    ok: bool,
    result: Option<TelegramResult>,
    description: Option<String>,
}

/// Result from Telegram API.
#[derive(Debug, Deserialize)]
struct TelegramResult {
    message_id: Option<i64>,
}

#[async_trait]
impl CardRenderer for TelegramCardRenderer {
    fn channel_type(&self) -> &'static str {
        "telegram"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let text = Self::build_message_text(request);
        let keyboard = Self::build_inline_keyboard(&request.id);

        let payload = SendMessageRequest {
            chat_id: channel_id.to_string(),
            text,
            parse_mode: "HTML".to_string(),
            reply_markup: ReplyMarkup {
                inline_keyboard: keyboard,
            },
        };

        let url = format!("{}/telegram/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send Telegram message: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to send Telegram card ({}): {}",
                status,
                body
            ));
        }

        let result: TelegramResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Telegram response: {}", e))?;

        if !result.ok {
            return Err(anyhow!(
                "Telegram API error: {}",
                result.description.unwrap_or_else(|| "unknown".to_string())
            ));
        }

        let message_id = result
            .result
            .and_then(|r| r.message_id)
            .ok_or_else(|| anyhow!("Missing message_id in Telegram response"))?;

        Ok(message_id.to_string())
    }

    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()> {
        let text = Self::build_status_text(request);
        let msg_id: i64 = message_id
            .parse()
            .map_err(|_| anyhow!("Invalid message_id: {}", message_id))?;

        let payload = EditMessageRequest {
            chat_id: request.channel.clone(),
            message_id: msg_id,
            text,
            parse_mode: "HTML".to_string(),
        };

        let url = format!("{}/telegram/edit", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to edit Telegram message: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to update Telegram card ({}): {}",
                status,
                body
            ));
        }

        let result: TelegramResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Telegram response: {}", e))?;

        if !result.ok {
            return Err(anyhow!(
                "Telegram API error: {}",
                result.description.unwrap_or_else(|| "unknown".to_string())
            ));
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let callback: TelegramCallback = serde_json::from_slice(payload)
            .map_err(|e| anyhow!("Failed to parse Telegram callback: {}", e))?;

        // Parse callback_data format: "action:request_id"
        let parts: Vec<&str> = callback.data.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid callback data format: {}", callback.data));
        }

        let (action_str, request_id) = (parts[0], parts[1]);

        let action = match action_str {
            "approve" => CallbackAction::Approve,
            "reject" => CallbackAction::Reject { reason: None },
            other => {
                return Err(anyhow!("Unknown Telegram action: {}", other));
            }
        };

        let platform_callback_id = callback
            .message
            .as_ref()
            .map(|m| format!("{}:{}", m.chat.id, m.message_id))
            .unwrap_or_else(|| callback.id.clone());

        Ok(CallbackData {
            request_id: request_id.to_string(),
            action,
            user_id: callback.from.id.to_string(),
            platform_callback_id,
        })
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
            channel: "123456789".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_build_inline_keyboard() {
        let keyboard = TelegramCardRenderer::build_inline_keyboard("req-123");

        assert_eq!(keyboard.len(), 1); // One row
        assert_eq!(keyboard[0].len(), 2); // Two buttons

        assert_eq!(keyboard[0][0].text, "✅ 批准");
        assert_eq!(keyboard[0][0].callback_data, "approve:req-123");

        assert_eq!(keyboard[0][1].text, "❌ 拒绝");
        assert_eq!(keyboard[0][1].callback_data, "reject:req-123");
    }

    #[test]
    fn test_build_message_text() {
        let request = create_test_request();
        let text = TelegramCardRenderer::build_message_text(&request);

        assert!(text.contains("🔐"));
        assert!(text.contains("审批请求"));
        assert!(text.contains("Add new feature"));
        assert!(text.contains("github"));
    }

    #[test]
    fn test_build_status_text_approved() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: now,
        };

        let text = TelegramCardRenderer::build_status_text(&request);

        assert!(text.contains("审批结果"));
        assert!(text.contains("✅ 已批准"));
        assert!(text.contains("审批人: admin"));
    }

    #[test]
    fn test_build_status_text_rejected() {
        let now = Utc::now();
        let mut request = create_test_request();
        request.status = ApprovalStatus::Rejected {
            by: "reviewer".to_string(),
            reason: Some("Not ready".to_string()),
            at: now,
        };

        let text = TelegramCardRenderer::build_status_text(&request);

        assert!(text.contains("❌ 已拒绝"));
        assert!(text.contains("原因: Not ready"));
    }

    #[test]
    fn test_parse_callback_approve() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "id": "cb-123",
            "from": { "id": 123456789, "username": "testuser" },
            "message": { "message_id": 42, "chat": { "id": 987654321 } },
            "data": "approve:req-abc-123"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-abc-123");
        assert_eq!(callback_data.action, CallbackAction::Approve);
        assert_eq!(callback_data.user_id, "123456789");
        assert_eq!(callback_data.platform_callback_id, "987654321:42");
    }

    #[test]
    fn test_parse_callback_reject() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "id": "cb-456",
            "from": { "id": 111222333 },
            "data": "reject:req-xyz-789"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-xyz-789");
        assert_eq!(callback_data.action, CallbackAction::Reject { reason: None });
        assert_eq!(callback_data.user_id, "111222333");
        // Falls back to callback ID when message is not present
        assert_eq!(callback_data.platform_callback_id, "cb-456");
    }

    #[test]
    fn test_parse_callback_invalid_format() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "id": "cb-789",
            "from": { "id": 123 },
            "data": "invalid-no-colon"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid callback data format"));
    }

    #[test]
    fn test_parse_callback_unknown_action() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "id": "cb-abc",
            "from": { "id": 123 },
            "data": "unknown:req-123"
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown Telegram action"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("a < b"), "a &lt; b");
        assert_eq!(escape_html("a > b"), "a &gt; b");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn test_channel_type() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431");
        assert_eq!(renderer.channel_type(), "telegram");
    }

    #[test]
    fn test_inline_keyboard_button_serialization() {
        let button = InlineKeyboardButton {
            text: "Test".to_string(),
            callback_data: "test:123".to_string(),
        };

        let json = serde_json::to_string(&button).unwrap();
        assert!(json.contains("\"text\":\"Test\""));
        assert!(json.contains("\"callback_data\":\"test:123\""));
    }

    #[test]
    fn test_callback_data_length() {
        // Telegram has a 64-byte limit on callback_data
        // "approve:" = 8 bytes, leaving 56 for request_id
        // "reject:" = 7 bytes, leaving 57 for request_id
        let keyboard = TelegramCardRenderer::build_inline_keyboard("req-12345678-uuid-format");

        for row in &keyboard {
            for button in row {
                assert!(
                    button.callback_data.len() <= 64,
                    "Callback data exceeds 64 bytes: {}",
                    button.callback_data
                );
            }
        }
    }

    #[test]
    fn test_build_message_with_special_chars() {
        let now = Utc::now();
        let request = ApprovalRequest {
            id: "req-123".to_string(),
            approval_type: ApprovalType::ConfigChange {
                key: "max<tokens>".to_string(),
                old_value: "a & b".to_string(),
                new_value: "<new>".to_string(),
            },
            status: ApprovalStatus::Pending,
            requester: "user<script>".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Config & Update".to_string(),
            description: None,
            channel: "123".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        };

        let text = TelegramCardRenderer::build_message_text(&request);

        // Should escape HTML characters
        assert!(text.contains("&lt;"));
        assert!(text.contains("&gt;"));
        assert!(text.contains("&amp;"));
        // Should not contain raw HTML tags
        assert!(!text.contains("<script>"));
    }
}
