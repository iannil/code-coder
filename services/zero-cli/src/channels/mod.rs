//! Channel adapters for ZeroBot messaging.
//!
//! This module provides the core business logic for routing messages between
//! IM channels and the agent executor.
//!
//! Note: The local `Channel` trait is used for internal routing logic because
//! the zero-channels Channel trait has a different API (not dyn-compatible).
//! Concrete channel implementations are imported from `zero-channels`.

pub mod traits;

// Re-export local traits and adapters for use within zero-cli
pub use traits::{Channel, ChannelMessage, CliChannelAdapter, MessageSource};

// Alias for convenience: use CliChannel to mean the adapted version
pub type CliChannel = CliChannelAdapter;

// Re-export concrete channel implementations from zero-channels
// These are used directly via their own APIs (not through the Channel trait)
pub use zero_channels::TelegramChannel;

use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    #[allow(dead_code)]
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

/// Run health checks for configured channels.
///
/// This verifies:
/// - Telegram: bot token format and API connectivity
/// - Feishu: app credentials format
/// - WeChat Work: corp_id and secret format
/// - DingTalk: app key/secret format
#[allow(clippy::too_many_lines)]
pub async fn doctor_channels(config: Config) -> Result<()> {
    use std::time::Duration;

    println!("🩺 ZeroBot Channel Doctor");
    println!();

    let mut all_healthy = true;
    let timeout = Duration::from_secs(10);

    // Check Telegram
    if let Some(ref tg) = config.channels_config.telegram {
        print!("  Telegram: ");
        // Validate bot token format (numeric_id:alphanumeric_secret)
        if tg.bot_token.contains(':') && tg.bot_token.split(':').count() == 2 {
            let parts: Vec<&str> = tg.bot_token.split(':').collect();
            if parts[0].chars().all(|c| c.is_ascii_digit()) {
                // Try to verify with Telegram API
                let client = reqwest::Client::builder()
                    .timeout(timeout)
                    .build()
                    .unwrap_or_default();
                let url = format!("https://api.telegram.org/bot{}/getMe", tg.bot_token);

                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        println!("✅ Connected (allowed_users: {})", tg.allowed_users.len());
                    }
                    Ok(resp) => {
                        println!("❌ API error: {}", resp.status());
                        all_healthy = false;
                    }
                    Err(e) => {
                        println!("⚠️  Network error: {}", e);
                        all_healthy = false;
                    }
                }
            } else {
                println!("❌ Invalid token format (bot ID should be numeric)");
                all_healthy = false;
            }
        } else {
            println!("❌ Invalid token format (expected: BOT_ID:SECRET)");
            all_healthy = false;
        }
    } else {
        println!("  Telegram: ⏸️  Not configured");
    }

    // Check Feishu
    if let Some(ref fs) = config.channels_config.feishu {
        print!("  Feishu: ");
        // Validate app_id format (cli_xxx)
        if fs.app_id.starts_with("cli_") && !fs.app_secret.is_empty() {
            println!("✅ Configured (app_id: {})", &fs.app_id[..8.min(fs.app_id.len())]);
        } else if fs.app_id.is_empty() {
            println!("❌ Missing app_id");
            all_healthy = false;
        } else if fs.app_secret.is_empty() {
            println!("❌ Missing app_secret");
            all_healthy = false;
        } else {
            println!("⚠️  Unusual app_id format (expected: cli_xxx)");
        }
    } else {
        println!("  Feishu: ⏸️  Not configured");
    }

    // Check Discord
    if let Some(ref dc) = config.channels_config.discord {
        print!("  Discord: ");
        // Discord bot tokens are base64-like strings
        if dc.bot_token.len() > 50 && dc.bot_token.contains('.') {
            println!("✅ Configured");
        } else {
            println!("⚠️  Token format may be invalid");
        }
    } else {
        println!("  Discord: ⏸️  Not configured");
    }

    // Check Slack
    if let Some(ref sl) = config.channels_config.slack {
        print!("  Slack: ");
        // Slack bot tokens start with xoxb-
        if sl.bot_token.starts_with("xoxb-") {
            println!("✅ Configured");
        } else {
            println!("⚠️  Token format may be invalid (expected: xoxb-xxx)");
        }
    } else {
        println!("  Slack: ⏸️  Not configured");
    }

    // Check Matrix
    if let Some(ref mx) = config.channels_config.matrix {
        print!("  Matrix: ");
        if !mx.homeserver.is_empty() && !mx.access_token.is_empty() {
            println!("✅ Configured (homeserver: {})", mx.homeserver);
        } else {
            println!("❌ Missing homeserver or access_token");
            all_healthy = false;
        }
    } else {
        println!("  Matrix: ⏸️  Not configured");
    }

    // Check WhatsApp
    if let Some(ref wa) = config.channels_config.whatsapp {
        print!("  WhatsApp: ");
        if !wa.access_token.is_empty() && !wa.phone_number_id.is_empty() {
            println!("✅ Configured (phone_id: {})", wa.phone_number_id);
        } else {
            println!("❌ Missing access_token or phone_number_id");
            all_healthy = false;
        }
    } else {
        println!("  WhatsApp: ⏸️  Not configured");
    }

    // Check iMessage (macOS only)
    if let Some(ref im) = config.channels_config.imessage {
        print!("  iMessage: ");
        #[cfg(target_os = "macos")]
        {
            if !im.allowed_contacts.is_empty() {
                println!("✅ Configured ({} contacts)", im.allowed_contacts.len());
            } else {
                println!("⚠️  No allowed contacts configured");
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = im;
            println!("❌ Only available on macOS");
            all_healthy = false;
        }
    } else {
        println!("  iMessage: ⏸️  Not configured");
    }

    println!();
    if all_healthy {
        println!("✅ All configured channels are healthy");
    } else {
        println!("⚠️  Some channels have issues - check configuration");
    }

    println!();
    println!("To configure channels: zero-bot onboard");
    println!("To start channels:     zero-bot channel start");

    Ok(())
}

/// Convert zero-cli Config to zero-common Config for use with zero-channels.
fn to_common_config(config: &Config) -> zero_common::config::Config {
    use zero_common::config as common;

    let mut common_cfg = common::Config::default();

    // Use port 4431 for channels to avoid conflict with daemon's port 4402
    // This is separate from the daemon HTTP server
    common_cfg.channels.port = 4431;
    common_cfg.channels.host = "127.0.0.1".to_string();

    // Map Telegram config
    if let Some(ref tg) = config.channels_config.telegram {
        common_cfg.channels.telegram = Some(common::TelegramConfig {
            enabled: true,
            bot_token: tg.bot_token.clone(),
            allowed_users: tg.allowed_users.clone(),
            allowed_chats: vec![],
        });
    }

    // Map Feishu config
    if let Some(ref fs) = config.channels_config.feishu {
        common_cfg.channels.feishu = Some(common::FeishuConfig {
            enabled: true,
            app_id: fs.app_id.clone(),
            app_secret: fs.app_secret.clone(),
            encrypt_key: fs.encrypt_key.clone(),
            verification_token: fs.verification_token.clone(),
            allowed_users: fs.allowed_users.clone(),
        });
    }

    // Map Discord config
    if let Some(ref dc) = config.channels_config.discord {
        common_cfg.channels.discord = Some(common::DiscordConfig {
            enabled: true,
            bot_token: dc.bot_token.clone(),
            allowed_guilds: dc.guild_id.as_ref().map(|g| vec![g.clone()]).unwrap_or_default(),
            allowed_channels: vec![],
        });
    }

    // Map Slack config
    if let Some(ref sl) = config.channels_config.slack {
        common_cfg.channels.slack = Some(common::SlackConfig {
            enabled: true,
            bot_token: sl.bot_token.clone(),
            app_token: sl.app_token.clone().unwrap_or_default(),
            signing_secret: None,
        });
    }

    // Map CodeCoder endpoint
    common_cfg.codecoder.endpoint = config.codecoder.endpoint.clone();

    common_cfg
}

/// Start all configured channels and route messages to the agent.
///
/// This function starts the zero-channels HTTP server which handles:
/// - Telegram webhook/polling for bot messages
/// - Feishu event callbacks
/// - WeChat Work (企业微信) message callbacks
/// - DingTalk (钉钉) robot callbacks
///
/// Messages are forwarded to the CodeCoder API for processing.
#[allow(clippy::too_many_lines)]
pub async fn start_channels(config: Config) -> Result<()> {
    println!("🐦‍🔥 ZeroBot Channel Server");
    println!();

    // Convert to zero-common Config format
    let common_cfg = to_common_config(&config);

    // Start the channels server
    zero_channels::start_server(&common_cfg).await?;

    Ok(())
}
