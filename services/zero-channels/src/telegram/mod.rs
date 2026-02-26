//! Telegram channel adapter.
//!
//! Provides the `TelegramChannel` implementation for receiving and sending messages
//! through the Telegram Bot API.

pub mod format;

use crate::message::{Attachment, AttachmentType, ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::stt::SpeechToText;
use crate::tts::TextToSpeech;
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

// ============================================================================
// Document Processing Constants
// ============================================================================

/// Maximum file size in bytes for inline content injection (32KB)
const MAX_INLINE_SIZE: usize = 32 * 1024;

/// MIME types that can be read as UTF-8 text
const TEXT_MIME_TYPES: &[&str] = &[
    "text/plain",
    "text/markdown",
    "text/csv",
    "text/xml",
    "text/html",
    "application/json",
    "application/xml",
    "application/x-yaml",
    "text/x-python",
    "text/x-rust",
    "text/x-c",
    "text/x-java",
];

/// File extensions that should be treated as text files
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "csv", "json", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf",
    "log", "py", "rs", "js", "ts", "jsx", "tsx", "java", "c", "cpp", "h", "hpp", "go", "rb",
    "php", "sh", "bash", "zsh", "html", "htm", "css", "scss", "less", "sql", "graphql", "proto",
];

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

// ============================================================================
// Telegram Channel
// ============================================================================

/// Telegram channel - long-polls the Bot API for updates.
pub struct TelegramChannel {
    bot_token: String,
    allowed_users: Vec<String>,
    client: reqwest::Client,
    /// Optional STT client for voice message transcription
    stt: Option<Arc<dyn SpeechToText>>,
    /// Optional TTS client for voice responses
    tts: Option<Arc<dyn TextToSpeech>>,
    /// Optional callback query sender for inline button clicks
    callback_tx: Option<mpsc::Sender<CallbackQuery>>,
}

impl TelegramChannel {
    /// Create a new Telegram channel.
    pub fn new(bot_token: String, allowed_users: Vec<String>) -> Self {
        Self {
            bot_token,
            allowed_users,
            client: reqwest::Client::new(),
            stt: None,
            tts: None,
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
            tts: None,
            callback_tx: None,
        }
    }

    /// Create a new Telegram channel with both STT and TTS support.
    pub fn with_voice(
        bot_token: String,
        allowed_users: Vec<String>,
        stt: Option<Arc<dyn SpeechToText>>,
        tts: Option<Arc<dyn TextToSpeech>>,
    ) -> Self {
        Self {
            bot_token,
            allowed_users,
            client: reqwest::Client::new(),
            stt,
            tts,
            callback_tx: None,
        }
    }

    /// Set the TTS client for voice responses.
    pub fn set_tts(&mut self, tts: Arc<dyn TextToSpeech>) {
        self.tts = Some(tts);
    }

    /// Get a reference to the TTS client if configured.
    pub fn tts(&self) -> Option<&Arc<dyn TextToSpeech>> {
        self.tts.as_ref()
    }

    /// Set a callback query sender for inline button click handling.
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

    /// Check if a MIME type or file extension represents a text file
    fn is_text_file(mime_type: Option<&str>, file_name: &str) -> bool {
        if let Some(mime) = mime_type {
            if TEXT_MIME_TYPES.iter().any(|t| mime.starts_with(t)) {
                return true;
            }
        }

        let ext = file_name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();
        TEXT_EXTENSIONS.contains(&ext.as_str())
    }

    /// Extract text content from a document.
    fn extract_document_content(
        file_bytes: &[u8],
        file_name: &str,
        mime_type: Option<&str>,
    ) -> Option<String> {
        // Handle PDF files
        if mime_type == Some("application/pdf") || file_name.to_lowercase().ends_with(".pdf") {
            return Some(Self::extract_pdf_content(file_bytes));
        }

        // Handle text-based files
        if Self::is_text_file(mime_type, file_name) {
            return match String::from_utf8(file_bytes.to_vec()) {
                Ok(text) => Some(text),
                Err(_) => Some(String::from_utf8_lossy(file_bytes).into_owned()),
            };
        }

        None
    }

    /// Extract text content from a PDF file
    fn extract_pdf_content(file_bytes: &[u8]) -> String {
        match pdf_extract::extract_text_from_mem(file_bytes) {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    "[PDF contains no extractable text - may be image-based]".to_string()
                } else {
                    trimmed.to_string()
                }
            }
            Err(e) => {
                tracing::warn!("PDF extraction failed: {e}");
                format!("[Failed to extract PDF text: {e}]")
            }
        }
    }

    /// Format document message for agent consumption.
    fn format_document_message(
        file_bytes: &[u8],
        file_name: &str,
        mime_type: Option<&str>,
        caption: Option<&str>,
    ) -> String {
        let file_size = file_bytes.len();
        let size_str = Self::format_file_size(file_size);
        let content = Self::extract_document_content(file_bytes, file_name, mime_type);

        match content {
            Some(text) if file_size <= MAX_INLINE_SIZE => {
                let mut message = format!(
                    "[Document: {file_name} ({size_str})]\n--- Content Start ---\n{text}\n--- Content End ---"
                );
                if let Some(cap) = caption {
                    if !cap.is_empty() {
                        message.push_str("\n\n");
                        message.push_str(cap);
                    }
                }
                message
            }
            Some(text) => {
                let truncated = if text.len() > 2000 {
                    format!("{}...\n[Content truncated, {size_str} total]", &text[..2000])
                } else {
                    text
                };
                let mut message = format!(
                    "[Document: {file_name} ({size_str})]\n--- Content Preview ---\n{truncated}\n--- End Preview ---"
                );
                if let Some(cap) = caption {
                    if !cap.is_empty() {
                        message.push_str("\n\n");
                        message.push_str(cap);
                    }
                }
                message
            }
            None => {
                let mut message = format!(
                    "[Document received: {file_name} ({size_str})] - Unsupported format for text extraction"
                );
                if let Some(cap) = caption {
                    if !cap.is_empty() {
                        message.push_str("\n\n");
                        message.push_str(cap);
                    }
                }
                message
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn format_file_size(bytes: usize) -> String {
        if bytes < 1024 {
            format!("{bytes} B")
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
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

    /// Send a voice message from bytes
    pub async fn send_voice_bytes(
        &self,
        chat_id: &str,
        file_bytes: Vec<u8>,
        file_name: &str,
        caption: Option<&str>,
    ) -> anyhow::Result<()> {
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

    /// Send a message with inline keyboard buttons
    pub async fn send_with_inline_keyboard(
        &self,
        chat_id: &str,
        text: &str,
        buttons: Vec<Vec<InlineButton>>,
    ) -> anyhow::Result<i64> {
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
            "parse_mode": "HTML",
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

        Ok(message_id)
    }

    /// Answer a callback query (acknowledge button click)
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

        Ok(())
    }

    /// Edit the text of an existing message
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
            "parse_mode": "HTML"
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

        Ok(())
    }

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

    /// Send a single message chunk with HTML parsing.
    async fn send_single_chunk(&self, message: &str, chat_id: &str) -> anyhow::Result<()> {
        let converted = format::convert_to_telegram_html(message);

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": converted,
            "parse_mode": "HTML"
        });

        let resp = self
            .client
            .post(self.api_url("sendMessage"))
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            return Ok(());
        }

        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();

        // Telegram returns "Bad Request: can't parse entities" for HTML errors
        if status.as_u16() == 400 && error_text.contains("parse entities") {
            tracing::warn!(
                "Telegram HTML parsing failed, retrying without parse_mode: {}",
                error_text
            );

            let body_plain = serde_json::json!({
                "chat_id": chat_id,
                "text": message
            });

            let resp_plain = self
                .client
                .post(self.api_url("sendMessage"))
                .json(&body_plain)
                .send()
                .await?;

            if resp_plain.status().is_success() {
                return Ok(());
            }

            let plain_error = resp_plain.text().await.unwrap_or_default();
            anyhow::bail!("Telegram sendMessage failed: {plain_error}");
        }

        anyhow::bail!("Telegram sendMessage failed: {error_text}")
    }
}

/// Split a message into chunks that fit within Telegram's limit.
fn split_message(message: &str, max_len: usize) -> Vec<String> {
    if message.len() <= max_len {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        let chunk = &remaining[..max_len];
        let split_pos = chunk
            .rfind("\n\n")
            .or_else(|| chunk.rfind('\n'))
            .or_else(|| chunk.rfind(". "))
            .or_else(|| chunk.rfind(' '))
            .unwrap_or(max_len);

        let actual_split = if split_pos == 0 { max_len } else { split_pos };

        chunks.push(remaining[..actual_split].to_string());
        remaining = remaining[actual_split..].trim_start();
    }

    chunks
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &'static str {
        "telegram"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        // Verify bot token by calling getMe
        let resp = self
            .client
            .get(self.api_url("getMe"))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(e.to_string()))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Auth(format!("Invalid bot token: {err}")));
        }

        tracing::info!("Telegram channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        const MAX_MESSAGE_LEN: usize = 4096;

        match message.content {
            OutgoingContent::Text { text } | OutgoingContent::Markdown { text } => {
                let chunks = split_message(&text, MAX_MESSAGE_LEN);
                for chunk in chunks {
                    self.send_single_chunk(&chunk, &message.channel_id)
                        .await
                        .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
                }
            }
            OutgoingContent::Voice { data, format } => {
                let filename = format!("voice.{format}");
                self.send_voice_bytes(&message.channel_id, data, &filename, None)
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
            }
            OutgoingContent::File { data, filename } => {
                self.send_document_bytes(&message.channel_id, data, &filename, None)
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;
            }
            OutgoingContent::Image { data, caption } => {
                let part = Part::bytes(data).file_name("image.png".to_string());
                let mut form = Form::new()
                    .text("chat_id", message.channel_id.clone())
                    .part("photo", part);

                if let Some(cap) = caption {
                    form = form.text("caption", cap);
                }

                let resp = self
                    .client
                    .post(self.api_url("sendPhoto"))
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

                if !resp.status().is_success() {
                    let err = resp.text().await.unwrap_or_default();
                    return Err(ChannelError::SendFailed(format!("sendPhoto failed: {err}")));
                }
            }
        }

        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        let mut offset: i64 = 0;

        tracing::info!("Telegram channel listening for messages...");

        loop {
            let url = self.api_url("getUpdates");
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
                    if let Some(uid) = update.get("update_id").and_then(serde_json::Value::as_i64) {
                        offset = uid + 1;
                    }

                    // Handle callback_query
                    if let Some(cb) = update.get("callback_query") {
                        if let Some(ref callback_tx) = self.callback_tx {
                            if let Some(query) = self.parse_callback_query(cb) {
                                let username = query.from_username.as_deref().unwrap_or("unknown");
                                let mut identities = vec![username];
                                identities.push(&query.from_user_id);

                                if self.is_any_user_allowed(identities.iter().copied()) {
                                    let _ = callback_tx.send(query).await;
                                }
                            }
                        }
                        continue;
                    }

                    let Some(message) = update.get("message") else {
                        continue;
                    };

                    let chat_id = message
                        .get("chat")
                        .and_then(|c| c.get("id"))
                        .and_then(serde_json::Value::as_i64)
                        .map(|id| id.to_string())
                        .unwrap_or_default();

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
                            "Telegram: ignoring message from unauthorized user: {}",
                            username
                        );
                        continue;
                    }

                    // Handle text message
                    let (content, attachments) =
                        if let Some(text) = message.get("text").and_then(|v| v.as_str()) {
                            tracing::info!(
                                channel = "telegram",
                                user_id = %user_id_str.as_deref().unwrap_or(username),
                                chat_id = %chat_id,
                                message_type = "text",
                                text = %text,
                                "IM message received"
                            );
                            (
                                MessageContent::Text {
                                    text: text.to_string(),
                                },
                                vec![],
                            )
                        }
                        // Handle voice message
                        else if let Some(voice) = message.get("voice") {
                            let Some(ref stt) = self.stt else {
                                continue;
                            };

                            let Some(file_id) = voice.get("file_id").and_then(|v| v.as_str())
                            else {
                                continue;
                            };

                            let audio_bytes = match self.download_file(file_id).await {
                                Ok(bytes) => bytes,
                                Err(e) => {
                                    tracing::error!("Failed to download voice: {e}");
                                    continue;
                                }
                            };

                            match stt.transcribe(&audio_bytes, "ogg").await {
                                Ok(transcription) => {
                                    tracing::info!(
                                        channel = "telegram",
                                        user_id = %user_id_str.as_deref().unwrap_or(username),
                                        chat_id = %chat_id,
                                        message_type = "voice",
                                        text = %transcription,
                                        audio_size_bytes = %audio_bytes.len(),
                                        "Voice message transcribed"
                                    );
                                    (
                                        MessageContent::Text {
                                            text: transcription,
                                        },
                                        vec![Attachment {
                                            attachment_type: AttachmentType::Audio,
                                            url: format!("telegram://file/{file_id}"),
                                            filename: Some("voice.ogg".to_string()),
                                            mime_type: Some("audio/ogg".to_string()),
                                            size_bytes: Some(audio_bytes.len() as u64),
                                        }],
                                    )
                                }
                                Err(e) => {
                                    tracing::error!("Voice transcription failed: {e}");
                                    continue;
                                }
                            }
                        }
                        // Handle document
                        else if let Some(doc) = message.get("document") {
                            let Some(file_id) = doc.get("file_id").and_then(|v| v.as_str()) else {
                                continue;
                            };

                            let file_name = doc
                                .get("file_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("document");

                            let mime_type = doc.get("mime_type").and_then(|v| v.as_str());

                            let caption = message.get("caption").and_then(|v| v.as_str());

                            let doc_bytes = match self.download_file(file_id).await {
                                Ok(bytes) => bytes,
                                Err(e) => {
                                    tracing::error!("Failed to download document: {e}");
                                    continue;
                                }
                            };

                            let text =
                                Self::format_document_message(&doc_bytes, file_name, mime_type, caption);

                            (
                                MessageContent::Text { text },
                                vec![Attachment {
                                    attachment_type: AttachmentType::Document,
                                    url: format!("telegram://file/{file_id}"),
                                    filename: Some(file_name.to_string()),
                                    mime_type: mime_type.map(String::from),
                                    size_bytes: Some(doc_bytes.len() as u64),
                                }],
                            )
                        } else {
                            continue;
                        };

                    let msg = ChannelMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        channel_type: ChannelType::Telegram,
                        channel_id: chat_id,
                        user_id: user_id_str.unwrap_or_else(|| username.to_string()),
                        content,
                        attachments,
                        metadata: std::collections::HashMap::new(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                        // Initialize tracing context for distributed tracing
                        trace_id: zero_common::logging::generate_trace_id(),
                        span_id: zero_common::logging::generate_span_id(),
                        parent_span_id: None,
                    };

                    tracing::info!(
                        trace_id = %msg.trace_id,
                        span_id = %msg.span_id,
                        message_id = %msg.id,
                        channel_id = %msg.channel_id,
                        user_id = %msg.user_id,
                        "Telegram message received - initiating trace"
                    );

                    callback(msg);
                }
            }
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        let resp = self
            .client
            .get(self.api_url("getMe"))
            .send()
            .await
            .map_err(|e| ChannelError::Connection(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ChannelError::NotReady)
        }
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        tracing::info!("Telegram channel shutting down");
        Ok(())
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
    fn is_text_file_by_extension() {
        assert!(TelegramChannel::is_text_file(None, "file.txt"));
        assert!(TelegramChannel::is_text_file(None, "readme.md"));
        assert!(TelegramChannel::is_text_file(None, "data.json"));
        assert!(!TelegramChannel::is_text_file(None, "image.png"));
    }

    #[test]
    fn format_file_size_bytes() {
        assert_eq!(TelegramChannel::format_file_size(500), "500 B");
        assert_eq!(TelegramChannel::format_file_size(1024), "1.0 KB");
        assert_eq!(TelegramChannel::format_file_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn split_message_short() {
        let result = split_message("Hello, World!", 4096);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Hello, World!");
    }

    #[test]
    fn split_message_long() {
        let msg = "x".repeat(5000);
        let result = split_message(&msg, 4096);
        assert_eq!(result.len(), 2);
    }
}
