//! Feishu/Lark channel adapter.
//!
//! Uses the Feishu Open Platform Bot API for messaging.
//! Supports text messages, event callbacks, and user authorization.
//! Implements AES-256-CBC decryption for encrypted event callbacks.

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

// ============================================================================
// Constants
// ============================================================================

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";
const LARK_API_BASE: &str = "https://open.larksuite.com/open-apis";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;

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

// ============================================================================
// Event Callback Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct EventCallback {
    pub schema: Option<String>,
    pub header: Option<EventHeader>,
    pub event: Option<serde_json::Value>,
    pub challenge: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventHeader {
    pub event_id: String,
    pub event_type: String,
    pub create_time: String,
    pub token: Option<String>,
    pub app_id: String,
    pub tenant_key: String,
}

#[derive(Debug, Deserialize)]
struct MessageReceiveEvent {
    sender: MessageSender,
    message: FeishuMessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageSender {
    sender_id: SenderIds,
    #[allow(dead_code)]
    sender_type: String,
}

#[derive(Debug, Deserialize)]
struct SenderIds {
    open_id: Option<String>,
    #[allow(dead_code)]
    user_id: Option<String>,
    #[allow(dead_code)]
    union_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeishuMessageContent {
    #[allow(dead_code)]
    message_id: String,
    chat_id: String,
    #[allow(dead_code)]
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
pub struct FeishuChannel {
    app_id: String,
    app_secret: String,
    encrypt_key: Option<String>,
    #[allow(dead_code)]
    verification_token: Option<String>,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
    use_lark: bool,
}

impl FeishuChannel {
    /// Create a new Feishu channel.
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

        let receive_id_type = if receive_id.starts_with("ou_") {
            "open_id"
        } else if receive_id.starts_with("oc_") {
            "chat_id"
        } else {
            "open_id"
        };

        let url = format!(
            "{}?receive_id_type={}",
            self.api_url("/im/v1/messages"),
            receive_id_type
        );

        let content = serde_json::json!({ "text": text });

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
    pub fn parse_event(&self, payload: &str) -> anyhow::Result<EventCallback> {
        let json_value: serde_json::Value = serde_json::from_str(payload)?;

        let decrypted_payload = if let Some(encrypt) = json_value.get("encrypt").and_then(|e| e.as_str()) {
            match &self.encrypt_key {
                Some(key) => {
                    let decrypted = Self::decrypt_aes_cbc(key, encrypt)?;
                    tracing::debug!("Feishu event decrypted successfully");
                    decrypted
                }
                None => {
                    anyhow::bail!(
                        "Received encrypted Feishu event but no encrypt_key configured"
                    );
                }
            }
        } else {
            payload.to_string()
        };

        let event: EventCallback = serde_json::from_str(&decrypted_payload)?;
        Ok(event)
    }

    /// Decrypt Feishu encrypted event using AES-256-CBC.
    fn decrypt_aes_cbc(encrypt_key: &str, ciphertext_b64: &str) -> anyhow::Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(encrypt_key.as_bytes());
        let key_hash = hasher.finalize();

        let key: [u8; 32] = key_hash.into();
        let iv: [u8; 16] = key_hash[..16].try_into()?;

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(ciphertext_b64)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64 ciphertext: {e}"))?;

        let decryptor = Aes256CbcDec::new(&key.into(), &iv.into());
        let mut buffer = ciphertext.clone();
        let decrypted = decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| anyhow::anyhow!("AES decryption failed: {e}"))?;

        String::from_utf8(decrypted.to_vec())
            .map_err(|e| anyhow::anyhow!("Decrypted content is not valid UTF-8: {e}"))
    }

    /// Extract text content from message content JSON.
    fn extract_text_content(content: &str) -> Option<String> {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
        None
    }

    /// Parse event callback and return the decrypted JSON value.
    /// This is a convenience method for gateway/webhook handlers.
    pub fn parse_event_gateway(&self, payload: &str) -> anyhow::Result<serde_json::Value> {
        let json_value: serde_json::Value = serde_json::from_str(payload)?;

        // Check if the payload is encrypted
        if let Some(encrypt) = json_value.get("encrypt").and_then(|e| e.as_str()) {
            match &self.encrypt_key {
                Some(key) => {
                    let decrypted = Self::decrypt_aes_cbc(key, encrypt)?;
                    let decrypted_json: serde_json::Value = serde_json::from_str(&decrypted)?;
                    Ok(decrypted_json)
                }
                None => anyhow::bail!("Received encrypted Feishu event but no encrypt_key configured"),
            }
        } else {
            Ok(json_value)
        }
    }

    /// Send a simple text message (convenience method for gateway handlers).
    pub async fn send_simple(&self, text: &str, receive_id: &str) -> anyhow::Result<()> {
        self.send_text(receive_id, text).await
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn name(&self) -> &'static str {
        "feishu"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Try to get an access token to verify credentials
        self.get_access_token()
            .await
            .map_err(|e| ChannelError::Auth(e.to_string()))?;

        tracing::info!("Feishu channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        match message.content {
            OutgoingContent::Text { text } | OutgoingContent::Markdown { text } => {
                self.send_text(&message.channel_id, &text)
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
            }
            _ => {
                return Err(ChannelError::SendFailed(
                    "Feishu only supports text messages currently".to_string(),
                ));
            }
        }
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn listen<F>(&self, _callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        tracing::info!("Feishu channel listening for messages...");
        tracing::info!(
            "Feishu uses webhook-based events. Configure webhook at Feishu Open Platform."
        );

        // Feishu uses webhooks, keep running until shutdown
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
        tracing::info!("Feishu channel shutting down");
        Ok(())
    }
}

/// Process an incoming Feishu event callback.
///
/// This function should be called from a webhook handler.
pub fn process_event_callback(
    channel: &FeishuChannel,
    payload: &str,
) -> anyhow::Result<(Option<String>, Option<ChannelMessage>)> {
    let event = channel.parse_event(payload)?;

    // Handle URL verification challenge
    if let Some(challenge) = event.challenge {
        tracing::info!("Feishu URL verification challenge received");
        let response = serde_json::json!({ "challenge": challenge }).to_string();
        return Ok((Some(response), None));
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

                if !channel.is_user_allowed(&sender_open_id) {
                    tracing::warn!(
                        "Feishu: ignoring message from unauthorized user: {}",
                        sender_open_id
                    );
                    return Ok((None, None));
                }

                if msg_event.message.message_type == "text" {
                    if let Some(text) = FeishuChannel::extract_text_content(&msg_event.message.content)
                    {
                        let msg = ChannelMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            channel_type: ChannelType::Feishu,
                            channel_id: msg_event.message.chat_id,
                            user_id: sender_open_id,
                            content: MessageContent::Text { text },
                            attachments: vec![],
                            metadata: std::collections::HashMap::new(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as i64,
                            trace_id: zero_common::logging::generate_trace_id(),
                            span_id: zero_common::logging::generate_span_id(),
                            parent_span_id: None,
                        };
                        return Ok((None, Some(msg)));
                    }
                }
            }
        }
    }

    Ok((None, None))
}

// ============================================================================
// Meeting Minutes API Types
// ============================================================================

/// Meeting summary from Feishu calendar.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeetingSummary {
    /// Meeting ID
    pub meeting_id: String,
    /// Meeting title
    pub title: String,
    /// Meeting start time (RFC3339)
    pub start_time: String,
    /// Meeting end time (RFC3339)
    pub end_time: String,
    /// Meeting organizer
    pub organizer: String,
    /// List of attendees
    pub attendees: Vec<String>,
    /// Meeting notes/minutes content
    pub notes: String,
    /// Action items extracted
    pub action_items: Vec<ActionItem>,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Meeting location or link
    pub location: Option<String>,
}

/// Action item from meeting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionItem {
    /// Task description
    pub task: String,
    /// Assignee
    pub assignee: Option<String>,
    /// Due date
    pub due_date: Option<String>,
    /// Priority (high/medium/low)
    pub priority: Option<String>,
}

/// Meeting minutes retrieval response.
#[derive(Debug, serde::Deserialize)]
struct MeetingListResponse {
    code: i32,
    msg: String,
    data: Option<MeetingListData>,
}

#[derive(Debug, serde::Deserialize)]
struct MeetingListData {
    items: Option<Vec<MeetingItem>>,
    page_token: Option<String>,
    has_more: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct MeetingItem {
    event_id: String,
    summary: Option<String>,
    start_time: Option<TimeInfo>,
    end_time: Option<TimeInfo>,
    organizer: Option<Organizer>,
    attendees: Option<Vec<Attendee>>,
    location: Option<Location>,
    description: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TimeInfo {
    timestamp: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Organizer {
    user_id: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Attendee {
    user_id: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Location {
    name: Option<String>,
}

impl FeishuChannel {
    /// Get recent meetings from user's calendar.
    ///
    /// Requires `calendar:calendar` scope.
    pub async fn get_recent_meetings(
        &self,
        calendar_id: &str,
        days_back: u32,
    ) -> anyhow::Result<Vec<MeetingSummary>> {
        let token = self.get_access_token().await?;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(days_back as i64);

        let url = format!(
            "{}/calendar/v4/calendars/{}/events?start_time={}&end_time={}&page_size=50",
            self.api_url(""),
            calendar_id,
            start.timestamp(),
            now.timestamp()
        );

        let response: MeetingListResponse = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?
            .json()
            .await?;

        if response.code != 0 {
            anyhow::bail!("Failed to fetch meetings: {} - {}", response.code, response.msg);
        }

        let items = response
            .data
            .and_then(|d| d.items)
            .unwrap_or_default();

        let summaries = items
            .into_iter()
            .map(|item| {
                let notes = item.description.clone().unwrap_or_default();
                let action_items = Self::extract_action_items(&notes);
                let decisions = Self::extract_decisions(&notes);

                MeetingSummary {
                    meeting_id: item.event_id,
                    title: item.summary.unwrap_or_else(|| "Untitled Meeting".to_string()),
                    start_time: item
                        .start_time
                        .and_then(|t| t.timestamp)
                        .unwrap_or_default(),
                    end_time: item
                        .end_time
                        .and_then(|t| t.timestamp)
                        .unwrap_or_default(),
                    organizer: item
                        .organizer
                        .and_then(|o| o.display_name)
                        .unwrap_or_else(|| "Unknown".to_string()),
                    attendees: item
                        .attendees
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|a| a.display_name)
                        .collect(),
                    notes,
                    action_items,
                    decisions,
                    location: item.location.and_then(|l| l.name),
                }
            })
            .collect();

        Ok(summaries)
    }

    /// Extract action items from meeting notes.
    fn extract_action_items(notes: &str) -> Vec<ActionItem> {
        let mut items = Vec::new();

        // Look for common action item patterns
        // - [ ] Task @assignee due:2024-01-01
        // - TODO: Task
        // - Action: Task
        for line in notes.lines() {
            let line = line.trim();

            // Markdown checkbox pattern
            if line.starts_with("- [ ]") || line.starts_with("- [x]") {
                let task = line[5..].trim();
                items.push(Self::parse_action_item(task));
            }
            // TODO pattern
            else if line.to_uppercase().starts_with("TODO:") {
                let task = line[5..].trim();
                items.push(Self::parse_action_item(task));
            }
            // Action pattern
            else if line.to_uppercase().starts_with("ACTION:") {
                let task = line[7..].trim();
                items.push(Self::parse_action_item(task));
            }
            // Chinese patterns
            else if line.starts_with("待办:") || line.starts_with("待办：") {
                let task = line.chars().skip(3).collect::<String>();
                items.push(Self::parse_action_item(task.trim()));
            }
        }

        items
    }

    /// Parse action item with optional assignee and due date.
    fn parse_action_item(text: &str) -> ActionItem {
        let mut task = text.to_string();
        let mut assignee = None;
        let mut due_date = None;
        let mut priority = None;

        // Extract @assignee
        if let Some(at_pos) = text.find('@') {
            let end = text[at_pos + 1..]
                .find(|c: char| c.is_whitespace())
                .map(|i| at_pos + 1 + i)
                .unwrap_or(text.len());
            assignee = Some(text[at_pos + 1..end].to_string());
            task = text[..at_pos].to_string() + &text[end..];
        }

        // Extract due:YYYY-MM-DD
        if let Some(due_pos) = text.find("due:") {
            let date_start = due_pos + 4;
            let date_end = text[date_start..]
                .find(|c: char| c.is_whitespace())
                .map(|i| date_start + i)
                .unwrap_or(text.len());
            due_date = Some(text[date_start..date_end].to_string());
        }

        // Extract priority indicators
        if text.contains("!!") || text.to_lowercase().contains("urgent") {
            priority = Some("high".to_string());
        } else if text.contains("!") || text.to_lowercase().contains("important") {
            priority = Some("medium".to_string());
        }

        ActionItem {
            task: task.trim().to_string(),
            assignee,
            due_date,
            priority,
        }
    }

    /// Extract key decisions from meeting notes.
    fn extract_decisions(notes: &str) -> Vec<String> {
        let mut decisions = Vec::new();

        for line in notes.lines() {
            let line = line.trim();

            // Decision patterns
            if line.to_uppercase().starts_with("DECISION:") {
                decisions.push(line[9..].trim().to_string());
            } else if line.starts_with("决定:") || line.starts_with("决定：") {
                decisions.push(line.chars().skip(3).collect::<String>().trim().to_string());
            } else if line.to_uppercase().starts_with("AGREED:") {
                decisions.push(line[7..].trim().to_string());
            }
        }

        decisions
    }

    /// Create a document in Feishu Docs.
    ///
    /// Requires `docs:doc` scope.
    pub async fn create_document(
        &self,
        folder_token: &str,
        title: &str,
        content: &str,
    ) -> anyhow::Result<String> {
        let token = self.get_access_token().await?;

        let url = format!("{}/docx/v1/documents", self.api_url(""));

        let body = serde_json::json!({
            "folder_token": folder_token,
            "title": title
        });

        let response: serde_json::Value = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let code = response["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            anyhow::bail!("Failed to create document: {} - {}", code, msg);
        }

        let doc_token = response["data"]["document"]["document_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No document_id in response"))?
            .to_string();

        // Update document content
        self.update_document_content(&doc_token, content).await?;

        Ok(doc_token)
    }

    /// Update document content.
    async fn update_document_content(&self, doc_token: &str, content: &str) -> anyhow::Result<()> {
        let token = self.get_access_token().await?;

        // Get document blocks first
        let url = format!(
            "{}/docx/v1/documents/{}/blocks",
            self.api_url(""),
            doc_token
        );

        let blocks_response: serde_json::Value = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?
            .json()
            .await?;

        // Find the document root block
        let root_block_id = blocks_response["data"]["items"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["block_id"].as_str())
            .unwrap_or(doc_token);

        // Create text block with content
        let create_url = format!(
            "{}/docx/v1/documents/{}/blocks/{}/children",
            self.api_url(""),
            doc_token,
            root_block_id
        );

        let block_body = serde_json::json!({
            "children": [{
                "block_type": 2, // Text block
                "text": {
                    "elements": [{
                        "text_run": {
                            "content": content
                        }
                    }]
                }
            }]
        });

        let response: serde_json::Value = self
            .client
            .post(&create_url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&block_body)
            .send()
            .await?
            .json()
            .await?;

        let code = response["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            tracing::warn!("Failed to update document content: {} - {}", code, msg);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feishu_channel_name() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec!["*".into()]);
        assert_eq!(ch.name(), "feishu");
    }

    #[test]
    fn feishu_api_url() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec![]);
        assert!(ch.api_url("/auth").contains("feishu.cn"));
    }

    #[test]
    fn lark_api_url() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec![]).with_lark_api();
        assert!(ch.api_url("/auth").contains("larksuite.com"));
    }

    #[test]
    fn feishu_user_allowed_wildcard() {
        let ch = FeishuChannel::new("app".into(), "secret".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("ou_12345"));
    }

    #[test]
    fn feishu_user_allowed_specific() {
        let ch = FeishuChannel::new(
            "app".into(),
            "secret".into(),
            vec!["ou_alice".into()],
        );
        assert!(ch.is_user_allowed("ou_alice"));
        assert!(!ch.is_user_allowed("ou_eve"));
    }

    #[test]
    fn feishu_extract_text_content() {
        let content = r#"{"text": "Hello"}"#;
        assert_eq!(
            FeishuChannel::extract_text_content(content),
            Some("Hello".to_string())
        );
    }

    #[test]
    fn feishu_aes_decrypt_roundtrip() {
        use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
        use base64::Engine;
        use sha2::{Digest, Sha256};

        type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

        let encrypt_key = "test_key_12345";
        let plaintext = r#"{"challenge":"test"}"#;

        let mut hasher = Sha256::new();
        hasher.update(encrypt_key.as_bytes());
        let key_hash = hasher.finalize();
        let key: [u8; 32] = key_hash.into();
        let iv: [u8; 16] = key_hash[..16].try_into().unwrap();

        let encryptor = Aes256CbcEnc::new(&key.into(), &iv.into());
        let mut buffer = vec![0u8; plaintext.len() + 16];
        buffer[..plaintext.len()].copy_from_slice(plaintext.as_bytes());
        let ciphertext = encryptor
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len())
            .unwrap();
        let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(ciphertext);

        let decrypted = FeishuChannel::decrypt_aes_cbc(encrypt_key, &ciphertext_b64).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
