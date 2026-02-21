//! Matrix channel for zero-channels.
//!
//! Connects to Matrix homeservers using the Client-Server API.

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Matrix channel using Client-Server API.
pub struct MatrixChannel {
    homeserver: String,
    access_token: String,
    room_id: String,
    allowed_users: Vec<String>,
    client: Client,
    connected: Arc<RwLock<bool>>,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    next_batch: String,
    #[serde(default)]
    rooms: Rooms,
}

#[derive(Debug, Deserialize, Default)]
struct Rooms {
    #[serde(default)]
    join: HashMap<String, JoinedRoom>,
}

#[derive(Debug, Deserialize)]
struct JoinedRoom {
    #[serde(default)]
    timeline: Timeline,
}

#[derive(Debug, Deserialize, Default)]
struct Timeline {
    #[serde(default)]
    events: Vec<TimelineEvent>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: String,
    #[serde(default)]
    content: EventContent,
    #[serde(default)]
    event_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct EventContent {
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    msgtype: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhoAmIResponse {
    user_id: String,
}

impl MatrixChannel {
    /// Create a new Matrix channel.
    pub fn new(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
    ) -> Self {
        let homeserver = homeserver.trim_end_matches('/').to_string();
        Self {
            homeserver,
            access_token,
            room_id,
            allowed_users,
            client: Client::new(),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    fn is_user_allowed(&self, sender: &str) -> bool {
        if self.allowed_users.iter().any(|u| u == "*") {
            return true;
        }
        self.allowed_users
            .iter()
            .any(|u| u.eq_ignore_ascii_case(sender))
    }

    async fn get_my_user_id(&self) -> ChannelResult<String> {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("whoami failed: {e}")))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Auth(format!("Matrix whoami failed: {err}")));
        }

        let who: WhoAmIResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to parse whoami: {e}")))?;

        Ok(who.user_id)
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &'static str {
        "matrix"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Verify token by calling whoami
        self.get_my_user_id().await?;
        *self.connected.write().await = true;
        tracing::info!("Matrix channel initialized for room {}", self.room_id);
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        let text = match &message.content {
            OutgoingContent::Text { text } => text.clone(),
            OutgoingContent::Markdown { text } => text.clone(),
            _ => {
                return Err(ChannelError::InvalidMessage(
                    "Matrix only supports text messages".into(),
                ))
            }
        };

        let txn_id = format!("zc_{}", chrono::Utc::now().timestamp_millis());
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver, self.room_id, txn_id
        );

        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": text
        });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Matrix send error: {e}")))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Matrix send failed: {err}"
            )));
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to parse response: {e}")))?;

        let event_id = result
            .get("event_id")
            .and_then(|id| id.as_str())
            .unwrap_or(&txn_id)
            .to_string();

        Ok(event_id)
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        tracing::info!("Matrix channel listening on room {}...", self.room_id);

        let my_user_id = self.get_my_user_id().await?;

        // Initial sync to get the since token
        let url = format!(
            "{}/_matrix/client/v3/sync?timeout=30000&filter={{\"room\":{{\"timeline\":{{\"limit\":1}}}}}}",
            self.homeserver
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Initial sync failed: {e}")))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Connection(format!(
                "Matrix initial sync failed: {err}"
            )));
        }

        let sync: SyncResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to parse sync: {e}")))?;

        let mut since = sync.next_batch;

        // Long-poll loop
        loop {
            let url = format!(
                "{}/_matrix/client/v3/sync?since={}&timeout=30000",
                self.homeserver, since
            );

            let resp = match self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Matrix sync error: {e}, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            if !resp.status().is_success() {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }

            let sync: SyncResponse = match resp.json().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Matrix parse error: {e}");
                    continue;
                }
            };

            since = sync.next_batch;

            // Process events from our room
            if let Some(room) = sync.rooms.join.get(&self.room_id) {
                for event in &room.timeline.events {
                    // Skip our own messages
                    if event.sender == my_user_id {
                        continue;
                    }

                    // Only process text messages
                    if event.event_type != "m.room.message" {
                        continue;
                    }

                    if event.content.msgtype.as_deref() != Some("m.text") {
                        continue;
                    }

                    let Some(ref body) = event.content.body else {
                        continue;
                    };

                    if !self.is_user_allowed(&event.sender) {
                        continue;
                    }

                    let msg = ChannelMessage {
                        id: event
                            .event_id
                            .clone()
                            .unwrap_or_else(|| format!("mx_{}", chrono::Utc::now().timestamp_millis())),
                        channel_type: ChannelType::Matrix,
                        channel_id: self.room_id.clone(),
                        user_id: event.sender.clone(),
                        content: MessageContent::Text { text: body.clone() },
                        attachments: vec![],
                        metadata: HashMap::new(),
                        timestamp: chrono::Utc::now().timestamp_millis(),
                    };

                    callback(msg);
                }
            }
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        let url = format!("{}/_matrix/client/v3/account/whoami", self.homeserver);
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
            Err(ChannelError::Auth("Matrix authentication failed".into()))
        }
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        *self.connected.write().await = false;
        tracing::info!("Matrix channel shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel() -> MatrixChannel {
        MatrixChannel::new(
            "https://matrix.org".to_string(),
            "syt_test_token".to_string(),
            "!room:matrix.org".to_string(),
            vec!["@user:matrix.org".to_string()],
        )
    }

    #[test]
    fn creates_with_correct_fields() {
        let ch = make_channel();
        assert_eq!(ch.homeserver, "https://matrix.org");
        assert_eq!(ch.room_id, "!room:matrix.org");
    }

    #[test]
    fn strips_trailing_slash() {
        let ch = MatrixChannel::new(
            "https://matrix.org/".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    #[test]
    fn wildcard_allows_anyone() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["*".to_string()],
        );
        assert!(ch.is_user_allowed("@anyone:matrix.org"));
    }

    #[test]
    fn specific_user_allowed() {
        let ch = make_channel();
        assert!(ch.is_user_allowed("@user:matrix.org"));
    }

    #[test]
    fn unknown_user_denied() {
        let ch = make_channel();
        assert!(!ch.is_user_allowed("@stranger:matrix.org"));
    }

    #[test]
    fn user_case_insensitive() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["@User:Matrix.org".to_string()],
        );
        assert!(ch.is_user_allowed("@user:matrix.org"));
    }

    #[test]
    fn name_returns_matrix() {
        let ch = make_channel();
        assert_eq!(ch.name(), "matrix");
    }
}
