//! Slack channel for zero-channels.
//!
//! Uses Slack Web API for messaging with polling-based message retrieval.

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Slack channel using Web API.
pub struct SlackChannel {
    bot_token: String,
    channel_id: Option<String>,
    allowed_users: Vec<String>,
    client: Client,
    connected: Arc<RwLock<bool>>,
}

impl SlackChannel {
    /// Create a new Slack channel.
    pub fn new(bot_token: String, channel_id: Option<String>, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            channel_id,
            allowed_users,
            client: Client::new(),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == user_id)
    }

    async fn get_bot_user_id(&self) -> Option<String> {
        let resp: serde_json::Value = self
            .client
            .get("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

        resp.get("user_id")
            .and_then(|u| u.as_str())
            .map(String::from)
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &'static str {
        "slack"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Verify token
        let resp = self
            .client
            .get("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .map_err(|e| ChannelError::Auth(format!("Failed to verify Slack token: {e}")))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::Auth(format!("Invalid response: {e}")))?;

        if !data.get("ok").and_then(|o| o.as_bool()).unwrap_or(false) {
            let error = data
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown");
            return Err(ChannelError::Auth(format!("Slack auth failed: {error}")));
        }

        *self.connected.write().await = true;
        tracing::info!("Slack channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        let text = match &message.content {
            OutgoingContent::Text { text } => text.clone(),
            OutgoingContent::Markdown { text } => text.clone(),
            _ => {
                return Err(ChannelError::InvalidMessage(
                    "Slack only supports text messages".into(),
                ))
            }
        };

        let body = serde_json::json!({
            "channel": message.channel_id,
            "text": text
        });

        let resp = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Slack send error: {e}")))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to parse response: {e}")))?;

        if !data.get("ok").and_then(|o| o.as_bool()).unwrap_or(false) {
            let error = data
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown");
            return Err(ChannelError::SendFailed(format!(
                "Slack send failed: {error}"
            )));
        }

        let ts = data
            .get("ts")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ts)
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        let channel_id = self
            .channel_id
            .clone()
            .ok_or_else(|| ChannelError::NotReady)?;

        let bot_user_id = self.get_bot_user_id().await.unwrap_or_default();
        let mut last_ts = String::new();

        tracing::info!("Slack channel listening on #{channel_id}...");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let mut params = vec![("channel", channel_id.clone()), ("limit", "10".to_string())];
            if !last_ts.is_empty() {
                params.push(("oldest", last_ts.clone()));
            }

            let resp = match self
                .client
                .get("https://slack.com/api/conversations.history")
                .bearer_auth(&self.bot_token)
                .query(&params)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Slack poll error: {e}");
                    continue;
                }
            };

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Slack parse error: {e}");
                    continue;
                }
            };

            if let Some(messages) = data.get("messages").and_then(|m| m.as_array()) {
                for msg in messages.iter().rev() {
                    let ts = msg.get("ts").and_then(|t| t.as_str()).unwrap_or("");
                    let user = msg
                        .get("user")
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown");
                    let text = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");

                    if user == bot_user_id {
                        continue;
                    }

                    if !self.is_user_allowed(user) {
                        tracing::warn!("Slack: ignoring message from unauthorized user: {user}");
                        continue;
                    }

                    if text.is_empty() || ts <= last_ts.as_str() {
                        continue;
                    }

                    last_ts = ts.to_string();

                    let channel_msg = ChannelMessage {
                        id: ts.to_string(),
                        channel_type: ChannelType::Slack,
                        channel_id: channel_id.clone(),
                        user_id: user.to_string(),
                        content: MessageContent::Text {
                            text: text.to_string(),
                        },
                        attachments: vec![],
                        metadata: HashMap::new(),
                        timestamp: chrono::Utc::now().timestamp_millis(),
                    };

                    callback(channel_msg);
                }
            }
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        let resp = self
            .client
            .get("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Health check failed: {e}")))?;

        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await.unwrap_or_default();
            if data.get("ok").and_then(|o| o.as_bool()).unwrap_or(false) {
                return Ok(());
            }
        }

        Err(ChannelError::Auth("Slack authentication failed".into()))
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        *self.connected.write().await = false;
        tracing::info!("Slack channel shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_channel_name() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert_eq!(ch.name(), "slack");
    }

    #[test]
    fn slack_channel_with_channel_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), Some("C12345".into()), vec![]);
        assert_eq!(ch.channel_id, Some("C12345".to_string()));
    }

    #[test]
    fn empty_allowlist_denies_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert!(!ch.is_user_allowed("U12345"));
    }

    #[test]
    fn wildcard_allows_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()]);
        assert!(ch.is_user_allowed("U12345"));
    }

    #[test]
    fn specific_allowlist_filters() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into(), "U222".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(ch.is_user_allowed("U222"));
        assert!(!ch.is_user_allowed("U333"));
    }
}
