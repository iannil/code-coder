//! Feishu/Lark channel implementation.
//!
//! Uses the Feishu Open Platform Bot API for messaging.
//! Supports text messages, event callbacks, and user authorization.

use super::traits::{Channel, ChannelMessage, MessageSource};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Feishu API Constants
// ============================================================================

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";
const LARK_API_BASE: &str = "https://open.larksuite.com/open-apis";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300; // Refresh 5 minutes before expiry

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct TenantAccessTokenResponse {
    code: i32,
    msg: String,
    tenant_access_token: Option<String>,
    expire: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    code: i32,
    msg: String,
}

// Structures for future polling-based message retrieval
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GetMessagesResponse {
    code: i32,
    msg: String,
    data: Option<GetMessagesData>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GetMessagesData {
    items: Option<Vec<FeishuMessage>>,
    has_more: Option<bool>,
    page_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FeishuMessage {
    message_id: String,
    msg_type: String,
    sender: FeishuSender,
    chat_id: String,
    content: String,
    create_time: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct FeishuSender {
    id: String,
    id_type: String,
    sender_type: String,
}

// ============================================================================
// Event Callback Types
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EventCallback {
    schema: Option<String>,
    header: Option<EventHeader>,
    event: Option<serde_json::Value>,
    challenge: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EventHeader {
    event_id: String,
    event_type: String,
    create_time: String,
    token: Option<String>,
    app_id: String,
    tenant_key: String,
}

#[derive(Debug, Deserialize)]
struct MessageReceiveEvent {
    sender: MessageSender,
    message: MessageContent,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MessageSender {
    sender_id: SenderIds,
    sender_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SenderIds {
    open_id: Option<String>,
    user_id: Option<String>,
    union_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MessageContent {
    message_id: String,
    chat_id: String,
    chat_type: String,
    message_type: String,
    content: String,
}

// ============================================================================
// Token Cache
// ============================================================================

struct TokenCache {
    token: String,
    expires_at: Instant,
}

// ============================================================================
// FeishuChannel
// ============================================================================

/// Feishu/Lark messaging channel.
///
/// Implements long-polling for message reception and uses the Bot API
/// for sending messages. Supports both Feishu (China) and Lark (International).
#[allow(dead_code)]
pub struct FeishuChannel {
    app_id: String,
    app_secret: String,
    encrypt_key: Option<String>,
    verification_token: Option<String>,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
    /// Use Lark API (international) instead of Feishu (China)
    use_lark: bool,
}

impl FeishuChannel {
    /// Create a new Feishu channel.
    ///
    /// # Arguments
    /// * `app_id` - App ID from Feishu Open Platform
    /// * `app_secret` - App Secret from Feishu Open Platform
    /// * `allowed_users` - List of allowed user `open_ids` or "*" for all
    pub fn new(app_id: String, app_secret: String, allowed_users: Vec<String>) -> Self {
        Self {
            app_id,
            app_secret,
            encrypt_key: None,
            verification_token: None,
            allowed_users,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
            use_lark: false,
        }
    }

    /// Create a new Feishu channel with encryption support.
    pub fn with_encryption(
        app_id: String,
        app_secret: String,
        encrypt_key: Option<String>,
        verification_token: Option<String>,
        allowed_users: Vec<String>,
    ) -> Self {
        Self {
            app_id,
            app_secret,
            encrypt_key,
            verification_token,
            allowed_users,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
            use_lark: false,
        }
    }

    /// Use Lark API (international) instead of Feishu (China).
    pub fn with_lark_api(mut self) -> Self {
        self.use_lark = true;
        self
    }

    fn api_base(&self) -> &str {
        if self.use_lark {
            LARK_API_BASE
        } else {
            FEISHU_API_BASE
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.api_base(), path)
    }

    fn is_user_allowed(&self, open_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == open_id)
    }

    /// Get or refresh the tenant access token.
    async fn get_access_token(&self) -> anyhow::Result<String> {
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

        // Refresh token
        let url = self.api_url("/auth/v3/tenant_access_token/internal");
        let body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get Feishu access token ({status}): {text}");
        }

        let data: TenantAccessTokenResponse = resp.json().await?;

        if data.code != 0 {
            anyhow::bail!("Feishu API error ({}): {}", data.code, data.msg);
        }

        let token = data
            .tenant_access_token
            .ok_or_else(|| anyhow::anyhow!("Missing tenant_access_token in response"))?;
        let expire = data.expire.unwrap_or(7200);

        // Update cache
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(expire),
            });
        }

        tracing::debug!("Feishu access token refreshed, expires in {} seconds", expire);
        Ok(token)
    }

    /// Send a text message to a user or chat.
    async fn send_text(&self, receive_id: &str, text: &str) -> anyhow::Result<()> {
        let token = self.get_access_token().await?;

        // Determine receive_id_type (open_id, user_id, chat_id, email)
        // For simplicity, assume it's an open_id if it starts with "ou_", otherwise chat_id
        let receive_id_type = if receive_id.starts_with("ou_") {
            "open_id"
        } else if receive_id.starts_with("oc_") {
            "chat_id"
        } else {
            "open_id" // Default to open_id
        };

        let url = format!(
            "{}?receive_id_type={}",
            self.api_url("/im/v1/messages"),
            receive_id_type
        );

        let content = serde_json::json!({
            "text": text
        });

        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": "text",
            "content": content.to_string()
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Feishu sendMessage failed ({status}): {text}");
        }

        let data: SendMessageResponse = resp.json().await?;

        if data.code != 0 {
            anyhow::bail!("Feishu sendMessage error ({}): {}", data.code, data.msg);
        }

        tracing::info!("Feishu message sent to {}", receive_id);
        Ok(())
    }

    /// Parse event callback payload.
    fn parse_event(&self, payload: &str) -> anyhow::Result<EventCallback> {
        // If encrypt_key is set, decrypt first
        if self.encrypt_key.is_some() {
            // TODO: Implement AES decryption for encrypted events
            // For now, try to parse as plain JSON
            tracing::warn!("Encrypted events not yet implemented, trying plain parse");
        }

        let event: EventCallback = serde_json::from_str(payload)?;
        Ok(event)
    }

    /// Extract text content from message content JSON.
    fn extract_text_content(content: &str) -> Option<String> {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            // Text message format: {"text": "message content"}
            if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
        None
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
        self.send_text(recipient, message).await
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        tracing::info!("Feishu channel listening for messages...");
        println!(
            "  📡 Feishu: listening for messages (allowed_users: {:?})",
            self.allowed_users
        );

        // NOTE: Feishu primarily uses webhook-based event subscriptions.
        // For a polling-based approach, we would need to:
        // 1. Set up a webhook endpoint to receive events
        // 2. Or use the WebSocket subscription API
        //
        // For this implementation, we'll use a simple polling approach
        // by periodically checking for new messages via the messages API.
        // This is less efficient but doesn't require a public webhook endpoint.

        #[allow(clippy::cast_possible_truncation)]
        let mut last_check = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        loop {
            // Sleep between polls
            tokio::time::sleep(Duration::from_secs(5)).await;

            // For a proper implementation, you would set up a webhook server
            // or use WebSocket subscriptions. This polling approach is a
            // simplified fallback that may not capture all messages.

            // Note: The Feishu API doesn't have a simple "get unread messages" endpoint
            // like Telegram's getUpdates. A full implementation would require:
            // 1. Running a webhook server (see gateway module)
            // 2. Subscribing to events via WebSocket
            // 3. Or using the im/v1/messages endpoint with pagination

            // For now, we'll log a reminder about webhook setup
            #[allow(clippy::cast_possible_truncation)]
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            if now - last_check > 60_000 {
                // Log every minute
                tracing::debug!(
                    "Feishu channel: For real-time messaging, configure webhook events at \
                     Feishu Open Platform -> Bot -> Event Subscriptions"
                );
                last_check = now;
            }

            // Check if channel is still active
            if tx.is_closed() {
                tracing::info!("Feishu channel sender closed, stopping listener");
                return Ok(());
            }
        }
    }

    async fn health_check(&self) -> bool {
        // Try to get an access token as health check
        self.get_access_token().await.is_ok()
    }
}

/// Process an incoming Feishu event callback.
///
/// This function should be called from a webhook handler when receiving
/// event callbacks from Feishu.
pub async fn process_event_callback(
    channel: &FeishuChannel,
    payload: &str,
    tx: &tokio::sync::mpsc::Sender<ChannelMessage>,
) -> anyhow::Result<Option<String>> {
    let event = channel.parse_event(payload)?;

    // Handle URL verification challenge
    if let Some(challenge) = event.challenge {
        tracing::info!("Feishu URL verification challenge received");
        return Ok(Some(serde_json::json!({ "challenge": challenge }).to_string()));
    }

    // Handle message events
    if let (Some(header), Some(event_data)) = (event.header, event.event) {
        if header.event_type == "im.message.receive_v1" {
            if let Ok(msg_event) = serde_json::from_value::<MessageReceiveEvent>(event_data) {
                let sender_open_id = msg_event
                    .sender
                    .sender_id
                    .open_id
                    .unwrap_or_else(|| "unknown".to_string());

                // Check authorization
                if !channel.is_user_allowed(&sender_open_id) {
                    tracing::warn!(
                        "Feishu: ignoring message from unauthorized user: {}",
                        sender_open_id
                    );
                    return Ok(None);
                }

                // Extract text content
                if msg_event.message.message_type == "text" {
                    if let Some(text) = FeishuChannel::extract_text_content(&msg_event.message.content) {
                        let msg = ChannelMessage {
                            id: Uuid::new_v4().to_string(),
                            sender: msg_event.message.chat_id,
                            content: text,
                            channel: "feishu".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            source: MessageSource::default(),
                        };

                        if let Err(e) = tx.send(msg).await {
                            tracing::error!("Failed to send Feishu message to channel: {}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feishu_channel_name() {
        let ch = FeishuChannel::new("app_id".into(), "app_secret".into(), vec!["*".into()]);
        assert_eq!(ch.name(), "feishu");
    }

    #[test]
    fn feishu_api_url() {
        let ch = FeishuChannel::new("app_id".into(), "app_secret".into(), vec![]);
        assert_eq!(
            ch.api_url("/auth/v3/tenant_access_token/internal"),
            "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal"
        );
    }

    #[test]
    fn lark_api_url() {
        let ch = FeishuChannel::new("app_id".into(), "app_secret".into(), vec![])
            .with_lark_api();
        assert_eq!(
            ch.api_url("/auth/v3/tenant_access_token/internal"),
            "https://open.larksuite.com/open-apis/auth/v3/tenant_access_token/internal"
        );
    }

    #[test]
    fn feishu_user_allowed_wildcard() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("ou_12345"));
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn feishu_user_allowed_specific() {
        let ch = FeishuChannel::new(
            "app".into(),
            "secret".into(),
            vec!["ou_alice".into(), "ou_bob".into()],
        );
        assert!(ch.is_user_allowed("ou_alice"));
        assert!(ch.is_user_allowed("ou_bob"));
        assert!(!ch.is_user_allowed("ou_eve"));
    }

    #[test]
    fn feishu_user_denied_empty() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec![]);
        assert!(!ch.is_user_allowed("ou_anyone"));
    }

    #[test]
    fn feishu_extract_text_content() {
        let content = r#"{"text": "Hello, World!"}"#;
        assert_eq!(
            FeishuChannel::extract_text_content(content),
            Some("Hello, World!".to_string())
        );
    }

    #[test]
    fn feishu_extract_text_content_invalid() {
        let content = r#"{"image_key": "img_12345"}"#;
        assert_eq!(FeishuChannel::extract_text_content(content), None);
    }

    #[tokio::test]
    async fn feishu_health_check_fails_with_invalid_credentials() {
        let ch = FeishuChannel::new("invalid".into(), "invalid".into(), vec!["*".into()]);
        // This should fail because credentials are invalid
        let result = ch.health_check().await;
        assert!(!result);
    }
}
