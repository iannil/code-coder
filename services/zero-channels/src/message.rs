//! Message types for channel communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Channel type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Telegram,
    Discord,
    Slack,
    Feishu,
    WhatsApp,
    Matrix,
    IMessage,
    Email,
    Cli,
}

impl ChannelType {
    /// Get the channel type as a string.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Telegram => "telegram",
            Self::Discord => "discord",
            Self::Slack => "slack",
            Self::Feishu => "feishu",
            Self::WhatsApp => "whatsapp",
            Self::Matrix => "matrix",
            Self::IMessage => "imessage",
            Self::Email => "email",
            Self::Cli => "cli",
        }
    }
}

/// Unified message format for all channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Message ID (channel-specific)
    pub id: String,
    /// Channel type
    pub channel_type: ChannelType,
    /// Channel-specific identifier (chat ID, channel ID, etc.)
    pub channel_id: String,
    /// User identifier
    pub user_id: String,
    /// Message content
    pub content: MessageContent,
    /// Attachments (images, files, voice, etc.)
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Timestamp (Unix millis)
    pub timestamp: i64,
}

/// Message content types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    /// Plain text message
    Text { text: String },
    /// Voice message (needs STT processing)
    Voice {
        /// URL or path to audio file
        url: String,
        /// Duration in seconds
        duration_secs: Option<f32>,
    },
    /// Image message
    Image {
        url: String,
        caption: Option<String>,
    },
    /// File/document message
    File {
        url: String,
        filename: String,
        mime_type: Option<String>,
    },
    /// Location message
    Location {
        latitude: f64,
        longitude: f64,
        title: Option<String>,
    },
}

/// Attachment types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment type
    pub attachment_type: AttachmentType,
    /// URL or path
    pub url: String,
    /// Filename
    pub filename: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Size in bytes
    pub size_bytes: Option<u64>,
}

/// Attachment type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentType {
    Image,
    Audio,
    Video,
    Document,
    Other,
}

/// Outgoing message to send to a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    /// Target channel type
    pub channel_type: ChannelType,
    /// Target channel ID
    pub channel_id: String,
    /// Reply to message ID (optional)
    pub reply_to: Option<String>,
    /// Message content
    pub content: OutgoingContent,
}

/// Outgoing message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutgoingContent {
    /// Plain text
    Text { text: String },
    /// Markdown text
    Markdown { text: String },
    /// Voice message (from TTS)
    Voice {
        data: Vec<u8>,
        format: String,
    },
    /// Image
    Image {
        data: Vec<u8>,
        caption: Option<String>,
    },
    /// File
    File {
        data: Vec<u8>,
        filename: String,
    },
}

impl ChannelMessage {
    /// Get the text content if this is a text message.
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Alias for user_id for compatibility with legacy code.
    pub fn sender(&self) -> &str {
        &self.user_id
    }

    /// Alias for id for compatibility with legacy code.
    pub fn message_id(&self) -> &str {
        &self.id
    }

    /// Get the channel type as a string.
    pub fn channel_type_str(&self) -> &'static str {
        self.channel_type.as_str()
    }

    /// Check if this is a voice message.
    pub const fn is_voice(&self) -> bool {
        matches!(self.content, MessageContent::Voice { .. })
    }

    /// Check if this is an image message.
    pub const fn is_image(&self) -> bool {
        matches!(self.content, MessageContent::Image { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_message_serialization() {
        let msg = ChannelMessage {
            id: "123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello, world!".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChannelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "123");
        assert_eq!(parsed.text(), Some("Hello, world!"));
    }
}
