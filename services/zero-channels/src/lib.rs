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
pub mod cli;
pub mod discord;
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
pub mod whatsapp;

// Re-export commonly used types
pub use bridge::{ChatRequest, ChatResponse, CodeCoderBridge, TokenUsage};
pub use cli::CliChannel;
pub use discord::DiscordChannel;
pub use feishu::FeishuChannel;
pub use imessage::IMessageChannel;
pub use matrix::MatrixChannel;
pub use message::{
    Attachment, AttachmentType, ChannelMessage, ChannelType, MessageContent, OutgoingContent,
    OutgoingMessage,
};
pub use outbound::{OutboundRouter, PendingResponse, SendResult};
pub use routes::{build_router, create_state, ChannelsState};
pub use slack::SlackChannel;
pub use stt::{create_stt, CompatibleStt, OpenAiStt as OpenAiSpeechToText, SpeechToText};
pub use telegram::TelegramChannel;
pub use traits::{Channel, ChannelError, ChannelResult, MessageHandler};
pub use tts::{
    create_tts, AudioFormat, ElevenLabsTts, OpenAiTts as OpenAiTextToSpeech, SynthesisOptions,
    TextToSpeech, VoiceInfo,
};
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

    // Build outbound router with channel instances
    let mut outbound = OutboundRouter::new();
    if let Some(ref t) = telegram {
        outbound = outbound.with_telegram(t.clone());
    }
    if let Some(ref f) = feishu {
        outbound = outbound.with_feishu(f.clone());
    }
    let outbound = Arc::new(outbound);

    let codecoder_endpoint = config.codecoder.endpoint.clone();

    let (state, rx) = create_state(feishu, codecoder_endpoint);
    let router = build_router(state).layer(cors);

    (router, rx, outbound)
}

/// Start the channels HTTP server with bidirectional messaging.
pub async fn start_server(config: &Config) -> anyhow::Result<()> {
    let addr = SocketAddr::from((
        config.channels.host.parse::<std::net::IpAddr>()?,
        config.channels.port,
    ));

    let (router, rx, outbound) = build_channels_router(config);

    // Create the bridge
    let bridge = Arc::new(CodeCoderBridge::new(
        config.codecoder.endpoint.clone(),
        outbound.clone(),
    ));

    // Spawn the message processor
    let processor_handle = CodeCoderBridge::spawn_processor(bridge, rx);

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

    Ok(())
}
