//! DingTalk (钉钉) channel adapter.
//!
//! Uses the DingTalk Server API for messaging.
//! Supports text messages, Markdown messages, Outgoing Robot webhooks, and event callbacks.
//!
//! ## API Documentation
//! - https://open.dingtalk.com/document/orgapp/robot-message-types-and-data-format
//! - https://open.dingtalk.com/document/orgapp/create-an-internal-robot

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Constants
// ============================================================================

const DINGTALK_API_BASE: &str = "https://api.dingtalk.com";
const OAPI_BASE: &str = "https://oapi.dingtalk.com";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "expireIn")]
    expire_in: Option<u64>,
    // Error fields (may be present in older API)
    errcode: Option<i32>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    #[serde(default)]
    errcode: i32,
    #[serde(default)]
    errmsg: String,
}

// ============================================================================
// Outgoing Robot Webhook Types
// ============================================================================

/// Incoming webhook message from DingTalk Outgoing Robot.
#[derive(Debug, Clone, Deserialize)]
pub struct OutgoingRobotMessage {
    /// Message ID
    #[serde(rename = "msgId")]
    pub msg_id: Option<String>,
    /// Conversation ID
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// Conversation type: "1" for 1:1, "2" for group
    #[serde(rename = "conversationType")]
    pub conversation_type: String,
    /// Conversation title (group name for groups)
    #[serde(rename = "conversationTitle")]
    pub conversation_title: Option<String>,
    /// The @ed user's staff ID
    #[serde(rename = "atUsers")]
    pub at_users: Option<Vec<AtUser>>,
    /// Chat bot user ID
    #[serde(rename = "chatbotUserId")]
    pub chatbot_user_id: Option<String>,
    /// Sender's staff ID
    #[serde(rename = "senderId")]
    pub sender_id: String,
    /// Sender's nick
    #[serde(rename = "senderNick")]
    pub sender_nick: Option<String>,
    /// Sender's staff ID (enterprise internal)
    #[serde(rename = "senderStaffId")]
    pub sender_staff_id: Option<String>,
    /// Sender's corp ID
    #[serde(rename = "senderCorpId")]
    pub sender_corp_id: Option<String>,
    /// Is it an admin?
    #[serde(rename = "isAdmin")]
    pub is_admin: Option<bool>,
    /// Session webhook URL for replying
    #[serde(rename = "sessionWebhook")]
    pub session_webhook: Option<String>,
    /// Session webhook expire time
    #[serde(rename = "sessionWebhookExpiredTime")]
    pub session_webhook_expired_time: Option<i64>,
    /// Message type: "text", "richText", etc.
    #[serde(rename = "msgtype")]
    pub msg_type: String,
    /// Text content (for text messages)
    pub text: Option<TextContent>,
    /// Creation timestamp
    #[serde(rename = "createAt")]
    pub create_at: Option<i64>,
    /// Robot code
    #[serde(rename = "robotCode")]
    pub robot_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtUser {
    /// DingTalk user ID
    #[serde(rename = "dingtalkId")]
    pub dingtalk_id: String,
    /// Staff ID (for enterprise internal users)
    #[serde(rename = "staffId")]
    pub staff_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextContent {
    pub content: String,
}

// ============================================================================
// Message Types for Sending
// ============================================================================

#[derive(Debug, Serialize)]
struct TextMessage {
    msgtype: &'static str,
    text: TextBody,
}

#[derive(Debug, Serialize)]
struct TextBody {
    content: String,
}

#[derive(Debug, Serialize)]
struct MarkdownMessage {
    msgtype: &'static str,
    markdown: MarkdownBody,
}

#[derive(Debug, Serialize)]
struct MarkdownBody {
    title: String,
    text: String,
}

// ============================================================================
// Token Cache
// ============================================================================

struct TokenCache {
    token: String,
    expires_at: Instant,
}

// ============================================================================
// DingTalkChannel
// ============================================================================

/// DingTalk (钉钉) messaging channel.
pub struct DingTalkChannel {
    app_key: String,
    app_secret: String,
    robot_code: Option<String>,
    outgoing_token: Option<String>,
    allowed_users: Vec<String>,
    #[allow(dead_code)]
    stream_mode: bool,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl DingTalkChannel {
    /// Create a new DingTalk channel.
    pub fn new(
        app_key: String,
        app_secret: String,
        allowed_users: Vec<String>,
    ) -> Self {
        Self {
            app_key,
            app_secret,
            robot_code: None,
            outgoing_token: None,
            allowed_users,
            stream_mode: false,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new DingTalk channel with full configuration.
    pub fn with_config(
        app_key: String,
        app_secret: String,
        robot_code: Option<String>,
        outgoing_token: Option<String>,
        allowed_users: Vec<String>,
        stream_mode: bool,
    ) -> Self {
        Self {
            app_key,
            app_secret,
            robot_code,
            outgoing_token,
            allowed_users,
            stream_mode,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// Get or refresh the access token.
    pub async fn get_access_token(&self) -> anyhow::Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                let now = Instant::now();
                if cached.expires_at > now + Duration::from_secs(TOKEN_REFRESH_MARGIN_SECS) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Use new API endpoint
        let url = format!("{}/v1.0/oauth2/accessToken", DINGTALK_API_BASE);
        let body = serde_json::json!({
            "appKey": self.app_key,
            "appSecret": self.app_secret
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get DingTalk access token ({status}): {text}");
        }

        let data: AccessTokenResponse = resp.json().await?;

        if let Some(errcode) = data.errcode {
            if errcode != 0 {
                anyhow::bail!(
                    "DingTalk API error ({}): {}",
                    errcode,
                    data.errmsg.unwrap_or_default()
                );
            }
        }

        let token = data
            .access_token
            .ok_or_else(|| anyhow::anyhow!("Missing access_token in response"))?;
        let expire = data.expire_in.unwrap_or(7200);

        // Update cache
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(expire),
            });
        }

        tracing::debug!(
            "DingTalk access token refreshed, expires in {} seconds",
            expire
        );
        Ok(token)
    }

    /// Send a text message using session webhook.
    pub async fn send_via_webhook(&self, webhook_url: &str, text: &str) -> anyhow::Result<()> {
        let message = TextMessage {
            msgtype: "text",
            text: TextBody {
                content: text.to_string(),
            },
        };

        let resp = self.client.post(webhook_url).json(&message).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DingTalk webhook send failed ({status}): {err}");
        }

        tracing::info!("DingTalk message sent via webhook");
        Ok(())
    }

    /// Send a Markdown message using session webhook.
    pub async fn send_markdown_via_webhook(
        &self,
        webhook_url: &str,
        title: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let message = MarkdownMessage {
            msgtype: "markdown",
            markdown: MarkdownBody {
                title: title.to_string(),
                text: text.to_string(),
            },
        };

        let resp = self.client.post(webhook_url).json(&message).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DingTalk markdown send failed ({status}): {err}");
        }

        tracing::info!("DingTalk markdown sent via webhook");
        Ok(())
    }

    /// Send a message to a user/group via robot (requires robot_code).
    pub async fn send_robot_message(
        &self,
        open_conversation_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let robot_code = self
            .robot_code
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Robot code not configured"))?;

        let token = self.get_access_token().await?;
        let url = format!(
            "{}/robot/oToMessages/batchSend?access_token={}",
            OAPI_BASE, token
        );

        let body = serde_json::json!({
            "robotCode": robot_code,
            "msgKey": "sampleText",
            "msgParam": serde_json::json!({ "content": text }).to_string(),
            "openConversationId": open_conversation_id
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("DingTalk robot send failed ({status}): {err}");
        }

        let data: SendMessageResponse = resp.json().await?;
        if data.errcode != 0 {
            anyhow::bail!(
                "DingTalk robot send error ({}): {}",
                data.errcode,
                data.errmsg
            );
        }

        tracing::info!("DingTalk robot message sent");
        Ok(())
    }

    /// Verify outgoing robot signature.
    pub fn verify_signature(&self, timestamp: &str, sign: &str) -> bool {
        let secret = match &self.outgoing_token {
            Some(s) => s,
            None => return true, // No token configured, skip verification
        };

        let string_to_sign = format!("{}\n{}", timestamp, secret);

        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };

        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize().into_bytes();
        let computed_sign = base64::engine::general_purpose::STANDARD.encode(result);

        computed_sign == sign
    }

    /// Generate signature for outgoing webhook request.
    pub fn generate_signature(&self) -> anyhow::Result<(String, String)> {
        let secret = self
            .outgoing_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Outgoing token not configured"))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let string_to_sign = format!("{}\n{}", timestamp, secret);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| anyhow::anyhow!("HMAC error: {e}"))?;

        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize().into_bytes();
        let sign = base64::engine::general_purpose::STANDARD.encode(result);

        Ok((timestamp, sign))
    }

    /// Send a simple text message (convenience method for gateway handlers).
    pub async fn send_simple(&self, text: &str, webhook_url: &str) -> anyhow::Result<()> {
        self.send_via_webhook(webhook_url, text).await
    }
}

#[async_trait]
impl Channel for DingTalkChannel {
    fn name(&self) -> &'static str {
        "dingtalk"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Try to get an access token to verify credentials
        self.get_access_token()
            .await
            .map_err(|e| ChannelError::Auth(e.to_string()))?;

        tracing::info!("DingTalk channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        // For DingTalk, channel_id is expected to be the session webhook URL
        // or conversation ID depending on the use case
        let target = &message.channel_id;

        match message.content {
            OutgoingContent::Text { text } => {
                if target.starts_with("http") {
                    // It's a webhook URL
                    self.send_via_webhook(target, &text)
                        .await
                        .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
                } else {
                    // It's a conversation ID
                    self.send_robot_message(target, &text)
                        .await
                        .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
                }
            }
            OutgoingContent::Markdown { text } => {
                if target.starts_with("http") {
                    // Extract title from first line or use default
                    let (title, content) = text
                        .split_once('\n')
                        .map(|(t, c)| (t.trim_start_matches('#').trim(), c))
                        .unwrap_or(("Message", text.as_str()));

                    self.send_markdown_via_webhook(target, title, content)
                        .await
                        .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
                } else {
                    return Err(ChannelError::SendFailed(
                        "Markdown messages require session webhook URL".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ChannelError::SendFailed(
                    "DingTalk only supports text and markdown messages currently".to_string(),
                ));
            }
        }
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn listen<F>(&self, _callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        tracing::info!("DingTalk channel listening for messages...");
        tracing::info!(
            "DingTalk uses webhook-based events. Configure outgoing robot in DingTalk admin console."
        );

        // DingTalk uses webhooks, keep running until shutdown
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        self.get_access_token()
            .await
            .map_err(|e| ChannelError::Connection(e.to_string()))?;
        Ok(())
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        tracing::info!("DingTalk channel shutting down");
        Ok(())
    }
}

/// Process an incoming DingTalk outgoing robot callback.
///
/// This function should be called from a webhook handler.
pub fn process_outgoing_callback(
    channel: &DingTalkChannel,
    timestamp: Option<&str>,
    sign: Option<&str>,
    body: &str,
) -> anyhow::Result<(Option<String>, Option<ChannelMessage>)> {
    // Verify signature if timestamp and sign are provided
    if let (Some(ts), Some(s)) = (timestamp, sign) {
        if !channel.verify_signature(ts, s) {
            anyhow::bail!("Invalid DingTalk signature");
        }
    }

    // Parse the message
    let msg: OutgoingRobotMessage = serde_json::from_str(body)?;

    // Check if user is allowed
    if !channel.is_user_allowed(&msg.sender_id) {
        tracing::warn!(
            "DingTalk: ignoring message from unauthorized user: {}",
            msg.sender_id
        );
        return Ok((None, None));
    }

    // Handle text messages
    if msg.msg_type == "text" {
        if let Some(text_content) = msg.text {
            let channel_msg = ChannelMessage {
                id: msg.msg_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                channel_type: ChannelType::DingTalk,
                // Use session webhook as channel_id for easy replying
                channel_id: msg
                    .session_webhook
                    .unwrap_or_else(|| msg.conversation_id.clone()),
                user_id: msg.sender_staff_id.unwrap_or(msg.sender_id),
                content: MessageContent::Text {
                    text: text_content.content,
                },
                attachments: vec![],
                metadata: {
                    let mut m = std::collections::HashMap::new();
                    if let Some(nick) = msg.sender_nick {
                        m.insert("sender_nick".to_string(), nick);
                    }
                    if let Some(title) = msg.conversation_title {
                        m.insert("conversation_title".to_string(), title);
                    }
                    m.insert("conversation_type".to_string(), msg.conversation_type);
                    m
                },
                timestamp: msg.create_at.unwrap_or_else(|| {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64
                }),
                trace_id: zero_common::logging::generate_trace_id(),
                span_id: zero_common::logging::generate_span_id(),
                parent_span_id: None,
            };
            return Ok((None, Some(channel_msg)));
        }
    }

    Ok((None, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dingtalk_channel_name() {
        let ch = DingTalkChannel::new(
            "app_key".into(),
            "app_secret".into(),
            vec!["*".into()],
        );
        assert_eq!(ch.name(), "dingtalk");
    }

    #[test]
    fn dingtalk_user_allowed_wildcard() {
        let ch = DingTalkChannel::new(
            "app_key".into(),
            "app_secret".into(),
            vec!["*".into()],
        );
        assert!(ch.is_user_allowed("user123"));
    }

    #[test]
    fn dingtalk_user_allowed_specific() {
        let ch = DingTalkChannel::new(
            "app_key".into(),
            "app_secret".into(),
            vec!["user123".into()],
        );
        assert!(ch.is_user_allowed("user123"));
        assert!(!ch.is_user_allowed("user456"));
    }

    #[test]
    fn dingtalk_parse_outgoing_message() {
        let json = r#"{
            "conversationId": "cidconversation123",
            "conversationType": "2",
            "conversationTitle": "Test Group",
            "senderId": "user123",
            "senderNick": "Test User",
            "msgtype": "text",
            "text": {
                "content": "Hello World"
            },
            "createAt": 1699999999000
        }"#;

        let msg: OutgoingRobotMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.conversation_id, "cidconversation123");
        assert_eq!(msg.conversation_type, "2");
        assert_eq!(msg.msg_type, "text");
        assert_eq!(msg.text.unwrap().content, "Hello World");
    }

    #[test]
    fn dingtalk_verify_signature_no_token() {
        let ch = DingTalkChannel::new(
            "app_key".into(),
            "app_secret".into(),
            vec![],
        );
        // Should return true when no token is configured
        assert!(ch.verify_signature("1234567890", "somesign"));
    }
}
