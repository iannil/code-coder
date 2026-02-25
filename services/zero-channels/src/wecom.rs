//! WeChat Work (企业微信) channel adapter.
//!
//! Uses the WeChat Work Server API for messaging.
//! Supports text messages, Markdown messages, event callbacks, and message encryption.
//! Implements AES-256-CBC encryption/decryption for secure message handling.
//!
//! ## API Documentation
//! - https://developer.work.weixin.qq.com/document/path/90664
//! - https://developer.work.weixin.qq.com/document/path/90236

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

// ============================================================================
// Constants
// ============================================================================

const WECOM_API_BASE: &str = "https://qyapi.weixin.qq.com/cgi-bin";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    #[serde(default)]
    errcode: i32,
    #[serde(default)]
    errmsg: String,
    access_token: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    #[serde(default)]
    errcode: i32,
    #[serde(default)]
    errmsg: String,
}

// ============================================================================
// Event Callback Types
// ============================================================================

/// Parsed XML message from WeChat Work callback.
#[derive(Debug, Clone)]
pub struct WeComMessage {
    /// ToUserName - the corp ID
    pub to_user_name: String,
    /// FromUserName - the sender's user ID
    pub from_user_name: String,
    /// CreateTime - Unix timestamp
    pub create_time: i64,
    /// MsgType - "text", "image", "voice", etc.
    pub msg_type: String,
    /// Content - text content (for text messages)
    pub content: Option<String>,
    /// MsgId - unique message ID
    pub msg_id: Option<String>,
    /// AgentID - the agent that received the message
    pub agent_id: Option<i64>,
    /// Event - event type (for event messages)
    pub event: Option<String>,
    /// EventKey - event key (for click events)
    pub event_key: Option<String>,
}

// ============================================================================
// Token Cache
// ============================================================================

struct TokenCache {
    token: String,
    expires_at: Instant,
}

// ============================================================================
// WeComChannel
// ============================================================================

/// WeChat Work (企业微信) messaging channel.
pub struct WeComChannel {
    corp_id: String,
    agent_id: i64,
    secret: String,
    token: Option<String>,
    encoding_aes_key: Option<Vec<u8>>,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl WeComChannel {
    /// Create a new WeChat Work channel.
    pub fn new(
        corp_id: String,
        agent_id: i64,
        secret: String,
        allowed_users: Vec<String>,
    ) -> Self {
        Self {
            corp_id,
            agent_id,
            secret,
            token: None,
            encoding_aes_key: None,
            allowed_users,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new WeChat Work channel with encryption support.
    pub fn with_encryption(
        corp_id: String,
        agent_id: i64,
        secret: String,
        token: Option<String>,
        encoding_aes_key: Option<String>,
        allowed_users: Vec<String>,
    ) -> Self {
        let aes_key = encoding_aes_key.and_then(|key| {
            // WeChat Work AES key is base64 encoded + "=" padding
            let key_with_padding = format!("{}=", key);
            base64::engine::general_purpose::STANDARD
                .decode(&key_with_padding)
                .ok()
        });

        Self {
            corp_id,
            agent_id,
            secret,
            token,
            encoding_aes_key: aes_key,
            allowed_users,
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

        // Refresh token
        let url = format!(
            "{}/gettoken?corpid={}&corpsecret={}",
            WECOM_API_BASE, self.corp_id, self.secret
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get WeChat Work access token ({status}): {text}");
        }

        let data: AccessTokenResponse = resp.json().await?;

        if data.errcode != 0 {
            anyhow::bail!("WeChat Work API error ({}): {}", data.errcode, data.errmsg);
        }

        let token = data
            .access_token
            .ok_or_else(|| anyhow::anyhow!("Missing access_token in response"))?;
        let expire = data.expires_in.unwrap_or(7200);

        // Update cache
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(expire),
            });
        }

        tracing::debug!(
            "WeChat Work access token refreshed, expires in {} seconds",
            expire
        );
        Ok(token)
    }

    /// Send a text message to a user.
    pub async fn send_text(&self, user_id: &str, text: &str) -> anyhow::Result<()> {
        let token = self.get_access_token().await?;
        let url = format!("{}/message/send?access_token={}", WECOM_API_BASE, token);

        let body = serde_json::json!({
            "touser": user_id,
            "msgtype": "text",
            "agentid": self.agent_id,
            "text": {
                "content": text
            }
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("WeChat Work sendMessage failed ({status}): {text}");
        }

        let data: SendMessageResponse = resp.json().await?;

        if data.errcode != 0 {
            anyhow::bail!(
                "WeChat Work sendMessage error ({}): {}",
                data.errcode,
                data.errmsg
            );
        }

        tracing::info!("WeChat Work message sent to {}", user_id);
        Ok(())
    }

    /// Send a Markdown message to a user.
    pub async fn send_markdown(&self, user_id: &str, content: &str) -> anyhow::Result<()> {
        let token = self.get_access_token().await?;
        let url = format!("{}/message/send?access_token={}", WECOM_API_BASE, token);

        let body = serde_json::json!({
            "touser": user_id,
            "msgtype": "markdown",
            "agentid": self.agent_id,
            "markdown": {
                "content": content
            }
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("WeChat Work sendMarkdown failed ({status}): {text}");
        }

        let data: SendMessageResponse = resp.json().await?;

        if data.errcode != 0 {
            anyhow::bail!(
                "WeChat Work sendMarkdown error ({}): {}",
                data.errcode,
                data.errmsg
            );
        }

        tracing::info!("WeChat Work markdown sent to {}", user_id);
        Ok(())
    }

    /// Verify callback URL signature.
    ///
    /// Returns the decrypted echostr if verification succeeds.
    pub fn verify_url(
        &self,
        msg_signature: &str,
        timestamp: &str,
        nonce: &str,
        echostr: &str,
    ) -> anyhow::Result<String> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Token not configured for URL verification"))?;

        // Sort and concatenate
        let mut params = [token.as_str(), timestamp, nonce, echostr];
        params.sort();
        let concat = params.join("");

        // SHA1 hash
        let mut hasher = Sha1::new();
        hasher.update(concat.as_bytes());
        let result = hasher.finalize();
        let signature = hex::encode(result);

        if signature != msg_signature {
            anyhow::bail!("Signature verification failed");
        }

        // Decrypt echostr
        self.decrypt_message(echostr)
    }

    /// Decrypt an encrypted message.
    pub fn decrypt_message(&self, encrypted: &str) -> anyhow::Result<String> {
        let aes_key = self
            .encoding_aes_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EncodingAESKey not configured"))?;

        if aes_key.len() != 32 {
            anyhow::bail!("Invalid AES key length: expected 32, got {}", aes_key.len());
        }

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64: {e}"))?;

        // IV is the first 16 bytes of the key
        let key: [u8; 32] = aes_key[..32].try_into()?;
        let iv: [u8; 16] = aes_key[..16].try_into()?;

        let decryptor = Aes256CbcDec::new(&key.into(), &iv.into());
        let mut buffer = ciphertext.clone();
        let decrypted = decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| anyhow::anyhow!("AES decryption failed: {e}"))?;

        // WeChat Work message format:
        // random(16 bytes) + msg_len(4 bytes, network order) + msg + corp_id
        if decrypted.len() < 20 {
            anyhow::bail!("Decrypted message too short");
        }

        let msg_len_bytes: [u8; 4] = decrypted[16..20].try_into()?;
        let msg_len = u32::from_be_bytes(msg_len_bytes) as usize;

        if decrypted.len() < 20 + msg_len {
            anyhow::bail!("Invalid message length");
        }

        let msg = &decrypted[20..20 + msg_len];
        String::from_utf8(msg.to_vec())
            .map_err(|e| anyhow::anyhow!("Decrypted content is not valid UTF-8: {e}"))
    }

    /// Encrypt a message for response.
    pub fn encrypt_message(&self, msg: &str, timestamp: &str, nonce: &str) -> anyhow::Result<String> {
        let aes_key = self
            .encoding_aes_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EncodingAESKey not configured"))?;

        if aes_key.len() != 32 {
            anyhow::bail!("Invalid AES key length: expected 32, got {}", aes_key.len());
        }

        // Generate 16 random bytes
        let random: [u8; 16] = rand::random();

        // Build plaintext: random(16) + msg_len(4) + msg + corp_id
        let msg_bytes = msg.as_bytes();
        let msg_len = (msg_bytes.len() as u32).to_be_bytes();
        let corp_id_bytes = self.corp_id.as_bytes();

        let mut plaintext = Vec::with_capacity(16 + 4 + msg_bytes.len() + corp_id_bytes.len());
        plaintext.extend_from_slice(&random);
        plaintext.extend_from_slice(&msg_len);
        plaintext.extend_from_slice(msg_bytes);
        plaintext.extend_from_slice(corp_id_bytes);

        // PKCS#7 padding
        let block_size = 32;
        let padding_len = block_size - (plaintext.len() % block_size);
        plaintext.extend(std::iter::repeat_n(padding_len as u8, padding_len));

        // Encrypt
        let key: [u8; 32] = aes_key[..32].try_into()?;
        let iv: [u8; 16] = aes_key[..16].try_into()?;

        let encryptor = Aes256CbcEnc::new(&key.into(), &iv.into());
        let mut buffer = plaintext.clone();
        let ciphertext = encryptor
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext.len() - padding_len)
            .map_err(|e| anyhow::anyhow!("AES encryption failed: {e}"))?;

        let encrypted = base64::engine::general_purpose::STANDARD.encode(ciphertext);

        // Generate signature
        let token = self.token.as_deref().unwrap_or("");
        let mut params = [token, timestamp, nonce, &encrypted];
        params.sort();
        let concat = params.join("");

        let mut hasher = Sha1::new();
        hasher.update(concat.as_bytes());
        let signature = hex::encode(hasher.finalize());

        // Build XML response
        let xml = format!(
            r#"<xml>
<Encrypt><![CDATA[{}]]></Encrypt>
<MsgSignature><![CDATA[{}]]></MsgSignature>
<TimeStamp>{}</TimeStamp>
<Nonce><![CDATA[{}]]></Nonce>
</xml>"#,
            encrypted, signature, timestamp, nonce
        );

        Ok(xml)
    }

    /// Parse an XML message from WeChat Work.
    pub fn parse_xml_message(&self, xml: &str) -> anyhow::Result<WeComMessage> {
        // Simple XML parsing without external dependencies
        fn extract_tag(xml: &str, tag: &str) -> Option<String> {
            let start_pattern = format!("<{}>", tag);
            let cdata_pattern = format!("<{}><![CDATA[", tag);
            let end_pattern = format!("</{}>", tag);

            if let Some(start) = xml.find(&cdata_pattern) {
                let content_start = start + cdata_pattern.len();
                if let Some(end) = xml[content_start..].find("]]>") {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }

            if let Some(start) = xml.find(&start_pattern) {
                let content_start = start + start_pattern.len();
                if let Some(end) = xml[content_start..].find(&end_pattern) {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }

            None
        }

        let to_user_name = extract_tag(xml, "ToUserName")
            .ok_or_else(|| anyhow::anyhow!("Missing ToUserName"))?;
        let from_user_name = extract_tag(xml, "FromUserName")
            .ok_or_else(|| anyhow::anyhow!("Missing FromUserName"))?;
        let create_time = extract_tag(xml, "CreateTime")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let msg_type =
            extract_tag(xml, "MsgType").ok_or_else(|| anyhow::anyhow!("Missing MsgType"))?;

        Ok(WeComMessage {
            to_user_name,
            from_user_name,
            create_time,
            msg_type,
            content: extract_tag(xml, "Content"),
            msg_id: extract_tag(xml, "MsgId"),
            agent_id: extract_tag(xml, "AgentID").and_then(|s| s.parse().ok()),
            event: extract_tag(xml, "Event"),
            event_key: extract_tag(xml, "EventKey"),
        })
    }

    /// Verify message signature.
    pub fn verify_signature(
        &self,
        msg_signature: &str,
        timestamp: &str,
        nonce: &str,
        encrypted: &str,
    ) -> bool {
        let token = match &self.token {
            Some(t) => t.as_str(),
            None => return false,
        };

        let mut params = [token, timestamp, nonce, encrypted];
        params.sort();
        let concat = params.join("");

        let mut hasher = Sha1::new();
        hasher.update(concat.as_bytes());
        let signature = hex::encode(hasher.finalize());

        signature == msg_signature
    }

    /// Send a simple text message (convenience method for gateway handlers).
    pub async fn send_simple(&self, text: &str, user_id: &str) -> anyhow::Result<()> {
        self.send_text(user_id, text).await
    }
}

#[async_trait]
impl Channel for WeComChannel {
    fn name(&self) -> &'static str {
        "wecom"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Try to get an access token to verify credentials
        self.get_access_token()
            .await
            .map_err(|e| ChannelError::Auth(e.to_string()))?;

        tracing::info!("WeChat Work channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        match message.content {
            OutgoingContent::Text { text } => {
                self.send_text(&message.channel_id, &text)
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
            }
            OutgoingContent::Markdown { text } => {
                self.send_markdown(&message.channel_id, &text)
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
            }
            _ => {
                return Err(ChannelError::SendFailed(
                    "WeChat Work only supports text and markdown messages currently".to_string(),
                ));
            }
        }
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn listen<F>(&self, _callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        tracing::info!("WeChat Work channel listening for messages...");
        tracing::info!(
            "WeChat Work uses webhook-based events. Configure callback URL in WeChat Work admin console."
        );

        // WeChat Work uses webhooks, keep running until shutdown
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
        tracing::info!("WeChat Work channel shutting down");
        Ok(())
    }
}

/// Process an incoming WeChat Work event callback.
///
/// This function should be called from a webhook handler.
pub fn process_event_callback(
    channel: &WeComChannel,
    msg_signature: &str,
    timestamp: &str,
    nonce: &str,
    body: &str,
) -> anyhow::Result<(Option<String>, Option<ChannelMessage>)> {
    // Extract encrypted content from XML
    fn extract_encrypt(xml: &str) -> Option<String> {
        let start = xml.find("<Encrypt><![CDATA[")?;
        let content_start = start + "<Encrypt><![CDATA[".len();
        let end = xml[content_start..].find("]]>")?;
        Some(xml[content_start..content_start + end].to_string())
    }

    let encrypted = extract_encrypt(body)
        .ok_or_else(|| anyhow::anyhow!("Missing Encrypt field in callback"))?;

    // Verify signature
    if !channel.verify_signature(msg_signature, timestamp, nonce, &encrypted) {
        anyhow::bail!("Invalid message signature");
    }

    // Decrypt message
    let decrypted = channel.decrypt_message(&encrypted)?;
    let msg = channel.parse_xml_message(&decrypted)?;

    // Check if user is allowed
    if !channel.is_user_allowed(&msg.from_user_name) {
        tracing::warn!(
            "WeChat Work: ignoring message from unauthorized user: {}",
            msg.from_user_name
        );
        return Ok((None, None));
    }

    // Handle text messages
    if msg.msg_type == "text" {
        if let Some(content) = msg.content {
            let channel_msg = ChannelMessage {
                id: msg.msg_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                channel_type: ChannelType::WeCom,
                channel_id: msg.from_user_name.clone(),
                user_id: msg.from_user_name,
                content: MessageContent::Text { text: content },
                attachments: vec![],
                metadata: std::collections::HashMap::new(),
                timestamp: msg.create_time * 1000, // Convert to millis
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
    fn wecom_channel_name() {
        let ch = WeComChannel::new(
            "corp_id".into(),
            1000001,
            "secret".into(),
            vec!["*".into()],
        );
        assert_eq!(ch.name(), "wecom");
    }

    #[test]
    fn wecom_user_allowed_wildcard() {
        let ch = WeComChannel::new(
            "corp_id".into(),
            1000001,
            "secret".into(),
            vec!["*".into()],
        );
        assert!(ch.is_user_allowed("zhangsan"));
    }

    #[test]
    fn wecom_user_allowed_specific() {
        let ch = WeComChannel::new(
            "corp_id".into(),
            1000001,
            "secret".into(),
            vec!["zhangsan".into()],
        );
        assert!(ch.is_user_allowed("zhangsan"));
        assert!(!ch.is_user_allowed("lisi"));
    }

    #[test]
    fn wecom_parse_xml_message() {
        let ch = WeComChannel::new(
            "corp_id".into(),
            1000001,
            "secret".into(),
            vec![],
        );

        let xml = r#"<xml>
<ToUserName><![CDATA[toUser]]></ToUserName>
<FromUserName><![CDATA[fromUser]]></FromUserName>
<CreateTime>1348831860</CreateTime>
<MsgType><![CDATA[text]]></MsgType>
<Content><![CDATA[Hello World]]></Content>
<MsgId>1234567890123456</MsgId>
<AgentID>1000001</AgentID>
</xml>"#;

        let msg = ch.parse_xml_message(xml).unwrap();
        assert_eq!(msg.to_user_name, "toUser");
        assert_eq!(msg.from_user_name, "fromUser");
        assert_eq!(msg.msg_type, "text");
        assert_eq!(msg.content, Some("Hello World".to_string()));
        assert_eq!(msg.msg_id, Some("1234567890123456".to_string()));
        assert_eq!(msg.agent_id, Some(1000001));
    }
}
