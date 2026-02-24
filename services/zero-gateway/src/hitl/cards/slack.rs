//! Slack card renderer for approval requests.
//!
//! Implements the CardRenderer trait for Slack Block Kit messages.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{format_approval_summary, CallbackAction, CallbackData, CardRenderer};
use crate::hitl::ApprovalRequest;

/// Slack card renderer implementing Block Kit format.
pub struct SlackCardRenderer {
    /// Endpoint URL for the zero-channels service
    channels_endpoint: String,
    /// HTTP client for making requests
    client: reqwest::Client,
}

impl SlackCardRenderer {
    /// Create a new SlackCardRenderer.
    ///
    /// # Arguments
    /// * `channels_endpoint` - Base URL for the zero-channels service
    pub fn new(channels_endpoint: impl Into<String>) -> Self {
        Self {
            channels_endpoint: channels_endpoint.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Build Slack Block Kit blocks for an approval request.
    pub fn build_blocks(&self, request: &ApprovalRequest) -> Vec<SlackBlock> {
        let summary = format_approval_summary(request);

        vec![
            SlackBlock::Header {
                text: SlackTextObject {
                    r#type: "plain_text".to_string(),
                    text: "Approval Request".to_string(),
                    emoji: Some(true),
                },
            },
            SlackBlock::Section {
                text: SlackTextObject {
                    r#type: "mrkdwn".to_string(),
                    text: summary,
                    emoji: None,
                },
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackButtonElement {
                        r#type: "button".to_string(),
                        text: SlackTextObject {
                            r#type: "plain_text".to_string(),
                            text: "Approve".to_string(),
                            emoji: Some(true),
                        },
                        style: Some("primary".to_string()),
                        action_id: "hitl_approve".to_string(),
                        value: request.id.clone(),
                    },
                    SlackButtonElement {
                        r#type: "button".to_string(),
                        text: SlackTextObject {
                            r#type: "plain_text".to_string(),
                            text: "Reject".to_string(),
                            emoji: Some(true),
                        },
                        style: Some("danger".to_string()),
                        action_id: "hitl_reject".to_string(),
                        value: request.id.clone(),
                    },
                ],
            },
        ]
    }
}

/// Slack Block Kit block types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackBlock {
    /// Header block
    Header { text: SlackTextObject },
    /// Section block
    Section { text: SlackTextObject },
    /// Actions block containing interactive elements
    Actions { elements: Vec<SlackButtonElement> },
}

/// Slack text object for block content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackTextObject {
    /// Type of text: "plain_text" or "mrkdwn"
    pub r#type: String,
    /// The actual text content
    pub text: String,
    /// Whether to render emoji codes (only for plain_text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<bool>,
}

/// Slack button element for actions block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackButtonElement {
    /// Element type, always "button"
    pub r#type: String,
    /// Button text
    pub text: SlackTextObject,
    /// Button style: "primary" or "danger"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Action identifier for callback routing
    pub action_id: String,
    /// Value passed in callback
    pub value: String,
}

/// Slack callback payload structure.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackCallback {
    /// User who triggered the action
    pub user: SlackUser,
    /// Actions triggered
    pub actions: Vec<SlackAction>,
    /// Container information (optional)
    #[serde(default)]
    pub container: Option<SlackContainer>,
}

/// Slack user information in callback.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackUser {
    /// User ID in Slack
    pub id: String,
    /// Username (optional)
    #[serde(default)]
    pub username: Option<String>,
}

/// Slack action in callback.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackAction {
    /// Action ID that was triggered
    pub action_id: String,
    /// Value associated with the action
    pub value: String,
}

/// Slack container information.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackContainer {
    /// Message timestamp (used as message ID in Slack)
    #[serde(default)]
    pub message_ts: Option<String>,
}

/// Request payload for sending Slack message.
#[derive(Debug, Serialize)]
struct SendSlackMessageRequest {
    channel: String,
    blocks: Vec<SlackBlock>,
}

/// Response from sending Slack message.
#[derive(Debug, Deserialize)]
struct SendSlackMessageResponse {
    ok: bool,
    ts: Option<String>,
    error: Option<String>,
}

/// Request payload for updating Slack message.
#[derive(Debug, Serialize)]
struct UpdateSlackMessageRequest {
    channel: String,
    ts: String,
    blocks: Vec<SlackBlock>,
}

#[async_trait]
impl CardRenderer for SlackCardRenderer {
    fn channel_type(&self) -> &'static str {
        "slack"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let blocks = self.build_blocks(request);
        let payload = SendSlackMessageRequest {
            channel: channel_id.to_string(),
            blocks,
        };

        let url = format!("{}/slack/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send Slack message: {}", e))?;

        let result: SendSlackMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Slack response: {}", e))?;

        if result.ok {
            result
                .ts
                .ok_or_else(|| anyhow!("Slack response missing message timestamp"))
        } else {
            Err(anyhow!(
                "Slack API error: {}",
                result.error.unwrap_or_else(|| "unknown".to_string())
            ))
        }
    }

    async fn update_card(&self, request: &ApprovalRequest, message_id: &str) -> Result<()> {
        let blocks = self.build_blocks(request);
        let payload = UpdateSlackMessageRequest {
            channel: request.channel.clone(),
            ts: message_id.to_string(),
            blocks,
        };

        let url = format!("{}/slack/update", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to update Slack message: {}", e))?;

        let result: SendSlackMessageResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Slack response: {}", e))?;

        if result.ok {
            Ok(())
        } else {
            Err(anyhow!(
                "Slack API error: {}",
                result.error.unwrap_or_else(|| "unknown".to_string())
            ))
        }
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let callback: SlackCallback = serde_json::from_slice(payload)
            .map_err(|e| anyhow!("Failed to parse Slack callback: {}", e))?;

        let action = callback
            .actions
            .first()
            .ok_or_else(|| anyhow!("No actions in Slack callback"))?;

        let callback_action = match action.action_id.as_str() {
            "hitl_approve" => CallbackAction::Approve,
            "hitl_reject" => CallbackAction::Reject { reason: None },
            other => {
                return Err(anyhow!("Unknown Slack action_id: {}", other));
            }
        };

        let platform_callback_id = callback
            .container
            .as_ref()
            .and_then(|c| c.message_ts.clone())
            .unwrap_or_else(|| format!("slack-{}", callback.user.id));

        Ok(CallbackData {
            request_id: action.value.clone(),
            action: callback_action,
            user_id: callback.user.id.clone(),
            platform_callback_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitl::{ApprovalStatus, ApprovalType};
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
            channel: "C123456".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_build_slack_blocks() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");
        let request = create_test_request();

        let blocks = renderer.build_blocks(&request);

        assert_eq!(blocks.len(), 3);

        // Verify header block
        match &blocks[0] {
            SlackBlock::Header { text } => {
                assert_eq!(text.r#type, "plain_text");
                assert!(text.text.contains("Approval Request"));
            }
            _ => panic!("Expected Header block"),
        }

        // Verify section block
        match &blocks[1] {
            SlackBlock::Section { text } => {
                assert_eq!(text.r#type, "mrkdwn");
                assert!(text.text.contains("Add new feature"));
                assert!(text.text.contains("github"));
            }
            _ => panic!("Expected Section block"),
        }

        // Verify actions block
        match &blocks[2] {
            SlackBlock::Actions { elements } => {
                assert_eq!(elements.len(), 2);

                // Approve button
                assert_eq!(elements[0].text.text, "Approve");
                assert_eq!(elements[0].style, Some("primary".to_string()));
                assert_eq!(elements[0].action_id, "hitl_approve");
                assert_eq!(elements[0].value, "req-12345");

                // Reject button
                assert_eq!(elements[1].text.text, "Reject");
                assert_eq!(elements[1].style, Some("danger".to_string()));
                assert_eq!(elements[1].action_id, "hitl_reject");
                assert_eq!(elements[1].value, "req-12345");
            }
            _ => panic!("Expected Actions block"),
        }
    }

    #[test]
    fn test_parse_slack_callback_approve() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "user": { "id": "U123456" },
            "actions": [{ "action_id": "hitl_approve", "value": "req-12345" }],
            "container": { "message_ts": "1234567890.123456" }
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-12345");
        assert_eq!(callback_data.action, CallbackAction::Approve);
        assert_eq!(callback_data.user_id, "U123456");
        assert_eq!(callback_data.platform_callback_id, "1234567890.123456");
    }

    #[test]
    fn test_parse_slack_callback_reject() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "user": { "id": "U789" },
            "actions": [{ "action_id": "hitl_reject", "value": "req-abc" }]
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.request_id, "req-abc");
        assert_eq!(callback_data.action, CallbackAction::Reject { reason: None });
        assert_eq!(callback_data.user_id, "U789");
        // Without container, falls back to user-based ID
        assert!(callback_data.platform_callback_id.contains("U789"));
    }

    #[test]
    fn test_parse_slack_callback_unknown_action() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "user": { "id": "U123" },
            "actions": [{ "action_id": "unknown_action", "value": "req-123" }]
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown Slack action_id"));
    }

    #[test]
    fn test_parse_slack_callback_empty_actions() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "user": { "id": "U123" },
            "actions": []
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No actions"));
    }

    #[test]
    fn test_parse_slack_callback_invalid_json() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let invalid_json = b"not valid json";

        let result = renderer.parse_callback(invalid_json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse Slack callback"));
    }

    #[test]
    fn test_blocks_serialization() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");
        let request = create_test_request();

        let blocks = renderer.build_blocks(&request);
        let json = serde_json::to_string(&blocks).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"type\":\"header\""));
        assert!(json.contains("\"type\":\"section\""));
        assert!(json.contains("\"type\":\"actions\""));
        assert!(json.contains("\"action_id\":\"hitl_approve\""));
        assert!(json.contains("\"action_id\":\"hitl_reject\""));
        assert!(json.contains("\"style\":\"primary\""));
        assert!(json.contains("\"style\":\"danger\""));
    }

    #[test]
    fn test_channel_type() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");
        assert_eq!(renderer.channel_type(), "slack");
    }

    #[test]
    fn test_slack_callback_with_username() {
        let renderer = SlackCardRenderer::new("http://localhost:4431");

        let callback_json = r#"{
            "user": { "id": "U123456", "username": "johndoe" },
            "actions": [{ "action_id": "hitl_approve", "value": "req-xyz" }],
            "container": { "message_ts": "9999.8888" }
        }"#;

        let result = renderer.parse_callback(callback_json.as_bytes());
        assert!(result.is_ok());

        let callback_data = result.unwrap();
        assert_eq!(callback_data.user_id, "U123456");
    }
}
