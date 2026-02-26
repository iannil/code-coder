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
//! ## Service Ports
//! - `ZERO_GATEWAY_PORT` → services.gateway.port
//! - `ZERO_CHANNELS_PORT` → services.channels.port
//! - `ZERO_WORKFLOW_PORT` → services.workflow.port
//! - `CODECODER_PORT` → services.codecoder.port
//! - `ZERO_BIND_ADDRESS` → network.bind
//!
//! ## Auth
//! - `ZERO_JWT_SECRET` → auth.jwt_secret
//!
//! ## LLM API Keys (→ secrets.llm.*)
//! - `ANTHROPIC_API_KEY` → secrets.llm.anthropic
//! - `OPENAI_API_KEY` → secrets.llm.openai
//! - `GOOGLE_API_KEY` → secrets.llm.google
//! - `DEEPSEEK_API_KEY` → secrets.llm.deepseek
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

// ============================================================================
// Network Configuration (Global bind address)
// ============================================================================

/// Global network configuration.
///
/// Controls the bind address for all services. Default is `127.0.0.1` (local only).
/// Set to `0.0.0.0` to allow remote access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bind address for all services.
    /// Default: "127.0.0.1" (conservative, local only)
    /// Set to "0.0.0.0" for remote access
    #[serde(default = "default_bind_address")]
    pub bind: String,

    /// Public URL for callbacks (optional).
    /// Used when the service is behind a reverse proxy or tunnel.
    #[serde(default)]
    pub public_url: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind: default_bind_address(),
            public_url: None,
        }
    }
}

fn default_bind_address() -> String {
    "127.0.0.1".into()
}

// ============================================================================
// Services Port Configuration (Simplified)
// ============================================================================

/// Simplified service port configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesConfig {
    /// CodeCoder API service
    #[serde(default)]
    pub codecoder: ServicePortConfig,

    /// Gateway service
    #[serde(default)]
    pub gateway: ServicePortConfig,

    /// Channels service
    #[serde(default)]
    pub channels: ServicePortConfig,

    /// Workflow service
    #[serde(default)]
    pub workflow: ServicePortConfig,

    /// Trading service
    #[serde(default)]
    pub trading: ServicePortConfig,
}

/// Individual service port configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePortConfig {
    /// Port number for the service
    #[serde(default)]
    pub port: Option<u16>,
}

impl Default for ServicePortConfig {
    fn default() -> Self {
        Self { port: None }
    }
}

// ============================================================================
// Secrets Configuration (Grouped API keys)
// ============================================================================

/// Grouped secrets configuration.
///
/// All sensitive credentials organized by category for better security management.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretsConfig {
    /// LLM provider API keys
    #[serde(default)]
    pub llm: LlmSecretsConfig,

    /// IM channel credentials
    #[serde(default)]
    pub channels: ChannelSecretsConfig,

    /// External service credentials
    #[serde(default)]
    pub external: ExternalSecretsConfig,
}

/// LLM provider API keys.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmSecretsConfig {
    #[serde(default)]
    pub anthropic: Option<String>,
    #[serde(default)]
    pub openai: Option<String>,
    #[serde(default)]
    pub deepseek: Option<String>,
    #[serde(default)]
    pub google: Option<String>,
    #[serde(default)]
    pub openrouter: Option<String>,
    #[serde(default)]
    pub groq: Option<String>,
    #[serde(default)]
    pub mistral: Option<String>,
    #[serde(default)]
    pub xai: Option<String>,
    #[serde(default)]
    pub together: Option<String>,
    #[serde(default)]
    pub fireworks: Option<String>,
    #[serde(default)]
    pub perplexity: Option<String>,
    #[serde(default)]
    pub cohere: Option<String>,
    #[serde(default)]
    pub cloudflare: Option<String>,
    #[serde(default)]
    pub venice: Option<String>,
    #[serde(default)]
    pub moonshot: Option<String>,
    #[serde(default)]
    pub glm: Option<String>,
    #[serde(default)]
    pub minimax: Option<String>,
    #[serde(default)]
    pub qianfan: Option<String>,
}

/// IM channel credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelSecretsConfig {
    #[serde(default)]
    pub telegram_bot_token: Option<String>,
    #[serde(default)]
    pub discord_bot_token: Option<String>,
    #[serde(default)]
    pub slack_bot_token: Option<String>,
    #[serde(default)]
    pub slack_app_token: Option<String>,
    #[serde(default)]
    pub feishu_app_id: Option<String>,
    #[serde(default)]
    pub feishu_app_secret: Option<String>,
}

/// External service credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalSecretsConfig {
    /// Lixin (理杏仁) API token for A-share market data
    #[serde(default)]
    pub lixin: Option<String>,
    /// iTick API key for A-share market data (primary source)
    /// Get your API key at: https://itick.org
    #[serde(default)]
    pub itick: Option<String>,
    #[serde(default)]
    pub cloudflare_tunnel: Option<String>,
    #[serde(default)]
    pub ngrok_auth: Option<String>,
    #[serde(default)]
    pub elevenlabs: Option<String>,
}

// ============================================================================
// LLM Configuration (Simplified)
// ============================================================================

/// Simplified LLM configuration.
///
/// Consolidates all LLM-related settings including reliability and Ollama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default model in provider/model format
    #[serde(default = "default_llm_model")]
    pub default: String,

    /// Custom provider configurations
    #[serde(default)]
    pub providers: HashMap<String, LlmProviderConfig>,

    /// Fallback provider chain (tried in order when primary fails)
    #[serde(default)]
    pub fallbacks: Vec<String>,

    /// Number of retries before switching to fallback provider
    #[serde(default = "default_llm_retries")]
    pub retries: u32,

    /// Backoff time between retries in milliseconds
    #[serde(default = "default_llm_backoff_ms")]
    pub backoff_ms: u64,

    /// Ollama (local models) configuration
    #[serde(default)]
    pub ollama: LlmOllamaConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default: default_llm_model(),
            providers: HashMap::new(),
            fallbacks: vec![],
            retries: default_llm_retries(),
            backoff_ms: default_llm_backoff_ms(),
            ollama: LlmOllamaConfig::default(),
        }
    }
}

fn default_llm_retries() -> u32 {
    2
}

fn default_llm_backoff_ms() -> u64 {
    1000
}

/// Ollama (local models) configuration within LlmConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOllamaConfig {
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

impl Default for LlmOllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_ollama_url(),
            default_model: default_ollama_model(),
            timeout_secs: default_ollama_timeout(),
        }
    }
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

fn default_llm_model() -> String {
    "anthropic/claude-sonnet-4-20250514".into()
}

/// Custom LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Base URL for the provider API
    #[serde(default)]
    pub base_url: Option<String>,

    /// Available models
    #[serde(default)]
    pub models: Vec<String>,
}

// ============================================================================
// Provider Settings (global LLM settings within provider._settings)
// ============================================================================

/// Global LLM settings stored in `provider._settings`.
///
/// This replaces the top-level `llm` field for global settings while keeping
/// provider-specific configuration in `provider.<name>`.
///
/// Example JSON:
/// ```json
/// {
///   "provider": {
///     "_settings": {
///       "default": "deepseek/deepseek-chat",
///       "retries": 2,
///       "backoff_ms": 1000,
///       "fallbacks": []
///     },
///     "deepseek": {
///       "options": { "apiKey": "sk-xxx" }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderSettings {
    /// Default model in provider/model format
    #[serde(default)]
    pub default: Option<String>,

    /// Number of retries before switching to fallback provider
    #[serde(default)]
    pub retries: Option<u32>,

    /// Backoff time between retries in milliseconds
    #[serde(default)]
    pub backoff_ms: Option<u64>,

    /// Fallback provider chain (tried in order when primary fails)
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

// ============================================================================
// Unified Provider Configuration (Shared with TypeScript ccode)
// ============================================================================

/// Provider configuration compatible with TypeScript format.
///
/// This is the unified provider configuration that both Rust and TypeScript services read.
/// TypeScript uses camelCase field names, so we use serde aliases for compatibility.
///
/// Example JSON:
/// ```json
/// {
///   "provider": {
///     "deepseek": {
///       "api": "https://api.deepseek.com/v1",
///       "name": "DeepSeek",
///       "npm": "@ai-sdk/openai-compatible",
///       "models": {
///         "deepseek-chat": {
///           "id": "deepseek-chat",
///           "name": "DeepSeek Chat",
///           "tool_call": true,
///           "limit": { "context": 64000, "output": 8192 }
///         }
///       },
///       "options": {
///         "apiKey": "sk-xxx",
///         "baseURL": "https://api.deepseek.com"
///       }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedProviderConfig {
    /// Provider API base URL (maps from "api" field in TS format)
    #[serde(default, alias = "api")]
    pub base_url: Option<String>,

    /// Provider display name
    #[serde(default)]
    pub name: Option<String>,

    /// NPM package for the provider SDK (used by TypeScript)
    #[serde(default)]
    pub npm: Option<String>,

    /// Model definitions (TypeScript uses object format, Rust extracts model IDs)
    #[serde(default)]
    pub models: HashMap<String, UnifiedModelConfig>,

    /// Provider options including API key and base URL
    #[serde(default)]
    pub options: Option<UnifiedProviderOptions>,

    // =========================================================================
    // Settings fields (used by _settings entry, ignored by regular providers)
    // =========================================================================

    /// Default model in provider/model format (only used in _settings)
    #[serde(default)]
    pub default: Option<String>,

    /// Number of retries for failed requests (only used in _settings)
    #[serde(default)]
    pub retries: Option<u32>,

    /// Backoff time between retries in milliseconds (only used in _settings)
    #[serde(default)]
    pub backoff_ms: Option<u64>,

    /// Fallback provider chain (only used in _settings)
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

/// Provider options including credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedProviderOptions {
    /// API key for this provider
    #[serde(rename = "apiKey", default)]
    pub api_key: Option<String>,

    /// Base URL override (may differ from top-level base_url)
    #[serde(rename = "baseURL", default)]
    pub base_url: Option<String>,
}

/// Model configuration within a provider.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedModelConfig {
    /// Model ID
    #[serde(default)]
    pub id: Option<String>,

    /// Display name
    #[serde(default)]
    pub name: Option<String>,

    /// Whether this model supports tool calling
    #[serde(default)]
    pub tool_call: bool,

    /// Token limits
    #[serde(default)]
    pub limit: Option<UnifiedModelLimits>,
}

/// Token limits for a model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedModelLimits {
    /// Context window size (input tokens)
    #[serde(default)]
    pub context: Option<u64>,

    /// Maximum output tokens
    #[serde(default)]
    pub output: Option<u64>,
}

// ============================================================================
// Auth Configuration (Simplified)
// ============================================================================

/// Simplified authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication mode: "pairing" | "jwt" | "none"
    #[serde(default = "default_auth_mode")]
    pub mode: String,

    /// JWT secret (auto-generated if not set)
    #[serde(default)]
    pub jwt_secret: Option<String>,

    /// Token expiry in seconds
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: default_auth_mode(),
            jwt_secret: None,
            token_expiry_secs: default_token_expiry(),
        }
    }
}

// ============================================================================
// Voice Configuration (Simplified)
// ============================================================================

/// Simplified voice configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    /// TTS configuration
    #[serde(default)]
    pub tts: VoiceTtsConfig,

    /// STT configuration
    #[serde(default)]
    pub stt: VoiceSttConfig,
}

/// TTS provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTtsConfig {
    #[serde(default = "default_tts_provider")]
    pub provider: String,
    #[serde(default = "default_tts_voice")]
    pub voice: String,
}

impl Default for VoiceTtsConfig {
    fn default() -> Self {
        Self {
            provider: default_tts_provider(),
            voice: default_tts_voice(),
        }
    }
}

fn default_tts_provider() -> String {
    "compatible".into()
}

fn default_tts_voice() -> String {
    "nova".into()
}

/// STT provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSttConfig {
    #[serde(default = "default_stt_provider")]
    pub provider: String,
    #[serde(default = "default_stt_model")]
    pub model: String,
}

impl Default for VoiceSttConfig {
    fn default() -> Self {
        Self {
            provider: default_stt_provider(),
            model: default_stt_model(),
        }
    }
}

fn default_stt_provider() -> String {
    "local".into()
}

fn default_stt_model() -> String {
    "base".into()
}

// ============================================================================
// Simplified Channels Configuration
// ============================================================================

/// Simplified channel enable/disable configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelEnableConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// Root configuration structure for all Zero services.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// JSON Schema reference
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    // =========================================================================
    // New Simplified Configuration (Phase 1)
    // =========================================================================

    /// Global network configuration (bind address for all services)
    #[serde(default)]
    pub network: NetworkConfig,

    /// Simplified service port configuration
    #[serde(default)]
    pub services: ServicesConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// Tunnel configuration for external access
    #[serde(default)]
    pub tunnel: TunnelConfig,

    /// Grouped secrets (API keys organized by category)
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// Simplified LLM configuration
    #[serde(default)]
    pub llm: LlmConfig,

    /// Unified provider configuration (shared with TypeScript ccode)
    ///
    /// Contains custom LLM provider definitions including API keys.
    /// Both Rust services and TypeScript ccode read this field.
    #[serde(default)]
    pub provider: HashMap<String, UnifiedProviderConfig>,

    /// Voice configuration (TTS/STT)
    #[serde(default)]
    pub voice: VoiceConfig,

    // =========================================================================
    // Service Configuration (business logic, not port/host)
    // =========================================================================

    /// Gateway configuration (business settings only)
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Channels configuration (business settings only)
    #[serde(default)]
    pub channels: ChannelsConfig,

    /// Workflow configuration (business settings only)
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

    /// Agent execution configuration
    #[serde(default)]
    pub agent: AgentConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Audit logging configuration
    #[serde(default)]
    pub audit: crate::audit::AuditConfig,

    /// Sandbox (Docker) configuration for secure code execution
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// Qdrant vector database configuration for semantic memory
    #[serde(default)]
    pub qdrant: QdrantConfig,

    /// Human-in-the-Loop (HitL) configuration
    #[serde(default)]
    pub hitl: HitLConfig,

    /// Trading service configuration
    #[serde(default)]
    pub trading: Option<TradingConfig>,
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
        // Service port overrides (use services.* structure)
        if let Ok(port) = std::env::var("ZERO_GATEWAY_PORT") {
            if let Ok(p) = port.parse() {
                self.services.gateway.port = Some(p);
            }
        }
        if let Ok(port) = std::env::var("ZERO_CHANNELS_PORT") {
            if let Ok(p) = port.parse() {
                self.services.channels.port = Some(p);
            }
        }
        if let Ok(port) = std::env::var("ZERO_WORKFLOW_PORT") {
            if let Ok(p) = port.parse() {
                self.services.workflow.port = Some(p);
            }
        }
        if let Ok(port) = std::env::var("CODECODER_PORT") {
            if let Ok(p) = port.parse() {
                self.services.codecoder.port = Some(p);
            }
        }

        // Network bind address override
        if let Ok(bind) = std::env::var("ZERO_BIND_ADDRESS") {
            self.network.bind = bind;
        }

        // Auth overrides
        if let Ok(secret) = std::env::var("ZERO_JWT_SECRET") {
            self.auth.jwt_secret = Some(secret);
        }

        // Log level override
        if let Ok(level) = std::env::var("ZERO_LOG_LEVEL") {
            self.observability.log_level = level;
        }

        // Apply LLM API key env fallbacks to secrets.llm
        self.apply_llm_env_fallbacks();

        // Apply legacy endpoint field if present in config
        self.codecoder.apply_legacy_endpoint();
    }

    /// Apply LLM API key environment variable fallbacks.
    fn apply_llm_env_fallbacks(&mut self) {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            self.secrets.llm.anthropic = Some(key);
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.secrets.llm.openai = Some(key);
        }
        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            self.secrets.llm.deepseek = Some(key);
        }
        if let Ok(key) = std::env::var("GOOGLE_API_KEY").or_else(|_| std::env::var("GEMINI_API_KEY")) {
            self.secrets.llm.google = Some(key);
        }
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            self.secrets.llm.openrouter = Some(key);
        }
        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            self.secrets.llm.groq = Some(key);
        }
        if let Ok(key) = std::env::var("MISTRAL_API_KEY") {
            self.secrets.llm.mistral = Some(key);
        }
        if let Ok(key) = std::env::var("XAI_API_KEY") {
            self.secrets.llm.xai = Some(key);
        }
        if let Ok(key) = std::env::var("TOGETHER_API_KEY") {
            self.secrets.llm.together = Some(key);
        }
        if let Ok(key) = std::env::var("FIREWORKS_API_KEY") {
            self.secrets.llm.fireworks = Some(key);
        }
        if let Ok(key) = std::env::var("PERPLEXITY_API_KEY") {
            self.secrets.llm.perplexity = Some(key);
        }
        if let Ok(key) = std::env::var("COHERE_API_KEY") {
            self.secrets.llm.cohere = Some(key);
        }
        if let Ok(key) = std::env::var("CLOUDFLARE_API_KEY") {
            self.secrets.llm.cloudflare = Some(key);
        }
        if let Ok(key) = std::env::var("VENICE_API_KEY") {
            self.secrets.llm.venice = Some(key);
        }
        if let Ok(key) = std::env::var("MOONSHOT_API_KEY") {
            self.secrets.llm.moonshot = Some(key);
        }
        if let Ok(key) = std::env::var("GLM_API_KEY") {
            self.secrets.llm.glm = Some(key);
        }
        if let Ok(key) = std::env::var("MINIMAX_API_KEY") {
            self.secrets.llm.minimax = Some(key);
        }
        if let Ok(key) = std::env::var("QIANFAN_API_KEY") {
            self.secrets.llm.qianfan = Some(key);
        }
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

    // =========================================================================
    // Endpoint convenience methods
    // =========================================================================

    /// Get the effective bind address.
    ///
    /// Priority:
    /// 1. New `network.bind` field
    /// 2. Falls back to default "127.0.0.1"
    pub fn bind_address(&self) -> &str {
        &self.network.bind
    }

    /// Get the effective port for a service.
    ///
    /// Uses `services.<service>.port` with default fallback.
    pub fn codecoder_port(&self) -> u16 {
        self.services.codecoder.port.unwrap_or(4400)
    }

    pub fn gateway_port(&self) -> u16 {
        self.services.gateway.port.unwrap_or(4430)
    }

    pub fn channels_port(&self) -> u16 {
        self.services.channels.port.unwrap_or(4431)
    }

    pub fn workflow_port(&self) -> u16 {
        self.services.workflow.port.unwrap_or(4432)
    }

    pub fn trading_port(&self) -> u16 {
        self.services.trading.port.unwrap_or(4434)
    }

    /// Get the CodeCoder service endpoint URL.
    ///
    /// Uses network.bind and services.codecoder.port (with legacy fallbacks).
    /// Example: "http://127.0.0.1:4400"
    pub fn codecoder_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address(), self.codecoder_port())
    }

    /// Get the Gateway service endpoint URL.
    ///
    /// Uses network.bind and services.gateway.port (with legacy fallbacks).
    /// Example: "http://127.0.0.1:4430"
    pub fn gateway_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address(), self.gateway_port())
    }

    /// Get the Channels service endpoint URL.
    ///
    /// Uses network.bind and services.channels.port (with legacy fallbacks).
    /// Example: "http://127.0.0.1:4431"
    pub fn channels_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address(), self.channels_port())
    }

    /// Get the Workflow service endpoint URL.
    ///
    /// Uses network.bind and services.workflow.port (with legacy fallbacks).
    /// Example: "http://127.0.0.1:4432"
    pub fn workflow_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address(), self.workflow_port())
    }

    /// Get the Trading service endpoint URL.
    ///
    /// Uses network.bind and services.trading.port (with legacy fallbacks).
    /// Example: "http://127.0.0.1:4434"
    pub fn trading_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address(), self.trading_port())
    }

    // =========================================================================
    // API Key access methods (supports both new secrets and legacy api_keys)
    // =========================================================================

    /// Get an API key by provider name.
    ///
    /// Priority:
    /// 1. Unified provider config in `provider.<name>.options.apiKey`
    /// 2. Legacy secrets in `secrets.llm.<provider>` (backwards compatibility)
    pub fn get_api_key(&self, provider_name: &str) -> Option<String> {
        // Normalize provider name for lookup (handle aliases)
        let canonical_name = match provider_name {
            "gemini" => "google",
            "grok" => "xai",
            "together-ai" => "together",
            "fireworks-ai" => "fireworks",
            "cloudflare-ai" => "cloudflare",
            "kimi" => "moonshot",
            "zhipu" => "glm",
            "baidu" => "qianfan",
            other => other,
        };

        // 1. First check unified provider config (new preferred location)
        if let Some(key) = self
            .provider
            .get(canonical_name)
            .and_then(|p| p.options.as_ref())
            .and_then(|o| o.api_key.clone())
        {
            return Some(key);
        }

        // Also check with original name if different from canonical
        if canonical_name != provider_name {
            if let Some(key) = self
                .provider
                .get(provider_name)
                .and_then(|p| p.options.as_ref())
                .and_then(|o| o.api_key.clone())
            {
                return Some(key);
            }
        }

        // 2. Backwards compatibility: check secrets.llm (to be deprecated)
        match provider_name {
            "anthropic" => self.secrets.llm.anthropic.clone(),
            "openai" => self.secrets.llm.openai.clone(),
            "deepseek" => self.secrets.llm.deepseek.clone(),
            "google" | "gemini" => self.secrets.llm.google.clone(),
            "openrouter" => self.secrets.llm.openrouter.clone(),
            "groq" => self.secrets.llm.groq.clone(),
            "mistral" => self.secrets.llm.mistral.clone(),
            "xai" | "grok" => self.secrets.llm.xai.clone(),
            "together" | "together-ai" => self.secrets.llm.together.clone(),
            "fireworks" | "fireworks-ai" => self.secrets.llm.fireworks.clone(),
            "perplexity" => self.secrets.llm.perplexity.clone(),
            "cohere" => self.secrets.llm.cohere.clone(),
            "cloudflare" | "cloudflare-ai" => self.secrets.llm.cloudflare.clone(),
            "venice" => self.secrets.llm.venice.clone(),
            "moonshot" | "kimi" => self.secrets.llm.moonshot.clone(),
            "glm" | "zhipu" => self.secrets.llm.glm.clone(),
            "minimax" => self.secrets.llm.minimax.clone(),
            "qianfan" | "baidu" => self.secrets.llm.qianfan.clone(),
            _ => None,
        }
    }

    /// Get the base URL for a provider.
    ///
    /// Priority:
    /// 1. `provider.<name>.options.baseURL`
    /// 2. `provider.<name>.api` (alias for base_url)
    /// 3. `llm.providers.<name>.base_url`
    pub fn get_provider_base_url(&self, provider_name: &str) -> Option<String> {
        // First check unified provider config
        if let Some(p) = self.provider.get(provider_name) {
            // Prefer options.baseURL over top-level base_url/api
            if let Some(opts) = &p.options {
                if opts.base_url.is_some() {
                    return opts.base_url.clone();
                }
            }
            if p.base_url.is_some() {
                return p.base_url.clone();
            }
        }

        // Fall back to legacy llm.providers config
        self.llm
            .providers
            .get(provider_name)
            .and_then(|p| p.base_url.clone())
    }

    /// Get the global LLM settings.
    ///
    /// Priority:
    /// 1. `provider._settings` (new unified location)
    /// 2. `llm` top-level config (backwards compatibility)
    ///
    /// Returns a ProviderSettings struct with resolved values.
    pub fn get_llm_settings(&self) -> ProviderSettings {
        // Check provider._settings first
        if let Some(settings) = self.provider.get("_settings") {
            return ProviderSettings {
                default: settings.default.clone(),
                retries: settings.retries,
                backoff_ms: settings.backoff_ms,
                fallbacks: settings.fallbacks.clone(),
            };
        }

        // Backwards compatibility: use llm config
        ProviderSettings {
            default: Some(self.llm.default.clone()),
            retries: Some(self.llm.retries),
            backoff_ms: Some(self.llm.backoff_ms),
            fallbacks: self.llm.fallbacks.clone(),
        }
    }

    /// Get the default model string (provider/model format).
    ///
    /// Priority:
    /// 1. `provider._settings.default`
    /// 2. `llm.default`
    pub fn get_default_model(&self) -> String {
        self.get_llm_settings()
            .default
            .unwrap_or_else(|| self.llm.default.clone())
    }

    /// Check if using new config format (has network or services or secrets populated).
    pub fn is_new_format(&self) -> bool {
        // Check if any new-format fields are populated
        self.services.codecoder.port.is_some()
            || self.services.gateway.port.is_some()
            || self.secrets.llm.anthropic.is_some()
            || self.secrets.llm.openai.is_some()
            || self.secrets.llm.deepseek.is_some()
    }

    // =========================================================================
    // Channel credential accessors (from secrets.channels)
    // =========================================================================

    /// Get Telegram bot token from secrets.channels.
    pub fn telegram_bot_token(&self) -> Option<String> {
        self.secrets.channels.telegram_bot_token.clone()
    }

    /// Get Discord bot token from secrets.channels.
    pub fn discord_bot_token(&self) -> Option<String> {
        self.secrets.channels.discord_bot_token.clone()
    }

    /// Get Slack bot token from secrets.channels.
    pub fn slack_bot_token(&self) -> Option<String> {
        self.secrets.channels.slack_bot_token.clone()
    }

    /// Get Slack app token from secrets.channels.
    pub fn slack_app_token(&self) -> Option<String> {
        self.secrets.channels.slack_app_token.clone()
    }

    /// Get Feishu app ID from secrets.channels.
    pub fn feishu_app_id(&self) -> Option<String> {
        self.secrets.channels.feishu_app_id.clone()
    }

    /// Get Feishu app secret from secrets.channels.
    pub fn feishu_app_secret(&self) -> Option<String> {
        self.secrets.channels.feishu_app_secret.clone()
    }

    /// Check if Telegram channel is enabled and has credentials.
    pub fn telegram_enabled(&self) -> bool {
        self.channels.telegram.as_ref().map(|t| t.enabled).unwrap_or(false)
            && self.telegram_bot_token().is_some()
    }

    /// Check if Discord channel is enabled and has credentials.
    pub fn discord_enabled(&self) -> bool {
        self.channels.discord.as_ref().map(|d| d.enabled).unwrap_or(false)
            && self.discord_bot_token().is_some()
    }

    /// Check if Slack channel is enabled and has credentials.
    pub fn slack_enabled(&self) -> bool {
        self.channels.slack.as_ref().map(|s| s.enabled).unwrap_or(false)
            && self.slack_bot_token().is_some()
    }

    /// Check if Feishu channel is enabled and has credentials.
    pub fn feishu_enabled(&self) -> bool {
        self.channels.feishu.as_ref().map(|f| f.enabled).unwrap_or(false)
            && self.feishu_app_id().is_some()
    }

    // =========================================================================
    // Trading data source credential accessors (from secrets.external)
    // =========================================================================

    /// Get iTick API key for A-share market data.
    ///
    /// Priority: secrets.external.itick > trading.itick_api_key (deprecated)
    #[allow(deprecated)]
    pub fn itick_api_key(&self) -> Option<String> {
        self.secrets
            .external
            .itick
            .clone()
            .or_else(|| self.trading.as_ref().and_then(|t| t.itick_api_key.clone()))
    }

    /// Get Lixin API token for A-share market data.
    ///
    /// Priority: secrets.external.lixin > trading.lixin_token (deprecated)
    #[allow(deprecated)]
    pub fn lixin_token(&self) -> Option<String> {
        self.secrets
            .external
            .lixin
            .clone()
            .or_else(|| self.trading.as_ref().and_then(|t| t.lixin_token.clone()))
    }
}

/// Gateway service configuration.
///
/// Business-only settings. Port/host are in services.gateway/network.bind.
/// Auth settings are in the top-level auth config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Require pairing before accepting requests (pairing mode)
    #[serde(default)]
    pub require_pairing: bool,

    /// Paired tokens (managed by pairing system)
    #[serde(default)]
    pub paired_tokens: Vec<String>,

    /// Allow public binding (0.0.0.0) without authentication warning
    #[serde(default)]
    pub allow_public_bind: bool,

    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub rate_limiting: bool,

    /// Requests per minute per user
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            require_pairing: false,
            paired_tokens: vec![],
            allow_public_bind: false,
            rate_limiting: true,
            rate_limit_rpm: default_rate_limit(),
        }
    }
}

/// Channels service configuration.
///
/// Business-only settings. Port/host are in services.channels/network.bind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
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

    /// WeChat Work (企业微信) bot configuration
    #[serde(default)]
    pub wecom: Option<WeComConfig>,

    /// DingTalk (钉钉) bot configuration
    #[serde(default)]
    pub dingtalk: Option<DingTalkConfig>,

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

    /// Email channel configuration (IMAP/SMTP)
    #[serde(default)]
    pub email: Option<EmailConfig>,

    /// TTS configuration
    #[serde(default)]
    pub tts: Option<TtsConfig>,

    /// STT configuration
    #[serde(default)]
    pub stt: Option<SttConfig>,

    /// Asset capture configuration (for capturing and saving content to Feishu Docs/Notion)
    #[serde(default)]
    pub capture: Option<CaptureConfig>,

    /// Enable streaming progress feedback for IM channels (default: true)
    /// When enabled, users receive real-time progress updates as tasks execute
    #[serde(default = "default_streaming_enabled")]
    pub streaming_enabled: bool,

    /// Throttle interval for progress updates in milliseconds (default: 1000ms)
    /// Prevents excessive message edits that could hit rate limits
    #[serde(default = "default_progress_throttle_ms")]
    pub progress_throttle_ms: u64,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            telegram: None,
            discord: None,
            slack: None,
            feishu: None,
            wecom: None,
            dingtalk: None,
            matrix: None,
            whatsapp: None,
            imessage: None,
            cli: None,
            email: None,
            tts: None,
            stt: None,
            capture: None,
            streaming_enabled: default_streaming_enabled(),
            progress_throttle_ms: default_progress_throttle_ms(),
        }
    }
}

/// Telegram channel configuration.
///
/// Bot token is stored in secrets.channels.telegram_bot_token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub allowed_chats: Vec<i64>,
    /// Trading notification chat ID (auto-filled when user sends /bind_trading)
    #[serde(default)]
    pub trading_chat_id: Option<String>,
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

/// WeChat Work (企业微信) channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    pub enabled: bool,
    /// Enterprise ID (corpid)
    pub corp_id: String,
    /// Agent ID (agentid)
    pub agent_id: i64,
    /// Secret for the agent
    pub secret: String,
    /// Token for callback verification
    #[serde(default)]
    pub token: Option<String>,
    /// AES encoding key for message encryption/decryption
    #[serde(default)]
    pub encoding_aes_key: Option<String>,
    /// Allowed user IDs. Use "*" to allow everyone.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// DingTalk (钉钉) channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    pub enabled: bool,
    /// App Key for the robot
    pub app_key: String,
    /// App Secret for the robot
    pub app_secret: String,
    /// Robot code (for internal enterprise bot)
    #[serde(default)]
    pub robot_code: Option<String>,
    /// Outgoing webhook token for signature verification
    #[serde(default)]
    pub outgoing_token: Option<String>,
    /// Allowed user IDs. Use "*" to allow everyone.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Use Stream mode instead of webhook mode
    #[serde(default)]
    pub stream_mode: bool,
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

/// Email channel configuration (IMAP for inbound, SMTP for outbound).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Enable email channel
    #[serde(default)]
    pub enabled: bool,
    /// IMAP server hostname
    pub imap_host: String,
    /// IMAP server port (default: 993 for TLS)
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    /// IMAP folder to poll (default: INBOX)
    #[serde(default = "default_imap_folder")]
    pub imap_folder: String,
    /// SMTP server hostname
    pub smtp_host: String,
    /// SMTP server port (default: 587 for STARTTLS)
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    /// Use TLS for SMTP (default: true)
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    /// Email username for authentication
    pub username: String,
    /// Email password for authentication
    pub password: String,
    /// From address for outgoing emails
    pub from_address: String,
    /// Poll interval in seconds (default: 60)
    #[serde(default = "default_email_poll_interval")]
    pub poll_interval_secs: u64,
    /// Allowed sender addresses/domains (empty = deny all, ["*"] = allow all)
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

// ============================================================================
// Capture Configuration
// ============================================================================

/// Asset capture configuration for cross-platform content capture.
///
/// Enables users to forward content from IM channels (Telegram, WeChat, etc.)
/// and automatically extract, summarize, tag, and save to knowledge bases.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureConfig {
    /// Enable asset capture functionality
    #[serde(default)]
    pub enabled: bool,

    /// Feishu Docs configuration for saving captured assets
    #[serde(default)]
    pub feishu_docs: Option<FeishuDocsConfig>,

    /// Notion configuration for saving captured assets
    #[serde(default)]
    pub notion: Option<NotionConfig>,

    /// Auto-capture configuration
    #[serde(default)]
    pub auto_capture: AutoCaptureConfig,
}

/// Feishu Docs configuration for asset storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuDocsConfig {
    /// Feishu application ID (can reuse FeishuChannel's app_id)
    pub app_id: String,
    /// Feishu application secret (can reuse FeishuChannel's app_secret)
    pub app_secret: String,
    /// Default folder token to save documents
    pub folder_token: String,
    /// Document template ID (optional)
    #[serde(default)]
    pub template_id: Option<String>,
}

/// Notion configuration for asset storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionConfig {
    /// Notion Integration Token
    pub token: String,
    /// Default database ID for storing captured assets
    pub database_id: String,
}

/// Auto-capture configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoCaptureConfig {
    /// Automatically capture forwarded messages
    #[serde(default)]
    pub capture_forwarded: bool,
    /// Automatically capture messages containing links
    #[serde(default)]
    pub capture_links: bool,
    /// Trigger prefixes that activate capture (e.g., "#收藏", "#save", "@save")
    #[serde(default)]
    pub trigger_prefixes: Vec<String>,
}

/// Workflow service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Ticket/Issue automation configuration
    #[serde(default)]
    pub ticket: TicketConfig,

    /// Competitive intelligence monitoring configuration
    #[serde(default)]
    pub monitor: MonitorConfig,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            cron: CronConfig::default(),
            webhook: WebhookConfig::default(),
            git: GitIntegrationConfig::default(),
            ticket: TicketConfig::default(),
            monitor: MonitorConfig::default(),
        }
    }
}

// ============================================================================
// Monitor Configuration
// ============================================================================

/// Configuration for competitive intelligence monitoring.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitorConfig {
    /// Enable monitoring
    #[serde(default)]
    pub enabled: bool,

    /// Monitor tasks
    #[serde(default)]
    pub tasks: Vec<MonitorTask>,
}

/// A monitor task configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTask {
    /// Task ID
    pub id: String,

    /// Task name (e.g., "每日竞品早报")
    pub name: String,

    /// Cron expression (e.g., "0 0 9 * * *" for 9 AM daily)
    pub schedule: String,

    /// Sources to monitor
    #[serde(default)]
    pub sources: Vec<MonitorSourceConfig>,

    /// Notification configuration
    pub notification: MonitorNotificationConfig,
}

/// A monitor source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSourceConfig {
    /// Source ID
    pub id: String,

    /// Source name (e.g., "竞品A官网")
    pub name: String,

    /// URL to monitor
    pub url: String,

    /// Source type: website, rss, twitter
    #[serde(default = "default_source_type")]
    pub source_type: String,

    /// CSS selector for content extraction (optional)
    #[serde(default)]
    pub selector: Option<String>,
}

/// Notification configuration for monitor reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorNotificationConfig {
    /// Channel type (feishu, wecom, dingtalk)
    pub channel_type: String,

    /// Channel ID (group chat ID)
    pub channel_id: String,

    /// Report template: daily_brief, detailed, comparison
    #[serde(default = "default_monitor_template")]
    pub template: String,
}

fn default_source_type() -> String {
    "website".to_string()
}

fn default_monitor_template() -> String {
    "daily_brief".to_string()
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

    /// GitHub personal access token for API calls
    #[serde(default)]
    pub github_token: Option<String>,
}

/// Ticket/Issue automation configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TicketConfig {
    /// Enable ticket automation
    #[serde(default)]
    pub enabled: bool,

    /// GitHub configuration for creating issues
    #[serde(default)]
    pub github: Option<GitHubTicketConfig>,

    /// IM notification configuration
    #[serde(default)]
    pub notification: Option<TicketNotificationConfig>,
}

/// GitHub configuration for ticket automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubTicketConfig {
    /// GitHub Token (or use git.github_token)
    #[serde(default)]
    pub token: Option<String>,

    /// Default repository (owner/repo format)
    pub default_repo: String,

    /// Labels to add for bug reports
    #[serde(default = "default_bug_labels")]
    pub bug_labels: Vec<String>,

    /// Labels to add for feature requests
    #[serde(default = "default_feature_labels")]
    pub feature_labels: Vec<String>,
}

/// IM notification configuration for ticket automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketNotificationConfig {
    /// Enable notifications
    #[serde(default)]
    pub enabled: bool,

    /// Channel type (feishu, wecom, dingtalk)
    pub channel_type: String,

    /// Channel ID (group chat ID)
    pub channel_id: String,
}

/// `CodeCoder` integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCoderConfig {
    /// Enable `CodeCoder` integration
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// API timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for CodeCoderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: default_timeout(),
        }
    }
}

impl CodeCoderConfig {
    /// Apply legacy endpoint field if present (no-op, kept for backward compat).
    pub fn apply_legacy_endpoint(&mut self) {
        // No-op: port/host are now in services.codecoder and network.bind
    }
}

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error)
    /// Aliases: "level" for backward compatibility with existing config files
    #[serde(default = "default_log_level", alias = "level")]
    pub log_level: String,

    /// Log format (json, pretty)
    /// Aliases: "format" for backward compatibility with existing config files
    #[serde(default = "default_log_format", alias = "format")]
    pub log_format: String,

    /// Enable request tracing
    #[serde(default = "default_true")]
    pub tracing: bool,

    /// Show trace_id in pretty format logs (default: true)
    ///
    /// When enabled, trace IDs are included in log output for request tracking.
    #[serde(default = "default_true")]
    pub show_trace_id: bool,

    /// Additional module targets to exclude from logging.
    ///
    /// These modules will be set to `warn` level to reduce noise.
    /// Built-in noisy modules (hyper, reqwest, h2, rustls, tokio_util) are
    /// always filtered; this list allows adding custom modules.
    #[serde(default)]
    pub excluded_targets: Vec<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_format: default_log_format(),
            tracing: true,
            show_trace_id: true,
            excluded_targets: Vec::new(),
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
fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_token_expiry() -> u64 {
    86400 // 24 hours
}
fn default_rate_limit() -> u32 {
    60
}
fn default_timeout() -> u64 {
    1800 // 30 minutes - LLM tasks can run long
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

fn default_imap_port() -> u16 {
    993
}

fn default_smtp_port() -> u16 {
    587
}

fn default_imap_folder() -> String {
    "INBOX".into()
}

fn default_email_poll_interval() -> u64 {
    60
}

fn default_streaming_enabled() -> bool {
    true
}

fn default_progress_throttle_ms() -> u64 {
    1000 // 1 second
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

fn default_bug_labels() -> Vec<String> {
    vec!["bug".into(), "triage".into()]
}

fn default_feature_labels() -> Vec<String> {
    vec!["enhancement".into()]
}

fn default_auth_mode() -> String {
    "pairing".into() // Default to pairing mode for security
}

// ============================================================================
// Sandbox Configuration
// ============================================================================

/// Docker sandbox configuration for secure code execution.
///
/// Allows running untrusted code in isolated Docker containers with
/// configurable resource limits and network access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable Docker sandbox for code execution
    #[serde(default)]
    pub enabled: bool,

    /// Docker socket path
    #[serde(default = "default_docker_socket")]
    pub docker_socket: String,

    /// Default execution timeout in seconds
    #[serde(default = "default_sandbox_timeout_secs")]
    pub default_timeout_secs: u64,

    /// Maximum memory limit in MB
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,

    /// Enable network access for sandboxed containers
    #[serde(default)]
    pub network_enabled: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            docker_socket: default_docker_socket(),
            default_timeout_secs: default_sandbox_timeout_secs(),
            max_memory_mb: default_max_memory_mb(),
            network_enabled: false,
        }
    }
}

fn default_docker_socket() -> String {
    "/var/run/docker.sock".into()
}

fn default_sandbox_timeout_secs() -> u64 {
    60
}

fn default_max_memory_mb() -> u64 {
    256
}

// ============================================================================
// Qdrant Configuration
// ============================================================================

/// Qdrant vector database configuration for semantic memory.
///
/// Enables semantic search and retrieval of conversation history,
/// documents, and other knowledge artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    /// Enable Qdrant integration
    #[serde(default)]
    pub enabled: bool,

    /// Qdrant server URL
    #[serde(default = "default_qdrant_url")]
    pub url: String,

    /// Collection name for storing embeddings
    #[serde(default = "default_qdrant_collection")]
    pub collection: String,

    /// Embedding model provider (openai, cohere, local)
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_qdrant_url(),
            collection: default_qdrant_collection(),
            embedding_model: default_embedding_model(),
        }
    }
}

fn default_qdrant_url() -> String {
    "http://localhost:6333".into()
}

fn default_qdrant_collection() -> String {
    "codecoder_memory".into()
}

fn default_embedding_model() -> String {
    "openai".into()
}

// ============================================================================
// HitL (Human-in-the-Loop) Configuration
// ============================================================================

/// Human-in-the-Loop (HitL) configuration.
///
/// Enables approval workflows where certain agent actions require
/// human approval before execution. Approvals are sent via configured
/// channels (Telegram, Slack, etc.) and callbacks are received via HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitLConfig {
    /// Whether HitL is enabled
    #[serde(default = "default_hitl_enabled")]
    pub enabled: bool,

    /// Default approvers (user IDs)
    #[serde(default)]
    pub default_approvers: Vec<String>,

    /// Base URL for callbacks
    #[serde(default)]
    pub callback_base_url: String,

    /// Database path for HitL store
    #[serde(default = "default_hitl_db_path")]
    pub db_path: String,

    /// DEPRECATED: Legacy channels_endpoint field for backward compatibility.
    /// Use Config::channels_endpoint() instead. This field is ignored.
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    channels_endpoint: Option<String>,
}

impl Default for HitLConfig {
    fn default() -> Self {
        Self {
            enabled: default_hitl_enabled(),
            default_approvers: vec![],
            callback_base_url: String::new(),
            db_path: default_hitl_db_path(),
            channels_endpoint: None,
        }
    }
}

fn default_hitl_enabled() -> bool {
    true
}

fn default_hitl_db_path() -> String {
    "~/.codecoder/hitl.db".into()
}

// ============================================================================
// Tunnel Configuration
// ============================================================================

/// Tunnel configuration for external access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Tunnel provider: "none", "cloudflare", "tailscale", "ngrok", "custom"
    #[serde(default = "default_tunnel_provider")]
    pub provider: String,

    /// Cloudflare Tunnel configuration
    #[serde(default)]
    pub cloudflare: Option<CloudflareTunnelConfig>,

    /// Tailscale Tunnel configuration
    #[serde(default)]
    pub tailscale: Option<TailscaleTunnelConfig>,

    /// ngrok Tunnel configuration
    #[serde(default)]
    pub ngrok: Option<NgrokTunnelConfig>,

    /// Custom tunnel configuration
    #[serde(default)]
    pub custom: Option<CustomTunnelConfig>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: default_tunnel_provider(),
            cloudflare: None,
            tailscale: None,
            ngrok: None,
            custom: None,
        }
    }
}

fn default_tunnel_provider() -> String {
    "none".into()
}

/// Cloudflare Tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareTunnelConfig {
    /// Cloudflare Tunnel token (from Zero Trust dashboard)
    pub token: String,
}

/// Tailscale Tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TailscaleTunnelConfig {
    /// Use Tailscale Funnel (public internet) vs Serve (tailnet only)
    #[serde(default)]
    pub funnel: bool,
    /// Optional hostname override
    #[serde(default)]
    pub hostname: Option<String>,
}

/// ngrok Tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgrokTunnelConfig {
    /// ngrok auth token
    pub auth_token: String,
    /// Optional custom domain
    #[serde(default)]
    pub domain: Option<String>,
}

/// Custom tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTunnelConfig {
    /// Command template to start the tunnel. Use {port} and {host} placeholders.
    /// Example: "bore local {port} --to bore.pub"
    pub start_command: String,
    /// Optional URL to check tunnel health
    #[serde(default)]
    pub health_url: Option<String>,
    /// Optional regex to extract public URL from command stdout
    #[serde(default)]
    pub url_pattern: Option<String>,
}

// ============================================================================
// Trading Configuration
// ============================================================================

/// Trading service configuration for automated trading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Trading service HTTP port
    #[serde(default = "default_trading_port")]
    pub port: u16,

    /// Trading service HTTP host
    #[serde(default = "default_host")]
    pub host: String,

    /// DEPRECATED: Use secrets.external.lixin instead.
    /// 理杏仁 (Lixinger) API token for A-share data
    #[serde(default)]
    #[deprecated(since = "0.2.0", note = "Use secrets.external.lixin instead")]
    pub lixin_token: Option<String>,

    /// DEPRECATED: Use secrets.external.itick instead.
    /// iTick API key for A-share market data (primary source)
    /// Get your API key at: https://itick.org
    #[serde(default)]
    #[deprecated(since = "0.2.0", note = "Use secrets.external.itick instead")]
    pub itick_api_key: Option<String>,

    /// SMT pairs for divergence detection
    #[serde(default)]
    pub smt_pairs: Option<Vec<SmtPairConfig>>,

    /// Timeframes for multi-timeframe analysis (e.g., ["D", "H4", "H1"])
    #[serde(default)]
    pub timeframes: Option<Vec<String>>,

    /// Minimum bars for accumulation phase detection
    #[serde(default)]
    pub min_accumulation_bars: Option<usize>,

    /// Manipulation threshold (ATR multiple)
    #[serde(default)]
    pub manipulation_threshold: Option<f64>,

    /// Require multi-timeframe alignment
    #[serde(default)]
    pub require_alignment: Option<bool>,

    /// Signal expiry in minutes
    #[serde(default)]
    pub signal_expiry_minutes: Option<i64>,

    /// Maximum number of open positions
    #[serde(default)]
    pub max_positions: Option<usize>,

    /// Maximum capital per position (percentage)
    #[serde(default)]
    pub max_position_pct: Option<f64>,

    /// Maximum daily capital deployment (percentage)
    #[serde(default)]
    pub max_daily_capital_pct: Option<f64>,

    /// Default stop loss percentage
    #[serde(default)]
    pub default_stop_loss_pct: Option<f64>,

    /// Enable automatic execution
    #[serde(default)]
    pub auto_execute: Option<bool>,

    /// Enable paper trading (simulation mode)
    #[serde(default = "default_paper_trading")]
    pub paper_trading: Option<bool>,

    /// Enable macro economic filter
    #[serde(default)]
    pub macro_filter_enabled: Option<bool>,

    /// Macro data cache duration in seconds
    #[serde(default)]
    pub macro_cache_secs: Option<u64>,

    /// Telegram notification settings
    /// Aliases: "notification" for backward compatibility with existing config files
    #[serde(default, alias = "notification")]
    pub telegram_notification: Option<TradingNotificationConfig>,

    /// Macro agent configuration for intelligent analysis
    #[serde(default)]
    pub macro_agent: Option<MacroAgentConfig>,

    /// Trading loop configuration
    /// Aliases: "loop" for backward compatibility with existing config files
    #[serde(default, alias = "loop")]
    pub loop_config: Option<TradingLoopConfig>,

    /// Session schedule configuration
    #[serde(default)]
    pub schedule: Option<TradingScheduleConfig>,

    /// Multi-data-source configuration for market data
    #[serde(default)]
    pub data_sources: Option<DataSourcesConfig>,

    /// DEPRECATED: Legacy workflow_endpoint field for backward compatibility.
    /// Use Config::workflow_endpoint() instead. This field is ignored.
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    workflow_endpoint: Option<String>,
}

#[allow(deprecated)]
impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            port: default_trading_port(),
            host: default_host(),
            lixin_token: None,
            itick_api_key: None,
            smt_pairs: None,
            timeframes: None,
            min_accumulation_bars: None,
            manipulation_threshold: None,
            require_alignment: None,
            signal_expiry_minutes: None,
            max_positions: None,
            max_position_pct: None,
            max_daily_capital_pct: None,
            default_stop_loss_pct: None,
            auto_execute: None,
            paper_trading: Some(true),
            macro_filter_enabled: None,
            macro_cache_secs: None,
            telegram_notification: None,
            macro_agent: None,
            loop_config: None,
            schedule: None,
            data_sources: None,
            workflow_endpoint: None,
        }
    }
}

// ============================================================================
// Data Sources Configuration
// ============================================================================

/// Configuration for multi-data-source market data aggregation.
///
/// Enables automatic failover between data providers (iTick, Lixin)
/// based on health status and priority.
///
/// # Data Sources
/// - **iTick** (recommended primary): REST API with minute-level data
/// - **Lixin** (recommended backup): Daily data with high-quality fundamentals
/// - **Ashare** (deprecated): Web scraper, unstable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourcesConfig {
    /// List of data source configurations
    #[serde(default)]
    pub sources: Vec<DataSourceEntry>,

    /// Health check interval in seconds (default: 30)
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,

    /// Number of consecutive failures before marking provider unhealthy (default: 3)
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,

    /// Maximum retries per provider before failover (default: 2)
    #[serde(default = "default_data_source_retries")]
    pub max_retries: u32,

    /// Health check timeout in seconds (default: 10)
    #[serde(default = "default_health_check_timeout")]
    pub health_check_timeout_secs: u64,
}

impl Default for DataSourcesConfig {
    fn default() -> Self {
        Self {
            sources: vec![
                DataSourceEntry {
                    provider: "itick".to_string(),
                    enabled: true,
                    priority: 1,
                    config: None,
                },
                DataSourceEntry {
                    provider: "lixin".to_string(),
                    enabled: true,
                    priority: 2,
                    config: None,
                },
            ],
            health_check_interval_secs: default_health_check_interval(),
            unhealthy_threshold: default_unhealthy_threshold(),
            max_retries: default_data_source_retries(),
            health_check_timeout_secs: default_health_check_timeout(),
        }
    }
}

/// Configuration for a single data source provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceEntry {
    /// Provider name: "itick", "lixin", "ashare" (deprecated)
    pub provider: String,

    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Priority (lower = higher priority, 1 is highest)
    #[serde(default = "default_data_source_priority")]
    pub priority: u8,

    /// Provider-specific configuration (e.g., API key)
    #[serde(default)]
    pub config: Option<std::collections::HashMap<String, serde_json::Value>>,
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_unhealthy_threshold() -> u32 {
    3
}

fn default_data_source_retries() -> u32 {
    2
}

fn default_health_check_timeout() -> u64 {
    10
}

fn default_data_source_priority() -> u8 {
    10
}

/// SMT pair configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtPairConfig {
    /// Primary symbol (e.g., "000300.SH")
    pub primary: String,
    /// Reference symbol (e.g., "000905.SH")
    pub reference: String,
    /// Pair name for display
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
}

/// Trading notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingNotificationConfig {
    /// Enable notifications
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Telegram chat ID for notifications
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    /// Channel type (telegram, feishu, wecom, etc.)
    #[serde(default = "default_channel_type")]
    pub channel_type: String,
    /// Retry count for failed notifications
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    /// Notify on new signals
    #[serde(default = "default_true")]
    pub notify_signals: bool,
    /// Notify on order execution
    #[serde(default = "default_true")]
    pub notify_orders: bool,
    /// Notify on position changes
    #[serde(default = "default_true")]
    pub notify_positions: bool,

    /// DEPRECATED: Legacy channels_endpoint field for backward compatibility.
    /// Use Config::channels_endpoint() instead. This field is ignored.
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    channels_endpoint: Option<String>,
}

/// Macro agent configuration for intelligent analysis.
///
/// The macro agent integrates with CodeCoder to provide deep
/// macro-economic analysis when anomaly conditions are detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAgentConfig {
    /// Enable macro agent integration
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Request timeout in seconds
    #[serde(default = "default_macro_agent_timeout")]
    pub timeout_secs: u64,

    /// Cache duration for agent analysis in seconds
    #[serde(default = "default_macro_agent_cache")]
    pub cache_duration_secs: u64,

    /// Enable weekly macro reports
    #[serde(default = "default_true")]
    pub weekly_report_enabled: bool,

    /// Weekly report cron expression (default: Monday 9 AM)
    #[serde(default)]
    pub weekly_report_cron: Option<String>,

    /// Enable monthly macro reports
    #[serde(default = "default_true")]
    pub monthly_report_enabled: bool,

    /// Monthly report cron expression (default: 1st day 9 AM)
    #[serde(default)]
    pub monthly_report_cron: Option<String>,

    /// Enable daily morning reports (pre-market, 9:00 Beijing time)
    #[serde(default = "default_true")]
    pub daily_morning_enabled: bool,

    /// Daily morning report cron expression (default: 9:00 Beijing time)
    #[serde(default)]
    pub daily_morning_cron: Option<String>,

    /// Enable daily afternoon reports (post-market, 16:00 Beijing time)
    #[serde(default = "default_true")]
    pub daily_afternoon_enabled: bool,

    /// Daily afternoon report cron expression (default: 16:00 Beijing time)
    #[serde(default)]
    pub daily_afternoon_cron: Option<String>,

    /// Include index data in daily reports
    #[serde(default = "default_true")]
    pub include_index_data: bool,

    /// Index symbols to include in reports (default: major A-share indices)
    #[serde(default = "default_index_symbols")]
    pub index_symbols: Vec<String>,

    /// DEPRECATED: Legacy codecoder_endpoint field for backward compatibility.
    /// Use Config::codecoder_endpoint() instead. This field is ignored.
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    codecoder_endpoint: Option<String>,
}

impl Default for MacroAgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: default_macro_agent_timeout(),
            cache_duration_secs: default_macro_agent_cache(),
            weekly_report_enabled: true,
            weekly_report_cron: None,
            monthly_report_enabled: true,
            monthly_report_cron: None,
            daily_morning_enabled: true,
            daily_morning_cron: None,
            daily_afternoon_enabled: true,
            daily_afternoon_cron: None,
            include_index_data: true,
            index_symbols: default_index_symbols(),
            codecoder_endpoint: None,
        }
    }
}

fn default_macro_agent_timeout() -> u64 {
    30 // 30 seconds for agent calls
}

fn default_macro_agent_cache() -> u64 {
    3600 // 1 hour cache for agent analysis
}

fn default_index_symbols() -> Vec<String> {
    vec![
        "000300.SH".to_string(), // 沪深300
        "000905.SH".to_string(), // 中证500
        "000001.SH".to_string(), // 上证指数
    ]
}

fn default_trading_port() -> u16 {
    4434 // Part of 4430-4439 range for Rust microservices
}

fn default_channel_type() -> String {
    "telegram".to_string()
}

fn default_retry_count() -> u32 {
    3
}

fn default_paper_trading() -> Option<bool> {
    Some(true)
}

/// Trading loop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingLoopConfig {
    /// Main loop interval in seconds (for signal scanning)
    #[serde(default = "default_loop_interval")]
    pub interval_secs: u64,

    /// Price check interval in seconds (for stop-loss/take-profit)
    #[serde(default = "default_price_check_interval")]
    pub price_check_interval_secs: u64,

    /// Enable automatic order execution
    #[serde(default)]
    pub auto_execute: bool,
}

impl Default for TradingLoopConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_loop_interval(),
            price_check_interval_secs: default_price_check_interval(),
            auto_execute: false,
        }
    }
}

fn default_loop_interval() -> u64 {
    5 // 5 seconds for signal scanning
}

fn default_price_check_interval() -> u64 {
    1 // 1 second for price checking
}

/// Trading schedule configuration for automated session lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingScheduleConfig {
    /// Enable scheduled session control
    #[serde(default)]
    pub enabled: bool,

    /// Session start cron expression (default: 9:25 on weekdays after auction)
    #[serde(default = "default_session_start_cron")]
    pub session_start: String,

    /// Session pause cron expression (default: 11:30 lunch break)
    #[serde(default = "default_session_pause_cron")]
    pub session_pause: String,

    /// Session resume cron expression (default: 13:00 afternoon)
    #[serde(default = "default_session_resume_cron")]
    pub session_resume: String,

    /// Session stop cron expression (default: 15:00 market close)
    #[serde(default = "default_session_stop_cron")]
    pub session_stop: String,

    /// Daily review cron expression (default: 15:30 after market)
    #[serde(default = "default_daily_review_cron")]
    pub daily_review: String,

    /// Auto-start session on schedule
    #[serde(default = "default_true")]
    pub auto_start: bool,

    /// Persist state across restarts
    #[serde(default = "default_true")]
    pub persist_state: bool,

    /// Default trading mode (paper or live)
    #[serde(default = "default_trading_mode")]
    pub default_mode: String,
}

impl Default for TradingScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            session_start: default_session_start_cron(),
            session_pause: default_session_pause_cron(),
            session_resume: default_session_resume_cron(),
            session_stop: default_session_stop_cron(),
            daily_review: default_daily_review_cron(),
            auto_start: true,
            persist_state: true,
            default_mode: default_trading_mode(),
        }
    }
}

fn default_session_start_cron() -> String {
    "0 25 9 * * 1-5".to_string() // 9:25 on weekdays
}

fn default_session_pause_cron() -> String {
    "0 30 11 * * 1-5".to_string() // 11:30 on weekdays
}

fn default_session_resume_cron() -> String {
    "0 0 13 * * 1-5".to_string() // 13:00 on weekdays
}

fn default_session_stop_cron() -> String {
    "0 0 15 * * 1-5".to_string() // 15:00 on weekdays
}

fn default_daily_review_cron() -> String {
    "0 30 15 * * 1-5".to_string() // 15:30 on weekdays
}

fn default_trading_mode() -> String {
    "paper".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        // Test service port accessors
        assert_eq!(config.gateway_port(), 4430);
        assert_eq!(config.channels_port(), 4431);
        assert_eq!(config.workflow_port(), 4432);
        assert_eq!(config.codecoder_port(), 4400);
        // Test other defaults
        assert!(config.codecoder.enabled);
        assert!(config.agent.enabled);
        assert!(config.tools.shell_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        // Test that round-trip works with accessor methods
        assert_eq!(parsed.gateway_port(), config.gateway_port());
        assert_eq!(parsed.channels_port(), config.channels_port());
        assert_eq!(parsed.llm.default, config.llm.default);
    }

    #[test]
    fn test_get_api_key() {
        let mut config = Config::default();
        config.secrets.llm.anthropic = Some("sk-ant-123".into());
        config.secrets.llm.openai = Some("sk-openai-456".into());

        assert_eq!(config.get_api_key("anthropic"), Some("sk-ant-123".to_string()));
        assert_eq!(config.get_api_key("openai"), Some("sk-openai-456".to_string()));
        assert_eq!(config.get_api_key("unknown"), None);
    }

    #[test]
    fn test_get_api_key_aliases() {
        let mut config = Config::default();
        config.secrets.llm.google = Some("google-key".into());
        config.secrets.llm.xai = Some("xai-key".into());
        config.secrets.llm.together = Some("together-key".into());

        // Test aliases
        assert_eq!(config.get_api_key("gemini"), Some("google-key".to_string()));
        assert_eq!(config.get_api_key("grok"), Some("xai-key".to_string()));
        assert_eq!(config.get_api_key("together-ai"), Some("together-key".to_string()));
    }

    #[test]
    fn test_llm_config_defaults() {
        let llm = LlmConfig::default();
        assert_eq!(llm.default, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(llm.retries, 2);
        assert_eq!(llm.backoff_ms, 1000);
        assert!(llm.fallbacks.is_empty());
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
    fn test_llm_ollama_config_defaults() {
        let ollama = LlmOllamaConfig::default();
        assert_eq!(ollama.base_url, "http://localhost:11434");
        assert_eq!(ollama.default_model, "llama3");
        assert_eq!(ollama.timeout_secs, 300);
    }

    #[test]
    fn test_partial_config_deserialization() {
        // Test that partial JSON with only some fields works (uses defaults for rest)
        let json = r#"{"services": {"gateway": {"port": 8080}}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.gateway_port(), 8080);
        assert_eq!(config.bind_address(), "127.0.0.1"); // default
        assert_eq!(config.channels_port(), 4431); // default
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
    fn test_llm_config() {
        // Test that LLM config can be parsed with new structure
        let json = r#"{
            "llm": {
                "default": "openai/gpt-4-turbo",
                "retries": 3,
                "backoff_ms": 2000,
                "fallbacks": ["anthropic/claude-3-sonnet"],
                "ollama": {
                    "base_url": "http://192.168.1.100:11434"
                },
                "providers": {
                    "my-provider": {
                        "base_url": "https://my-api.example.com",
                        "models": ["model-1", "model-2"]
                    }
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.llm.default, "openai/gpt-4-turbo");
        assert_eq!(config.llm.retries, 3);
        assert_eq!(config.llm.backoff_ms, 2000);
        assert_eq!(config.llm.fallbacks.len(), 1);
        assert_eq!(config.llm.ollama.base_url, "http://192.168.1.100:11434");
        assert!(config.llm.providers.contains_key("my-provider"));
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

    #[test]
    fn test_ticket_config() {
        let json = r#"{
            "workflow": {
                "ticket": {
                    "enabled": true,
                    "github": {
                        "default_repo": "company/product",
                        "bug_labels": ["bug", "P1"],
                        "feature_labels": ["enhancement", "nice-to-have"]
                    },
                    "notification": {
                        "enabled": true,
                        "channel_type": "feishu",
                        "channel_id": "dev-group-123"
                    }
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.workflow.ticket.enabled);

        let github = config.workflow.ticket.github.unwrap();
        assert_eq!(github.default_repo, "company/product");
        assert_eq!(github.bug_labels, vec!["bug", "P1"]);
        assert_eq!(github.feature_labels, vec!["enhancement", "nice-to-have"]);

        let notification = config.workflow.ticket.notification.unwrap();
        assert!(notification.enabled);
        assert_eq!(notification.channel_type, "feishu");
        assert_eq!(notification.channel_id, "dev-group-123");
    }

    #[test]
    fn test_ticket_config_defaults() {
        let json = r#"{
            "workflow": {
                "ticket": {
                    "enabled": true,
                    "github": {
                        "default_repo": "company/product"
                    }
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let github = config.workflow.ticket.github.unwrap();

        // Should use default labels
        assert_eq!(github.bug_labels, vec!["bug", "triage"]);
        assert_eq!(github.feature_labels, vec!["enhancement"]);
    }

    #[test]
    fn test_monitor_config() {
        let json = r#"{
            "workflow": {
                "monitor": {
                    "enabled": true,
                    "tasks": [
                        {
                            "id": "daily-competitive",
                            "name": "每日竞品早报",
                            "schedule": "0 0 9 * * *",
                            "sources": [
                                {
                                    "id": "competitor-a",
                                    "name": "竞品A官网",
                                    "url": "https://competitor-a.com/news",
                                    "source_type": "website",
                                    "selector": ".news-list"
                                },
                                {
                                    "id": "competitor-b-blog",
                                    "name": "竞品B博客",
                                    "url": "https://competitor-b.com/blog/feed",
                                    "source_type": "rss"
                                }
                            ],
                            "notification": {
                                "channel_type": "feishu",
                                "channel_id": "ops-group-123",
                                "template": "daily_brief"
                            }
                        }
                    ]
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.workflow.monitor.enabled);
        assert_eq!(config.workflow.monitor.tasks.len(), 1);

        let task = &config.workflow.monitor.tasks[0];
        assert_eq!(task.id, "daily-competitive");
        assert_eq!(task.name, "每日竞品早报");
        assert_eq!(task.schedule, "0 0 9 * * *");
        assert_eq!(task.sources.len(), 2);

        let source1 = &task.sources[0];
        assert_eq!(source1.id, "competitor-a");
        assert_eq!(source1.source_type, "website");
        assert_eq!(source1.selector, Some(".news-list".to_string()));

        let source2 = &task.sources[1];
        assert_eq!(source2.source_type, "rss");
        assert!(source2.selector.is_none());

        assert_eq!(task.notification.channel_type, "feishu");
        assert_eq!(task.notification.channel_id, "ops-group-123");
        assert_eq!(task.notification.template, "daily_brief");
    }

    #[test]
    fn test_monitor_config_defaults() {
        let json = r#"{
            "workflow": {
                "monitor": {
                    "enabled": true,
                    "tasks": [
                        {
                            "id": "test",
                            "name": "Test",
                            "schedule": "0 0 9 * * *",
                            "sources": [
                                {
                                    "id": "src1",
                                    "name": "Source 1",
                                    "url": "https://example.com"
                                }
                            ],
                            "notification": {
                                "channel_type": "feishu",
                                "channel_id": "test-123"
                            }
                        }
                    ]
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let task = &config.workflow.monitor.tasks[0];

        // Should use default source_type and template
        assert_eq!(task.sources[0].source_type, "website");
        assert_eq!(task.notification.template, "daily_brief");
    }

    #[test]
    fn test_capture_config() {
        let json = r##"{
            "channels": {
                "capture": {
                    "enabled": true,
                    "feishu_docs": {
                        "app_id": "cli_xxx",
                        "app_secret": "secret_xxx",
                        "folder_token": "fldcnXXX"
                    },
                    "notion": {
                        "token": "secret_notion",
                        "database_id": "db_123"
                    },
                    "auto_capture": {
                        "capture_forwarded": true,
                        "capture_links": false,
                        "trigger_prefixes": ["#收藏", "#save", "@save"]
                    }
                }
            }
        }"##;
        let config: Config = serde_json::from_str(json).unwrap();
        let capture = config.channels.capture.unwrap();

        assert!(capture.enabled);
        assert!(capture.feishu_docs.is_some());
        assert!(capture.notion.is_some());

        let feishu_docs = capture.feishu_docs.unwrap();
        assert_eq!(feishu_docs.app_id, "cli_xxx");
        assert_eq!(feishu_docs.folder_token, "fldcnXXX");

        let notion = capture.notion.unwrap();
        assert_eq!(notion.database_id, "db_123");

        assert!(capture.auto_capture.capture_forwarded);
        assert!(!capture.auto_capture.capture_links);
        assert_eq!(capture.auto_capture.trigger_prefixes.len(), 3);
    }

    #[test]
    fn test_capture_config_defaults() {
        let json = r#"{
            "channels": {
                "capture": {
                    "enabled": true
                }
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let capture = config.channels.capture.unwrap();

        assert!(capture.enabled);
        assert!(capture.feishu_docs.is_none());
        assert!(capture.notion.is_none());
        assert!(!capture.auto_capture.capture_forwarded);
        assert!(!capture.auto_capture.capture_links);
        assert!(capture.auto_capture.trigger_prefixes.is_empty());
    }

    #[test]
    fn test_sandbox_config_defaults() {
        let sandbox = SandboxConfig::default();
        assert!(sandbox.enabled);
        assert_eq!(sandbox.docker_socket, "/var/run/docker.sock");
        assert_eq!(sandbox.default_timeout_secs, 60);
        assert_eq!(sandbox.max_memory_mb, 256);
        assert!(!sandbox.network_enabled);
    }

    #[test]
    fn test_sandbox_config() {
        let json = r#"{
            "sandbox": {
                "enabled": true,
                "docker_socket": "/custom/docker.sock",
                "default_timeout_secs": 120,
                "max_memory_mb": 512,
                "network_enabled": true
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.sandbox.enabled);
        assert_eq!(config.sandbox.docker_socket, "/custom/docker.sock");
        assert_eq!(config.sandbox.default_timeout_secs, 120);
        assert_eq!(config.sandbox.max_memory_mb, 512);
        assert!(config.sandbox.network_enabled);
    }

    #[test]
    fn test_qdrant_config_defaults() {
        let qdrant = QdrantConfig::default();
        assert!(!qdrant.enabled);
        assert_eq!(qdrant.url, "http://localhost:6333");
        assert_eq!(qdrant.collection, "codecoder_memory");
        assert_eq!(qdrant.embedding_model, "openai");
    }

    #[test]
    fn test_qdrant_config() {
        let json = r#"{
            "qdrant": {
                "enabled": true,
                "url": "http://qdrant.local:6333",
                "collection": "custom_collection",
                "embedding_model": "cohere"
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.qdrant.enabled);
        assert_eq!(config.qdrant.url, "http://qdrant.local:6333");
        assert_eq!(config.qdrant.collection, "custom_collection");
        assert_eq!(config.qdrant.embedding_model, "cohere");
    }

    #[test]
    fn test_config_with_sandbox_and_qdrant() {
        let config = Config::default();

        // Sandbox defaults
        assert!(config.sandbox.enabled);
        assert!(!config.sandbox.network_enabled);

        // Qdrant defaults
        assert!(!config.qdrant.enabled);
        assert_eq!(config.qdrant.url, "http://localhost:6333");
    }

    #[test]
    fn test_hitl_config_defaults() {
        let hitl = HitLConfig::default();
        assert!(hitl.enabled);
        assert!(hitl.default_approvers.is_empty());
        assert!(hitl.callback_base_url.is_empty());
        assert_eq!(hitl.db_path, "~/.codecoder/hitl.db");
    }

    #[test]
    fn test_hitl_config_from_json() {
        let json = r#"{
            "hitl": {
                "enabled": true,
                "default_approvers": ["user1", "user2"],
                "channels_endpoint": "http://channels.local:4431",
                "callback_base_url": "https://api.example.com",
                "db_path": "/var/lib/hitl/store.db"
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.hitl.enabled);
        assert_eq!(config.hitl.default_approvers, vec!["user1", "user2"]);
        // Note: channels_endpoint is now deprecated; use config.channels_endpoint() instead
        assert_eq!(config.hitl.callback_base_url, "https://api.example.com");
        assert_eq!(config.hitl.db_path, "/var/lib/hitl/store.db");
    }

    #[test]
    fn test_hitl_config_partial_json() {
        // Test that partial JSON uses defaults for missing fields
        let json = r#"{
            "hitl": {
                "enabled": false,
                "default_approvers": ["admin"]
            }
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(!config.hitl.enabled);
        assert_eq!(config.hitl.default_approvers, vec!["admin"]);
        // These should use defaults
        assert!(config.hitl.callback_base_url.is_empty());
        assert_eq!(config.hitl.db_path, "~/.codecoder/hitl.db");
    }

    #[test]
    fn test_hitl_config_empty_json() {
        // Test that empty hitl section uses all defaults
        let json = r#"{"hitl": {}}"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.hitl.enabled);
        assert!(config.hitl.default_approvers.is_empty());
    }

    #[test]
    fn test_config_without_hitl_section() {
        // Test that config without hitl section uses defaults
        let json = r#"{"gateway": {"port": 8080}}"#;
        let config: Config = serde_json::from_str(json).unwrap();

        // HitL should use all defaults
        assert!(config.hitl.enabled);
        assert!(config.hitl.default_approvers.is_empty());
        assert_eq!(config.hitl.db_path, "~/.codecoder/hitl.db");
    }

    #[test]
    fn test_config_with_hitl_serialization() {
        let mut config = Config::default();
        config.hitl.enabled = false;
        config.hitl.default_approvers = vec!["approver1".to_string()];
        config.hitl.callback_base_url = "https://callback.example.com".to_string();

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert!(!parsed.hitl.enabled);
        assert_eq!(parsed.hitl.default_approvers, vec!["approver1"]);
        assert_eq!(parsed.hitl.callback_base_url, "https://callback.example.com");
    }

    #[test]
    fn test_get_api_key_provider_priority() {
        // Test that provider field takes priority over secrets.llm
        let mut config = Config::default();

        // Set key in both places
        config.secrets.llm.deepseek = Some("secret-key".into());
        config.provider.insert(
            "deepseek".into(),
            UnifiedProviderConfig {
                options: Some(UnifiedProviderOptions {
                    api_key: Some("provider-key".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        // Provider field should take priority
        assert_eq!(
            config.get_api_key("deepseek"),
            Some("provider-key".to_string())
        );
    }

    #[test]
    fn test_get_api_key_fallback_to_secrets() {
        // Test backwards compatibility: falls back to secrets.llm if provider not set
        let mut config = Config::default();
        config.secrets.llm.anthropic = Some("secret-ant-key".into());

        // No provider config for anthropic
        assert_eq!(
            config.get_api_key("anthropic"),
            Some("secret-ant-key".to_string())
        );
    }

    #[test]
    fn test_get_api_key_provider_with_alias() {
        // Test that aliases work with provider field
        let mut config = Config::default();
        config.provider.insert(
            "google".into(),
            UnifiedProviderConfig {
                options: Some(UnifiedProviderOptions {
                    api_key: Some("google-provider-key".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        // "gemini" alias should find "google" in provider
        assert_eq!(
            config.get_api_key("gemini"),
            Some("google-provider-key".to_string())
        );
    }

    #[test]
    fn test_get_api_key_custom_provider() {
        // Test custom providers that only exist in provider field
        let mut config = Config::default();
        config.provider.insert(
            "zhipu-ai".into(),
            UnifiedProviderConfig {
                name: Some("Zhipu AI".into()),
                options: Some(UnifiedProviderOptions {
                    api_key: Some("zhipu-custom-key".into()),
                    base_url: Some("https://open.bigmodel.cn/api".into()),
                }),
                ..Default::default()
            },
        );

        assert_eq!(
            config.get_api_key("zhipu-ai"),
            Some("zhipu-custom-key".to_string())
        );
    }
}
