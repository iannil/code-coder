//! WhatsApp channel for zero-channels.
//!
//! Uses WhatsApp Business Cloud API for messaging.
//! Messages are received via webhook (push-based).

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// WhatsApp channel using Business Cloud API.
pub struct WhatsAppChannel {
    access_token: String,
    phone_number_id: String,
    verify_token: String,
    allowed_numbers: Vec<String>,
    client: Client,
    connected: Arc<RwLock<bool>>,
}

impl WhatsAppChannel {
    /// Create a new WhatsApp channel.
    pub fn new(
        access_token: String,
        phone_number_id: String,
        verify_token: String,
        allowed_numbers: Vec<String>,
    ) -> Self {
        Self {
            access_token,
            phone_number_id,
            verify_token,
            allowed_numbers,
            client: Client::new(),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if a phone number is allowed (E.164 format: +1234567890).
    pub fn is_number_allowed(&self, phone: &str) -> bool {
        self.allowed_numbers.iter().any(|n| n == "*" || n == phone)
    }

    /// Get the verify token for webhook verification.
    pub fn verify_token(&self) -> &str {
        &self.verify_token
    }

    /// Parse an incoming webhook payload and extract messages.
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        let Some(entries) = payload.get("entry").and_then(|e| e.as_array()) else {
            return messages;
        };

        for entry in entries {
            let Some(changes) = entry.get("changes").and_then(|c| c.as_array()) else {
                continue;
            };

            for change in changes {
                let Some(value) = change.get("value") else {
                    continue;
                };

                let Some(msgs) = value.get("messages").and_then(|m| m.as_array()) else {
                    continue;
                };

                for msg in msgs {
                    let Some(from) = msg.get("from").and_then(|f| f.as_str()) else {
                        continue;
                    };

                    let normalized_from = if from.starts_with('+') {
                        from.to_string()
                    } else {
                        format!("+{from}")
                    };

                    if !self.is_number_allowed(&normalized_from) {
                        tracing::warn!("WhatsApp: ignoring message from unauthorized number: {normalized_from}");
                        continue;
                    }

                    let content = if let Some(text_obj) = msg.get("text") {
                        text_obj
                            .get("body")
                            .and_then(|b| b.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        tracing::debug!("WhatsApp: skipping non-text message from {from}");
                        continue;
                    };

                    if content.is_empty() {
                        continue;
                    }

                    let timestamp = msg
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|t| t.parse::<i64>().ok())
                        .map(|ts| ts * 1000) // Convert to millis
                        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

                    let message_id = msg
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    messages.push(ChannelMessage {
                        id: message_id,
                        channel_type: ChannelType::WhatsApp,
                        channel_id: normalized_from.clone(),
                        user_id: normalized_from,
                        content: MessageContent::Text { text: content },
                        attachments: vec![],
                        metadata: HashMap::new(),
                        timestamp,
                        trace_id: zero_common::logging::generate_trace_id(),
                        span_id: zero_common::logging::generate_span_id(),
                        parent_span_id: None,
                    });
                }
            }
        }

        messages
    }

    /// Send a simple text message (convenience method for gateway handlers).
    pub async fn send_simple(&self, text: &str, recipient: &str) -> anyhow::Result<()> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.phone_number_id
        );

        // Normalize recipient (remove leading + for API)
        let to = recipient.strip_prefix('+').unwrap_or(recipient);

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": text
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("WhatsApp API error ({status}): {error}");
        }

        tracing::info!("WhatsApp message sent to {recipient}");
        Ok(())
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &'static str {
        "whatsapp"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Verify token by checking phone number info
        let url = format!(
            "https://graph.facebook.com/v18.0/{}",
            self.phone_number_id
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Auth(format!("Failed to verify WhatsApp token: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Auth(format!(
                "WhatsApp auth failed ({status}): {error}"
            )));
        }

        *self.connected.write().await = true;
        tracing::info!("WhatsApp channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        let text = match &message.content {
            OutgoingContent::Text { text } => text.clone(),
            OutgoingContent::Markdown { text } => text.clone(),
            _ => {
                return Err(ChannelError::InvalidMessage(
                    "WhatsApp only supports text messages".into(),
                ))
            }
        };

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.phone_number_id
        );

        // Normalize recipient (remove leading + for API)
        let to = message.channel_id.strip_prefix('+').unwrap_or(&message.channel_id);

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": text
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("WhatsApp send error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "WhatsApp API error ({status}): {error}"
            )));
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to parse response: {e}")))?;

        let message_id = result
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first())
            .and_then(|msg| msg.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(message_id)
    }

    async fn listen<F>(&self, _callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        // WhatsApp uses webhooks (push-based), not polling.
        // Messages are received via the gateway's /whatsapp endpoint.
        tracing::info!(
            "WhatsApp channel active (webhook mode). \
            Configure Meta webhook to POST to your gateway's /whatsapp endpoint."
        );

        // Keep the task alive
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}",
            self.phone_number_id
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Health check failed: {e}")))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ChannelError::Auth("WhatsApp authentication failed".into()))
        }
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        *self.connected.write().await = false;
        tracing::info!("WhatsApp channel shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel() -> WhatsAppChannel {
        WhatsAppChannel::new(
            "test-token".into(),
            "123456789".into(),
            "verify-me".into(),
            vec!["+1234567890".into()],
        )
    }

    #[test]
    fn whatsapp_channel_name() {
        let ch = make_channel();
        assert_eq!(ch.name(), "whatsapp");
    }

    #[test]
    fn whatsapp_verify_token() {
        let ch = make_channel();
        assert_eq!(ch.verify_token(), "verify-me");
    }

    #[test]
    fn whatsapp_number_allowed_exact() {
        let ch = make_channel();
        assert!(ch.is_number_allowed("+1234567890"));
        assert!(!ch.is_number_allowed("+9876543210"));
    }

    #[test]
    fn whatsapp_number_allowed_wildcard() {
        let ch = WhatsAppChannel::new("tok".into(), "123".into(), "ver".into(), vec!["*".into()]);
        assert!(ch.is_number_allowed("+1234567890"));
    }

    #[test]
    fn whatsapp_parse_empty_payload() {
        let ch = make_channel();
        let payload = serde_json::json!({});
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn whatsapp_parse_valid_text_message() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "1234567890",
                            "id": "wamid.xxx",
                            "timestamp": "1699999999",
                            "type": "text",
                            "text": { "body": "Hello!" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].user_id, "+1234567890");
        assert_eq!(msgs[0].text(), Some("Hello!"));
    }

    #[test]
    fn whatsapp_parse_unauthorized_number() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "9999999999",
                            "timestamp": "1699999999",
                            "type": "text",
                            "text": { "body": "Spam" }
                        }]
                    }
                }]
            }]
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }
}
