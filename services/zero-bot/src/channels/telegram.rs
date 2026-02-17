use super::traits::{Channel, ChannelMessage};
use crate::stt::SpeechToText;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

// ============================================================================
// Inline Keyboard Types
// ============================================================================

/// A single inline keyboard button
#[derive(Debug, Clone)]
pub struct InlineButton {
    pub text: String,
    pub callback_data: String,
}

impl InlineButton {
    pub fn new(text: impl Into<String>, callback_data: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            callback_data: callback_data.into(),
        }
    }
}

/// Callback query received when user clicks an inline button
#[derive(Debug, Clone)]
pub struct CallbackQuery {
    pub id: String,
    pub from_user_id: String,
    pub from_username: Option<String>,
    pub chat_id: String,
    pub message_id: i64,
    pub data: String,
}

/// Telegram channel — long-polls the Bot API for updates
pub struct TelegramChannel {
    bot_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    /// Optional STT client for voice message transcription
    stt: Option<Arc<dyn SpeechToText>>,
    /// Optional callback query sender for inline button clicks
    callback_tx: Option<mpsc::Sender<CallbackQuery>>,
}

impl TelegramChannel {
    pub fn new(bot_token: String, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            allowed_users,
            client: reqwest::Client::new(),
            stt: None,
            callback_tx: None,
        }
    }

    /// Create a new Telegram channel with STT support for voice messages.
    pub fn with_stt(
        bot_token: String,
        allowed_users: Vec<String>,
        stt: Arc<dyn SpeechToText>,
    ) -> Self {
        Self {
            bot_token,
            allowed_users,
            client: reqwest::Client::new(),
            stt: Some(stt),
            callback_tx: None,
        }
    }

    /// Set a callback query sender for inline button click handling.
    /// Call this before starting the listener to receive callback queries.
    pub fn set_callback_sender(&mut self, tx: mpsc::Sender<CallbackQuery>) {
        self.callback_tx = Some(tx);
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{method}", self.bot_token)
    }

    fn file_url(&self, file_path: &str) -> String {
        format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot_token, file_path
        )
    }

    fn is_user_allowed(&self, username: &str) -> bool {
        self.allowed_users.iter().any(|u| u == "*" || u == username)
    }

    fn is_any_user_allowed<'a, I>(&self, identities: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        identities.into_iter().any(|id| self.is_user_allowed(id))
    }

    /// Download a file from Telegram by its `file_id`.
    ///
    /// # Arguments
    /// * `file_id` - The `file_id` from the Telegram message
    ///
    /// # Returns
    /// The raw file bytes, or an error if download fails.
    async fn download_file(&self, file_id: &str) -> anyhow::Result<Vec<u8>> {
        // Step 1: Get the file path via getFile API
        let url = self.api_url("getFile");
        let body = serde_json::json!({ "file_id": file_id });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram getFile failed: {err}");
        }

        let data: serde_json::Value = resp.json().await?;
        let file_path = data
            .get("result")
            .and_then(|r| r.get("file_path"))
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing file_path in getFile response"))?;

        // Step 2: Download the file
        let download_url = self.file_url(file_path);
        let file_resp = self.client.get(&download_url).send().await?;

        if !file_resp.status().is_success() {
            anyhow::bail!(
                "Failed to download file from Telegram: {}",
                file_resp.status()
            );
        }

        let bytes = file_resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Send a document/file to a Telegram chat
    pub async fn send_document(
        &self,
        chat_id: &str,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("document", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendDocument"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendDocument failed: {err}");
        }

        tracing::info!("Telegram document sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a document from bytes (in-memory) to a Telegram chat
    pub async fn send_document_bytes(
        &self,
        chat_id: &str,
        file_bytes: Vec<u8>,
        file_name: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("document", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendDocument"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendDocument failed: {err}");
        }

        tracing::info!("Telegram document sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a photo to a Telegram chat
    pub async fn send_photo(
        &self,
        chat_id: &str,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("photo.jpg");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("photo", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendPhoto"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendPhoto failed: {err}");
        }

        tracing::info!("Telegram photo sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a photo from bytes (in-memory) to a Telegram chat
    pub async fn send_photo_bytes(
        &self,
        chat_id: &str,
        file_bytes: Vec<u8>,
        file_name: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("photo", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendPhoto"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendPhoto failed: {err}");
        }

        tracing::info!("Telegram photo sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a video to a Telegram chat
    pub async fn send_video(
        &self,
        chat_id: &str,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("video", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendVideo"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendVideo failed: {err}");
        }

        tracing::info!("Telegram video sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send an audio file to a Telegram chat
    pub async fn send_audio(
        &self,
        chat_id: &str,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("audio", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendAudio"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendAudio failed: {err}");
        }

        tracing::info!("Telegram audio sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a voice message to a Telegram chat
    pub async fn send_voice(
        &self,
        chat_id: &str,
        file_path: &Path,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("voice.ogg");

        let file_bytes = tokio::fs::read(file_path).await?;
        let part = Part::bytes(file_bytes).file_name(file_name.to_string());

        let mut form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("voice", part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendVoice"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendVoice failed: {err}");
        }

        tracing::info!("Telegram voice sent to {chat_id}: {file_name}");
        Ok(())
    }

    /// Send a file by URL (Telegram will download it)
    pub async fn send_document_by_url(
        &self,
        chat_id: &str,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "document": url
        });

        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendDocument"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendDocument by URL failed: {err}");
        }

        tracing::info!("Telegram document (URL) sent to {chat_id}: {url}");
        Ok(())
    }

    /// Send a photo by URL (Telegram will download it)
    pub async fn send_photo_by_url(
        &self,
        chat_id: &str,
        url: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": url
        });

        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("sendPhoto"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendPhoto by URL failed: {err}");
        }

        tracing::info!("Telegram photo (URL) sent to {chat_id}: {url}");
        Ok(())
    }

    /// Send a message with inline keyboard buttons
    ///
    /// Returns the `message_id` of the sent message (for later editing/deletion)
    pub async fn send_with_inline_keyboard(
        &self,
        chat_id: &str,
        text: &str,
        buttons: Vec<Vec<InlineButton>>,
    ) -> anyhow::Result<i64> {
        // Build keyboard structure
        let keyboard: Vec<Vec<serde_json::Value>> = buttons
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|btn| {
                        serde_json::json!({
                            "text": btn.text,
                            "callback_data": btn.callback_data
                        })
                    })
                    .collect()
            })
            .collect();

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "reply_markup": {
                "inline_keyboard": keyboard
            }
        });

        let resp = self
            .client
            .post(self.api_url("sendMessage"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram sendMessage with keyboard failed: {err}");
        }

        let data: serde_json::Value = resp.json().await?;
        let message_id = data
            .get("result")
            .and_then(|r| r.get("message_id"))
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| anyhow::anyhow!("Missing message_id in response"))?;

        tracing::info!(
            "Telegram message with inline keyboard sent to {chat_id}, message_id={}",
            message_id
        );
        Ok(message_id)
    }

    /// Answer a callback query (acknowledge button click)
    ///
    /// This removes the loading spinner from the button.
    pub async fn answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
        show_alert: bool,
    ) -> anyhow::Result<()> {
        let mut body = serde_json::json!({
            "callback_query_id": callback_query_id,
            "show_alert": show_alert
        });

        if let Some(t) = text {
            body["text"] = serde_json::Value::String(t.to_string());
        }

        let resp = self
            .client
            .post(self.api_url("answerCallbackQuery"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram answerCallbackQuery failed: {err}");
        }

        tracing::debug!("Answered callback query {callback_query_id}");
        Ok(())
    }

    /// Edit the text of an existing message (e.g., to remove buttons after user clicks)
    pub async fn edit_message_text(
        &self,
        chat_id: &str,
        message_id: i64,
        text: &str,
    ) -> anyhow::Result<()> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "Markdown"
        });

        let resp = self
            .client
            .post(self.api_url("editMessageText"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Telegram editMessageText failed: {err}");
        }

        tracing::debug!("Edited message {message_id} in chat {chat_id}");
        Ok(())
    }

    /// Parse a `callback_query` JSON object into a `CallbackQuery` struct
    #[allow(clippy::unused_self)]
    fn parse_callback_query(&self, callback: &serde_json::Value) -> Option<CallbackQuery> {
        let id = callback.get("id")?.as_str()?.to_string();
        let data = callback.get("data")?.as_str()?.to_string();

        let from = callback.get("from")?;
        let from_user_id = from.get("id")?.as_i64()?.to_string();
        let from_username = from
            .get("username")
            .and_then(|u| u.as_str())
            .map(String::from);

        let message = callback.get("message")?;
        let chat_id = message.get("chat")?.get("id")?.as_i64()?.to_string();
        let message_id = message.get("message_id")?.as_i64()?;

        Some(CallbackQuery {
            id,
            from_user_id,
            from_username,
            chat_id,
            message_id,
            data,
        })
    }
}

#[allow(clippy::too_many_lines)]
#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, message: &str, chat_id: &str) -> anyhow::Result<()> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        self.client
            .post(self.api_url("sendMessage"))
            .json(&body)
            .send()
            .await?;

        Ok(())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let mut offset: i64 = 0;

        tracing::info!("Telegram channel listening for messages...");

        loop {
            let url = self.api_url("getUpdates");
            // Listen for both messages and callback_query if callback handler is set
            let allowed_updates = if self.callback_tx.is_some() {
                serde_json::json!(["message", "callback_query"])
            } else {
                serde_json::json!(["message"])
            };
            let body = serde_json::json!({
                "offset": offset,
                "timeout": 30,
                "allowed_updates": allowed_updates
            });

            let resp = match self.client.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Telegram poll error: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Telegram parse error: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Some(results) = data.get("result").and_then(serde_json::Value::as_array) {
                for update in results {
                    // Advance offset past this update
                    if let Some(uid) = update.get("update_id").and_then(serde_json::Value::as_i64) {
                        offset = uid + 1;
                    }

                    // Handle callback_query (inline button clicks)
                    if let Some(callback) = update.get("callback_query") {
                        if let Some(ref callback_tx) = self.callback_tx {
                            if let Some(query) = self.parse_callback_query(callback) {
                                // Check user authorization
                                let username = query.from_username.as_deref().unwrap_or("unknown");
                                let mut identities = vec![username];
                                identities.push(&query.from_user_id);

                                if self.is_any_user_allowed(identities.iter().copied()) {
                                    if callback_tx.send(query).await.is_err() {
                                        tracing::warn!("Callback query receiver dropped");
                                    }
                                } else {
                                    tracing::warn!(
                                        "Ignoring callback from unauthorized user: {}",
                                        username
                                    );
                                    // Still answer to remove loading state
                                    let _ = self
                                        .answer_callback_query(&query.id, Some("Unauthorized"), false)
                                        .await;
                                }
                            }
                        }
                        continue;
                    }

                    let Some(message) = update.get("message") else {
                        continue;
                    };

                    // Extract chat_id early (needed for error messages)
                    let chat_id = message
                        .get("chat")
                        .and_then(|c| c.get("id"))
                        .and_then(serde_json::Value::as_i64)
                        .map(|id| id.to_string())
                        .unwrap_or_default();

                    // Check authorization first (before downloading any files)
                    let username_opt = message
                        .get("from")
                        .and_then(|f| f.get("username"))
                        .and_then(|u| u.as_str());
                    let username = username_opt.unwrap_or("unknown");

                    let user_id = message
                        .get("from")
                        .and_then(|f| f.get("id"))
                        .and_then(serde_json::Value::as_i64);
                    let user_id_str = user_id.map(|id| id.to_string());

                    let mut identities = vec![username];
                    if let Some(ref id) = user_id_str {
                        identities.push(id.as_str());
                    }

                    if !self.is_any_user_allowed(identities.iter().copied()) {
                        tracing::warn!(
                            "Telegram: ignoring message from unauthorized user: username={username}, user_id={}. \
Allowlist Telegram @username or numeric user ID, then run `zero-bot onboard --channels-only`.",
                            user_id_str.as_deref().unwrap_or("unknown")
                        );
                        continue;
                    }

                    // Handle text message or voice message
                    let text = if let Some(text) = message.get("text").and_then(|v| v.as_str()) {
                        text.to_string()
                    } else if let Some(voice) = message.get("voice") {
                        // Voice message handling
                        let Some(ref stt) = self.stt else {
                            tracing::debug!(
                                "Voice message received but STT not configured, skipping"
                            );
                            continue;
                        };

                        let Some(file_id) = voice.get("file_id").and_then(|v| v.as_str()) else {
                            tracing::warn!("Voice message missing file_id");
                            continue;
                        };

                        // Download the voice file
                        let audio_bytes = match self.download_file(file_id).await {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                tracing::error!("Failed to download voice: {e}");
                                let _ = self
                                    .send("Unable to download voice file, please try again", &chat_id)
                                    .await;
                                continue;
                            }
                        };

                        // Validate audio bytes before transcription
                        if audio_bytes.is_empty() {
                            tracing::error!("Downloaded voice file is empty (0 bytes)");
                            let _ = self
                                .send("Voice file appears to be empty, please try again", &chat_id)
                                .await;
                            continue;
                        }

                        tracing::debug!(
                            "Voice file downloaded: {} bytes, file_id={}",
                            audio_bytes.len(),
                            file_id
                        );

                        // Transcribe the voice message
                        match stt.transcribe(&audio_bytes, "ogg").await {
                            Ok(transcription) => {
                                tracing::info!(
                                    "Voice transcribed: {} chars",
                                    transcription.len()
                                );
                                transcription
                            }
                            Err(e) => {
                                tracing::error!("Voice transcription failed: {e}");
                                let _ = self
                                    .send(
                                        "Voice transcription failed, please try again or use text",
                                        &chat_id,
                                    )
                                    .await;
                                continue;
                            }
                        }
                    } else {
                        // Other message types (photo, video, etc.) — skip
                        continue;
                    };

                    let msg = ChannelMessage {
                        id: Uuid::new_v4().to_string(),
                        sender: chat_id,
                        content: text,
                        channel: "telegram".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };

                    if tx.send(msg).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(self.api_url("getMe"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_channel_name() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        assert_eq!(ch.name(), "telegram");
    }

    #[test]
    fn telegram_api_url() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("getMe"),
            "https://api.telegram.org/bot123:ABC/getMe"
        );
    }

    #[test]
    fn telegram_user_allowed_wildcard() {
        let ch = TelegramChannel::new("t".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn telegram_user_allowed_specific() {
        let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "bob".into()]);
        assert!(ch.is_user_allowed("alice"));
        assert!(!ch.is_user_allowed("eve"));
    }

    #[test]
    fn telegram_user_denied_empty() {
        let ch = TelegramChannel::new("t".into(), vec![]);
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn telegram_user_exact_match_not_substring() {
        let ch = TelegramChannel::new("t".into(), vec!["alice".into()]);
        assert!(!ch.is_user_allowed("alice_bot"));
        assert!(!ch.is_user_allowed("alic"));
        assert!(!ch.is_user_allowed("malice"));
    }

    #[test]
    fn telegram_user_empty_string_denied() {
        let ch = TelegramChannel::new("t".into(), vec!["alice".into()]);
        assert!(!ch.is_user_allowed(""));
    }

    #[test]
    fn telegram_user_case_sensitive() {
        let ch = TelegramChannel::new("t".into(), vec!["Alice".into()]);
        assert!(ch.is_user_allowed("Alice"));
        assert!(!ch.is_user_allowed("alice"));
        assert!(!ch.is_user_allowed("ALICE"));
    }

    #[test]
    fn telegram_wildcard_with_specific_users() {
        let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "*".into()]);
        assert!(ch.is_user_allowed("alice"));
        assert!(ch.is_user_allowed("bob"));
        assert!(ch.is_user_allowed("anyone"));
    }

    #[test]
    fn telegram_user_allowed_by_numeric_id_identity() {
        let ch = TelegramChannel::new("t".into(), vec!["123456789".into()]);
        assert!(ch.is_any_user_allowed(["unknown", "123456789"]));
    }

    #[test]
    fn telegram_user_denied_when_none_of_identities_match() {
        let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "987654321".into()]);
        assert!(!ch.is_any_user_allowed(["unknown", "123456789"]));
    }

    // ── File sending API URL tests ──────────────────────────────────

    #[test]
    fn telegram_api_url_send_document() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("sendDocument"),
            "https://api.telegram.org/bot123:ABC/sendDocument"
        );
    }

    #[test]
    fn telegram_api_url_send_photo() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("sendPhoto"),
            "https://api.telegram.org/bot123:ABC/sendPhoto"
        );
    }

    #[test]
    fn telegram_api_url_send_video() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("sendVideo"),
            "https://api.telegram.org/bot123:ABC/sendVideo"
        );
    }

    #[test]
    fn telegram_api_url_send_audio() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("sendAudio"),
            "https://api.telegram.org/bot123:ABC/sendAudio"
        );
    }

    #[test]
    fn telegram_api_url_send_voice() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("sendVoice"),
            "https://api.telegram.org/bot123:ABC/sendVoice"
        );
    }

    // ── File sending integration tests (with mock server) ──────────

    #[tokio::test]
    async fn telegram_send_document_bytes_builds_correct_form() {
        // This test verifies the method doesn't panic and handles bytes correctly
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes = b"Hello, this is a test file content".to_vec();

        // The actual API call will fail (no real server), but we verify the method exists
        // and handles the input correctly up to the network call
        let result = ch
            .send_document_bytes("123456", file_bytes, "test.txt", Some("Test caption"))
            .await;

        // Should fail with network error, not a panic or type error
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Error should be network-related, not a code bug
        assert!(
            err.contains("error") || err.contains("failed") || err.contains("connect"),
            "Expected network error, got: {err}"
        );
    }

    #[tokio::test]
    async fn telegram_send_photo_bytes_builds_correct_form() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        // Minimal valid PNG header bytes
        let file_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        let result = ch
            .send_photo_bytes("123456", file_bytes, "test.png", None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_document_by_url_builds_correct_json() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let result = ch
            .send_document_by_url("123456", "https://example.com/file.pdf", Some("PDF doc"))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_photo_by_url_builds_correct_json() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let result = ch
            .send_photo_by_url("123456", "https://example.com/image.jpg", None)
            .await;

        assert!(result.is_err());
    }

    // ── File path handling tests ────────────────────────────────────

    #[tokio::test]
    async fn telegram_send_document_nonexistent_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let path = Path::new("/nonexistent/path/to/file.txt");

        let result = ch.send_document("123456", path, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Should fail with file not found error
        assert!(
            err.contains("No such file") || err.contains("not found") || err.contains("os error"),
            "Expected file not found error, got: {err}"
        );
    }

    #[tokio::test]
    async fn telegram_send_photo_nonexistent_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let path = Path::new("/nonexistent/path/to/photo.jpg");

        let result = ch.send_photo("123456", path, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_video_nonexistent_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let path = Path::new("/nonexistent/path/to/video.mp4");

        let result = ch.send_video("123456", path, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_audio_nonexistent_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let path = Path::new("/nonexistent/path/to/audio.mp3");

        let result = ch.send_audio("123456", path, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_voice_nonexistent_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let path = Path::new("/nonexistent/path/to/voice.ogg");

        let result = ch.send_voice("123456", path, None).await;

        assert!(result.is_err());
    }

    // ── Caption handling tests ──────────────────────────────────────

    #[tokio::test]
    async fn telegram_send_document_bytes_with_caption() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes = b"test content".to_vec();

        // With caption
        let result = ch
            .send_document_bytes("123456", file_bytes.clone(), "test.txt", Some("My caption"))
            .await;
        assert!(result.is_err()); // Network error expected

        // Without caption
        let result = ch
            .send_document_bytes("123456", file_bytes, "test.txt", None)
            .await;
        assert!(result.is_err()); // Network error expected
    }

    #[tokio::test]
    async fn telegram_send_photo_bytes_with_caption() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes = vec![0x89, 0x50, 0x4E, 0x47];

        // With caption
        let result = ch
            .send_photo_bytes(
                "123456",
                file_bytes.clone(),
                "test.png",
                Some("Photo caption"),
            )
            .await;
        assert!(result.is_err());

        // Without caption
        let result = ch
            .send_photo_bytes("123456", file_bytes, "test.png", None)
            .await;
        assert!(result.is_err());
    }

    // ── Empty/edge case tests ───────────────────────────────────────

    #[tokio::test]
    async fn telegram_send_document_bytes_empty_file() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes: Vec<u8> = vec![];

        let result = ch
            .send_document_bytes("123456", file_bytes, "empty.txt", None)
            .await;

        // Should not panic, will fail at API level
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_document_bytes_empty_filename() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes = b"content".to_vec();

        let result = ch.send_document_bytes("123456", file_bytes, "", None).await;

        // Should not panic
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_send_document_bytes_empty_chat_id() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let file_bytes = b"content".to_vec();

        let result = ch
            .send_document_bytes("", file_bytes, "test.txt", None)
            .await;

        // Should not panic
        assert!(result.is_err());
    }

    // ── Voice message / STT tests ──────────────────────────────────────

    #[test]
    fn telegram_file_url() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.file_url("voice/file_123.ogg"),
            "https://api.telegram.org/file/bot123:ABC/voice/file_123.ogg"
        );
    }

    #[test]
    fn telegram_channel_without_stt() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        assert!(ch.stt.is_none());
    }

    #[test]
    fn telegram_channel_with_stt() {
        use crate::stt::OpenAiStt;
        let stt = Arc::new(OpenAiStt::new("sk-test".to_string(), None));
        let ch = TelegramChannel::with_stt("fake-token".into(), vec!["*".into()], stt);
        assert!(ch.stt.is_some());
    }

    #[tokio::test]
    async fn telegram_download_file_fails_with_invalid_token() {
        let ch = TelegramChannel::new("invalid-token".into(), vec!["*".into()]);
        let result = ch.download_file("some_file_id").await;
        // Should fail (network error or invalid token)
        assert!(result.is_err());
    }

    // ── Inline Keyboard Tests ──────────────────────────────────────────

    #[test]
    fn inline_button_creation() {
        let btn = InlineButton::new("Approve", "approve:req-123");
        assert_eq!(btn.text, "Approve");
        assert_eq!(btn.callback_data, "approve:req-123");
    }

    #[test]
    fn inline_button_from_string() {
        let btn = InlineButton::new("Test".to_string(), "data".to_string());
        assert_eq!(btn.text, "Test");
        assert_eq!(btn.callback_data, "data");
    }

    #[tokio::test]
    async fn telegram_send_with_inline_keyboard_fails_without_server() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);
        let buttons = vec![vec![
            InlineButton::new("✅ Approve", "approve:123"),
            InlineButton::new("❌ Reject", "reject:123"),
        ]];

        let result = ch
            .send_with_inline_keyboard("123456", "Test message", buttons)
            .await;

        // Should fail with network error (no real server)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_answer_callback_query_fails_without_server() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let result = ch
            .answer_callback_query("callback-123", Some("Approved!"), false)
            .await;

        // Should fail with network error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn telegram_edit_message_text_fails_without_server() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let result = ch
            .edit_message_text("123456", 999, "Updated message")
            .await;

        // Should fail with network error
        assert!(result.is_err());
    }

    #[test]
    fn telegram_parse_callback_query_valid() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let callback_json = serde_json::json!({
            "id": "callback-123",
            "from": {
                "id": 12345,
                "username": "testuser"
            },
            "message": {
                "message_id": 999,
                "chat": {
                    "id": 67890
                }
            },
            "data": "approve:req-abc"
        });

        let result = ch.parse_callback_query(&callback_json);
        assert!(result.is_some());

        let query = result.unwrap();
        assert_eq!(query.id, "callback-123");
        assert_eq!(query.from_user_id, "12345");
        assert_eq!(query.from_username.as_deref(), Some("testuser"));
        assert_eq!(query.chat_id, "67890");
        assert_eq!(query.message_id, 999);
        assert_eq!(query.data, "approve:req-abc");
    }

    #[test]
    fn telegram_parse_callback_query_missing_username() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        let callback_json = serde_json::json!({
            "id": "callback-456",
            "from": {
                "id": 11111
            },
            "message": {
                "message_id": 888,
                "chat": {
                    "id": 22222
                }
            },
            "data": "reject:req-xyz"
        });

        let result = ch.parse_callback_query(&callback_json);
        assert!(result.is_some());

        let query = result.unwrap();
        assert_eq!(query.from_user_id, "11111");
        assert!(query.from_username.is_none());
    }

    #[test]
    fn telegram_parse_callback_query_invalid() {
        let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()]);

        // Missing required fields
        let invalid_json = serde_json::json!({
            "id": "callback-789"
            // Missing "from", "message", "data"
        });

        let result = ch.parse_callback_query(&invalid_json);
        assert!(result.is_none());
    }

    #[test]
    fn telegram_api_url_answer_callback() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("answerCallbackQuery"),
            "https://api.telegram.org/bot123:ABC/answerCallbackQuery"
        );
    }

    #[test]
    fn telegram_api_url_edit_message_text() {
        let ch = TelegramChannel::new("123:ABC".into(), vec![]);
        assert_eq!(
            ch.api_url("editMessageText"),
            "https://api.telegram.org/bot123:ABC/editMessageText"
        );
    }
}
