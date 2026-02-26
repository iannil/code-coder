#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::unnecessary_literal_bound,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    dead_code
)]

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::{info, Level};

mod agent;
mod channels;
mod config;
mod credential;
mod cron;
mod daemon;
mod doctor;
mod health;
mod heartbeat;
mod integrations;
mod mcp;
mod memory;
mod migration;
mod observability;
mod onboard;
mod process;
mod providers;
mod runtime;
mod sandbox;
mod security;
mod service;
mod session;
mod skills;
mod tools;
mod trading;
mod tunnel;
mod util;

use config::Config;

/// `ZeroCLI` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "zero-cli")]
#[command(author = "theonlyhennygod")]
#[command(version = "0.1.0")]
#[command(about = "The fastest, smallest AI assistant CLI.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum ServiceCommands {
    /// Install daemon service unit for auto-start and restart
    Install,
    /// Start daemon service
    Start,
    /// Stop daemon service
    Stop,
    /// Check daemon service status
    Status,
    /// Uninstall daemon service unit
    Uninstall,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,

        /// Memory backend (sqlite, markdown, none) - used in quick mode, default: sqlite
        #[arg(long)]
        memory: Option<String>,
    },

    /// Start the AI agent loop
    Agent {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,
    },

    /// Start daemon (process orchestrator for gateway, channels, workflow)
    Daemon {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Gateway port (zero-gateway service)
        #[arg(long, default_value = "4430")]
        gateway_port: u16,

        /// Channels port (zero-channels service)
        #[arg(long, default_value = "4431")]
        channels_port: u16,

        /// Workflow port (zero-workflow service)
        #[arg(long, default_value = "4432")]
        workflow_port: u16,

        /// Trading port (zero-trading service)
        #[arg(long, default_value = "4434")]
        trading_port: u16,

        /// Directory for child service logs (default: ../.logs relative to working dir)
        #[arg(long)]
        log_dir: Option<std::path::PathBuf>,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    Service {
        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    Doctor,

    /// Show system status (full details)
    Status,

    /// Configure and manage scheduled tasks
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage channels (telegram, discord, slack)
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    Skills {
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// Manage credentials (API keys, OAuth, login)
    Credential {
        #[command(subcommand)]
        credential_command: CredentialCommands,
    },

    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Start MCP server to expose `Zero CLI` tools via Model Context Protocol
    #[command(name = "mcp-server")]
    McpServer {
        /// Run in stdio mode (for subprocess communication)
        #[arg(long)]
        stdio: bool,
    },

    /// Manage automated trading (PO3+SMT strategy)
    Trading {
        #[command(subcommand)]
        trading_command: trading::TradingCommands,
    },
}

#[derive(Subcommand, Debug)]
enum MigrateCommands {
    /// Import memory from an `OpenClaw` workspace into this `Zero CLI` workspace
    Openclaw {
        /// Optional path to `OpenClaw` workspace (defaults to ~/.openclaw/workspace)
        #[arg(long)]
        source: Option<std::path::PathBuf>,

        /// Validate and preview migration without writing any data
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CronCommands {
    /// List all scheduled tasks
    List,
    /// Add a new scheduled task
    Add {
        /// Cron expression
        expression: String,
        /// Command to run
        command: String,
    },
    /// Remove a scheduled task
    Remove {
        /// Task ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ChannelCommands {
    /// List configured channels
    List,
    /// Start all configured channels (Telegram, Discord, Slack)
    Start,
    /// Run health checks for configured channels
    Doctor,
    /// Add a new channel
    Add {
        /// Channel type
        channel_type: String,
        /// Configuration JSON
        config: String,
    },
    /// Remove a channel
    Remove {
        /// Channel name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum SkillCommands {
    /// List installed skills
    List,
    /// Install a skill from a GitHub URL or local path
    Install {
        /// GitHub URL or local path
        source: String,
    },
    /// Remove an installed skill
    Remove {
        /// Skill name
        name: String,
    },
    /// Search for skills in `SkillHub` registry
    Search {
        /// Search query
        query: String,
        /// Limit results (default: 10)
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Update installed skills
    Update {
        /// Skill name (omit to check all)
        name: Option<String>,
    },
    /// Show detailed information about a skill
    Info {
        /// Skill name
        name: String,
    },
    /// Publish a skill to `SkillHub` (requires GitHub account)
    Publish {
        /// Path to skill directory
        path: std::path::PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum IntegrationCommands {
    /// Show details about a specific integration
    Info {
        /// Integration name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum CredentialCommands {
    /// List all credentials (without sensitive data)
    List,
    /// Add a new credential
    Add {
        /// Credential type: `api_key`, oauth, login, `bearer_token`
        #[arg(short = 't', long)]
        credential_type: String,
        /// Service name (e.g., github, openai, google)
        #[arg(short, long)]
        service: String,
        /// Human-readable name for the credential
        #[arg(short, long)]
        name: Option<String>,
        /// API key or bearer token (for `api_key`/`bearer_token` types)
        #[arg(short, long)]
        key: Option<String>,
        /// Username (for login type)
        #[arg(long)]
        username: Option<String>,
        /// Password (for login type) - will prompt if not provided
        #[arg(long)]
        password: Option<String>,
        /// OAuth client ID (for oauth type)
        #[arg(long)]
        client_id: Option<String>,
        /// OAuth client secret (for oauth type)
        #[arg(long)]
        client_secret: Option<String>,
        /// URL patterns this credential applies to (comma-separated)
        #[arg(short, long)]
        patterns: Option<String>,
    },
    /// Remove a credential by ID
    Remove {
        /// Credential ID
        id: String,
    },
    /// Show details of a specific credential
    Show {
        /// Credential ID or service name
        id: String,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging with JSON format for structured tracing
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .json()
        .with_current_span(true)
        .with_target(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Onboard runs quick setup by default, or the interactive wizard with --interactive
    if let Commands::Onboard {
        interactive,
        channels_only,
        api_key,
        provider,
        memory,
    } = &cli.command
    {
        if *interactive && *channels_only {
            bail!("Use either --interactive or --channels-only, not both");
        }
        if *channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
            bail!("--channels-only does not accept --api-key, --provider, or --memory");
        }

        let config = if *channels_only {
            onboard::run_channels_repair_wizard()?
        } else if *interactive {
            onboard::run_wizard()?
        } else {
            onboard::run_quick_setup(api_key.as_deref(), provider.as_deref(), memory.as_deref())?
        };
        // Auto-start channels if user said yes during wizard
        if std::env::var("ZERO_CLI_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
            channels::start_channels(config).await?;
        }
        return Ok(());
    }

    // All other commands need config loaded first
    let config = Config::load_or_init()?;

    match cli.command {
        Commands::Onboard { .. } => unreachable!(),

        Commands::Agent {
            message,
            provider,
            model,
            temperature,
        } => agent::run(config, message, provider, model, temperature).await,

        Commands::Daemon {
            host,
            gateway_port,
            channels_port,
            workflow_port,
            trading_port,
            log_dir,
        } => {
            info!("🧠 Starting Zero CLI Daemon (process orchestrator)");
            info!("   Management API: http://{host}:4402");
            info!("   Gateway:  http://{host}:{gateway_port}");
            info!("   Channels: http://{host}:{channels_port}");
            info!("   Workflow: http://{host}:{workflow_port}");
            info!("   Trading:  http://{host}:{trading_port}");
            daemon::run_orchestrator(config, host, gateway_port, channels_port, workflow_port, trading_port, log_dir).await
        }

        Commands::Status => {
            println!("🐦‍🔥 Zero CLI Status");
            println!();
            println!("Version:     {}", env!("CARGO_PKG_VERSION"));
            println!("Workspace:   {}", config.workspace_dir.display());
            println!("Config:      {}", config.config_path.display());
            println!();
            println!(
                "🤖 Provider:      {}",
                config.default_provider.as_deref().unwrap_or("openrouter")
            );
            println!(
                "   Model:         {}",
                config.default_model.as_deref().unwrap_or("(default)")
            );
            println!("📊 Observability:  {}", config.observability.backend);
            println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
            println!("⚙️  Runtime:       {}", config.runtime.kind);
            println!(
                "💓 Heartbeat:      {}",
                if config.heartbeat.enabled {
                    format!("every {}min", config.heartbeat.interval_minutes)
                } else {
                    "disabled".into()
                }
            );
            println!(
                "🧠 Memory:         {} (auto-save: {})",
                config.memory.backend,
                if config.memory.auto_save { "on" } else { "off" }
            );

            println!();
            println!("Security:");
            println!("  Workspace only:    {}", config.autonomy.workspace_only);
            println!(
                "  Allowed commands:  {}",
                config.autonomy.allowed_commands.join(", ")
            );
            println!(
                "  Max actions/hour:  {}",
                config.autonomy.max_actions_per_hour
            );
            println!(
                "  Max cost/day:      ${:.2}",
                f64::from(config.autonomy.max_cost_per_day_cents) / 100.0
            );
            println!();
            println!("Channels:");
            println!("  CLI:      ✅ always");
            for (name, configured) in [
                ("Telegram", config.channels_config.telegram.is_some()),
                ("Discord", config.channels_config.discord.is_some()),
                ("Slack", config.channels_config.slack.is_some()),
                ("Webhook", config.channels_config.webhook.is_some()),
            ] {
                println!(
                    "  {name:9} {}",
                    if configured {
                        "✅ configured"
                    } else {
                        "❌ not configured"
                    }
                );
            }

            Ok(())
        }

        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        Commands::Service { service_command } => service::handle_command(&service_command, &config),

        Commands::Doctor => doctor::run(&config),

        Commands::Channel { channel_command } => match channel_command {
            ChannelCommands::Start => channels::start_channels(config).await,
            ChannelCommands::Doctor => channels::doctor_channels(config).await,
            other => channels::handle_command(other, &config),
        },

        Commands::Integrations {
            integration_command,
        } => integrations::handle_command(integration_command, &config),

        Commands::Skills { skill_command } => {
            skills::handle_command(skill_command, &config.workspace_dir)
        }

        Commands::Credential { credential_command } => {
            credential::handle_command(credential_command, &config)
        }

        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command, &config).await
        }

        Commands::McpServer { stdio } => {
            use std::sync::Arc;

            // Build tools for MCP server
            let security = Arc::new(security::SecurityPolicy::from_config(
                &config.autonomy,
                &config.workspace_dir,
            ));

            let mem: Arc<dyn memory::Memory> = Arc::from(memory::create_memory(
                &config.memory,
                &config.workspace_dir,
                config.api_key.as_deref(),
            )?);

            let tools_vec = tools::all_tools(
                &security,
                mem,
                &config.browser,
                &config.codecoder,
                &config.vault,
                &config.workspace_dir,
            );

            // Convert to Arc<dyn Tool>
            let tools: Vec<Arc<dyn tools::Tool>> = tools_vec
                .into_iter()
                .map(|t| Arc::from(t) as Arc<dyn tools::Tool>)
                .collect();

            let server = mcp::McpServer::new(tools);

            if stdio {
                info!("Starting MCP server in stdio mode");
                server.serve_stdio().await
            } else {
                // HTTP mode - use the gateway port
                let port = if config.mcp.server_api_key.is_some() { config.gateway.port } else { 8081 };

                info!("Starting MCP server on http://127.0.0.1:{port}/mcp");
                let app = Arc::new(server).routes();

                let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse()?;
                let listener = tokio::net::TcpListener::bind(addr).await?;
                axum::serve(listener, app).await?;
                Ok(())
            }
        }

        Commands::Trading { trading_command } => {
            trading::handle_command(trading_command, &config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_has_no_flag_conflicts() {
        Cli::command().debug_assert();
    }
}
