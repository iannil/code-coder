//! Channel adapters for ZeroBot messaging.
//!
//! This module provides the core business logic for routing messages between
//! IM channels and the agent executor.
//!
//! Note: The local `Channel` trait is used for internal routing logic because
//! the zero-channels Channel trait has a different API (not dyn-compatible).
//! Concrete channel implementations are imported from `zero-channels`.

pub mod email_channel;
pub mod traits;

// Re-export local traits and adapters for use within zero-cli
pub use traits::{Channel, ChannelMessage, CliChannelAdapter, MessageContent, MessageSource};

// Alias for convenience: use CliChannel to mean the adapted version
pub type CliChannel = CliChannelAdapter;

// Re-export concrete channel implementations from zero-channels
// These are used directly via their own APIs (not through the Channel trait)
pub use zero_channels::{
    DiscordChannel, FeishuChannel, IMessageChannel, MatrixChannel, SlackChannel, TelegramChannel,
    WhatsAppChannel,
};

use crate::agent::ToolContext;
use crate::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::security::SecurityPolicy;
use crate::session::compactor::format_session_context;
use crate::session::types::MessageRole;
use crate::session::{SessionCompactor, SessionStore};
use crate::tools;
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use zero_agent::AgentExecutor;
use zero_channels::tts::TextToSpeech;

/// Notification sink trait for sending notifications to channels
#[async_trait]
pub trait NotificationSink: Send + Sync {
    async fn send_notification(&self, channel: &str, user_id: &str, message: &str);
    async fn send_confirmation_request(
        &self,
        channel: &str,
        user_id: &str,
        request_id: &str,
        permission: &str,
        message: &str,
    ) -> anyhow::Result<()>;
    async fn update_confirmation_result(
        &self,
        channel: &str,
        user_id: &str,
        approved: bool,
        message: &str,
    ) -> anyhow::Result<()>;
}

/// Notification sink that routes notifications to the appropriate channel
struct ChannelNotificationSink {
    channels: RwLock<HashMap<String, Arc<dyn Channel>>>,
    /// Store Telegram channel separately for inline keyboard operations
    telegram_channel: RwLock<Option<Arc<TelegramChannel>>>,
}

impl ChannelNotificationSink {
    fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            telegram_channel: RwLock::new(None),
        }
    }

    #[allow(dead_code)]
    async fn register_channel(&self, name: &str, channel: Arc<dyn Channel>) {
        self.channels
            .write()
            .await
            .insert(name.to_string(), channel);
    }

    #[allow(dead_code)]
    async fn register_telegram_channel(&self, channel: Arc<TelegramChannel>) {
        *self.telegram_channel.write().await = Some(channel);
    }
}

#[async_trait]
impl NotificationSink for ChannelNotificationSink {
    async fn send_notification(&self, channel: &str, user_id: &str, message: &str) {
        if let Some(ch) = self.channels.read().await.get(channel) {
            if let Err(e) = ch.send(message, user_id).await {
                tracing::error!("Failed to send notification to {}: {}", channel, e);
            } else {
                tracing::info!("Sent notification to {} user {}", channel, user_id);
            }
        } else {
            tracing::warn!("Channel '{}' not found for notification", channel);
        }
    }

    async fn send_confirmation_request(
        &self,
        channel: &str,
        user_id: &str,
        request_id: &str,
        permission: &str,
        message: &str,
    ) -> anyhow::Result<()> {
        // For Telegram, use inline keyboard buttons
        if channel == "telegram" {
            if let Some(tg) = self.telegram_channel.read().await.as_ref() {
                let text = format!(
                    "🔐 *CodeCoder 授权请求*\n\n\
                    📋 *操作*: {}\n\
                    📝 *详情*: {}\n\n\
                    请选择批准或拒绝此操作：",
                    escape_markdown(permission),
                    escape_markdown(message)
                );

                let buttons = vec![vec![
                    zero_channels::telegram::InlineButton::new(
                        "✅ 批准",
                        format!("approve:{request_id}"),
                    ),
                    zero_channels::telegram::InlineButton::new(
                        "✅ 始终批准",
                        format!("always:{request_id}"),
                    ),
                    zero_channels::telegram::InlineButton::new(
                        "❌ 拒绝",
                        format!("reject:{request_id}"),
                    ),
                ]];

                tg.send_with_inline_keyboard(user_id, &text, buttons)
                    .await?;
                return Ok(());
            }
        }

        // Fallback: send plain text message for non-Telegram channels
        if let Some(ch) = self.channels.read().await.get(channel) {
            let text = format!(
                "🔐 CodeCoder Authorization Request\n\n\
                Operation: {permission}\n\
                Details: {message}\n\n\
                Reply 'approve {request_id}' or 'reject {request_id}' to respond."
            );
            ch.send(&text, user_id).await?;
        }

        Ok(())
    }

    async fn update_confirmation_result(
        &self,
        channel: &str,
        user_id: &str,
        approved: bool,
        message: &str,
    ) -> anyhow::Result<()> {
        // Just send a simple status message for now
        if let Some(ch) = self.channels.read().await.get(channel) {
            let status = if approved { "✅" } else { "❌" };
            let text = format!("{status} {message}");
            ch.send(&text, user_id).await?;
        }
        Ok(())
    }
}

/// Escape special Markdown characters for Telegram
fn escape_markdown(text: &str) -> String {
    text.replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('[', "\\[")
}

/// Maximum characters per injected workspace file (matches `OpenClaw` default).
const BOOTSTRAP_MAX_CHARS: usize = 20_000;

const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;

fn spawn_supervised_listener(
    ch: Arc<dyn Channel>,
    tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let component = format!("channel:{}", ch.name());
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            crate::health::mark_component_ok(&component);
            let result = ch.listen(tx.clone()).await;

            if tx.is_closed() {
                break;
            }

            match result {
                Ok(()) => {
                    tracing::warn!("Channel {} exited unexpectedly; restarting", ch.name());
                    crate::health::mark_component_error(&component, "listener exited unexpectedly");
                }
                Err(e) => {
                    tracing::error!("Channel {} error: {e}; restarting", ch.name());
                    crate::health::mark_component_error(&component, e.to_string());
                }
            }

            crate::health::bump_component_restart(&component);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

/// Load workspace identity files and build a system prompt.
#[allow(clippy::too_many_lines)]
pub fn build_system_prompt(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[crate::skills::Skill],
    codecoder_enabled: bool,
) -> String {
    use std::fmt::Write;
    let mut prompt = String::with_capacity(8192);

    // ── 0. CodeCoder Integration (if enabled) ──────────────────
    if codecoder_enabled {
        prompt.push_str("## CodeCoder Integration (IMPORTANT)\n\n");
        prompt.push_str(
            "You have access to CodeCoder, a powerful AI workbench with 23 specialized agents.\n\n\
             **DEFAULT BEHAVIOR**: For most tasks, you should use the `codecoder` tool to delegate to \
             the appropriate CodeCoder agent. This includes:\n\
             - General questions and research → `general` agent\n\
             - Code review and quality analysis → `code-reviewer` agent\n\
             - Security analysis → `security-reviewer` agent\n\
             - Architecture decisions → `architect` agent\n\
             - Test-driven development → `tdd-guide` agent\n\
             - Weather, news, and web searches → `general` agent (has WebSearch capability)\n\n\
             **EXCEPTION**: Only use your built-in tools (bash, file operations) when:\n\
             - The user explicitly asks NOT to use CodeCoder\n\
             - The task is extremely simple (e.g., `ls`, `pwd`)\n\
             - The user specifically requests a bash command\n\n\
             When using codecoder, provide the full user request as the prompt.\n\n",
        );
    }

    // ── 1. Tooling ──────────────────────────────────────────────
    if !tools.is_empty() {
        prompt.push_str("## Tools\n\n");
        prompt.push_str("You have access to the following tools:\n\n");
        for (name, desc) in tools {
            let _ = writeln!(prompt, "- **{name}**: {desc}");
        }
        prompt.push('\n');
    }

    // ── 2. Safety ───────────────────────────────────────────────
    prompt.push_str("## Safety\n\n");
    prompt.push_str(
        "- Do not exfiltrate private data.\n\
         - Do not run destructive commands without asking.\n\
         - Do not bypass oversight or approval mechanisms.\n\
         - Prefer `trash` over `rm` (recoverable beats gone forever).\n\
         - When in doubt, ask before acting externally.\n\n",
    );

    // ── 3. Skills (compact list — load on-demand) ───────────────
    if !skills.is_empty() {
        prompt.push_str("## Available Skills\n\n");
        prompt.push_str(
            "Skills are loaded on demand. Use `read` on the skill path to get full instructions.\n\n",
        );
        prompt.push_str("<available_skills>\n");
        for skill in skills {
            let _ = writeln!(prompt, "  <skill>");
            let _ = writeln!(prompt, "    <name>{}</name>", skill.name);
            let _ = writeln!(
                prompt,
                "    <description>{}</description>",
                skill.description
            );
            let location = skill.location.clone().unwrap_or_else(|| {
                workspace_dir
                    .join("skills")
                    .join(&skill.name)
                    .join("SKILL.md")
            });
            let _ = writeln!(prompt, "    <location>{}</location>", location.display());
            let _ = writeln!(prompt, "  </skill>");
        }
        prompt.push_str("</available_skills>\n\n");
    }

    // ── 4. Workspace ────────────────────────────────────────────
    let _ = writeln!(
        prompt,
        "## Workspace\n\nWorking directory: `{}`\n",
        workspace_dir.display()
    );

    // ── 5. Bootstrap files (injected into context) ──────────────
    prompt.push_str("## Project Context\n\n");
    prompt.push_str(
        "The following workspace files define your identity, behavior, and context.\n\n",
    );

    let bootstrap_files = [
        "AGENTS.md",
        "SOUL.md",
        "TOOLS.md",
        "IDENTITY.md",
        "USER.md",
        "HEARTBEAT.md",
    ];

    for filename in &bootstrap_files {
        inject_workspace_file(&mut prompt, workspace_dir, filename);
    }

    // BOOTSTRAP.md — only if it exists (first-run ritual)
    let bootstrap_path = workspace_dir.join("BOOTSTRAP.md");
    if bootstrap_path.exists() {
        inject_workspace_file(&mut prompt, workspace_dir, "BOOTSTRAP.md");
    }

    // MEMORY.md — curated long-term memory (main session only)
    inject_workspace_file(&mut prompt, workspace_dir, "MEMORY.md");

    // ── 6. Date & Time ──────────────────────────────────────────
    let now = chrono::Local::now();
    let tz = now.format("%Z").to_string();
    let _ = writeln!(prompt, "## Current Date & Time\n\nTimezone: {tz}\n");

    // ── 7. Runtime ──────────────────────────────────────────────
    let host =
        hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
    let _ = writeln!(
        prompt,
        "## Runtime\n\nHost: {host} | OS: {} | Model: {model_name}\n",
        std::env::consts::OS,
    );

    if prompt.is_empty() {
        "You are ZeroBot, a fast and efficient AI assistant built in Rust. Be helpful, concise, and direct.".to_string()
    } else {
        prompt
    }
}

/// Inject a single workspace file into the prompt with truncation and missing-file markers.
fn inject_workspace_file(prompt: &mut String, workspace_dir: &std::path::Path, filename: &str) {
    use std::fmt::Write;

    let path = workspace_dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return;
            }
            let _ = writeln!(prompt, "### {filename}\n");
            if trimmed.len() > BOOTSTRAP_MAX_CHARS {
                prompt.push_str(&trimmed[..BOOTSTRAP_MAX_CHARS]);
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {BOOTSTRAP_MAX_CHARS} chars — use `read` for full file]\n"
                );
            } else {
                prompt.push_str(trimmed);
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            // Missing-file marker (matches OpenClaw behavior)
            let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
        }
    }
}

pub fn handle_command(command: crate::ChannelCommands, config: &Config) -> Result<()> {
    match command {
        crate::ChannelCommands::Start => {
            anyhow::bail!("Start must be handled in main.rs (requires async runtime)")
        }
        crate::ChannelCommands::Doctor => {
            anyhow::bail!("Doctor must be handled in main.rs (requires async runtime)")
        }
        crate::ChannelCommands::List => {
            println!("Channels:");
            println!("  ✅ CLI (always available)");
            for (name, configured) in [
                ("Telegram", config.channels_config.telegram.is_some()),
                ("Discord", config.channels_config.discord.is_some()),
                ("Slack", config.channels_config.slack.is_some()),
                ("Webhook", config.channels_config.webhook.is_some()),
                ("iMessage", config.channels_config.imessage.is_some()),
                ("Matrix", config.channels_config.matrix.is_some()),
                ("WhatsApp", config.channels_config.whatsapp.is_some()),
                ("Feishu", config.channels_config.feishu.is_some()),
            ] {
                println!("  {} {name}", if configured { "✅" } else { "❌" });
            }
            println!("\nTo start channels: zero-bot channel start");
            println!("To check health:    zero-bot channel doctor");
            println!("To configure:      zero-bot onboard");
            Ok(())
        }
        crate::ChannelCommands::Add {
            channel_type,
            config: _,
        } => {
            anyhow::bail!(
                "Channel type '{channel_type}' — use `zero-bot onboard` to configure channels"
            );
        }
        crate::ChannelCommands::Remove { name } => {
            anyhow::bail!("Remove channel '{name}' — edit ~/.codecoder/config.toml directly");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelHealthState {
    Healthy,
    Unhealthy,
    Timeout,
}

fn classify_health_result(
    result: &std::result::Result<bool, tokio::time::error::Elapsed>,
) -> ChannelHealthState {
    match result {
        Ok(true) => ChannelHealthState::Healthy,
        Ok(false) => ChannelHealthState::Unhealthy,
        Err(_) => ChannelHealthState::Timeout,
    }
}

/// Run health checks for configured channels.
#[allow(clippy::too_many_lines)]
pub async fn doctor_channels(_config: Config) -> Result<()> {
    // TODO: Implement health checks using zero-channels concrete types
    // The zero-channels Channel trait has a different health_check signature
    println!("🩺 ZeroBot Channel Doctor");
    println!();
    println!("Channel health checks require refactoring to use zero-channels API.");
    println!("For now, use `zero-bot onboard` to configure channels.");
    Ok(())
}

/// Start all configured channels and route messages to the agent
#[allow(clippy::too_many_lines)]
pub async fn start_channels(_config: Config) -> Result<()> {
    // TODO: Implement start_channels using zero-channels concrete types
    // This requires significant refactoring to adapt to the zero-channels API
    println!("🐦‍🔥 ZeroBot Channel Server");
    println!();
    println!("Channel startup requires refactoring to use zero-channels API.");
    println!("For now, use the CodeCoder TUI directly: `bun dev`");
    Ok(())
}
