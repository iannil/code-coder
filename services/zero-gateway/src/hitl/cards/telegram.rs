//! Telegram card renderer for approval requests.
//!
//! Implements the CardRenderer trait for Telegram inline keyboards.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{format_approval_summary, CallbackAction, CallbackData, CardRenderer};
use crate::hitl::{ApprovalRequest, ApprovalStatus};

/// Telegram inline keyboard button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboardButton {
    /// Button display text
    pub text: String,
    /// Callback data sent when button is pressed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
}

/// Telegram inline keyboard markup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboardMarkup {
    /// Rows of buttons
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

/// Request payload for sending a message via zero-channels.
#[derive(Debug, Clone, Serialize)]
struct SendMessageRequest {
    /// Channel type (always "telegram")
    channel: String,
    /// Chat ID to send to
    chat_id: String,
    /// Message text
    text: String,
    /// Optional reply markup
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<InlineKeyboardMarkup>,
    /// Parse mode for message formatting
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<String>,
}

/// Request payload for editing a message via zero-channels.
#[derive(Debug, Clone, Serialize)]
struct EditMessageRequest {
    /// Channel type (always "telegram")
    channel: String,
    /// Chat ID containing the message
    chat_id: String,
    /// Message ID to edit
    message_id: String,
    /// New message text
    text: String,
    /// Optional reply markup (buttons)
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<InlineKeyboardMarkup>,
    /// Parse mode for message formatting
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<String>,
}

/// Response from zero-channels after sending/editing a message.
#[derive(Debug, Clone, Deserialize)]
struct ChannelResponse {
    /// Whether the operation succeeded
    success: bool,
    /// Message ID (for send operations)
    #[serde(default)]
    message_id: Option<String>,
    /// Error message if failed
    #[serde(default)]
    error: Option<String>,
}

/// Telegram callback query payload.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct TelegramCallbackQuery {
    /// Unique identifier for this query
    id: String,
    /// User who pressed the button
    from: TelegramUser,
    /// Callback data from the button
    data: Option<String>,
    /// Message that contained the button
    #[serde(default)]
    message: Option<TelegramMessage>,
}

/// Telegram user info.
#[derive(Debug, Clone, Deserialize)]
struct TelegramUser {
    /// User ID
    id: i64,
    /// Username (optional)
    #[serde(default)]
    username: Option<String>,
}

/// Telegram message info.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct TelegramMessage {
    /// Message ID
    message_id: i64,
    /// Chat info
    chat: TelegramChat,
}

/// Telegram chat info.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct TelegramChat {
    /// Chat ID
    id: i64,
}

/// Telegram card renderer implementing the CardRenderer trait.
pub struct TelegramCardRenderer {
    /// Endpoint URL for zero-channels service
    channels_endpoint: String,
    /// HTTP client for making requests
    client: reqwest::Client,
}

impl TelegramCardRenderer {
    /// Create a new TelegramCardRenderer.
    ///
    /// # Arguments
    /// * `channels_endpoint` - Base URL for the zero-channels service (e.g., "http://localhost:4431")
    pub fn new(channels_endpoint: String) -> Self {
        Self {
            channels_endpoint,
            client: reqwest::Client::new(),
        }
    }

    /// Build the inline keyboard for an approval request.
    fn build_approval_keyboard(request_id: &str) -> InlineKeyboardMarkup {
        InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton {
                    text: "✅ 批准".to_string(),
                    callback_data: Some(format!("hitl:approve:{}", request_id)),
                },
                InlineKeyboardButton {
                    text: "❌ 拒绝".to_string(),
                    callback_data: Some(format!("hitl:reject:{}", request_id)),
                },
            ]],
        }
    }

    /// Format the status text for a completed approval.
    fn format_status_text(request: &ApprovalRequest) -> String {
        let summary = format_approval_summary(request);
        match &request.status {
            ApprovalStatus::Pending => summary,
            ApprovalStatus::Approved { by, at } => {
                format!(
                    "{}\n\n✅ 已批准\n审批人: {}\n时间: {}",
                    summary,
                    by,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            ApprovalStatus::Rejected { by, reason, at } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                format!(
                    "{}\n\n❌ 已拒绝\n审批人: {}\n原因: {}\n时间: {}",
                    summary,
                    by,
                    reason_text,
                    at.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            ApprovalStatus::Cancelled { reason } => {
                format!("{}\n\n⚠️ 已取消\n原因: {}", summary, reason)
            }
        }
    }

    /// Parse callback data from format "hitl:action:request_id".
    fn parse_callback_data_str(data: &str) -> Result<(String, CallbackAction)> {
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() < 3 || parts[0] != "hitl" {
            anyhow::bail!("Invalid callback data format: {}", data);
        }

        let action = match parts[1] {
            "approve" => CallbackAction::Approve,
            "reject" => {
                let reason = if parts.len() > 3 {
                    Some(parts[3..].join(":"))
                } else {
                    None
                };
                CallbackAction::Reject { reason }
            }
            _ => anyhow::bail!("Unknown action: {}", parts[1]),
        };

        let request_id = parts[2].to_string();
        Ok((request_id, action))
    }
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
        let text = format_approval_summary(request);
        let keyboard = Self::build_approval_keyboard(&request.id);

        let payload = SendMessageRequest {
            channel: "telegram".to_string(),
            chat_id: channel_id.to_string(),
            text,
            reply_markup: Some(keyboard),
            parse_mode: None,
        };

        let response = self
            .client
            .post(format!("{}/send", self.channels_endpoint))
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to zero-channels")?;

        let channel_response: ChannelResponse = response
            .json()
            .await
            .context("Failed to parse zero-channels response")?;

        if !channel_response.success {
            anyhow::bail!(
                "Failed to send approval card: {}",
                channel_response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        channel_response
            .message_id
            .context("No message_id in response")
    }

    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()> {
        let text = Self::format_status_text(request);

        // For terminal statuses, remove the keyboard
        let reply_markup = if request.status.is_terminal() {
            None
        } else {
            Some(Self::build_approval_keyboard(&request.id))
        };

        // Extract chat_id from message_id (format: "chat_id:message_id")
        let parts: Vec<&str> = message_id.split(':').collect();
        let (chat_id, msg_id) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // Fallback: use request's channel as chat_id
            (request.channel.clone(), message_id.to_string())
        };

        let payload = EditMessageRequest {
            channel: "telegram".to_string(),
            chat_id,
            message_id: msg_id,
            text,
            reply_markup,
            parse_mode: None,
        };

        let response = self
            .client
            .post(format!("{}/edit", self.channels_endpoint))
            .json(&payload)
            .send()
            .await
            .context("Failed to send edit request to zero-channels")?;

        let channel_response: ChannelResponse = response
            .json()
            .await
            .context("Failed to parse zero-channels edit response")?;

        if !channel_response.success {
            anyhow::bail!(
                "Failed to update approval card: {}",
                channel_response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let callback_query: TelegramCallbackQuery =
            serde_json::from_slice(payload).context("Failed to parse Telegram callback query")?;

        let data = callback_query
            .data
            .as_ref()
            .context("No callback data in query")?;

        let (request_id, action) = Self::parse_callback_data_str(data)?;

        let user_id = callback_query
            .from
            .username
            .unwrap_or_else(|| callback_query.from.id.to_string());

        Ok(CallbackData {
            request_id,
            action,
            user_id,
            platform_callback_id: callback_query.id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_data_approve() {
        let (request_id, action) =
            TelegramCardRenderer::parse_callback_data_str("hitl:approve:req-123").unwrap();

        assert_eq!(request_id, "req-123");
        assert_eq!(action, CallbackAction::Approve);
    }

    #[test]
    fn test_parse_callback_data_reject() {
        let (request_id, action) =
            TelegramCardRenderer::parse_callback_data_str("hitl:reject:req-456").unwrap();

        assert_eq!(request_id, "req-456");
        assert_eq!(action, CallbackAction::Reject { reason: None });
    }

    #[test]
    fn test_parse_callback_data_reject_with_reason() {
        let (request_id, action) =
            TelegramCardRenderer::parse_callback_data_str("hitl:reject:req-789:code quality")
                .unwrap();

        assert_eq!(request_id, "req-789");
        assert_eq!(
            action,
            CallbackAction::Reject {
                reason: Some("code quality".to_string())
            }
        );
    }

    #[test]
    fn test_parse_callback_data_invalid_format() {
        let result = TelegramCardRenderer::parse_callback_data_str("invalid");
        assert!(result.is_err());

        let result = TelegramCardRenderer::parse_callback_data_str("other:approve:123");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_approval_keyboard() {
        let keyboard = TelegramCardRenderer::build_approval_keyboard("test-request-id");

        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2);

        let approve_btn = &keyboard.inline_keyboard[0][0];
        assert_eq!(approve_btn.text, "✅ 批准");
        assert_eq!(
            approve_btn.callback_data,
            Some("hitl:approve:test-request-id".to_string())
        );

        let reject_btn = &keyboard.inline_keyboard[0][1];
        assert_eq!(reject_btn.text, "❌ 拒绝");
        assert_eq!(
            reject_btn.callback_data,
            Some("hitl:reject:test-request-id".to_string())
        );
    }

    #[test]
    fn test_channel_type() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".to_string());
        assert_eq!(renderer.channel_type(), "telegram");
    }

    #[test]
    fn test_parse_callback_full_payload() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".to_string());

        let payload = serde_json::json!({
            "id": "callback-123",
            "from": {
                "id": 12345678,
                "username": "testuser"
            },
            "data": "hitl:approve:req-abc",
            "message": {
                "message_id": 999,
                "chat": {
                    "id": -100123456
                }
            }
        });

        let result = renderer.parse_callback(payload.to_string().as_bytes()).unwrap();

        assert_eq!(result.request_id, "req-abc");
        assert_eq!(result.action, CallbackAction::Approve);
        assert_eq!(result.user_id, "testuser");
        assert_eq!(result.platform_callback_id, "callback-123");
    }

    #[test]
    fn test_parse_callback_without_username() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".to_string());

        let payload = serde_json::json!({
            "id": "callback-456",
            "from": {
                "id": 87654321
            },
            "data": "hitl:reject:req-xyz"
        });

        let result = renderer.parse_callback(payload.to_string().as_bytes()).unwrap();

        assert_eq!(result.request_id, "req-xyz");
        assert_eq!(result.action, CallbackAction::Reject { reason: None });
        assert_eq!(result.user_id, "87654321");
        assert_eq!(result.platform_callback_id, "callback-456");
    }

    #[test]
    fn test_format_status_text_pending() {
        use chrono::Utc;

        let now = Utc::now();
        let request = ApprovalRequest {
            id: "test-id".to_string(),
            approval_type: crate::hitl::ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Pending,
            requester: "developer".to_string(),
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

        let text = TelegramCardRenderer::format_status_text(&request);
        assert!(text.contains("审批请求: Test PR"));
        assert!(!text.contains("已批准"));
        assert!(!text.contains("已拒绝"));
    }

    #[test]
    fn test_format_status_text_approved() {
        use chrono::Utc;

        let now = Utc::now();
        let request = ApprovalRequest {
            id: "test-id".to_string(),
            approval_type: crate::hitl::ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Approved {
                by: "admin".to_string(),
                at: now,
            },
            requester: "developer".to_string(),
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

        let text = TelegramCardRenderer::format_status_text(&request);
        assert!(text.contains("✅ 已批准"));
        assert!(text.contains("审批人: admin"));
    }

    #[test]
    fn test_format_status_text_rejected() {
        use chrono::Utc;

        let now = Utc::now();
        let request = ApprovalRequest {
            id: "test-id".to_string(),
            approval_type: crate::hitl::ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Rejected {
                by: "reviewer".to_string(),
                reason: Some("Needs more tests".to_string()),
                at: now,
            },
            requester: "developer".to_string(),
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

        let text = TelegramCardRenderer::format_status_text(&request);
        assert!(text.contains("❌ 已拒绝"));
        assert!(text.contains("审批人: reviewer"));
        assert!(text.contains("原因: Needs more tests"));
    }

    #[test]
    fn test_inline_keyboard_serialization() {
        let keyboard = TelegramCardRenderer::build_approval_keyboard("req-123");
        let json = serde_json::to_string(&keyboard).unwrap();

        assert!(json.contains("inline_keyboard"));
        assert!(json.contains("callback_data"));
        assert!(json.contains("hitl:approve:req-123"));
        assert!(json.contains("hitl:reject:req-123"));
    }
}
