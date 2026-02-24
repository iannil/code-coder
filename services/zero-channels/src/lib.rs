//! Zero Channels - Channel adapters for the Zero ecosystem.
//!
//! This crate provides adapters for various messaging channels:
//! - Telegram
//! - Discord
//! - Slack
//! - Feishu
//! - WhatsApp
//! - Matrix
//! - iMessage
//! - CLI (for local testing)
//!
//! ## Architecture
//!
//! The channels service receives messages from IM platforms via webhooks or polling,
//! forwards them to CodeCoder for processing, and routes responses back to the
//! originating channel.
//!
//! ```text
//! User IM → webhook → zero-channels → CodeCoder
//!                          ↓              ↓
//! User ←── send ←── OutboundRouter ← Response
//! ```

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod bridge;
pub mod capture_bridge;
pub mod cli;
pub mod dingtalk;
pub mod discord;
pub mod email;
pub mod feishu;
pub mod imessage;
pub mod matrix;
pub mod message;
pub mod outbound;
pub mod routes;
pub mod slack;
pub mod stt;
pub mod telegram;
pub mod traits;
pub mod tts;
pub mod wecom;
pub mod whatsapp;

// Re-export commonly used types
pub use bridge::{ChatApiResponse, ChatRequest, ChatResponseData, CodeCoderBridge, TokenUsage};
pub use capture_bridge::{
    AssetContentType, CaptureBridge, CapturedAsset, ExtractedContent, FeishuDocsClient,
    NotionClient, SavedLocation, SummaryResult,
};
pub use cli::CliChannel;
pub use dingtalk::DingTalkChannel;
pub use discord::DiscordChannel;
pub use email::EmailChannel;
pub use feishu::FeishuChannel;
pub use imessage::IMessageChannel;
pub use matrix::MatrixChannel;
pub use message::{
    Attachment, AttachmentType, ChannelMessage, ChannelType, MessageContent, OutgoingContent,
    OutgoingMessage,
};
pub use outbound::{OutboundRouter, PendingResponse, SendResult};
pub use routes::{build_router, create_state, create_state_extended, create_state_with_capture, ChannelsState};
pub use slack::SlackChannel;
pub use stt::{create_stt, CompatibleStt, OpenAiStt as OpenAiSpeechToText, SpeechToText};
pub use telegram::TelegramChannel;
pub use traits::{Channel, ChannelError, ChannelResult, MessageHandler};
pub use tts::{
    create_tts, AudioFormat, ElevenLabsTts, OpenAiTts as OpenAiTextToSpeech, SynthesisOptions,
    TextToSpeech, VoiceInfo,
};
pub use wecom::WeComChannel;
pub use whatsapp::WhatsAppChannel;

use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use zero_common::config::Config;

/// Build the channels router with CORS middleware and outbound router.
pub fn build_channels_router(
    config: &Config,
) -> (
    axum::Router,
    tokio::sync::mpsc::Receiver<ChannelMessage>,
    Arc<OutboundRouter>,
    Option<Arc<EmailChannel>>,
    Option<Arc<TelegramChannel>>,
    tokio::sync::mpsc::Sender<ChannelMessage>,
) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create Telegram channel if configured
    let telegram = config
        .channels
        .telegram
        .as_ref()
        .filter(|t| t.enabled)
        .map(|t| Arc::new(TelegramChannel::new(t.bot_token.clone(), t.allowed_users.clone())));

    // Create Feishu channel if configured
    let feishu = config
        .channels
        .feishu
        .as_ref()
        .filter(|f| f.enabled)
        .map(|f| {
            Arc::new(FeishuChannel::with_encryption(
                f.app_id.clone(),
                f.app_secret.clone(),
                f.encrypt_key.clone(),
                f.verification_token.clone(),
                f.allowed_users.clone(),
            ))
        });

    // Create WeChat Work channel if configured
    let wecom = config
        .channels
        .wecom
        .as_ref()
        .filter(|w| w.enabled)
        .map(|w| {
            Arc::new(WeComChannel::with_encryption(
                w.corp_id.clone(),
                w.agent_id,
                w.secret.clone(),
                w.token.clone(),
                w.encoding_aes_key.clone(),
                w.allowed_users.clone(),
            ))
        });

    // Create DingTalk channel if configured
    let dingtalk = config
        .channels
        .dingtalk
        .as_ref()
        .filter(|d| d.enabled)
        .map(|d| {
            Arc::new(DingTalkChannel::with_config(
                d.app_key.clone(),
                d.app_secret.clone(),
                d.robot_code.clone(),
                d.outgoing_token.clone(),
                d.allowed_users.clone(),
                d.stream_mode,
            ))
        });

    // Create WhatsApp channel if configured
    let whatsapp = config
        .channels
        .whatsapp
        .as_ref()
        .filter(|w| w.enabled)
        .map(|w| {
            Arc::new(WhatsAppChannel::new(
                w.access_token.clone(),
                w.phone_number_id.clone(),
                w.verify_token.clone().unwrap_or_default(),
                w.allowed_numbers.clone(),
            ))
        });

    // Create Email channel if configured
    let email = config
        .channels
        .email
        .as_ref()
        .filter(|e| e.enabled)
        .map(|e| Arc::new(EmailChannel::new(e.clone())));

    // WhatsApp app secret for webhook signature verification
    // Priority: environment variable > config (not yet in zero-common config)
    let whatsapp_app_secret: Option<Arc<str>> = std::env::var("ZERO_BOT_WHATSAPP_APP_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| Arc::from(secret))
        });

    // Build outbound router with channel instances
    let mut outbound = OutboundRouter::new();
    if let Some(ref t) = telegram {
        outbound = outbound.with_telegram(t.clone());
    }
    if let Some(ref f) = feishu {
        outbound = outbound.with_feishu(f.clone());
    }
    if let Some(ref w) = wecom {
        outbound = outbound.with_wecom(w.clone());
    }
    if let Some(ref d) = dingtalk {
        outbound = outbound.with_dingtalk(d.clone());
    }
    if let Some(ref w) = whatsapp {
        outbound = outbound.with_whatsapp(w.clone());
    }
    if let Some(ref e) = email {
        outbound = outbound.with_email(e.clone());
    }
    let outbound = Arc::new(outbound);

    let codecoder_endpoint = config.codecoder.endpoint.clone();

    // Create state with all channels including WhatsApp
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let tx_clone = tx.clone();
    let state = Arc::new(routes::ChannelsState {
        feishu,
        wecom,
        dingtalk,
        whatsapp,
        whatsapp_app_secret,
        message_tx: tx,
        codecoder_endpoint,
        outbound: Some(outbound.clone()),
        capture: None,
    });
    let router = build_router(state).layer(cors);

    (router, rx, outbound, email, telegram, tx_clone)
}

/// Start the channels HTTP server with bidirectional messaging.
pub async fn start_server(config: &Config) -> anyhow::Result<()> {
    let addr = SocketAddr::from((
        config.channels.host.parse::<std::net::IpAddr>()?,
        config.channels.port,
    ));

    let (router, rx, outbound, email, telegram, tx) = build_channels_router(config);

    // Create the bridge
    let bridge = Arc::new(CodeCoderBridge::new(
        config.codecoder.endpoint.clone(),
        outbound.clone(),
    ));

    // Spawn the message processor
    let processor_handle = CodeCoderBridge::spawn_processor(bridge, rx);

    // Spawn email polling task if email channel is configured
    let email_handle = if let Some(email_channel) = email {
        let email_tx = tx.clone();
        Some(tokio::spawn(async move {
            if let Err(e) = email_channel.start_polling(email_tx).await {
                tracing::error!(error = %e, "Email polling failed");
            }
        }))
    } else {
        None
    };

    // Spawn Telegram polling task if Telegram channel is configured
    let telegram_handle = if let Some(telegram_channel) = telegram {
        let telegram_tx = tx.clone();
        tracing::info!("Starting Telegram long-polling...");
        Some(tokio::spawn(async move {
            let callback = move |msg: ChannelMessage| {
                let tx = telegram_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = tx.send(msg).await {
                        tracing::error!(error = %e, "Failed to forward Telegram message");
                    }
                });
            };
            if let Err(e) = telegram_channel.listen(callback).await {
                tracing::error!(error = %e, "Telegram polling failed");
            }
        }))
    } else {
        None
    };

    // Spawn cleanup task for stale pending responses
    let cleanup_outbound = outbound.clone();
    let cleanup_handle = tokio::spawn(async move {
        let ttl_ms = 300_000; // 5 minutes
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;
            cleanup_outbound.cleanup_stale(ttl_ms).await;
        }
    });

    tracing::info!("Starting Zero Channels on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    // Clean up on shutdown
    cleanup_handle.abort();
    processor_handle.abort();
    if let Some(handle) = email_handle {
        handle.abort();
    }
    if let Some(handle) = telegram_handle {
        handle.abort();
    }

    Ok(())
}
