//! Configuration management for Zero services.
//!
//! All Zero services share a unified configuration file at `~/.codecoder/config.json`.
//!
//! # Configuration Priority
//!
//! 1. Explicit config file values
//! 2. Environment variables (ZERO_* prefix)
//! 3. Default values
//!
//! # Environment Variable Mapping
//!
//! - `ZERO_GATEWAY_PORT` → gateway.port
//! - `ZERO_JWT_SECRET` → gateway.jwt_secret
//! - `ANTHROPIC_API_KEY` → api_keys.anthropic
//! - `OPENAI_API_KEY` → api_keys.openai
//! - `GOOGLE_API_KEY` → api_keys.google
//! - `DEEPSEEK_API_KEY` → api_keys.deepseek
//! - etc.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Get the configuration directory path.
pub fn config_dir() -> PathBuf {
    directories::UserDirs::new()
        .map_or_else(
            || PathBuf::from(".codecoder"),
            |dirs| dirs.home_dir().join(".codecoder"),
        )
}

/// Get the configuration file path.
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Root configuration structure for all Zero services.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Gateway configuration
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Channels configuration
    #[serde(default)]
    pub channels: ChannelsConfig,

    /// Workflow configuration
    #[serde(default)]
    pub workflow: WorkflowConfig,

    /// `CodeCoder` integration
    #[serde(default)]
    pub codecoder: CodeCoderConfig,

    /// Observability configuration
    #[serde(default)]
    pub observability: ObservabilityConfig,

    /// Memory/persistence configuration
    #[serde(default)]
    pub memory: MemoryConfig,

    /// API keys for LLM providers
    #[serde(default)]
    pub api_keys: ApiKeysConfig,

    /// Provider configuration (routing, fallbacks)
    #[serde(default)]
    pub providers: ProvidersConfig,

    /// Agent execution configuration
    #[serde(default)]
    pub agent: AgentConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Config {
    /// Load configuration from the default path.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            tracing::info!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    /// Load configuration with environment variable fallbacks.
    pub fn load_with_env() -> Result<Self> {
        let mut config = Self::load()?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply environment variable overrides to the configuration.
    pub fn apply_env_overrides(&mut self) {
        // Gateway overrides
        if let Ok(port) = std::env::var("ZERO_GATEWAY_PORT") {
            if let Ok(p) = port.parse() {
                self.gateway.port = p;
            }
        }
        if let Ok(host) = std::env::var("ZERO_GATEWAY_HOST") {
            self.gateway.host = host;
        }
        if let Ok(secret) = std::env::var("ZERO_JWT_SECRET") {
            self.gateway.jwt_secret = Some(secret);
        }

        // Channels overrides
        if let Ok(port) = std::env::var("ZERO_CHANNELS_PORT") {
            if let Ok(p) = port.parse() {
                self.channels.port = p;
            }
        }

        // CodeCoder overrides
        if let Ok(endpoint) = std::env::var("CODECODER_ENDPOINT") {
            self.codecoder.endpoint = endpoint;
        }

        // Log level override
        if let Ok(level) = std::env::var("ZERO_LOG_LEVEL") {
            self.observability.log_level = level;
        }

        // Apply API key env fallbacks
        self.api_keys = self.api_keys.with_env_fallback();
    }

    /// Save configuration to the default path.
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        let dir = config_dir();

        if !dir.exists() {
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create config directory {}", dir.display()))?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))
    }
}

/// Gateway service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway HTTP port
    #[serde(default = "default_gateway_port")]
    pub port: u16,

    /// Gateway HTTP host
    #[serde(default = "default_gateway_host")]
    pub host: String,

    /// JWT secret for token signing (auto-generated if not set)
    #[serde(default)]
    pub jwt_secret: Option<String>,

    /// Token expiry in seconds
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,

    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub rate_limiting: bool,

    /// Requests per minute per user
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,

    /// `CodeCoder` API endpoint to proxy to
    #[serde(default = "default_codecoder_endpoint")]
    pub codecoder_endpoint: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_gateway_port(),
            host: default_gateway_host(),
            jwt_secret: None,
            token_expiry_secs: default_token_expiry(),
            rate_limiting: true,
            rate_limit_rpm: default_rate_limit(),
            codecoder_endpoint: default_codecoder_endpoint(),
        }
    }
}

/// Channels service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    /// Channels HTTP port
    #[serde(default = "default_channels_port")]
    pub port: u16,

    /// Channels HTTP host
    #[serde(default = "default_channels_host")]
    pub host: String,

    /// Telegram bot configuration
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,

    /// Discord bot configuration
    #[serde(default)]
    pub discord: Option<DiscordConfig>,

    /// Slack bot configuration
    #[serde(default)]
    pub slack: Option<SlackConfig>,

    /// Feishu bot configuration
    #[serde(default)]
    pub feishu: Option<FeishuConfig>,

    /// Matrix bot configuration
    #[serde(default)]
    pub matrix: Option<MatrixConfig>,

    /// WhatsApp bot configuration
    #[serde(default)]
    pub whatsapp: Option<WhatsAppConfig>,

    /// iMessage configuration (macOS only)
    #[serde(default)]
    pub imessage: Option<IMessageConfig>,

    /// CLI channel configuration
    #[serde(default)]
    pub cli: Option<CliChannelConfig>,

    /// TTS configuration
    #[serde(default)]
    pub tts: Option<TtsConfig>,

    /// STT configuration
    #[serde(default)]
    pub stt: Option<SttConfig>,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            port: default_channels_port(),
            host: default_channels_host(),
            telegram: None,
            discord: None,
            slack: None,
            feishu: None,
            matrix: None,
            whatsapp: None,
            imessage: None,
            cli: None,
            tts: None,
            stt: None,
        }
    }
}

/// Telegram channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub allowed_chats: Vec<i64>,
}

/// Discord channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_guilds: Vec<String>,
    #[serde(default)]
    pub allowed_channels: Vec<String>,
}

/// Slack channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub app_token: String,
    #[serde(default)]
    pub signing_secret: Option<String>,
}

/// Feishu channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_secret: String,
    #[serde(default)]
    pub encrypt_key: Option<String>,
    #[serde(default)]
    pub verification_token: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// TTS (Text-to-Speech) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
}

/// STT (Speech-to-Text) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Matrix channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub enabled: bool,
    /// Matrix homeserver URL (e.g., "https://matrix.org")
    pub homeserver: String,
    /// Access token for the bot user
    pub access_token: String,
    /// Room ID to listen/send to (e.g., "!room:matrix.org")
    pub room_id: String,
    /// Allowed user IDs (e.g., "@user:matrix.org"). Use "*" to allow everyone.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// WhatsApp channel configuration (via WhatsApp Business API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub enabled: bool,
    /// WhatsApp Business API phone number ID
    pub phone_number_id: String,
    /// WhatsApp Business API access token
    pub access_token: String,
    /// Webhook verify token
    #[serde(default)]
    pub verify_token: Option<String>,
    /// Allowed phone numbers (E.164 format). Use "*" to allow everyone.
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

/// iMessage channel configuration (macOS only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    pub enabled: bool,
    /// Path to Messages.app database (defaults to ~/Library/Messages/chat.db)
    #[serde(default)]
    pub database_path: Option<String>,
    /// Allowed phone numbers or email addresses
    #[serde(default)]
    pub allowed_contacts: Vec<String>,
    /// Poll interval in seconds
    #[serde(default = "default_imessage_poll_interval")]
    pub poll_interval_secs: u64,
}

/// CLI channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliChannelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Enable color output
    #[serde(default = "default_true")]
    pub color: bool,
    /// Prompt string
    #[serde(default = "default_cli_prompt")]
    pub prompt: String,
}

/// Workflow service configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowConfig {
    /// Cron scheduler configuration
    #[serde(default)]
    pub cron: CronConfig,

    /// Webhook configuration
    #[serde(default)]
    pub webhook: WebhookConfig,

    /// Git integration configuration
    #[serde(default)]
    pub git: GitIntegrationConfig,
}

/// Cron scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronConfig {
    /// Enable cron scheduler
    #[serde(default)]
    pub enabled: bool,

    /// Scheduled tasks
    #[serde(default)]
    pub tasks: Vec<CronTask>,
}

/// A scheduled cron task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTask {
    /// Task ID
    pub id: String,
    /// Cron expression
    pub expression: String,
    /// Command or workflow to execute
    pub command: String,
    /// Task description
    #[serde(default)]
    pub description: Option<String>,
}

/// Webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    /// Enable webhook receiver
    #[serde(default)]
    pub enabled: bool,

    /// Webhook port (if separate from gateway)
    #[serde(default)]
    pub port: Option<u16>,

    /// Webhook secret for signature verification
    #[serde(default)]
    pub secret: Option<String>,
}

/// Git integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitIntegrationConfig {
    /// Enable Git webhook handling
    #[serde(default)]
    pub enabled: bool,

    /// GitHub webhook secret
    #[serde(default)]
    pub github_secret: Option<String>,

    /// GitLab webhook token
    #[serde(default)]
    pub gitlab_token: Option<String>,
}

/// `CodeCoder` integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCoderConfig {
    /// Enable `CodeCoder` integration
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// `CodeCoder` API endpoint
    #[serde(default = "default_codecoder_endpoint")]
    pub endpoint: String,

    /// API timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for CodeCoderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: default_codecoder_endpoint(),
            timeout_secs: default_timeout(),
        }
    }
}

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Log format (json, pretty)
    #[serde(default = "default_log_format")]
    pub log_format: String,

    /// Enable request tracing
    #[serde(default = "default_true")]
    pub tracing: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_format: default_log_format(),
            tracing: true,
        }
    }
}

/// Memory/persistence configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Backend type (sqlite, postgres)
    #[serde(default = "default_memory_backend")]
    pub backend: String,

    /// Database path (for `SQLite`)
    #[serde(default)]
    pub path: Option<String>,

    /// Connection string (for `PostgreSQL`)
    #[serde(default)]
    pub connection_string: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_memory_backend(),
            path: None,
            connection_string: None,
        }
    }
}

/// API keys for LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeysConfig {
    /// Anthropic API key (env: ANTHROPIC_API_KEY)
    #[serde(default)]
    pub anthropic: Option<String>,

    /// OpenAI API key (env: OPENAI_API_KEY)
    #[serde(default)]
    pub openai: Option<String>,

    /// Google AI (Gemini) API key (env: GOOGLE_API_KEY or GEMINI_API_KEY)
    #[serde(default)]
    pub google: Option<String>,

    /// DeepSeek API key (env: DEEPSEEK_API_KEY)
    #[serde(default)]
    pub deepseek: Option<String>,

    /// OpenRouter API key (env: OPENROUTER_API_KEY)
    #[serde(default)]
    pub openrouter: Option<String>,

    /// Groq API key (env: GROQ_API_KEY)
    #[serde(default)]
    pub groq: Option<String>,

    /// Mistral API key (env: MISTRAL_API_KEY)
    #[serde(default)]
    pub mistral: Option<String>,

    /// xAI (Grok) API key (env: XAI_API_KEY)
    #[serde(default)]
    pub xai: Option<String>,

    /// Together AI API key (env: TOGETHER_API_KEY)
    #[serde(default)]
    pub together: Option<String>,

    /// Fireworks AI API key (env: FIREWORKS_API_KEY)
    #[serde(default)]
    pub fireworks: Option<String>,

    /// Perplexity API key (env: PERPLEXITY_API_KEY)
    #[serde(default)]
    pub perplexity: Option<String>,

    /// Cohere API key (env: COHERE_API_KEY)
    #[serde(default)]
    pub cohere: Option<String>,

    /// Cloudflare AI API key (env: CLOUDFLARE_API_KEY)
    #[serde(default)]
    pub cloudflare: Option<String>,

    /// Venice AI API key (env: VENICE_API_KEY)
    #[serde(default)]
    pub venice: Option<String>,

    /// Moonshot (Kimi) API key (env: MOONSHOT_API_KEY)
    #[serde(default)]
    pub moonshot: Option<String>,

    /// GLM (Zhipu) API key (env: GLM_API_KEY)
    #[serde(default)]
    pub glm: Option<String>,

    /// MiniMax API key (env: MINIMAX_API_KEY)
    #[serde(default)]
    pub minimax: Option<String>,

    /// Qianfan (Baidu) API key (env: QIANFAN_API_KEY)
    #[serde(default)]
    pub qianfan: Option<String>,

    /// ElevenLabs API key for TTS (env: ELEVENLABS_API_KEY)
    #[serde(default)]
    pub elevenlabs: Option<String>,
}

impl ApiKeysConfig {
    /// Load API keys from environment variables, merging with config values.
    /// Environment variables take precedence.
    pub fn with_env_fallback(&self) -> Self {
        Self {
            anthropic: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| self.anthropic.clone()),
            openai: std::env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| self.openai.clone()),
            google: std::env::var("GOOGLE_API_KEY")
                .ok()
                .or_else(|| std::env::var("GEMINI_API_KEY").ok())
                .or_else(|| self.google.clone()),
            deepseek: std::env::var("DEEPSEEK_API_KEY")
                .ok()
                .or_else(|| self.deepseek.clone()),
            openrouter: std::env::var("OPENROUTER_API_KEY")
                .ok()
                .or_else(|| self.openrouter.clone()),
            groq: std::env::var("GROQ_API_KEY")
                .ok()
                .or_else(|| self.groq.clone()),
            mistral: std::env::var("MISTRAL_API_KEY")
                .ok()
                .or_else(|| self.mistral.clone()),
            xai: std::env::var("XAI_API_KEY")
                .ok()
                .or_else(|| self.xai.clone()),
            together: std::env::var("TOGETHER_API_KEY")
                .ok()
                .or_else(|| self.together.clone()),
            fireworks: std::env::var("FIREWORKS_API_KEY")
                .ok()
                .or_else(|| self.fireworks.clone()),
            perplexity: std::env::var("PERPLEXITY_API_KEY")
                .ok()
                .or_else(|| self.perplexity.clone()),
            cohere: std::env::var("COHERE_API_KEY")
                .ok()
                .or_else(|| self.cohere.clone()),
            cloudflare: std::env::var("CLOUDFLARE_API_KEY")
                .ok()
                .or_else(|| self.cloudflare.clone()),
            venice: std::env::var("VENICE_API_KEY")
                .ok()
                .or_else(|| self.venice.clone()),
            moonshot: std::env::var("MOONSHOT_API_KEY")
                .ok()
                .or_else(|| self.moonshot.clone()),
            glm: std::env::var("GLM_API_KEY")
                .ok()
                .or_else(|| self.glm.clone()),
            minimax: std::env::var("MINIMAX_API_KEY")
                .ok()
                .or_else(|| self.minimax.clone()),
            qianfan: std::env::var("QIANFAN_API_KEY")
                .ok()
                .or_else(|| self.qianfan.clone()),
            elevenlabs: std::env::var("ELEVENLABS_API_KEY")
                .ok()
                .or_else(|| self.elevenlabs.clone()),
        }
    }

    /// Get API key for a provider by name.
    pub fn get(&self, provider: &str) -> Option<&String> {
        match provider {
            "anthropic" => self.anthropic.as_ref(),
            "openai" => self.openai.as_ref(),
            "google" | "gemini" | "google-gemini" => self.google.as_ref(),
            "deepseek" => self.deepseek.as_ref(),
            "openrouter" => self.openrouter.as_ref(),
            "groq" => self.groq.as_ref(),
            "mistral" => self.mistral.as_ref(),
            "xai" | "grok" => self.xai.as_ref(),
            "together" | "together-ai" => self.together.as_ref(),
            "fireworks" | "fireworks-ai" => self.fireworks.as_ref(),
            "perplexity" => self.perplexity.as_ref(),
            "cohere" => self.cohere.as_ref(),
            "cloudflare" | "cloudflare-ai" => self.cloudflare.as_ref(),
            "venice" => self.venice.as_ref(),
            "moonshot" | "kimi" => self.moonshot.as_ref(),
            "glm" | "zhipu" => self.glm.as_ref(),
            "minimax" => self.minimax.as_ref(),
            "qianfan" | "baidu" => self.qianfan.as_ref(),
            "elevenlabs" => self.elevenlabs.as_ref(),
            _ => None,
        }
    }
}

/// Provider configuration for LLM routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Default provider name
    #[serde(default = "default_provider")]
    pub default: String,

    /// Default model for the default provider
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Ollama configuration (local models)
    #[serde(default)]
    pub ollama: OllamaConfig,

    /// Reliability configuration for retries and fallbacks
    #[serde(default)]
    pub reliability: ReliabilityConfig,

    /// Custom provider endpoints (for self-hosted or alternative APIs)
    #[serde(default)]
    pub custom_endpoints: HashMap<String, String>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            default: default_provider(),
            default_model: default_model(),
            ollama: OllamaConfig::default(),
            reliability: ReliabilityConfig::default(),
            custom_endpoints: HashMap::new(),
        }
    }
}

/// Ollama (local models) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama API base URL
    #[serde(default = "default_ollama_url")]
    pub base_url: String,

    /// Default model for Ollama
    #[serde(default = "default_ollama_model")]
    pub default_model: String,

    /// Request timeout in seconds (local models can be slow)
    #[serde(default = "default_ollama_timeout")]
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_ollama_url(),
            default_model: default_ollama_model(),
            timeout_secs: default_ollama_timeout(),
        }
    }
}

/// Reliability configuration for provider retries and fallbacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Number of retries before switching to fallback provider
    #[serde(default = "default_provider_retries")]
    pub provider_retries: u32,

    /// Backoff time between retries in milliseconds
    #[serde(default = "default_provider_backoff")]
    pub provider_backoff_ms: u64,

    /// Fallback provider chain (tried in order)
    #[serde(default)]
    pub fallback_providers: Vec<String>,

    /// Initial backoff for channel reconnection
    #[serde(default = "default_channel_initial_backoff")]
    pub channel_initial_backoff_secs: u64,

    /// Maximum backoff for channel reconnection
    #[serde(default = "default_channel_max_backoff")]
    pub channel_max_backoff_secs: u64,

    /// Scheduler poll interval
    #[serde(default = "default_scheduler_poll")]
    pub scheduler_poll_secs: u64,

    /// Scheduler task retries
    #[serde(default = "default_scheduler_retries")]
    pub scheduler_retries: u32,
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            provider_retries: default_provider_retries(),
            provider_backoff_ms: default_provider_backoff(),
            fallback_providers: vec![],
            channel_initial_backoff_secs: default_channel_initial_backoff(),
            channel_max_backoff_secs: default_channel_max_backoff(),
            scheduler_poll_secs: default_scheduler_poll(),
            scheduler_retries: default_scheduler_retries(),
        }
    }
}

/// Agent execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Enable agent execution
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum tool call iterations per request
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Request confirmation before executing dangerous tools
    #[serde(default = "default_true")]
    pub require_confirmation: bool,

    /// Dangerous tool patterns that require confirmation
    #[serde(default = "default_dangerous_patterns")]
    pub dangerous_patterns: Vec<String>,

    /// System prompt template path (optional)
    #[serde(default)]
    pub system_prompt_path: Option<String>,

    /// Default temperature for agent responses
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_iterations: default_max_iterations(),
            require_confirmation: true,
            dangerous_patterns: default_dangerous_patterns(),
            system_prompt_path: None,
            temperature: default_temperature(),
        }
    }
}

/// Tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Enable shell tool
    #[serde(default = "default_true")]
    pub shell_enabled: bool,

    /// Enable file read tool
    #[serde(default = "default_true")]
    pub file_read_enabled: bool,

    /// Enable file write tool
    #[serde(default = "default_true")]
    pub file_write_enabled: bool,

    /// Enable browser tool
    #[serde(default)]
    pub browser_enabled: bool,

    /// Enable memory tools
    #[serde(default = "default_true")]
    pub memory_enabled: bool,

    /// Enable CodeCoder tool (calls CodeCoder API)
    #[serde(default = "default_true")]
    pub codecoder_enabled: bool,

    /// Shell working directory (defaults to current directory)
    #[serde(default)]
    pub shell_cwd: Option<String>,

    /// Shell timeout in seconds
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout_secs: u64,

    /// File size limit for reading (in bytes)
    #[serde(default = "default_file_size_limit")]
    pub file_size_limit: u64,

    /// Blocked shell commands (security)
    #[serde(default)]
    pub blocked_commands: Vec<String>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            shell_enabled: true,
            file_read_enabled: true,
            file_write_enabled: true,
            browser_enabled: false,
            memory_enabled: true,
            codecoder_enabled: true,
            shell_cwd: None,
            shell_timeout_secs: default_shell_timeout(),
            file_size_limit: default_file_size_limit(),
            blocked_commands: vec![],
        }
    }
}

// Default value functions
fn default_gateway_port() -> u16 {
    4402
}
fn default_gateway_host() -> String {
    "127.0.0.1".into()
}
fn default_channels_port() -> u16 {
    4404
}
fn default_channels_host() -> String {
    "127.0.0.1".into()
}
fn default_token_expiry() -> u64 {
    86400 // 24 hours
}
fn default_rate_limit() -> u32 {
    60
}
fn default_codecoder_endpoint() -> String {
    "http://127.0.0.1:4400".into()
}
fn default_timeout() -> u64 {
    300 // 5 minutes
}
fn default_log_level() -> String {
    "info".into()
}
fn default_log_format() -> String {
    "pretty".into()
}
fn default_memory_backend() -> String {
    "sqlite".into()
}
fn default_true() -> bool {
    true
}

// New default functions for extended config

fn default_imessage_poll_interval() -> u64 {
    5 // 5 seconds
}

fn default_cli_prompt() -> String {
    "> ".into()
}

fn default_provider() -> String {
    "anthropic".into()
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".into()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}

fn default_ollama_model() -> String {
    "llama3".into()
}

fn default_ollama_timeout() -> u64 {
    300 // 5 minutes, local models can be slow
}

fn default_provider_retries() -> u32 {
    2
}

fn default_provider_backoff() -> u64 {
    1000 // 1 second
}

fn default_channel_initial_backoff() -> u64 {
    2
}

fn default_channel_max_backoff() -> u64 {
    60
}

fn default_scheduler_poll() -> u64 {
    15
}

fn default_scheduler_retries() -> u32 {
    2
}

fn default_max_iterations() -> u32 {
    10
}

fn default_dangerous_patterns() -> Vec<String> {
    vec![
        "rm -rf".into(),
        "sudo".into(),
        "chmod 777".into(),
        "format".into(),
        "mkfs".into(),
    ]
}

fn default_temperature() -> f64 {
    0.7
}

fn default_shell_timeout() -> u64 {
    120 // 2 minutes
}

fn default_file_size_limit() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.gateway.port, 4402);
        assert_eq!(config.channels.port, 4404);
        assert!(config.codecoder.enabled);
        assert_eq!(config.providers.default, "anthropic");
        assert!(config.agent.enabled);
        assert!(config.tools.shell_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gateway.port, config.gateway.port);
        assert_eq!(parsed.channels.port, config.channels.port);
        assert_eq!(parsed.providers.default, config.providers.default);
    }

    #[test]
    fn test_api_keys_get() {
        let keys = ApiKeysConfig {
            anthropic: Some("sk-ant-123".into()),
            openai: Some("sk-openai-456".into()),
            ..Default::default()
        };

        assert_eq!(keys.get("anthropic"), Some(&"sk-ant-123".into()));
        assert_eq!(keys.get("openai"), Some(&"sk-openai-456".into()));
        assert_eq!(keys.get("unknown"), None);
    }

    #[test]
    fn test_api_keys_aliases() {
        let keys = ApiKeysConfig {
            google: Some("google-key".into()),
            xai: Some("xai-key".into()),
            together: Some("together-key".into()),
            ..Default::default()
        };

        // Test aliases
        assert_eq!(keys.get("gemini"), Some(&"google-key".into()));
        assert_eq!(keys.get("google-gemini"), Some(&"google-key".into()));
        assert_eq!(keys.get("grok"), Some(&"xai-key".into()));
        assert_eq!(keys.get("together-ai"), Some(&"together-key".into()));
    }

    #[test]
    fn test_reliability_config_defaults() {
        let reliability = ReliabilityConfig::default();
        assert_eq!(reliability.provider_retries, 2);
        assert_eq!(reliability.provider_backoff_ms, 1000);
        assert!(reliability.fallback_providers.is_empty());
    }

    #[test]
    fn test_agent_config_defaults() {
        let agent = AgentConfig::default();
        assert!(agent.enabled);
        assert_eq!(agent.max_iterations, 10);
        assert!(agent.require_confirmation);
        assert!(!agent.dangerous_patterns.is_empty());
    }

    #[test]
    fn test_tools_config_defaults() {
        let tools = ToolsConfig::default();
        assert!(tools.shell_enabled);
        assert!(tools.file_read_enabled);
        assert!(tools.file_write_enabled);
        assert!(!tools.browser_enabled);
        assert!(tools.memory_enabled);
        assert!(tools.codecoder_enabled);
    }

    #[test]
    fn test_ollama_config_defaults() {
        let ollama = OllamaConfig::default();
        assert_eq!(ollama.base_url, "http://localhost:11434");
        assert_eq!(ollama.default_model, "llama3");
        assert_eq!(ollama.timeout_secs, 300);
    }

    #[test]
    fn test_partial_config_deserialization() {
        // Test that partial JSON with only some fields works (uses defaults for rest)
        let json = r#"{"gateway": {"port": 8080}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.gateway.port, 8080);
        assert_eq!(config.gateway.host, "127.0.0.1"); // default
        assert_eq!(config.channels.port, 4404); // default
    }

    #[test]
    fn test_matrix_config() {
        let json = r#"{
            "channels": {
                "matrix": {
                    "enabled": true,
                    "homeserver": "https://matrix.org",
                    "access_token": "syt_xxx",
                    "room_id": "!room:matrix.org",
                    "allowed_users": ["@user:matrix.org"]
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let matrix = config.channels.matrix.unwrap();
        assert!(matrix.enabled);
        assert_eq!(matrix.homeserver, "https://matrix.org");
    }

    #[test]
    fn test_providers_config() {
        let json = r#"{
            "providers": {
                "default": "openai",
                "default_model": "gpt-4-turbo",
                "ollama": {
                    "base_url": "http://192.168.1.100:11434"
                },
                "custom_endpoints": {
                    "my-provider": "https://my-api.example.com"
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.providers.default, "openai");
        assert_eq!(config.providers.default_model, "gpt-4-turbo");
        assert_eq!(config.providers.ollama.base_url, "http://192.168.1.100:11434");
        assert!(config.providers.custom_endpoints.contains_key("my-provider"));
    }

    #[test]
    fn test_agent_config() {
        let json = r#"{
            "agent": {
                "max_iterations": 20,
                "require_confirmation": false,
                "temperature": 0.5
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.agent.max_iterations, 20);
        assert!(!config.agent.require_confirmation);
        assert!((config.agent.temperature - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_tools_config() {
        let json = r#"{
            "tools": {
                "shell_enabled": false,
                "browser_enabled": true,
                "shell_timeout_secs": 60
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(!config.tools.shell_enabled);
        assert!(config.tools.browser_enabled);
        assert_eq!(config.tools.shell_timeout_secs, 60);
    }
}
