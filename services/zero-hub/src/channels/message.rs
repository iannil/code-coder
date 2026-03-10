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
    /// WeChat Work (企业微信)
    WeCom,
    /// DingTalk (钉钉)
    DingTalk,
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
            Self::WeCom => "wecom",
            Self::DingTalk => "dingtalk",
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
    /// Trace ID for distributed tracing (UUID format)
    #[serde(default)]
    pub trace_id: String,
    /// Current span ID for this operation
    #[serde(default)]
    pub span_id: String,
    /// Parent span ID (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
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

    /// Check if this message has valid tracing context.
    pub fn has_tracing(&self) -> bool {
        !self.trace_id.is_empty() && !self.span_id.is_empty()
    }

    /// Create a RequestContext from this message for distributed tracing.
    pub fn to_request_context(&self, service: impl Into<String>) -> zero_core::common::logging::RequestContext {
        zero_core::common::logging::RequestContext {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: self.parent_span_id.clone(),
            service: service.into(),
            user_id: Some(self.user_id.clone()),
            baggage: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ────────────────────────────────────────────────────────────────────────────
    // Helper Functions
    // ────────────────────────────────────────────────────────────────────────────

    fn create_test_message(content: MessageContent) -> ChannelMessage {
        ChannelMessage {
            id: "test-123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456".into(),
            user_id: "user1".into(),
            content,
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: "trace-abc".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // ChannelType Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_channel_type_as_str() {
        assert_eq!(ChannelType::Telegram.as_str(), "telegram");
        assert_eq!(ChannelType::Discord.as_str(), "discord");
        assert_eq!(ChannelType::Slack.as_str(), "slack");
        assert_eq!(ChannelType::Feishu.as_str(), "feishu");
        assert_eq!(ChannelType::WeCom.as_str(), "wecom");
        assert_eq!(ChannelType::DingTalk.as_str(), "dingtalk");
        assert_eq!(ChannelType::WhatsApp.as_str(), "whatsapp");
        assert_eq!(ChannelType::Matrix.as_str(), "matrix");
        assert_eq!(ChannelType::IMessage.as_str(), "imessage");
        assert_eq!(ChannelType::Email.as_str(), "email");
        assert_eq!(ChannelType::Cli.as_str(), "cli");
    }

    #[test]
    fn test_channel_type_serialization() {
        let json = serde_json::to_string(&ChannelType::Telegram).unwrap();
        assert_eq!(json, "\"telegram\"");

        let parsed: ChannelType = serde_json::from_str("\"discord\"").unwrap();
        assert_eq!(parsed, ChannelType::Discord);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageContent: Text Tests
    // ────────────────────────────────────────────────────────────────────────────

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
            trace_id: "trace-abc-123".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChannelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "123");
        assert_eq!(parsed.text(), Some("Hello, world!"));
        assert_eq!(parsed.trace_id, "trace-abc-123");
        assert_eq!(parsed.span_id, "span-xyz");
    }

    #[test]
    fn test_text_message_helpers() {
        let msg = create_test_message(MessageContent::Text {
            text: "Hello".into(),
        });
        assert_eq!(msg.text(), Some("Hello"));
        assert!(!msg.is_voice());
        assert!(!msg.is_image());
        assert_eq!(msg.sender(), "user1");
        assert_eq!(msg.message_id(), "test-123");
        assert_eq!(msg.channel_type_str(), "telegram");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageContent: Voice Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_voice_message_serialization() {
        let content = MessageContent::Voice {
            url: "https://example.com/audio.ogg".into(),
            duration_secs: Some(15.5),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"voice\""));
        assert!(json.contains("\"url\":\"https://example.com/audio.ogg\""));
        assert!(json.contains("\"duration_secs\":15.5"));

        let parsed: MessageContent = serde_json::from_str(&json).unwrap();
        match parsed {
            MessageContent::Voice { url, duration_secs } => {
                assert_eq!(url, "https://example.com/audio.ogg");
                assert_eq!(duration_secs, Some(15.5));
            }
            _ => panic!("Expected Voice content"),
        }
    }

    #[test]
    fn test_voice_message_without_duration() {
        let json = r#"{"type": "voice", "url": "https://example.com/audio.ogg", "duration_secs": null}"#;
        let parsed: MessageContent = serde_json::from_str(json).unwrap();
        match parsed {
            MessageContent::Voice { url, duration_secs } => {
                assert_eq!(url, "https://example.com/audio.ogg");
                assert!(duration_secs.is_none());
            }
            _ => panic!("Expected Voice content"),
        }
    }

    #[test]
    fn test_voice_message_helpers() {
        let msg = create_test_message(MessageContent::Voice {
            url: "https://example.com/audio.ogg".into(),
            duration_secs: Some(10.0),
        });
        assert!(msg.is_voice());
        assert!(!msg.is_image());
        assert!(msg.text().is_none());
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageContent: Image Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_image_message_serialization() {
        let content = MessageContent::Image {
            url: "https://example.com/photo.jpg".into(),
            caption: Some("A beautiful sunset".into()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"url\":\"https://example.com/photo.jpg\""));
        assert!(json.contains("\"caption\":\"A beautiful sunset\""));

        let parsed: MessageContent = serde_json::from_str(&json).unwrap();
        match parsed {
            MessageContent::Image { url, caption } => {
                assert_eq!(url, "https://example.com/photo.jpg");
                assert_eq!(caption, Some("A beautiful sunset".into()));
            }
            _ => panic!("Expected Image content"),
        }
    }

    #[test]
    fn test_image_message_without_caption() {
        let json = r#"{"type": "image", "url": "https://example.com/photo.jpg", "caption": null}"#;
        let parsed: MessageContent = serde_json::from_str(json).unwrap();
        match parsed {
            MessageContent::Image { url, caption } => {
                assert_eq!(url, "https://example.com/photo.jpg");
                assert!(caption.is_none());
            }
            _ => panic!("Expected Image content"),
        }
    }

    #[test]
    fn test_image_message_helpers() {
        let msg = create_test_message(MessageContent::Image {
            url: "https://example.com/photo.jpg".into(),
            caption: None,
        });
        assert!(msg.is_image());
        assert!(!msg.is_voice());
        assert!(msg.text().is_none());
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageContent: File Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_file_message_serialization() {
        let content = MessageContent::File {
            url: "https://example.com/document.pdf".into(),
            filename: "report.pdf".into(),
            mime_type: Some("application/pdf".into()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"filename\":\"report.pdf\""));
        assert!(json.contains("\"mime_type\":\"application/pdf\""));

        let parsed: MessageContent = serde_json::from_str(&json).unwrap();
        match parsed {
            MessageContent::File {
                url,
                filename,
                mime_type,
            } => {
                assert_eq!(url, "https://example.com/document.pdf");
                assert_eq!(filename, "report.pdf");
                assert_eq!(mime_type, Some("application/pdf".into()));
            }
            _ => panic!("Expected File content"),
        }
    }

    #[test]
    fn test_file_message_without_mime_type() {
        let json = r#"{"type": "file", "url": "https://example.com/doc.txt", "filename": "doc.txt", "mime_type": null}"#;
        let parsed: MessageContent = serde_json::from_str(json).unwrap();
        match parsed {
            MessageContent::File {
                url,
                filename,
                mime_type,
            } => {
                assert_eq!(url, "https://example.com/doc.txt");
                assert_eq!(filename, "doc.txt");
                assert!(mime_type.is_none());
            }
            _ => panic!("Expected File content"),
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // MessageContent: Location Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_location_message_serialization() {
        let content = MessageContent::Location {
            latitude: 37.7749,
            longitude: -122.4194,
            title: Some("San Francisco".into()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"location\""));
        assert!(json.contains("\"latitude\":37.7749"));
        assert!(json.contains("\"longitude\":-122.4194"));
        assert!(json.contains("\"title\":\"San Francisco\""));

        let parsed: MessageContent = serde_json::from_str(&json).unwrap();
        match parsed {
            MessageContent::Location {
                latitude,
                longitude,
                title,
            } => {
                assert!((latitude - 37.7749).abs() < 0.0001);
                assert!((longitude - (-122.4194)).abs() < 0.0001);
                assert_eq!(title, Some("San Francisco".into()));
            }
            _ => panic!("Expected Location content"),
        }
    }

    #[test]
    fn test_location_message_without_title() {
        let json = r#"{"type": "location", "latitude": 35.6762, "longitude": 139.6503, "title": null}"#;
        let parsed: MessageContent = serde_json::from_str(json).unwrap();
        match parsed {
            MessageContent::Location {
                latitude,
                longitude,
                title,
            } => {
                assert!((latitude - 35.6762).abs() < 0.0001);
                assert!((longitude - 139.6503).abs() < 0.0001);
                assert!(title.is_none());
            }
            _ => panic!("Expected Location content"),
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Attachment Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_attachment_serialization() {
        let attachment = Attachment {
            attachment_type: AttachmentType::Image,
            url: "https://example.com/image.png".into(),
            filename: Some("screenshot.png".into()),
            mime_type: Some("image/png".into()),
            size_bytes: Some(1024000),
        };

        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("\"attachment_type\":\"image\""));
        assert!(json.contains("\"filename\":\"screenshot.png\""));

        let parsed: Attachment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.attachment_type, AttachmentType::Image);
        assert_eq!(parsed.size_bytes, Some(1024000));
    }

    #[test]
    fn test_attachment_types() {
        let types = [
            (AttachmentType::Image, "\"image\""),
            (AttachmentType::Audio, "\"audio\""),
            (AttachmentType::Video, "\"video\""),
            (AttachmentType::Document, "\"document\""),
            (AttachmentType::Other, "\"other\""),
        ];

        for (att_type, expected_json) in types {
            let json = serde_json::to_string(&att_type).unwrap();
            assert_eq!(json, expected_json);
        }
    }

    #[test]
    fn test_message_with_attachments() {
        let mut msg = create_test_message(MessageContent::Text { text: "See attached".into() });
        msg.attachments = vec![
            Attachment {
                attachment_type: AttachmentType::Document,
                url: "https://example.com/file.pdf".into(),
                filename: Some("file.pdf".into()),
                mime_type: Some("application/pdf".into()),
                size_bytes: Some(50000),
            },
        ];

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChannelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.attachments.len(), 1);
        assert_eq!(parsed.attachments[0].attachment_type, AttachmentType::Document);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // OutgoingMessage Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_outgoing_text_content() {
        let content = OutgoingContent::Text {
            text: "Hello, user!".into(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, user!\""));
    }

    #[test]
    fn test_outgoing_markdown_content() {
        let content = OutgoingContent::Markdown {
            text: "# Title\n\n**Bold** text".into(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"markdown\""));
    }

    #[test]
    fn test_outgoing_voice_content() {
        let content = OutgoingContent::Voice {
            data: vec![0, 1, 2, 3, 4],
            format: "mp3".into(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"voice\""));
        assert!(json.contains("\"format\":\"mp3\""));
    }

    #[test]
    fn test_outgoing_image_content() {
        let content = OutgoingContent::Image {
            data: vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG magic bytes
            caption: Some("Test image".into()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"caption\":\"Test image\""));
    }

    #[test]
    fn test_outgoing_file_content() {
        let content = OutgoingContent::File {
            data: vec![0x25, 0x50, 0x44, 0x46], // PDF magic bytes
            filename: "document.pdf".into(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"filename\":\"document.pdf\""));
    }

    #[test]
    fn test_outgoing_message_serialization() {
        let msg = OutgoingMessage {
            channel_type: ChannelType::Telegram,
            channel_id: "123456".into(),
            reply_to: Some("msg-789".into()),
            content: OutgoingContent::Text {
                text: "Reply text".into(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"channel_type\":\"telegram\""));
        assert!(json.contains("\"channel_id\":\"123456\""));
        assert!(json.contains("\"reply_to\":\"msg-789\""));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Tracing Context Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_has_tracing() {
        let msg_with_tracing = ChannelMessage {
            id: "123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: "trace-abc".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        };

        assert!(msg_with_tracing.has_tracing());

        let msg_without_tracing = ChannelMessage {
            id: "123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: String::new(),
            span_id: String::new(),
            parent_span_id: None,
        };

        assert!(!msg_without_tracing.has_tracing());
    }

    #[test]
    fn test_tracing_fields_serialization_defaults() {
        // Test that tracing fields default to empty when deserializing JSON without them
        let json = r#"{
            "id": "123",
            "channel_type": "telegram",
            "channel_id": "456",
            "user_id": "user1",
            "content": {"type": "text", "text": "Hello"},
            "timestamp": 1234567890000
        }"#;

        let parsed: ChannelMessage = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.trace_id, "");
        assert_eq!(parsed.span_id, "");
        assert!(parsed.parent_span_id.is_none());
    }

    #[test]
    fn test_parent_span_id_skipped_when_none() {
        let msg = ChannelMessage {
            id: "123".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text {
                text: "Hello".into(),
            },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: "trace-abc".into(),
            span_id: "span-xyz".into(),
            parent_span_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("parent_span_id"));

        let msg_with_parent = ChannelMessage {
            parent_span_id: Some("parent-123".into()),
            ..msg
        };

        let json = serde_json::to_string(&msg_with_parent).unwrap();
        assert!(json.contains("parent_span_id"));
        assert!(json.contains("parent-123"));
    }

    #[test]
    fn test_to_request_context() {
        let msg = create_test_message(MessageContent::Text { text: "Test".into() });
        let ctx = msg.to_request_context("test-service");

        assert_eq!(ctx.trace_id, "trace-abc");
        assert_eq!(ctx.span_id, "span-xyz");
        assert_eq!(ctx.service, "test-service");
        assert_eq!(ctx.user_id, Some("user1".into()));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Metadata Tests
    // ────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_message_with_metadata() {
        let mut msg = create_test_message(MessageContent::Text { text: "Test".into() });
        msg.metadata.insert("debug_mode".into(), "true".into());
        msg.metadata.insert("source".into(), "telegram".into());

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChannelMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.metadata.get("debug_mode"), Some(&"true".to_string()));
        assert_eq!(parsed.metadata.get("source"), Some(&"telegram".to_string()));
    }
}
