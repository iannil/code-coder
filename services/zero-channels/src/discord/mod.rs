//! Discord channel for zero-channels.
//!
//! Connects via Discord Gateway WebSocket for real-time messages.

pub mod format;

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;

/// Discord channel using Gateway WebSocket for real-time messaging.
pub struct DiscordChannel {
    bot_token: String,
    guild_id: Option<String>,
    allowed_users: Vec<String>,
    client: Client,
    connected: Arc<RwLock<bool>>,
}

impl DiscordChannel {
    /// Create a new Discord channel.
    pub fn new(bot_token: String, guild_id: Option<String>, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            guild_id,
            allowed_users,
            client: Client::new(),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if a Discord user ID is in the allowlist.
    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    /// Extract bot user ID from token.
    fn bot_user_id_from_token(token: &str) -> Option<String> {
        let part = token.split('.').next()?;
        base64_decode(part)
    }
}

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Minimal base64 decode for extracting bot user ID.
#[allow(clippy::cast_possible_truncation)]
fn base64_decode(input: &str) -> Option<String> {
    let padded = match input.len() % 4 {
        2 => format!("{input}=="),
        3 => format!("{input}="),
        _ => input.to_string(),
    };

    let mut bytes = Vec::new();
    let chars: Vec<u8> = padded.bytes().collect();

    for chunk in chars.chunks(4) {
        if chunk.len() < 4 {
            break;
        }

        let mut v = [0usize; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                v[i] = 0;
            } else {
                v[i] = BASE64_ALPHABET.iter().position(|&a| a == b)?;
            }
        }

        bytes.push(((v[0] << 2) | (v[1] >> 4)) as u8);
        if chunk[2] != b'=' {
            bytes.push((((v[1] & 0xF) << 4) | (v[2] >> 2)) as u8);
        }
        if chunk[3] != b'=' {
            bytes.push((((v[2] & 0x3) << 6) | v[3]) as u8);
        }
    }

    String::from_utf8(bytes).ok()
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &'static str {
        "discord"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Verify token by calling auth endpoint
        let resp = self
            .client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| ChannelError::Auth(format!("Failed to verify Discord token: {e}")))?;

        if !resp.status().is_success() {
            return Err(ChannelError::Auth("Invalid Discord bot token".into()));
        }

        *self.connected.write().await = true;
        tracing::info!("Discord channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        let text = match &message.content {
            OutgoingContent::Text { text } => text.clone(),
            OutgoingContent::Markdown { text } => {
                // Convert markdown to Discord format
                format::convert_to_discord_markdown(text)
            }
            _ => return Err(ChannelError::InvalidMessage("Discord only supports text messages".into())),
        };

        // Split message if too long
        let chunks = format::split_message(&text);
        let mut last_message_id = String::new();

        for chunk in chunks {
            let url = format!(
                "https://discord.com/api/v10/channels/{}/messages",
                message.channel_id
            );

            let mut body = json!({ "content": chunk });

            // Add reply reference if specified (only for first chunk)
            if last_message_id.is_empty() {
                if let Some(ref reply_to) = message.reply_to {
                    body["message_reference"] = json!({ "message_id": reply_to });
                }
            }

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.bot_token))
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Discord send error: {e}")))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let error = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Discord API error ({status}): {error}"
                )));
            }

            let msg_data: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| ChannelError::Internal(format!("Failed to parse response: {e}")))?;

            last_message_id = msg_data
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string();
        }

        Ok(last_message_id)
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        let bot_user_id = Self::bot_user_id_from_token(&self.bot_token).unwrap_or_default();

        // Get Gateway URL
        let gw_resp: serde_json::Value = self
            .client
            .get("https://discord.com/api/v10/gateway/bot")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Failed to get gateway: {e}")))?
            .json()
            .await
            .map_err(|e| ChannelError::Connection(format!("Invalid gateway response: {e}")))?;

        let gw_url = gw_resp
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("wss://gateway.discord.gg");

        let ws_url = format!("{gw_url}/?v=10&encoding=json");
        tracing::info!("Discord: connecting to gateway...");

        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| ChannelError::Connection(format!("WebSocket connection failed: {e}")))?;

        let (mut write, mut read) = ws_stream.split();

        // Read Hello (opcode 10)
        let hello = read
            .next()
            .await
            .ok_or_else(|| ChannelError::Connection("No hello from gateway".into()))?
            .map_err(|e| ChannelError::Connection(format!("WebSocket error: {e}")))?;

        let hello_data: serde_json::Value = serde_json::from_str(&hello.to_string())
            .map_err(|e| ChannelError::Connection(format!("Invalid hello: {e}")))?;

        let heartbeat_interval = hello_data
            .get("d")
            .and_then(|d| d.get("heartbeat_interval"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(41250);

        // Send Identify (opcode 2)
        let identify = json!({
            "op": 2,
            "d": {
                "token": self.bot_token,
                "intents": 33281, // GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT | DIRECT_MESSAGES
                "properties": {
                    "os": "linux",
                    "browser": "zero-channels",
                    "device": "zero-channels"
                }
            }
        });

        write
            .send(Message::Text(identify.to_string()))
            .await
            .map_err(|e| ChannelError::Connection(format!("Failed to identify: {e}")))?;

        tracing::info!("Discord: connected and identified");

        // Spawn heartbeat task
        let (hb_tx, mut hb_rx) = tokio::sync::mpsc::channel::<()>(1);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(heartbeat_interval));
            loop {
                interval.tick().await;
                if hb_tx.send(()).await.is_err() {
                    break;
                }
            }
        });

        let guild_filter = self.guild_id.clone();
        let allowed_users = self.allowed_users.clone();

        loop {
            tokio::select! {
                _ = hb_rx.recv() => {
                    let hb = json!({"op": 1, "d": null});
                    if write.send(Message::Text(hb.to_string())).await.is_err() {
                        break;
                    }
                }
                msg = read.next() => {
                    let msg = match msg {
                        Some(Ok(Message::Text(t))) => t,
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => continue,
                    };

                    let event: serde_json::Value = match serde_json::from_str(&msg) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    let event_type = event.get("t").and_then(|t| t.as_str()).unwrap_or("");
                    if event_type != "MESSAGE_CREATE" {
                        continue;
                    }

                    let Some(d) = event.get("d") else {
                        continue;
                    };

                    let author_id = d.get("author").and_then(|a| a.get("id")).and_then(|i| i.as_str()).unwrap_or("");
                    if author_id == bot_user_id {
                        continue;
                    }

                    if d.get("author").and_then(|a| a.get("bot")).and_then(serde_json::Value::as_bool).unwrap_or(false) {
                        continue;
                    }

                    if !allowed_users.iter().any(|u| u == "*" || u == author_id) {
                        tracing::warn!("Discord: ignoring message from unauthorized user: {author_id}");
                        continue;
                    }

                    if let Some(ref gid) = guild_filter {
                        let msg_guild = d.get("guild_id").and_then(serde_json::Value::as_str).unwrap_or("");
                        if msg_guild != gid {
                            continue;
                        }
                    }

                    let content = d.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    if content.is_empty() {
                        continue;
                    }

                    let channel_id = d.get("channel_id").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    let message_id = d.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();

                    tracing::info!(
                        channel = "discord",
                        user_id = %author_id,
                        chat_id = %channel_id,
                        message_type = "text",
                        text = %content,
                        "IM message received"
                    );

                    let channel_msg = ChannelMessage {
                        id: message_id,
                        channel_type: ChannelType::Discord,
                        channel_id,
                        user_id: author_id.to_string(),
                        content: MessageContent::Text { text: content.to_string() },
                        attachments: vec![],
                        metadata: HashMap::new(),
                        timestamp: chrono::Utc::now().timestamp_millis(),
                        trace_id: zero_common::logging::generate_trace_id(),
                        span_id: zero_common::logging::generate_span_id(),
                        parent_span_id: None,
                    };

                    callback(channel_msg);
                }
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> ChannelResult<()> {
        let resp = self
            .client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Health check failed: {e}")))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ChannelError::Auth("Discord authentication failed".into()))
        }
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        *self.connected.write().await = false;
        tracing::info!("Discord channel shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_channel_name() {
        let ch = DiscordChannel::new("fake".into(), None, vec![]);
        assert_eq!(ch.name(), "discord");
    }

    #[test]
    fn base64_decode_bot_id() {
        let decoded = base64_decode("MTIzNDU2");
        assert_eq!(decoded, Some("123456".to_string()));
    }

    #[test]
    fn bot_user_id_extraction() {
        let token = "MTIzNDU2.fake.hmac";
        let id = DiscordChannel::bot_user_id_from_token(token);
        assert_eq!(id, Some("123456".to_string()));
    }

    #[test]
    fn empty_allowlist_denies_everyone() {
        let ch = DiscordChannel::new("fake".into(), None, vec![]);
        assert!(!ch.is_user_allowed("12345"));
    }

    #[test]
    fn wildcard_allows_everyone() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["*".into()]);
        assert!(ch.is_user_allowed("12345"));
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn specific_allowlist_filters() {
        let ch = DiscordChannel::new("fake".into(), None, vec!["111".into(), "222".into()]);
        assert!(ch.is_user_allowed("111"));
        assert!(ch.is_user_allowed("222"));
        assert!(!ch.is_user_allowed("333"));
    }
}
