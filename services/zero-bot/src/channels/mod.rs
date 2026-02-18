pub mod cli;
pub mod discord;
pub mod email_channel;
pub mod feishu;
pub mod imessage;
pub mod matrix;
pub mod slack;
pub mod telegram;
pub mod traits;
pub mod whatsapp;

pub use cli::CliChannel;
pub use discord::DiscordChannel;
pub use feishu::FeishuChannel;
pub use imessage::IMessageChannel;
pub use matrix::MatrixChannel;
pub use slack::SlackChannel;
pub use telegram::TelegramChannel;
pub use traits::Channel;
pub use whatsapp::WhatsAppChannel;

use crate::agent::confirmation::{self, NotificationSink, ConfirmationResponse};
use crate::agent::AgentExecutor;
use crate::config::Config;
use crate::memory::{self, Memory};
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
use telegram::{CallbackQuery, InlineButton};
use tokio::sync::RwLock;

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

    async fn register_channel(&self, name: &str, channel: Arc<dyn Channel>) {
        self.channels.write().await.insert(name.to_string(), channel);
    }

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
                    InlineButton::new("✅ 批准", format!("approve:{request_id}")),
                    InlineButton::new("✅ 始终批准", format!("always:{request_id}")),
                    InlineButton::new("❌ 拒绝", format!("reject:{request_id}")),
                ]];

                tg.send_with_inline_keyboard(user_id, &text, buttons).await?;
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
        // In the future, we could edit the original message
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

/// Handle Telegram callback queries (button clicks) for confirmation requests
async fn handle_telegram_callbacks(
    tg: Arc<TelegramChannel>,
    mut rx: tokio::sync::mpsc::Receiver<CallbackQuery>,
) {
    tracing::info!("Starting Telegram callback query handler");

    while let Some(query) = rx.recv().await {
        tracing::debug!(
            "Received callback query: id={}, data={}, from={}",
            query.id,
            query.data,
            query.from_user_id
        );

        // Parse callback data: "approve:request_id", "always:request_id", or "reject:request_id"
        let parts: Vec<&str> = query.data.splitn(2, ':').collect();
        if parts.len() != 2 {
            tracing::warn!("Invalid callback data format: {}", query.data);
            let _ = tg
                .answer_callback_query(&query.id, Some("无效的回调数据"), false)
                .await;
            continue;
        }

        let (action, request_id) = (parts[0], parts[1]);
        let response = match action {
            "approve" => ConfirmationResponse::Once,
            "always" => ConfirmationResponse::Always,
            _ => ConfirmationResponse::Reject,
        };
        let approved = response.is_approved();
        let is_always = response.is_always();

        // Respond to the confirmation registry with the full response type
        let handled = confirmation::handle_confirmation_response_with_type(request_id, response).await;

        if handled {
            let response_text = if is_always {
                "✅ 已始终批准此类操作"
            } else if approved {
                "✅ 已批准操作"
            } else {
                "❌ 已拒绝操作"
            };

            // Answer the callback query to remove loading state
            let _ = tg
                .answer_callback_query(&query.id, Some(response_text), false)
                .await;

            // Update the original message to show the result
            let update_text = if is_always {
                format!("✅ *已始终批准*\n\n此类操作将自动批准。\n请求 ID: {request_id}")
            } else if approved {
                format!("✅ *已批准*\n\n请求 ID: {request_id}")
            } else {
                format!("❌ *已拒绝*\n\n请求 ID: {request_id}")
            };

            let _ = tg
                .edit_message_text(&query.chat_id, query.message_id, &update_text)
                .await;

            tracing::info!(
                "Confirmation {} {} by user {} (always={})",
                request_id,
                if approved { "approved" } else { "rejected" },
                query.from_user_id,
                is_always
            );
        } else {
            // Request not found (expired or already handled)
            let _ = tg
                .answer_callback_query(&query.id, Some("⚠️ 请求已过期或已处理"), true)
                .await;
            tracing::warn!(
                "Callback for unknown/expired request: {} (user: {})",
                request_id,
                query.from_user_id
            );
        }
    }

    tracing::info!("Telegram callback query handler stopped");
}

/// Maximum characters per injected workspace file (matches `OpenClaw` default).
const BOOTSTRAP_MAX_CHARS: usize = 20_000;

const DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS: u64 = 2;
const DEFAULT_CHANNEL_MAX_BACKOFF_SECS: u64 = 60;

fn spawn_supervised_listener(
    ch: Arc<dyn Channel>,
    tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
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
///
/// Follows the `OpenClaw` framework structure:
/// 1. Tooling — tool list + descriptions
/// 2. Safety — guardrail reminder
/// 3. Skills — compact list with paths (loaded on-demand)
/// 4. Workspace — working directory
/// 5. Bootstrap files — AGENTS, SOUL, TOOLS, IDENTITY, USER, HEARTBEAT, BOOTSTRAP, MEMORY
/// 6. Date & Time — timezone for cache stability
/// 7. Runtime — host, OS, model
///
/// Daily memory files (`memory/*.md`) are NOT injected — they are accessed
/// on-demand via `memory_recall` / `memory_search` tools.
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
    prompt
        .push_str("The following workspace files define your identity, behavior, and context.\n\n");

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
pub async fn doctor_channels(config: Config) -> Result<()> {
    let mut channels: Vec<(&'static str, Arc<dyn Channel>)> = Vec::new();

    if let Some(ref tg) = config.channels_config.telegram {
        // Note: For health check, we don't need STT - just check basic connectivity
        channels.push((
            "Telegram",
            Arc::new(TelegramChannel::new(
                tg.bot_token.clone(),
                tg.allowed_users.clone(),
            )),
        ));
    }

    if let Some(ref dc) = config.channels_config.discord {
        channels.push((
            "Discord",
            Arc::new(DiscordChannel::new(
                dc.bot_token.clone(),
                dc.guild_id.clone(),
                dc.allowed_users.clone(),
            )),
        ));
    }

    if let Some(ref sl) = config.channels_config.slack {
        channels.push((
            "Slack",
            Arc::new(SlackChannel::new(
                sl.bot_token.clone(),
                sl.channel_id.clone(),
                sl.allowed_users.clone(),
            )),
        ));
    }

    if let Some(ref im) = config.channels_config.imessage {
        channels.push((
            "iMessage",
            Arc::new(IMessageChannel::new(im.allowed_contacts.clone())),
        ));
    }

    if let Some(ref mx) = config.channels_config.matrix {
        channels.push((
            "Matrix",
            Arc::new(MatrixChannel::new(
                mx.homeserver.clone(),
                mx.access_token.clone(),
                mx.room_id.clone(),
                mx.allowed_users.clone(),
            )),
        ));
    }

    if let Some(ref wa) = config.channels_config.whatsapp {
        channels.push((
            "WhatsApp",
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone(),
                wa.phone_number_id.clone(),
                wa.verify_token.clone(),
                wa.allowed_numbers.clone(),
            )),
        ));
    }

    if let Some(ref fs) = config.channels_config.feishu {
        channels.push((
            "Feishu",
            Arc::new(FeishuChannel::with_encryption(
                fs.app_id.clone(),
                fs.app_secret.clone(),
                fs.encrypt_key.clone(),
                fs.verification_token.clone(),
                fs.allowed_users.clone(),
            )),
        ));
    }

    if channels.is_empty() {
        println!("No real-time channels configured. Run `zero-bot onboard` first.");
        return Ok(());
    }

    println!("🩺 ZeroBot Channel Doctor");
    println!();

    let mut healthy = 0_u32;
    let mut unhealthy = 0_u32;
    let mut timeout = 0_u32;

    for (name, channel) in channels {
        let result = tokio::time::timeout(Duration::from_secs(10), channel.health_check()).await;
        let state = classify_health_result(&result);

        match state {
            ChannelHealthState::Healthy => {
                healthy += 1;
                println!("  ✅ {name:<9} healthy");
            }
            ChannelHealthState::Unhealthy => {
                unhealthy += 1;
                println!("  ❌ {name:<9} unhealthy (auth/config/network)");
            }
            ChannelHealthState::Timeout => {
                timeout += 1;
                println!("  ⏱️  {name:<9} timed out (>10s)");
            }
        }
    }

    if config.channels_config.webhook.is_some() {
        println!("  ℹ️  Webhook   check via `zero-bot gateway` then GET /health");
    }

    println!();
    println!("Summary: {healthy} healthy, {unhealthy} unhealthy, {timeout} timed out");
    Ok(())
}

/// Start all configured channels and route messages to the agent
#[allow(clippy::too_many_lines)]
pub async fn start_channels(config: Config) -> Result<()> {
    let provider: Arc<dyn Provider> = Arc::from(providers::create_resilient_provider(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        &config.reliability,
    )?);

    // Warm up the provider connection pool (TLS handshake, DNS, HTTP/2 setup)
    // so the first real message doesn't hit a cold-start timeout.
    if let Err(e) = provider.warmup().await {
        tracing::warn!("Provider warmup failed (non-fatal): {e}");
    }

    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // Initialize session store for multi-turn conversation context
    let session_store: Option<Arc<SessionStore>> = if config.session.enabled {
        let db_path = config.workspace_dir.join("sessions.db");
        match SessionStore::new(&db_path) {
            Ok(store) => {
                tracing::info!("Session store initialized: {}", db_path.display());
                Some(Arc::new(store))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize session store: {e}");
                None
            }
        }
    } else {
        None
    };

    // Initialize session compactor for context compression
    let session_compactor: Option<Arc<SessionCompactor>> = session_store.as_ref().map(|_| {
        Arc::new(SessionCompactor::new(provider.clone(), model.clone()))
    });

    // Session configuration for auto-compaction
    let session_config = config.session.clone();

    // Create security policy
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    // Create actual tools for execution
    let vault_path = config.config_path.parent()
        .map_or_else(|| config.workspace_dir.clone(), std::path::Path::to_path_buf);
    let tool_instances = tools::all_tools(
        &security,
        mem.clone(),
        &config.browser,
        &config.codecoder,
        &config.vault,
        &vault_path,
    );

    // Build system prompt from workspace identity files + skills
    let workspace = config.workspace_dir.clone();
    let skills = crate::skills::load_skills(&workspace);

    // Collect tool descriptions for the prompt (from actual tools)
    let tool_descs: Vec<(&str, &str)> = vec![
        (
            "shell",
            "Execute terminal commands. Use when: running local checks, build/test commands, diagnostics.",
        ),
        (
            "file_read",
            "Read file contents. Use when: inspecting project files, configs, logs.",
        ),
        (
            "file_write",
            "Write file contents. Use when: applying focused edits, scaffolding files.",
        ),
        (
            "memory_store",
            "Save to memory. Use when: preserving durable preferences, decisions, key context.",
        ),
        (
            "memory_recall",
            "Search memory. Use when: retrieving prior decisions, user preferences.",
        ),
        (
            "memory_forget",
            "Delete a memory entry. Use when: memory is incorrect/stale.",
        ),
        (
            "codecoder",
            "Invoke CodeCoder AI agents. Use for: web search, code review, security analysis, architecture, etc. Pass the full user request as prompt.",
        ),
    ];

    // Build base system prompt
    let mut system_prompt = build_system_prompt(&workspace, &model, &tool_descs, &skills, config.codecoder.enabled);

    // Add tool calling instructions
    system_prompt.push_str("\n## Tool Calling Format\n\n");
    system_prompt.push_str("To use a tool, output a JSON block:\n\n");
    system_prompt.push_str("```json\n");
    system_prompt.push_str("{\"tool\": \"tool_name\", \"args\": {\"param\": \"value\"}}\n");
    system_prompt.push_str("```\n\n");
    system_prompt.push_str("After tool execution, you'll receive results. Continue using tools or provide a final text response.\n\n");
    system_prompt.push_str("### Tool Parameter Schemas:\n\n");
    for tool in &tool_instances {
        let _ = std::fmt::Write::write_fmt(
            &mut system_prompt,
            format_args!("**{}**: {}\n", tool.name(), tool.description()),
        );
        let _ = std::fmt::Write::write_fmt(
            &mut system_prompt,
            format_args!("Parameters: `{}`\n\n", tool.parameters_schema()),
        );
    }

    // Create the agent executor
    let executor = Arc::new(AgentExecutor::new(
        provider.clone(),
        tool_instances,
        system_prompt.clone(),
        model.clone(),
        temperature,
    ));

    if !skills.is_empty() {
        println!(
            "  🧩 Skills:   {}",
            skills
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Collect active channels
    let mut channels: Vec<Arc<dyn Channel>> = Vec::new();
    // Keep a separate reference to Telegram channel for inline keyboard operations
    let mut telegram_channel_ref: Option<Arc<TelegramChannel>> = None;
    // Callback channel for Telegram inline button confirmations
    let mut telegram_callback_rx: Option<tokio::sync::mpsc::Receiver<CallbackQuery>> = None;

    if let Some(ref tg) = config.channels_config.telegram {
        // Create callback channel for inline button clicks
        let (callback_tx, callback_rx) = tokio::sync::mpsc::channel::<CallbackQuery>(100);
        telegram_callback_rx = Some(callback_rx);

        // Create STT client if voice transcription is enabled
        let mut telegram_channel =
            if let Some(ref voice) = tg.voice {
                if voice.enabled {
                    // Get API key: prefer voice-specific key, then fall back to main key
                    let stt_api_key = voice
                        .stt_api_key
                        .as_deref()
                        .or(config.api_key.as_deref());

                    if let Some(key) = stt_api_key {
                        match crate::stt::create_stt(
                            &voice.stt_provider,
                            key,
                            voice.stt_model.as_deref(),
                            voice.stt_base_url.as_deref(),
                        ) {
                            Ok(stt) => {
                                tracing::info!(
                                    "Telegram voice transcription enabled (provider: {})",
                                    voice.stt_provider
                                );
                                TelegramChannel::with_stt(
                                    tg.bot_token.clone(),
                                    tg.allowed_users.clone(),
                                    stt,
                                )
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to create STT client, voice disabled: {e}"
                                );
                                TelegramChannel::new(
                                    tg.bot_token.clone(),
                                    tg.allowed_users.clone(),
                                )
                            }
                        }
                    } else {
                        tracing::warn!(
                            "Voice transcription enabled but no API key configured, voice disabled"
                        );
                        TelegramChannel::new(
                            tg.bot_token.clone(),
                            tg.allowed_users.clone(),
                        )
                    }
                } else {
                    TelegramChannel::new(
                        tg.bot_token.clone(),
                        tg.allowed_users.clone(),
                    )
                }
            } else {
                TelegramChannel::new(
                    tg.bot_token.clone(),
                    tg.allowed_users.clone(),
                )
            };

        // Set callback sender BEFORE wrapping in Arc
        telegram_channel.set_callback_sender(callback_tx);

        let telegram_arc = Arc::new(telegram_channel);
        telegram_channel_ref = Some(telegram_arc.clone());
        channels.push(telegram_arc);
    }

    if let Some(ref dc) = config.channels_config.discord {
        channels.push(Arc::new(DiscordChannel::new(
            dc.bot_token.clone(),
            dc.guild_id.clone(),
            dc.allowed_users.clone(),
        )));
    }

    if let Some(ref sl) = config.channels_config.slack {
        channels.push(Arc::new(SlackChannel::new(
            sl.bot_token.clone(),
            sl.channel_id.clone(),
            sl.allowed_users.clone(),
        )));
    }

    if let Some(ref im) = config.channels_config.imessage {
        channels.push(Arc::new(IMessageChannel::new(im.allowed_contacts.clone())));
    }

    if let Some(ref mx) = config.channels_config.matrix {
        channels.push(Arc::new(MatrixChannel::new(
            mx.homeserver.clone(),
            mx.access_token.clone(),
            mx.room_id.clone(),
            mx.allowed_users.clone(),
        )));
    }

    if let Some(ref wa) = config.channels_config.whatsapp {
        channels.push(Arc::new(WhatsAppChannel::new(
            wa.access_token.clone(),
            wa.phone_number_id.clone(),
            wa.verify_token.clone(),
            wa.allowed_numbers.clone(),
        )));
    }

    if let Some(ref fs) = config.channels_config.feishu {
        channels.push(Arc::new(FeishuChannel::with_encryption(
            fs.app_id.clone(),
            fs.app_secret.clone(),
            fs.encrypt_key.clone(),
            fs.verification_token.clone(),
            fs.allowed_users.clone(),
        )));
    }

    if channels.is_empty() {
        println!("No channels configured. Run `zero-bot onboard` to set up channels.");
        return Ok(());
    }

    // Initialize the confirmation registry for interactive approvals
    confirmation::init_confirmation_registry().await;

    // Register notification sink for confirmation messages
    let notification_sink = Arc::new(ChannelNotificationSink::new());
    for ch in &channels {
        notification_sink.register_channel(ch.name(), ch.clone()).await;
    }
    // Register Telegram channel separately for inline keyboard operations
    if let Some(ref tg) = telegram_channel_ref {
        notification_sink.register_telegram_channel(tg.clone()).await;
    }
    confirmation::set_notification_sink(notification_sink).await;

    println!("🐦‍🔥 ZeroBot Channel Server");
    println!("  🤖 Model:    {model}");
    println!(
        "  🧠 Memory:   {} (auto-save: {})",
        config.memory.backend,
        if config.memory.auto_save { "on" } else { "off" }
    );
    println!(
        "  📡 Channels: {}",
        channels
            .iter()
            .map(|c| c.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    println!("  Listening for messages... (Ctrl+C to stop)");
    println!();

    crate::health::mark_component_ok("channels");

    let initial_backoff_secs = config
        .reliability
        .channel_initial_backoff_secs
        .max(DEFAULT_CHANNEL_INITIAL_BACKOFF_SECS);
    let max_backoff_secs = config
        .reliability
        .channel_max_backoff_secs
        .max(DEFAULT_CHANNEL_MAX_BACKOFF_SECS);

    // Single message bus — all channels send messages here
    let (tx, mut rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(100);

    // Spawn a listener for each channel
    let mut handles = Vec::new();
    for ch in &channels {
        handles.push(spawn_supervised_listener(
            ch.clone(),
            tx.clone(),
            initial_backoff_secs,
            max_backoff_secs,
        ));
    }

    // Spawn callback handler for Telegram inline button confirmations
    // (callbacks are received by the main listener and sent to callback_rx)
    if let (Some(tg), Some(callback_rx)) = (telegram_channel_ref.clone(), telegram_callback_rx) {
        handles.push(tokio::spawn(async move {
            handle_telegram_callbacks(tg, callback_rx).await;
        }));
        tracing::info!("Telegram callback query handler started for interactive confirmations");
    }

    drop(tx); // Drop our copy so rx closes when all channels stop

    // Process incoming messages — use the agent executor
    while let Some(msg) = rx.recv().await {
        let session_key = format!("{}:{}", msg.channel, msg.sender);

        println!(
            "  💬 [{}] from {}: {}",
            msg.channel,
            msg.sender,
            truncate_with_ellipsis(&msg.content, 80)
        );

        // Handle session commands (/new, /compact)
        if let Some(ref store) = session_store {
            let content_trimmed = msg.content.trim();

            // /new - Reset session
            if content_trimmed == "/new" {
                match store.clear_session(&session_key) {
                    Ok(deleted) => {
                        let reply = format!("🆕 会话已重置，开始新对话。（已清除 {deleted} 条消息）");
                        for ch in &channels {
                            if ch.name() == msg.channel {
                                let _ = ch.send(&reply, &msg.sender).await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to clear session: {e}");
                    }
                }
                continue;
            }

            // /compact - Manual compaction
            if content_trimmed == "/compact" {
                if let Some(ref compactor) = session_compactor {
                    let messages = store.get_messages(&session_key).unwrap_or_default();
                    if messages.is_empty() {
                        for ch in &channels {
                            if ch.name() == msg.channel {
                                let _ = ch.send("📭 当前会话为空，无需压缩。", &msg.sender).await;
                                break;
                            }
                        }
                    } else {
                        match compactor.compact(&messages).await {
                            Ok(summary) => {
                                if let Err(e) = store.compact_session(&session_key, &summary, session_config.keep_recent) {
                                    tracing::error!("Failed to compact session: {e}");
                                } else {
                                    for ch in &channels {
                                        if ch.name() == msg.channel {
                                            let _ = ch.send("📦 上下文已压缩。", &msg.sender).await;
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to generate summary: {e}");
                                for ch in &channels {
                                    if ch.name() == msg.channel {
                                        let _ = ch.send(&format!("⚠️ 压缩失败: {e}"), &msg.sender).await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                continue;
            }
        }

        // Auto-compaction check
        if let (Some(ref store), Some(ref compactor)) = (&session_store, &session_compactor) {
            let token_count = store.get_token_count(&session_key).unwrap_or(0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
            let threshold = (session_config.context_window as f32 * session_config.compact_threshold) as usize;

            if token_count > threshold {
                tracing::info!(
                    "Session {} exceeds threshold ({} > {}), auto-compacting",
                    session_key, token_count, threshold
                );

                let messages = store.get_messages(&session_key).unwrap_or_default();
                if !messages.is_empty() {
                    if let Ok(summary) = compactor.compact(&messages).await {
                        if let Err(e) = store.compact_session(&session_key, &summary, session_config.keep_recent) {
                            tracing::error!("Auto-compact failed: {e}");
                        } else {
                            for ch in &channels {
                                if ch.name() == msg.channel {
                                    let _ = ch.send("📦 上下文已自动压缩以保持响应质量。", &msg.sender).await;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build enriched message with session context
        let enriched_content = if let Some(ref store) = session_store {
            let history = store.get_messages(&session_key).unwrap_or_default();
            if history.is_empty() {
                msg.content.clone()
            } else {
                let context = format_session_context(&history);
                format!("{context}\n[当前消息]\n{}", msg.content)
            }
        } else {
            msg.content.clone()
        };

        // Save user message to session
        if let Some(ref store) = session_store {
            if let Err(e) = store.add_message(&session_key, MessageRole::User, &msg.content) {
                tracing::warn!("Failed to save user message to session: {e}");
            }
        }

        // Auto-save to memory
        if config.memory.auto_save {
            let _ = mem
                .store(
                    &format!("{}_{}", msg.channel, msg.sender),
                    &msg.content,
                    crate::memory::MemoryCategory::Conversation,
                )
                .await;
        }

        // Create tool context with channel info
        let tool_context = crate::agent::ToolContext::new(&msg.channel, &msg.sender);

        // Use the agent executor for tool-calling loop with context
        match executor.execute_with_context(&enriched_content, Some(tool_context)).await {
            Ok(response) => {
                // Save assistant response to session
                if let Some(ref store) = session_store {
                    if let Err(e) = store.add_message(&session_key, MessageRole::Assistant, &response) {
                        tracing::warn!("Failed to save assistant message to session: {e}");
                    }
                }

                println!(
                    "  🤖 Reply: {}",
                    truncate_with_ellipsis(&response, 200)
                );
                // Find the channel that sent this message and reply
                for ch in &channels {
                    if ch.name() == msg.channel {
                        if let Err(e) = ch.send(&response, &msg.sender).await {
                            eprintln!("  ❌ Failed to reply on {}: {e}", ch.name());
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("  ❌ Agent error: {e}");
                for ch in &channels {
                    if ch.name() == msg.channel {
                        let _ = ch.send(&format!("⚠️ Error: {e}"), &msg.sender).await;
                        break;
                    }
                }
            }
        }
    }

    // Wait for all channel tasks
    for h in handles {
        let _ = h.await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        // Create minimal workspace files
        std::fs::write(tmp.path().join("SOUL.md"), "# Soul\nBe helpful.").unwrap();
        std::fs::write(tmp.path().join("IDENTITY.md"), "# Identity\nName: ZeroBot").unwrap();
        std::fs::write(tmp.path().join("USER.md"), "# User\nName: Test User").unwrap();
        std::fs::write(
            tmp.path().join("AGENTS.md"),
            "# Agents\nFollow instructions.",
        )
        .unwrap();
        std::fs::write(tmp.path().join("TOOLS.md"), "# Tools\nUse shell carefully.").unwrap();
        std::fs::write(
            tmp.path().join("HEARTBEAT.md"),
            "# Heartbeat\nCheck status.",
        )
        .unwrap();
        std::fs::write(tmp.path().join("MEMORY.md"), "# Memory\nUser likes Rust.").unwrap();
        tmp
    }

    #[test]
    fn prompt_contains_all_sections() {
        let ws = make_workspace();
        let tools = vec![("shell", "Run commands"), ("file_read", "Read files")];
        let prompt = build_system_prompt(ws.path(), "test-model", &tools, &[], false);

        // Section headers
        assert!(prompt.contains("## Tools"), "missing Tools section");
        assert!(prompt.contains("## Safety"), "missing Safety section");
        assert!(prompt.contains("## Workspace"), "missing Workspace section");
        assert!(
            prompt.contains("## Project Context"),
            "missing Project Context"
        );
        assert!(
            prompt.contains("## Current Date & Time"),
            "missing Date/Time"
        );
        assert!(prompt.contains("## Runtime"), "missing Runtime section");
    }

    #[test]
    fn prompt_injects_tools() {
        let ws = make_workspace();
        let tools = vec![
            ("shell", "Run commands"),
            ("memory_recall", "Search memory"),
        ];
        let prompt = build_system_prompt(ws.path(), "gpt-4o", &tools, &[], false);

        assert!(prompt.contains("**shell**"));
        assert!(prompt.contains("Run commands"));
        assert!(prompt.contains("**memory_recall**"));
    }

    #[test]
    fn prompt_injects_safety() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        assert!(prompt.contains("Do not exfiltrate private data"));
        assert!(prompt.contains("Do not run destructive commands"));
        assert!(prompt.contains("Prefer `trash` over `rm`"));
    }

    #[test]
    fn prompt_injects_workspace_files() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        assert!(prompt.contains("### SOUL.md"), "missing SOUL.md header");
        assert!(prompt.contains("Be helpful"), "missing SOUL content");
        assert!(prompt.contains("### IDENTITY.md"), "missing IDENTITY.md");
        assert!(
            prompt.contains("Name: ZeroBot"),
            "missing IDENTITY content"
        );
        assert!(prompt.contains("### USER.md"), "missing USER.md");
        assert!(prompt.contains("### AGENTS.md"), "missing AGENTS.md");
        assert!(prompt.contains("### TOOLS.md"), "missing TOOLS.md");
        assert!(prompt.contains("### HEARTBEAT.md"), "missing HEARTBEAT.md");
        assert!(prompt.contains("### MEMORY.md"), "missing MEMORY.md");
        assert!(prompt.contains("User likes Rust"), "missing MEMORY content");
    }

    #[test]
    fn prompt_missing_file_markers() {
        let tmp = TempDir::new().unwrap();
        // Empty workspace — no files at all
        let prompt = build_system_prompt(tmp.path(), "model", &[], &[], false);

        assert!(prompt.contains("[File not found: SOUL.md]"));
        assert!(prompt.contains("[File not found: AGENTS.md]"));
        assert!(prompt.contains("[File not found: IDENTITY.md]"));
    }

    #[test]
    fn prompt_bootstrap_only_if_exists() {
        let ws = make_workspace();
        // No BOOTSTRAP.md — should not appear
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);
        assert!(
            !prompt.contains("### BOOTSTRAP.md"),
            "BOOTSTRAP.md should not appear when missing"
        );

        // Create BOOTSTRAP.md — should appear
        std::fs::write(ws.path().join("BOOTSTRAP.md"), "# Bootstrap\nFirst run.").unwrap();
        let prompt2 = build_system_prompt(ws.path(), "model", &[], &[], false);
        assert!(
            prompt2.contains("### BOOTSTRAP.md"),
            "BOOTSTRAP.md should appear when present"
        );
        assert!(prompt2.contains("First run"));
    }

    #[test]
    fn prompt_no_daily_memory_injection() {
        let ws = make_workspace();
        let memory_dir = ws.path().join("memory");
        std::fs::create_dir_all(&memory_dir).unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        std::fs::write(
            memory_dir.join(format!("{today}.md")),
            "# Daily\nSome note.",
        )
        .unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        // Daily notes should NOT be in the system prompt (on-demand via tools)
        assert!(
            !prompt.contains("Daily Notes"),
            "daily notes should not be auto-injected"
        );
        assert!(
            !prompt.contains("Some note"),
            "daily content should not be in prompt"
        );
    }

    #[test]
    fn prompt_runtime_metadata() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "claude-sonnet-4", &[], &[], false);

        assert!(prompt.contains("Model: claude-sonnet-4"));
        assert!(prompt.contains(&format!("OS: {}", std::env::consts::OS)));
        assert!(prompt.contains("Host:"));
    }

    #[test]
    fn prompt_skills_compact_list() {
        let ws = make_workspace();
        let skills = vec![crate::skills::Skill {
            name: "code-review".into(),
            description: "Review code for bugs".into(),
            version: "1.0.0".into(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec!["Long prompt content that should NOT appear in system prompt".into()],
            location: None,
        }];

        let prompt = build_system_prompt(ws.path(), "model", &[], &skills, false);

        assert!(prompt.contains("<available_skills>"), "missing skills XML");
        assert!(prompt.contains("<name>code-review</name>"));
        assert!(prompt.contains("<description>Review code for bugs</description>"));
        assert!(prompt.contains("SKILL.md</location>"));
        assert!(
            prompt.contains("loaded on demand"),
            "should mention on-demand loading"
        );
        // Full prompt content should NOT be dumped
        assert!(!prompt.contains("Long prompt content that should NOT appear"));
    }

    #[test]
    fn prompt_truncation() {
        let ws = make_workspace();
        // Write a file larger than BOOTSTRAP_MAX_CHARS
        let big_content = "x".repeat(BOOTSTRAP_MAX_CHARS + 1000);
        std::fs::write(ws.path().join("AGENTS.md"), &big_content).unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        assert!(
            prompt.contains("truncated at"),
            "large files should be truncated"
        );
        assert!(
            !prompt.contains(&big_content),
            "full content should not appear"
        );
    }

    #[test]
    fn prompt_empty_files_skipped() {
        let ws = make_workspace();
        std::fs::write(ws.path().join("TOOLS.md"), "").unwrap();

        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        // Empty file should not produce a header
        assert!(
            !prompt.contains("### TOOLS.md"),
            "empty files should be skipped"
        );
    }

    #[test]
    fn prompt_workspace_path() {
        let ws = make_workspace();
        let prompt = build_system_prompt(ws.path(), "model", &[], &[], false);

        assert!(prompt.contains(&format!("Working directory: `{}`", ws.path().display())));
    }

    #[test]
    fn classify_health_ok_true() {
        let state = classify_health_result(&Ok(true));
        assert_eq!(state, ChannelHealthState::Healthy);
    }

    #[test]
    fn classify_health_ok_false() {
        let state = classify_health_result(&Ok(false));
        assert_eq!(state, ChannelHealthState::Unhealthy);
    }

    #[tokio::test]
    async fn classify_health_timeout() {
        let result = tokio::time::timeout(Duration::from_millis(1), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            true
        })
        .await;
        let state = classify_health_result(&result);
        assert_eq!(state, ChannelHealthState::Timeout);
    }

    struct AlwaysFailChannel {
        name: &'static str,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Channel for AlwaysFailChannel {
        fn name(&self) -> &str {
            self.name
        }

        async fn send(&self, _message: &str, _recipient: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
        ) -> anyhow::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("listen boom")
        }
    }

    #[tokio::test]
    async fn supervised_listener_marks_error_and_restarts_on_failures() {
        let calls = Arc::new(AtomicUsize::new(0));
        let channel: Arc<dyn Channel> = Arc::new(AlwaysFailChannel {
            name: "test-supervised-fail",
            calls: Arc::clone(&calls),
        });

        let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(1);
        let handle = spawn_supervised_listener(channel, tx, 1, 1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        drop(rx);
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["channel:test-supervised-fail"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("listen boom"));
        assert!(calls.load(Ordering::SeqCst) >= 1);
    }
}
