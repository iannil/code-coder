use crate::security::AutonomyLevel;
use anyhow::{Context, Result};
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── JSON config structures for CodeCoder integration ──────────────────

/// Intermediate structure for deserializing `ZeroBot` config from JSON
#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonConfig {
    default_provider: Option<String>,
    default_model: Option<String>,
    default_temperature: Option<f64>,
    workspace_dir: Option<String>,

    #[serde(default)]
    observability: Option<ZeroBotJsonObservability>,
    #[serde(default)]
    autonomy: Option<ZeroBotJsonAutonomy>,
    #[serde(default)]
    runtime: Option<ZeroBotJsonRuntime>,
    #[serde(default)]
    reliability: Option<ZeroBotJsonReliability>,
    #[serde(default)]
    heartbeat: Option<ZeroBotJsonHeartbeat>,
    #[serde(default)]
    memory: Option<ZeroBotJsonMemory>,
    #[serde(default)]
    gateway: Option<ZeroBotJsonGateway>,
    #[serde(default)]
    tunnel: Option<ZeroBotJsonTunnel>,
    #[serde(default)]
    channels: Option<ZeroBotJsonChannels>,
    #[serde(default)]
    browser: Option<ZeroBotJsonBrowser>,
    #[serde(default)]
    identity: Option<ZeroBotJsonIdentity>,
    #[serde(default)]
    codecoder: Option<ZeroBotJsonCodeCoder>,
    #[serde(default)]
    session: Option<ZeroBotJsonSession>,
    #[serde(default)]
    tts: Option<ZeroBotJsonTts>,
    #[serde(default)]
    mcp: Option<ZeroBotJsonMcp>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonObservability {
    backend: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonAutonomy {
    level: Option<String>,
    workspace_only: Option<bool>,
    allowed_commands: Option<Vec<String>>,
    forbidden_paths: Option<Vec<String>>,
    max_actions_per_hour: Option<u32>,
    max_cost_per_day_cents: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonRuntime {
    kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonReliability {
    provider_retries: Option<u32>,
    provider_backoff_ms: Option<u64>,
    fallback_providers: Option<Vec<String>>,
    channel_initial_backoff_secs: Option<u64>,
    channel_max_backoff_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonHeartbeat {
    enabled: Option<bool>,
    interval_minutes: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonMemory {
    backend: Option<String>,
    auto_save: Option<bool>,
    hygiene_enabled: Option<bool>,
    embedding_provider: Option<String>,
    embedding_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonGateway {
    port: Option<u16>,
    host: Option<String>,
    require_pairing: Option<bool>,
    allow_public_bind: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonTunnel {
    provider: Option<String>,
    cloudflare: Option<ZeroBotJsonTunnelCloudflare>,
    tailscale: Option<ZeroBotJsonTunnelTailscale>,
    ngrok: Option<ZeroBotJsonTunnelNgrok>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonTunnelCloudflare {
    token: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonTunnelTailscale {
    funnel: Option<bool>,
    hostname: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonTunnelNgrok {
    auth_token: String,
    domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonChannels {
    cli: Option<bool>,
    telegram: Option<ZeroBotJsonTelegram>,
    discord: Option<ZeroBotJsonDiscord>,
    slack: Option<ZeroBotJsonSlack>,
    whatsapp: Option<ZeroBotJsonWhatsApp>,
    feishu: Option<ZeroBotJsonFeishu>,
}

/// Feishu config fields - some reserved for future use
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ZeroBotJsonFeishu {
    #[serde(default)]
    enabled: Option<bool>,
    app_id: String,
    app_secret: String,
    #[serde(default)]
    encrypt_key: Option<String>,
    #[serde(default)]
    verification_token: Option<String>,
    #[serde(default)]
    allowed_users: Vec<String>,
    #[serde(default)]
    use_lark_api: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonTelegram {
    bot_token: String,
    #[serde(default)]
    allowed_users: Vec<String>,
    #[serde(default)]
    voice: Option<ZeroBotJsonTelegramVoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonTelegramVoice {
    #[serde(default = "default_json_true")]
    enabled: bool,
    #[serde(default)]
    stt_provider: Option<String>,
    #[serde(default)]
    stt_api_key: Option<String>,
    #[serde(default)]
    stt_model: Option<String>,
    #[serde(default)]
    stt_base_url: Option<String>,
}

fn default_json_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonDiscord {
    bot_token: String,
    guild_id: Option<String>,
    #[serde(default)]
    allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonSlack {
    bot_token: String,
    app_token: Option<String>,
    channel_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ZeroBotJsonWhatsApp {
    access_token: String,
    phone_number_id: String,
    verify_token: String,
    #[serde(default)]
    allowed_numbers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonBrowser {
    enabled: Option<bool>,
    allowed_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonIdentity {
    format: Option<String>,
    aieos_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonCodeCoder {
    enabled: Option<bool>,
    endpoint: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonSession {
    enabled: Option<bool>,
    context_window: Option<usize>,
    compact_threshold: Option<f32>,
    keep_recent: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonTts {
    enabled: Option<bool>,
    provider: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    voice: Option<String>,
    #[serde(alias = "default_voice")]
    default_voice: Option<String>,
    base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ZeroBotJsonMcp {
    #[serde(default)]
    servers: Option<std::collections::HashMap<String, ZeroBotJsonMcpServer>>,
    #[serde(default)]
    server_enabled: Option<bool>,
    #[serde(default)]
    server_api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum ZeroBotJsonMcpServer {
    #[serde(rename = "local")]
    Local {
        command: Vec<String>,
        #[serde(default)]
        environment: Option<std::collections::HashMap<String, String>>,
        #[serde(default = "default_mcp_enabled")]
        enabled: bool,
    },
    #[serde(rename = "remote")]
    Remote {
        url: String,
        #[serde(default)]
        headers: Option<std::collections::HashMap<String, String>>,
        #[serde(default = "default_mcp_enabled")]
        enabled: bool,
    },
}

fn default_mcp_enabled() -> bool {
    true
}

/// Strip JSONC-style comments (// and /* */)
fn strip_json_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' && !in_string {
            in_string = true;
            result.push(c);
        } else if c == '"' && in_string {
            in_string = false;
            result.push(c);
        } else if c == '\\' && in_string {
            // Handle escape sequences in strings
            result.push(c);
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else if !in_string && c == '/' {
            if chars.peek() == Some(&'/') {
                // Single-line comment: skip to end of line
                chars.next(); // consume second /
                while chars.next().is_some_and(|ch| ch != '\n') {}
                result.push('\n');
            } else if chars.peek() == Some(&'*') {
                // Multi-line comment: skip to */
                chars.next(); // consume *
                while let Some(ch) = chars.next() {
                    if ch == '*' && chars.peek() == Some(&'/') {
                        chars.next(); // consume /
                        break;
                    }
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Resolve {env:VAR} placeholders in a string
fn resolve_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    // Find all {env:VAR} patterns and replace them
    while let Some(start) = result.find("{env:") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 5..start + end];
            let value = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..start], value, &result[start + end + 1..]);
        } else {
            break;
        }
    }
    result
}

// ── Top-level config ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace_dir: PathBuf,
    pub config_path: PathBuf,
    pub api_key: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default)]
    pub autonomy: AutonomyConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    #[serde(default)]
    pub reliability: ReliabilityConfig,

    #[serde(default)]
    pub heartbeat: HeartbeatConfig,

    #[serde(default)]
    pub channels_config: ChannelsConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub tunnel: TunnelConfig,

    #[serde(default)]
    pub gateway: GatewayConfig,

    #[serde(default)]
    pub secrets: SecretsConfig,

    #[serde(default)]
    pub vault: VaultConfig,

    #[serde(default)]
    pub browser: BrowserConfig,

    #[serde(default)]
    pub codecoder: CodeCoderConfig,

    #[serde(default)]
    pub identity: IdentityConfig,

    #[serde(default)]
    pub session: SessionConfig,

    #[serde(default)]
    pub tts: TtsConfig,

    #[serde(default)]
    pub voice_wake: VoiceWakeConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    /// Workflow service configuration
    pub workflow_host: Option<String>,
    pub workflow_port: Option<u16>,
}

// ── Identity (AIEOS / OpenClaw format) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Identity format: "openclaw" (default) or "aieos"
    #[serde(default = "default_identity_format")]
    pub format: String,
    /// Path to AIEOS JSON file (relative to workspace)
    #[serde(default)]
    pub aieos_path: Option<String>,
    /// Inline AIEOS JSON (alternative to file path)
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

fn default_identity_format() -> String {
    "openclaw".into()
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            format: default_identity_format(),
            aieos_path: None,
            aieos_inline: None,
        }
    }
}

// ── Gateway security ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway port (default: 8080)
    #[serde(default = "default_gateway_port")]
    pub port: u16,
    /// Gateway host (default: 127.0.0.1)
    #[serde(default = "default_gateway_host")]
    pub host: String,
    /// Require pairing before accepting requests (default: true)
    #[serde(default = "default_true")]
    pub require_pairing: bool,
    /// Allow binding to non-localhost without a tunnel (default: false)
    #[serde(default)]
    pub allow_public_bind: bool,
    /// Paired bearer tokens (managed automatically, not user-edited)
    #[serde(default)]
    pub paired_tokens: Vec<String>,
}

fn default_gateway_port() -> u16 {
    3000
}

fn default_gateway_host() -> String {
    "127.0.0.1".into()
}

fn default_true() -> bool {
    true
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_gateway_port(),
            host: default_gateway_host(),
            require_pairing: true,
            allow_public_bind: false,
            paired_tokens: Vec::new(),
        }
    }
}

// ── Secrets (encrypted credential store) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Enable encryption for API keys and tokens in config.toml
    #[serde(default = "default_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

// ── Credential Vault ────────────────────────────────────────────

/// Configuration for the credential vault (API keys, OAuth, login creds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Enable the credential vault (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Auto-inject credentials into HTTP requests (default: true)
    #[serde(default = "default_true")]
    pub auto_inject: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_inject: true,
        }
    }
}

// ── Browser (friendly-service browsing only) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserConfig {
    /// Enable `browser_open` tool (opens URLs in Brave without scraping)
    #[serde(default)]
    pub enabled: bool,
    /// Allowed domains for `browser_open` (exact or subdomain match)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Browser session name (for agent-browser automation)
    #[serde(default)]
    pub session_name: Option<String>,
}

// ── CodeCoder (bridge to CodeCoder AI agents) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCoderConfig {
    /// Enable `CodeCoder` tool for invoking 23 AI agents
    #[serde(default)]
    pub enabled: bool,
    /// `CodeCoder` API endpoint (default: `http://127.0.0.1:4400`)
    #[serde(default = "default_codecoder_endpoint")]
    pub endpoint: String,
    /// API key for authenticating with `CodeCoder` server
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_codecoder_endpoint() -> String {
    // Use 127.0.0.1 instead of localhost to avoid IPv6 resolution issues.
    // When using 'localhost', the system may first try IPv6 (::1) which fails
    // if the server only listens on IPv4, causing SSE connection failures.
    "http://127.0.0.1:4400".into()
}

impl Default for CodeCoderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_codecoder_endpoint(),
            api_key: None,
        }
    }
}

// ── MCP (Model Context Protocol) ────────────────────────────────

/// MCP configuration for connecting to external MCP servers and exposing tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct McpConfig {
    /// MCP servers to connect to
    #[serde(default)]
    pub servers: std::collections::HashMap<String, McpServerConfig>,
    /// Enable MCP server mode (expose `ZeroBot` tools via MCP)
    #[serde(default)]
    pub server_enabled: bool,
    /// API key for MCP server authentication
    #[serde(default)]
    pub server_api_key: Option<String>,
}


/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    /// Local MCP server via stdio
    #[serde(rename = "local")]
    Local {
        /// Command to spawn the MCP server
        command: Vec<String>,
        /// Environment variables for the process
        #[serde(default)]
        environment: Option<std::collections::HashMap<String, String>>,
        /// Whether this server is enabled
        #[serde(default = "default_true")]
        enabled: bool,
    },
    /// Remote MCP server via HTTP
    #[serde(rename = "remote")]
    Remote {
        /// URL of the MCP server
        url: String,
        /// HTTP headers for authentication
        #[serde(default)]
        headers: Option<std::collections::HashMap<String, String>>,
        /// Whether this server is enabled
        #[serde(default = "default_true")]
        enabled: bool,
    },
}

impl McpServerConfig {
    /// Check if this server is enabled
    pub fn enabled(&self) -> bool {
        match self {
            McpServerConfig::Local { enabled, .. } | McpServerConfig::Remote { enabled, .. } => {
                *enabled
            }
        }
    }
}

// ── Session (conversation context management) ──────────────────

/// Session management configuration for multi-turn conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Whether session management is enabled (default: true)
    #[serde(default = "default_session_enabled")]
    pub enabled: bool,

    /// Model context window size in tokens (default: 128000)
    #[serde(default = "default_context_window")]
    pub context_window: usize,

    /// Threshold ratio to trigger auto-compaction (default: 0.8)
    #[serde(default = "default_compact_threshold")]
    pub compact_threshold: f32,

    /// Number of recent messages to keep after compaction (default: 5)
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}

fn default_session_enabled() -> bool {
    true
}

fn default_context_window() -> usize {
    128_000
}

fn default_compact_threshold() -> f32 {
    0.8
}

fn default_keep_recent() -> usize {
    5
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: default_session_enabled(),
            context_window: default_context_window(),
            compact_threshold: default_compact_threshold(),
            keep_recent: default_keep_recent(),
        }
    }
}

// ── Memory ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// "sqlite" | "markdown" | "none"
    pub backend: String,
    /// Auto-save conversation context to memory
    pub auto_save: bool,
    /// Run memory/session hygiene (archiving + retention cleanup)
    #[serde(default = "default_hygiene_enabled")]
    pub hygiene_enabled: bool,
    /// Archive daily/session files older than this many days
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u32,
    /// Purge archived files older than this many days
    #[serde(default = "default_purge_after_days")]
    pub purge_after_days: u32,
    /// For sqlite backend: prune conversation rows older than this many days
    #[serde(default = "default_conversation_retention_days")]
    pub conversation_retention_days: u32,
    /// Embedding provider: "none" | "openai" | "custom:URL"
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    /// Embedding model name (e.g. "text-embedding-3-small")
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Embedding vector dimensions
    #[serde(default = "default_embedding_dims")]
    pub embedding_dimensions: usize,
    /// Weight for vector similarity in hybrid search (0.0–1.0)
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    /// Weight for keyword BM25 in hybrid search (0.0–1.0)
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f64,
    /// Max embedding cache entries before LRU eviction
    #[serde(default = "default_cache_size")]
    pub embedding_cache_size: usize,
    /// Max tokens per chunk for document splitting
    #[serde(default = "default_chunk_size")]
    pub chunk_max_tokens: usize,
}

fn default_embedding_provider() -> String {
    "none".into()
}
fn default_hygiene_enabled() -> bool {
    true
}
fn default_archive_after_days() -> u32 {
    7
}
fn default_purge_after_days() -> u32 {
    30
}
fn default_conversation_retention_days() -> u32 {
    30
}
fn default_embedding_model() -> String {
    "text-embedding-3-small".into()
}
fn default_embedding_dims() -> usize {
    1536
}
fn default_vector_weight() -> f64 {
    0.7
}
fn default_keyword_weight() -> f64 {
    0.3
}
fn default_cache_size() -> usize {
    10_000
}
fn default_chunk_size() -> usize {
    512
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            auto_save: true,
            hygiene_enabled: default_hygiene_enabled(),
            archive_after_days: default_archive_after_days(),
            purge_after_days: default_purge_after_days(),
            conversation_retention_days: default_conversation_retention_days(),
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
            embedding_dimensions: default_embedding_dims(),
            vector_weight: default_vector_weight(),
            keyword_weight: default_keyword_weight(),
            embedding_cache_size: default_cache_size(),
            chunk_max_tokens: default_chunk_size(),
        }
    }
}

// ── Observability ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// "none" | "log" | "prometheus" | "otel"
    pub backend: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            backend: "none".into(),
        }
    }
}

// ── Autonomy / Security ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    pub level: AutonomyLevel,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: u32,
    pub max_cost_per_day_cents: u32,
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::Supervised,
            workspace_only: true,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            max_actions_per_hour: 20,
            max_cost_per_day_cents: 500,
        }
    }
}

// ── Runtime ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime kind (currently supported: "native").
    ///
    /// Reserved values (not implemented yet): "docker", "cloudflare".
    pub kind: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: "native".into(),
        }
    }
}

// ── Reliability / supervision ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Retries per provider before failing over.
    #[serde(default = "default_provider_retries")]
    pub provider_retries: u32,
    /// Base backoff (ms) for provider retry delay.
    #[serde(default = "default_provider_backoff_ms")]
    pub provider_backoff_ms: u64,
    /// Fallback provider chain (e.g. `["anthropic", "openai"]`).
    #[serde(default)]
    pub fallback_providers: Vec<String>,
    /// Initial backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_secs")]
    pub channel_initial_backoff_secs: u64,
    /// Max backoff for channel/daemon restarts.
    #[serde(default = "default_channel_backoff_max_secs")]
    pub channel_max_backoff_secs: u64,
    /// Scheduler polling cadence in seconds.
    #[serde(default = "default_scheduler_poll_secs")]
    pub scheduler_poll_secs: u64,
    /// Max retries for cron job execution attempts.
    #[serde(default = "default_scheduler_retries")]
    pub scheduler_retries: u32,
}

fn default_provider_retries() -> u32 {
    2
}

fn default_provider_backoff_ms() -> u64 {
    500
}

fn default_channel_backoff_secs() -> u64 {
    2
}

fn default_channel_backoff_max_secs() -> u64 {
    60
}

fn default_scheduler_poll_secs() -> u64 {
    15
}

fn default_scheduler_retries() -> u32 {
    2
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            provider_retries: default_provider_retries(),
            provider_backoff_ms: default_provider_backoff_ms(),
            fallback_providers: Vec::new(),
            channel_initial_backoff_secs: default_channel_backoff_secs(),
            channel_max_backoff_secs: default_channel_backoff_max_secs(),
            scheduler_poll_secs: default_scheduler_poll_secs(),
            scheduler_retries: default_scheduler_retries(),
        }
    }
}

// ── Heartbeat ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_minutes: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 30,
        }
    }
}

// ── Tunnel ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// "none", "cloudflare", "tailscale", "ngrok", "custom"
    pub provider: String,

    #[serde(default)]
    pub cloudflare: Option<CloudflareTunnelConfig>,

    #[serde(default)]
    pub tailscale: Option<TailscaleTunnelConfig>,

    #[serde(default)]
    pub ngrok: Option<NgrokTunnelConfig>,

    #[serde(default)]
    pub custom: Option<CustomTunnelConfig>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            cloudflare: None,
            tailscale: None,
            ngrok: None,
            custom: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareTunnelConfig {
    /// Cloudflare Tunnel token (from Zero Trust dashboard)
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleTunnelConfig {
    /// Use Tailscale Funnel (public internet) vs Serve (tailnet only)
    #[serde(default)]
    pub funnel: bool,
    /// Optional hostname override
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgrokTunnelConfig {
    /// ngrok auth token
    pub auth_token: String,
    /// Optional custom domain
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTunnelConfig {
    /// Command template to start the tunnel. Use {port} and {host} placeholders.
    /// Example: "bore local {port} --to bore.pub"
    pub start_command: String,
    /// Optional URL to check tunnel health
    pub health_url: Option<String>,
    /// Optional regex to extract public URL from command stdout
    pub url_pattern: Option<String>,
}

// ── Channels ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub cli: bool,
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub webhook: Option<WebhookConfig>,
    pub imessage: Option<IMessageConfig>,
    pub matrix: Option<MatrixConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub feishu: Option<FeishuConfig>,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: None,
            matrix: None,
            whatsapp: None,
            feishu: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
    /// Voice message transcription configuration (optional)
    #[serde(default)]
    pub voice: Option<TelegramVoiceConfig>,
}

/// Configuration for Telegram voice message transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramVoiceConfig {
    /// Enable voice message transcription (default: true when this section exists)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// STT provider: "openai", "uniapi", "groq", "deepinfra", "compatible" (default: "openai")
    #[serde(default = "default_stt_provider")]
    pub stt_provider: String,
    /// API key for STT provider (optional, defaults to main `api_key`)
    #[serde(default)]
    pub stt_api_key: Option<String>,
    /// STT model name (optional, defaults to "whisper-1" for `OpenAI`)
    #[serde(default)]
    pub stt_model: Option<String>,
    /// Base URL for OpenAI-compatible STT providers (optional)
    #[serde(default)]
    pub stt_base_url: Option<String>,
}

fn default_stt_provider() -> String {
    "openai".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub guild_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub port: u16,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    pub allowed_contacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver: String,
    pub access_token: String,
    pub room_id: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Access token from Meta Business Suite
    pub access_token: String,
    /// Phone number ID from Meta Business API
    pub phone_number_id: String,
    /// Webhook verify token (you define this, Meta sends it back for verification)
    pub verify_token: String,
    /// App secret for webhook signature verification (X-Hub-Signature-256)
    #[serde(default)]
    pub app_secret: Option<String>,
    /// Allowed phone numbers (E.164 format: +1234567890) or "*" for all
    #[serde(default)]
    pub allowed_numbers: Vec<String>,
}

/// Feishu/Lark channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID from Feishu Open Platform
    pub app_id: String,
    /// App Secret from Feishu Open Platform
    pub app_secret: String,
    /// Encrypt key for event callback decryption (optional)
    #[serde(default)]
    pub encrypt_key: Option<String>,
    /// Verification token for event callback verification (optional)
    #[serde(default)]
    pub verification_token: Option<String>,
    /// Allowed user `open_ids` or "*" for all
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

// ── TTS (Text-to-Speech) ────────────────────────────────────────

/// Text-to-Speech configuration for voice responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Enable TTS responses (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// TTS provider: "openai", "elevenlabs" (default: "openai")
    #[serde(default = "default_tts_provider")]
    pub provider: String,
    /// API key for TTS provider (optional, defaults to main `api_key`)
    #[serde(default)]
    pub api_key: Option<String>,
    /// TTS model name (optional, e.g., `tts-1` for `OpenAI`, `eleven_multilingual_v2` for `ElevenLabs`)
    #[serde(default)]
    pub model: Option<String>,
    /// Voice ID (e.g., `alloy` for `OpenAI`, `voice_id` for `ElevenLabs`)
    #[serde(default)]
    pub voice: Option<String>,
    /// Base URL for OpenAI-compatible TTS providers (optional)
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_tts_provider() -> String {
    "openai".into()
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_tts_provider(),
            api_key: None,
            model: None,
            voice: None,
            base_url: None,
        }
    }
}

// ── Voice Wake ──────────────────────────────────────────────────

/// Voice wake word detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceWakeConfig {
    /// Enable voice wake detection (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Wake phrases to listen for (e.g., `["hey zero", "小零"]`)
    #[serde(default)]
    pub wake_phrases: Vec<String>,
    /// Detection sensitivity (0.0 - 1.0, default: 0.5)
    #[serde(default = "default_wake_sensitivity")]
    pub sensitivity: f32,
    /// Audio input device name (optional, uses default if not specified)
    #[serde(default)]
    pub audio_device: Option<String>,
}

fn default_wake_sensitivity() -> f32 {
    0.5
}

impl Default for VoiceWakeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wake_phrases: vec!["hey zero".into()],
            sensitivity: default_wake_sensitivity(),
            audio_device: None,
        }
    }
}

// ── Config impl ──────────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        let home =
            UserDirs::new().map_or_else(|| PathBuf::from("."), |u| u.home_dir().to_path_buf());
        let zerobot_dir = home.join(".codecoder");

        Self {
            workspace_dir: zerobot_dir.join("workspace"),
            config_path: zerobot_dir.join("config.toml"),
            api_key: None,
            default_provider: Some("openrouter".to_string()),
            default_model: Some("anthropic/claude-sonnet-4-20250514".to_string()),
            default_temperature: 0.7,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            heartbeat: HeartbeatConfig::default(),
            channels_config: ChannelsConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            secrets: SecretsConfig::default(),
            vault: VaultConfig::default(),
            browser: BrowserConfig::default(),
            codecoder: CodeCoderConfig::default(),
            identity: IdentityConfig::default(),
            session: SessionConfig::default(),
            tts: TtsConfig::default(),
            voice_wake: VoiceWakeConfig::default(),
            mcp: McpConfig::default(),
            workflow_host: None,
            workflow_port: None,
        }
    }
}

impl Config {
    /// Try to load `ZeroBot` config from `CodeCoder`'s config file
    fn load_from_codecoder() -> Option<Self> {
        let home = UserDirs::new()?.home_dir().to_path_buf();
        let zerobot_dir = home.join(".codecoder");

        // Try multiple possible config paths
        let candidates = [
            // Primary: ~/.codecoder/config.json (shared with CodeCoder)
            home.join(".codecoder/config.json"),
            home.join(".codecoder/codecoder.json"),
            home.join(".codecoder/codecoder.jsonc"),
        ];

        for path in candidates {
            if let Ok(content) = fs::read_to_string(&path) {
                // Strip JSONC comments and resolve env vars
                let content = resolve_env_vars(&strip_json_comments(&content));

                // Parse the full CodeCoder config
                if let Ok(full_config) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Extract zerobot section
                    if let Some(zerobot_value) = full_config.get("zerobot") {
                        if let Ok(mut zb_config) =
                            serde_json::from_value::<ZeroBotJsonConfig>(zerobot_value.clone())
                        {
                            // Extract provider config from CodeCoder's provider section
                            let (resolved_provider, api_key) = zb_config
                                .default_provider
                                .as_ref()
                                .map_or_else(
                                    || (String::new(), None),
                                    |provider_name| {
                                        // Check if it's already a custom:URL format
                                        if provider_name.starts_with("custom:") {
                                            return (provider_name.clone(), None);
                                        }

                                        // Look up provider in CodeCoder's provider config
                                        let provider_config = full_config
                                            .get("provider")
                                            .and_then(|p| p.get(provider_name));

                                        if let Some(pc) = provider_config {
                                            // Extract baseURL and apiKey
                                            let base_url = pc
                                                .get("options")
                                                .and_then(|o| o.get("baseURL"))
                                                .and_then(|u| u.as_str())
                                                .or_else(|| pc.get("api").and_then(|a| a.as_str()))
                                                .map(resolve_env_vars);

                                            let api_key = pc
                                                .get("options")
                                                .and_then(|o| o.get("apiKey"))
                                                .and_then(|k| k.as_str())
                                                .map(resolve_env_vars);

                                            // Convert to custom:URL format if baseURL found
                                            if let Some(url) = base_url {
                                                let custom_provider = format!("custom:{url}");
                                                tracing::info!(
                                                    "Resolved provider '{}' to '{}'",
                                                    provider_name,
                                                    custom_provider
                                                );
                                                (custom_provider, api_key)
                                            } else {
                                                // Keep original provider name (might be built-in)
                                                (provider_name.clone(), api_key)
                                            }
                                        } else {
                                            // Provider not found in config, keep original
                                            (provider_name.clone(), None)
                                        }
                                    },
                                );

                            // Update the provider in zb_config
                            if !resolved_provider.is_empty() {
                                zb_config.default_provider = Some(resolved_provider);
                            }

                            tracing::info!(
                                "Loaded ZeroBot config from CodeCoder: {}",
                                path.display()
                            );

                            return Some(Self::from_json_config(
                                zb_config,
                                api_key,
                                path,
                                &zerobot_dir,
                            ));
                        }
                    }
                }
            }
        }
        None
    }

    /// Convert JSON config to Config struct
    #[allow(clippy::too_many_lines)]
    fn from_json_config(
        json: ZeroBotJsonConfig,
        api_key: Option<String>,
        config_path: PathBuf,
        zerobot_dir: &std::path::Path,
    ) -> Self {
        let defaults = Config::default();

        // Build observability config
        let observability = json.observability.map_or(defaults.observability.clone(), |o| {
            ObservabilityConfig {
                backend: o.backend.unwrap_or(defaults.observability.backend.clone()),
            }
        });

        // Build autonomy config
        let autonomy = json.autonomy.map_or(defaults.autonomy.clone(), |a| {
            let level = a
                .level
                .as_deref()
                .map_or(defaults.autonomy.level, |s| match s {
                    "readonly" => AutonomyLevel::ReadOnly,
                    "full" => AutonomyLevel::Full,
                    _ => AutonomyLevel::Supervised,
                });

            AutonomyConfig {
                level,
                workspace_only: a.workspace_only.unwrap_or(defaults.autonomy.workspace_only),
                allowed_commands: a
                    .allowed_commands
                    .unwrap_or(defaults.autonomy.allowed_commands.clone()),
                forbidden_paths: a
                    .forbidden_paths
                    .unwrap_or(defaults.autonomy.forbidden_paths.clone()),
                max_actions_per_hour: a
                    .max_actions_per_hour
                    .unwrap_or(defaults.autonomy.max_actions_per_hour),
                max_cost_per_day_cents: a
                    .max_cost_per_day_cents
                    .unwrap_or(defaults.autonomy.max_cost_per_day_cents),
            }
        });

        // Build runtime config
        let runtime = json.runtime.map_or(defaults.runtime.clone(), |r| RuntimeConfig {
            kind: r.kind.unwrap_or(defaults.runtime.kind.clone()),
        });

        // Build reliability config
        let reliability = json.reliability.map_or(defaults.reliability.clone(), |r| {
            ReliabilityConfig {
                provider_retries: r
                    .provider_retries
                    .unwrap_or(defaults.reliability.provider_retries),
                provider_backoff_ms: r
                    .provider_backoff_ms
                    .unwrap_or(defaults.reliability.provider_backoff_ms),
                fallback_providers: r
                    .fallback_providers
                    .unwrap_or(defaults.reliability.fallback_providers.clone()),
                channel_initial_backoff_secs: r
                    .channel_initial_backoff_secs
                    .unwrap_or(defaults.reliability.channel_initial_backoff_secs),
                channel_max_backoff_secs: r
                    .channel_max_backoff_secs
                    .unwrap_or(defaults.reliability.channel_max_backoff_secs),
                scheduler_poll_secs: defaults.reliability.scheduler_poll_secs,
                scheduler_retries: defaults.reliability.scheduler_retries,
            }
        });

        // Build heartbeat config
        let heartbeat = json.heartbeat.map_or(defaults.heartbeat.clone(), |h| {
            HeartbeatConfig {
                enabled: h.enabled.unwrap_or(defaults.heartbeat.enabled),
                interval_minutes: h
                    .interval_minutes
                    .unwrap_or(defaults.heartbeat.interval_minutes),
            }
        });

        // Build memory config
        let memory = json.memory.map_or(defaults.memory.clone(), |m| MemoryConfig {
            backend: m.backend.unwrap_or(defaults.memory.backend.clone()),
            auto_save: m.auto_save.unwrap_or(defaults.memory.auto_save),
            hygiene_enabled: m.hygiene_enabled.unwrap_or(defaults.memory.hygiene_enabled),
            embedding_provider: m
                .embedding_provider
                .unwrap_or(defaults.memory.embedding_provider.clone()),
            embedding_model: m
                .embedding_model
                .unwrap_or(defaults.memory.embedding_model.clone()),
            // Use defaults for remaining fields
            archive_after_days: defaults.memory.archive_after_days,
            purge_after_days: defaults.memory.purge_after_days,
            conversation_retention_days: defaults.memory.conversation_retention_days,
            embedding_dimensions: defaults.memory.embedding_dimensions,
            vector_weight: defaults.memory.vector_weight,
            keyword_weight: defaults.memory.keyword_weight,
            embedding_cache_size: defaults.memory.embedding_cache_size,
            chunk_max_tokens: defaults.memory.chunk_max_tokens,
        });

        // Build gateway config
        let gateway = json.gateway.map_or(defaults.gateway.clone(), |g| GatewayConfig {
            port: g.port.unwrap_or(defaults.gateway.port),
            host: g.host.unwrap_or(defaults.gateway.host.clone()),
            require_pairing: g.require_pairing.unwrap_or(defaults.gateway.require_pairing),
            allow_public_bind: g
                .allow_public_bind
                .unwrap_or(defaults.gateway.allow_public_bind),
            paired_tokens: defaults.gateway.paired_tokens.clone(),
        });

        // Build tunnel config
        let tunnel = json.tunnel.map_or(defaults.tunnel.clone(), |t| TunnelConfig {
            provider: t.provider.unwrap_or(defaults.tunnel.provider.clone()),
            cloudflare: t.cloudflare.map(|c| CloudflareTunnelConfig { token: c.token }),
            tailscale: t.tailscale.map(|ts| TailscaleTunnelConfig {
                funnel: ts.funnel.unwrap_or(false),
                hostname: ts.hostname,
            }),
            ngrok: t.ngrok.map(|n| NgrokTunnelConfig {
                auth_token: n.auth_token,
                domain: n.domain,
            }),
            custom: None,
        });

        // Build channels config
        let channels_config = json.channels.map_or(defaults.channels_config.clone(), |c| {
            ChannelsConfig {
                cli: c.cli.unwrap_or(defaults.channels_config.cli),
                telegram: c.telegram.map(|t| TelegramConfig {
                    bot_token: t.bot_token,
                    allowed_users: t.allowed_users,
                    voice: t.voice.map(|v| TelegramVoiceConfig {
                        enabled: v.enabled,
                        stt_provider: v.stt_provider.unwrap_or_else(default_stt_provider),
                        stt_api_key: v.stt_api_key,
                        stt_model: v.stt_model,
                        stt_base_url: v.stt_base_url,
                    }),
                }),
                discord: c.discord.map(|d| DiscordConfig {
                    bot_token: d.bot_token,
                    guild_id: d.guild_id,
                    allowed_users: d.allowed_users,
                }),
                slack: c.slack.map(|s| SlackConfig {
                    bot_token: s.bot_token,
                    app_token: s.app_token,
                    channel_id: s.channel_id,
                    allowed_users: Vec::new(),
                }),
                webhook: None,
                imessage: None,
                matrix: None,
                whatsapp: c.whatsapp.map(|w| WhatsAppConfig {
                    access_token: w.access_token,
                    phone_number_id: w.phone_number_id,
                    verify_token: w.verify_token,
                    app_secret: None,
                    allowed_numbers: w.allowed_numbers,
                }),
                feishu: c.feishu.map(|f| FeishuConfig {
                    app_id: f.app_id,
                    app_secret: f.app_secret,
                    encrypt_key: f.encrypt_key,
                    verification_token: f.verification_token,
                    allowed_users: f.allowed_users,
                }),
            }
        });

        // Build browser config
        let browser = json.browser.map_or(defaults.browser.clone(), |b| BrowserConfig {
            enabled: b.enabled.unwrap_or(defaults.browser.enabled),
            allowed_domains: b
                .allowed_domains
                .unwrap_or(defaults.browser.allowed_domains.clone()),
            session_name: None,
        });

        // Build identity config
        let identity = json.identity.map_or(defaults.identity.clone(), |i| IdentityConfig {
            format: i.format.unwrap_or(defaults.identity.format.clone()),
            aieos_path: i.aieos_path.or(defaults.identity.aieos_path.clone()),
            aieos_inline: None,
        });

        // Build codecoder config
        let codecoder = json.codecoder.map_or(defaults.codecoder.clone(), |c| CodeCoderConfig {
            enabled: c.enabled.unwrap_or(defaults.codecoder.enabled),
            endpoint: c.endpoint.unwrap_or(defaults.codecoder.endpoint.clone()),
            api_key: c.api_key.or(defaults.codecoder.api_key.clone()),
        });

        // Build session config
        let session = json.session.map_or(defaults.session.clone(), |s| SessionConfig {
            enabled: s.enabled.unwrap_or(defaults.session.enabled),
            context_window: s.context_window.unwrap_or(defaults.session.context_window),
            compact_threshold: s.compact_threshold.unwrap_or(defaults.session.compact_threshold),
            keep_recent: s.keep_recent.unwrap_or(defaults.session.keep_recent),
        });

        // Build TTS config
        let tts = json.tts.map_or(defaults.tts.clone(), |t| TtsConfig {
            enabled: t.enabled.unwrap_or(defaults.tts.enabled),
            provider: t.provider.unwrap_or(defaults.tts.provider.clone()),
            api_key: t.api_key.or(defaults.tts.api_key.clone()),
            model: t.model.or(defaults.tts.model.clone()),
            // Prefer 'voice' but fall back to 'default_voice' for backwards compatibility
            voice: t.voice.or(t.default_voice).or(defaults.tts.voice.clone()),
            base_url: t.base_url.or(defaults.tts.base_url.clone()),
        });

        // Build MCP config
        let mcp = json.mcp.map_or(defaults.mcp.clone(), |m| {
            let servers = m.servers.map_or(std::collections::HashMap::new(), |servers| {
                servers
                    .into_iter()
                    .map(|(name, server)| {
                        let config = match server {
                            ZeroBotJsonMcpServer::Local {
                                command,
                                environment,
                                enabled,
                            } => McpServerConfig::Local {
                                command,
                                environment,
                                enabled,
                            },
                            ZeroBotJsonMcpServer::Remote {
                                url,
                                headers,
                                enabled,
                            } => McpServerConfig::Remote {
                                url,
                                headers,
                                enabled,
                            },
                        };
                        (name, config)
                    })
                    .collect()
            });

            McpConfig {
                servers,
                server_enabled: m.server_enabled.unwrap_or(defaults.mcp.server_enabled),
                server_api_key: m.server_api_key.or(defaults.mcp.server_api_key.clone()),
            }
        });

        Config {
            workspace_dir: json
                .workspace_dir
                .map_or_else(|| zerobot_dir.join("workspace"), PathBuf::from),
            config_path,
            api_key,
            default_provider: json.default_provider.or(defaults.default_provider),
            default_model: json.default_model.or(defaults.default_model),
            default_temperature: json.default_temperature.unwrap_or(defaults.default_temperature),
            observability,
            autonomy,
            runtime,
            reliability,
            heartbeat,
            channels_config,
            memory,
            tunnel,
            gateway,
            secrets: defaults.secrets,
            vault: defaults.vault,
            browser,
            codecoder,
            identity,
            session,
            tts,
            voice_wake: defaults.voice_wake,
            mcp,
            workflow_host: defaults.workflow_host,
            workflow_port: defaults.workflow_port,
        }
    }

    pub fn load_or_init() -> Result<Self> {
        // 1. First try to load from CodeCoder config
        if let Some(config) = Self::load_from_codecoder() {
            let mut config = config;
            config.apply_env_overrides();
            return Ok(config);
        }

        // 2. Fall back to original TOML config
        let home = UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .context("Could not find home directory")?;
        let zerobot_dir = home.join(".codecoder");
        let config_path = zerobot_dir.join("config.toml");

        if !zerobot_dir.exists() {
            fs::create_dir_all(&zerobot_dir).context("Failed to create .codecoder directory")?;
            fs::create_dir_all(zerobot_dir.join("workspace"))
                .context("Failed to create workspace directory")?;
        }

        if config_path.exists() {
            let contents =
                fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            config.apply_env_overrides();
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            let mut config = config;
            config.apply_env_overrides();
            Ok(config)
        }
    }

    /// Apply environment variable overrides to config
    pub fn apply_env_overrides(&mut self) {
        // API Key: ZERO_BOT_API_KEY or API_KEY
        if let Ok(key) = std::env::var("ZERO_BOT_API_KEY").or_else(|_| std::env::var("API_KEY")) {
            if !key.is_empty() {
                self.api_key = Some(key);
            }
        }

        // Provider: ZERO_BOT_PROVIDER or PROVIDER
        if let Ok(provider) =
            std::env::var("ZERO_BOT_PROVIDER").or_else(|_| std::env::var("PROVIDER"))
        {
            if !provider.is_empty() {
                self.default_provider = Some(provider);
            }
        }

        // Model: ZERO_BOT_MODEL
        if let Ok(model) = std::env::var("ZERO_BOT_MODEL") {
            if !model.is_empty() {
                self.default_model = Some(model);
            }
        }

        // Workspace directory: ZERO_BOT_WORKSPACE
        if let Ok(workspace) = std::env::var("ZERO_BOT_WORKSPACE") {
            if !workspace.is_empty() {
                self.workspace_dir = PathBuf::from(workspace);
            }
        }

        // Gateway port: ZERO_BOT_GATEWAY_PORT or PORT
        if let Ok(port_str) =
            std::env::var("ZERO_BOT_GATEWAY_PORT").or_else(|_| std::env::var("PORT"))
        {
            if let Ok(port) = port_str.parse::<u16>() {
                self.gateway.port = port;
            }
        }

        // Gateway host: ZERO_BOT_GATEWAY_HOST or HOST
        if let Ok(host) = std::env::var("ZERO_BOT_GATEWAY_HOST").or_else(|_| std::env::var("HOST"))
        {
            if !host.is_empty() {
                self.gateway.host = host;
            }
        }

        // Temperature: ZERO_BOT_TEMPERATURE
        if let Ok(temp_str) = std::env::var("ZERO_BOT_TEMPERATURE") {
            if let Ok(temp) = temp_str.parse::<f64>() {
                if (0.0..=2.0).contains(&temp) {
                    self.default_temperature = temp;
                }
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let toml_str = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&self.config_path, toml_str).context("Failed to write config file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Defaults ─────────────────────────────────────────────

    #[test]
    fn config_default_has_sane_values() {
        let c = Config::default();
        assert_eq!(c.default_provider.as_deref(), Some("openrouter"));
        assert!(c.default_model.as_deref().unwrap().contains("claude"));
        assert!((c.default_temperature - 0.7).abs() < f64::EPSILON);
        assert!(c.api_key.is_none());
        assert!(c.workspace_dir.to_string_lossy().contains("workspace"));
        assert!(c.config_path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    fn observability_config_default() {
        let o = ObservabilityConfig::default();
        assert_eq!(o.backend, "none");
    }

    #[test]
    fn autonomy_config_default() {
        let a = AutonomyConfig::default();
        assert_eq!(a.level, AutonomyLevel::Supervised);
        assert!(a.workspace_only);
        assert!(a.allowed_commands.contains(&"git".to_string()));
        assert!(a.allowed_commands.contains(&"cargo".to_string()));
        assert!(a.forbidden_paths.contains(&"/etc".to_string()));
        assert_eq!(a.max_actions_per_hour, 20);
        assert_eq!(a.max_cost_per_day_cents, 500);
    }

    #[test]
    fn runtime_config_default() {
        let r = RuntimeConfig::default();
        assert_eq!(r.kind, "native");
    }

    #[test]
    fn heartbeat_config_default() {
        let h = HeartbeatConfig::default();
        assert!(!h.enabled);
        assert_eq!(h.interval_minutes, 30);
    }

    #[test]
    fn memory_config_default_hygiene_settings() {
        let m = MemoryConfig::default();
        assert_eq!(m.backend, "sqlite");
        assert!(m.auto_save);
        assert!(m.hygiene_enabled);
        assert_eq!(m.archive_after_days, 7);
        assert_eq!(m.purge_after_days, 30);
        assert_eq!(m.conversation_retention_days, 30);
    }

    #[test]
    fn channels_config_default() {
        let c = ChannelsConfig::default();
        assert!(c.cli);
        assert!(c.telegram.is_none());
        assert!(c.discord.is_none());
    }

    // ── Serde round-trip ─────────────────────────────────────

    #[test]
    fn config_toml_roundtrip() {
        let config = Config {
            workspace_dir: PathBuf::from("/tmp/test/workspace"),
            config_path: PathBuf::from("/tmp/test/config.toml"),
            api_key: Some("sk-test-key".into()),
            default_provider: Some("openrouter".into()),
            default_model: Some("gpt-4o".into()),
            default_temperature: 0.5,
            observability: ObservabilityConfig {
                backend: "log".into(),
            },
            autonomy: AutonomyConfig {
                level: AutonomyLevel::Full,
                workspace_only: false,
                allowed_commands: vec!["docker".into()],
                forbidden_paths: vec!["/secret".into()],
                max_actions_per_hour: 50,
                max_cost_per_day_cents: 1000,
            },
            runtime: RuntimeConfig {
                kind: "docker".into(),
            },
            reliability: ReliabilityConfig::default(),
            heartbeat: HeartbeatConfig {
                enabled: true,
                interval_minutes: 15,
            },
            channels_config: ChannelsConfig {
                cli: true,
                telegram: Some(TelegramConfig {
                    bot_token: "123:ABC".into(),
                    allowed_users: vec!["user1".into()],
                    voice: None,
                }),
                discord: None,
                slack: None,
                webhook: None,
                imessage: None,
                matrix: None,
                whatsapp: None,
                feishu: None,
            },
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            secrets: SecretsConfig::default(),
            vault: VaultConfig::default(),
            browser: BrowserConfig::default(),
            codecoder: CodeCoderConfig::default(),
            identity: IdentityConfig::default(),
            session: SessionConfig::default(),
            tts: TtsConfig::default(),
            voice_wake: VoiceWakeConfig::default(),
            mcp: McpConfig::default(),
            workflow_host: None,
            workflow_port: None,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.default_provider, config.default_provider);
        assert_eq!(parsed.default_model, config.default_model);
        assert!((parsed.default_temperature - config.default_temperature).abs() < f64::EPSILON);
        assert_eq!(parsed.observability.backend, "log");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Full);
        assert!(!parsed.autonomy.workspace_only);
        assert_eq!(parsed.runtime.kind, "docker");
        assert!(parsed.heartbeat.enabled);
        assert_eq!(parsed.heartbeat.interval_minutes, 15);
        assert!(parsed.channels_config.telegram.is_some());
        assert_eq!(
            parsed.channels_config.telegram.unwrap().bot_token,
            "123:ABC"
        );
    }

    #[test]
    fn config_minimal_toml_uses_defaults() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(parsed.api_key.is_none());
        assert!(parsed.default_provider.is_none());
        assert_eq!(parsed.observability.backend, "none");
        assert_eq!(parsed.autonomy.level, AutonomyLevel::Supervised);
        assert_eq!(parsed.runtime.kind, "native");
        assert!(!parsed.heartbeat.enabled);
        assert!(parsed.channels_config.cli);
        assert!(parsed.memory.hygiene_enabled);
        assert_eq!(parsed.memory.archive_after_days, 7);
        assert_eq!(parsed.memory.purge_after_days, 30);
        assert_eq!(parsed.memory.conversation_retention_days, 30);
    }

    #[test]
    fn config_save_and_load_tmpdir() {
        let dir = std::env::temp_dir().join("zerobot_test_config");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config_path = dir.join("config.toml");
        let config = Config {
            workspace_dir: dir.join("workspace"),
            config_path: config_path.clone(),
            api_key: Some("sk-roundtrip".into()),
            default_provider: Some("openrouter".into()),
            default_model: Some("test-model".into()),
            default_temperature: 0.9,
            observability: ObservabilityConfig::default(),
            autonomy: AutonomyConfig::default(),
            runtime: RuntimeConfig::default(),
            reliability: ReliabilityConfig::default(),
            heartbeat: HeartbeatConfig::default(),
            channels_config: ChannelsConfig::default(),
            memory: MemoryConfig::default(),
            tunnel: TunnelConfig::default(),
            gateway: GatewayConfig::default(),
            secrets: SecretsConfig::default(),
            vault: VaultConfig::default(),
            browser: BrowserConfig::default(),
            codecoder: CodeCoderConfig::default(),
            identity: IdentityConfig::default(),
            session: SessionConfig::default(),
            tts: TtsConfig::default(),
            voice_wake: VoiceWakeConfig::default(),
            mcp: McpConfig::default(),
            workflow_host: None,
            workflow_port: None,
        };

        config.save().unwrap();
        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = toml::from_str(&contents).unwrap();
        assert_eq!(loaded.api_key.as_deref(), Some("sk-roundtrip"));
        assert_eq!(loaded.default_model.as_deref(), Some("test-model"));
        assert!((loaded.default_temperature - 0.9).abs() < f64::EPSILON);

        let _ = fs::remove_dir_all(&dir);
    }

    // ── Telegram / Discord config ────────────────────────────

    #[test]
    fn telegram_config_serde() {
        let tc = TelegramConfig {
            bot_token: "123:XYZ".into(),
            allowed_users: vec!["alice".into(), "bob".into()],
            voice: None,
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: TelegramConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "123:XYZ");
        assert_eq!(parsed.allowed_users.len(), 2);
    }

    #[test]
    fn discord_config_serde() {
        let dc = DiscordConfig {
            bot_token: "discord-token".into(),
            guild_id: Some("12345".into()),
            allowed_users: vec![],
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, "discord-token");
        assert_eq!(parsed.guild_id.as_deref(), Some("12345"));
    }

    #[test]
    fn discord_config_optional_guild() {
        let dc = DiscordConfig {
            bot_token: "tok".into(),
            guild_id: None,
            allowed_users: vec![],
        };
        let json = serde_json::to_string(&dc).unwrap();
        let parsed: DiscordConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.guild_id.is_none());
    }

    // ── iMessage / Matrix config ────────────────────────────

    #[test]
    fn imessage_config_serde() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["+1234567890".into(), "user@icloud.com".into()],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.allowed_contacts.len(), 2);
        assert_eq!(parsed.allowed_contacts[0], "+1234567890");
    }

    #[test]
    fn imessage_config_empty_contacts() {
        let ic = IMessageConfig {
            allowed_contacts: vec![],
        };
        let json = serde_json::to_string(&ic).unwrap();
        let parsed: IMessageConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.allowed_contacts.is_empty());
    }

    #[test]
    fn imessage_config_wildcard() {
        let ic = IMessageConfig {
            allowed_contacts: vec!["*".into()],
        };
        let toml_str = toml::to_string(&ic).unwrap();
        let parsed: IMessageConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_contacts, vec!["*"]);
    }

    #[test]
    fn matrix_config_serde() {
        let mc = MatrixConfig {
            homeserver: "https://matrix.org".into(),
            access_token: "syt_token_abc".into(),
            room_id: "!room123:matrix.org".into(),
            allowed_users: vec!["@user:matrix.org".into()],
        };
        let json = serde_json::to_string(&mc).unwrap();
        let parsed: MatrixConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.homeserver, "https://matrix.org");
        assert_eq!(parsed.access_token, "syt_token_abc");
        assert_eq!(parsed.room_id, "!room123:matrix.org");
        assert_eq!(parsed.allowed_users.len(), 1);
    }

    #[test]
    fn matrix_config_toml_roundtrip() {
        let mc = MatrixConfig {
            homeserver: "https://synapse.local:8448".into(),
            access_token: "tok".into(),
            room_id: "!abc:synapse.local".into(),
            allowed_users: vec!["@admin:synapse.local".into(), "*".into()],
        };
        let toml_str = toml::to_string(&mc).unwrap();
        let parsed: MatrixConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.homeserver, "https://synapse.local:8448");
        assert_eq!(parsed.allowed_users.len(), 2);
    }

    #[test]
    fn channels_config_with_imessage_and_matrix() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: Some(IMessageConfig {
                allowed_contacts: vec!["+1".into()],
            }),
            matrix: Some(MatrixConfig {
                homeserver: "https://m.org".into(),
                access_token: "tok".into(),
                room_id: "!r:m".into(),
                allowed_users: vec!["@u:m".into()],
            }),
            whatsapp: None,
            feishu: None,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.imessage.is_some());
        assert!(parsed.matrix.is_some());
        assert_eq!(parsed.imessage.unwrap().allowed_contacts, vec!["+1"]);
        assert_eq!(parsed.matrix.unwrap().homeserver, "https://m.org");
    }

    #[test]
    fn channels_config_default_has_no_imessage_matrix() {
        let c = ChannelsConfig::default();
        assert!(c.imessage.is_none());
        assert!(c.matrix.is_none());
    }

    // ── Edge cases: serde(default) for allowed_users ─────────

    #[test]
    fn discord_config_deserializes_without_allowed_users() {
        // Old configs won't have allowed_users — serde(default) should fill vec![]
        let json = r#"{"bot_token":"tok","guild_id":"123"}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn discord_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"tok","guild_id":"123","allowed_users":["111","222"]}"#;
        let parsed: DiscordConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["111", "222"]);
    }

    #[test]
    fn slack_config_deserializes_without_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok"}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_users.is_empty());
    }

    #[test]
    fn slack_config_deserializes_with_allowed_users() {
        let json = r#"{"bot_token":"xoxb-tok","allowed_users":["U111"]}"#;
        let parsed: SlackConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.allowed_users, vec!["U111"]);
    }

    #[test]
    fn discord_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "tok"
guild_id = "123"
"#;
        let parsed: DiscordConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.bot_token, "tok");
    }

    #[test]
    fn slack_config_toml_backward_compat() {
        let toml_str = r#"
bot_token = "xoxb-tok"
channel_id = "C123"
"#;
        let parsed: SlackConfig = toml::from_str(toml_str).unwrap();
        assert!(parsed.allowed_users.is_empty());
        assert_eq!(parsed.channel_id.as_deref(), Some("C123"));
    }

    #[test]
    fn webhook_config_with_secret() {
        let json = r#"{"port":8080,"secret":"my-secret-key"}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.secret.as_deref(), Some("my-secret-key"));
    }

    #[test]
    fn webhook_config_without_secret() {
        let json = r#"{"port":8080}"#;
        let parsed: WebhookConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.secret.is_none());
        assert_eq!(parsed.port, 8080);
    }

    // ── WhatsApp config ──────────────────────────────────────

    #[test]
    fn whatsapp_config_serde() {
        let wc = WhatsAppConfig {
            access_token: "EAABx...".into(),
            phone_number_id: "123456789".into(),
            verify_token: "my-verify-token".into(),
            app_secret: None,
            allowed_numbers: vec!["+1234567890".into(), "+9876543210".into()],
        };
        let json = serde_json::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "EAABx...");
        assert_eq!(parsed.phone_number_id, "123456789");
        assert_eq!(parsed.verify_token, "my-verify-token");
        assert_eq!(parsed.allowed_numbers.len(), 2);
    }

    #[test]
    fn whatsapp_config_toml_roundtrip() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "12345".into(),
            verify_token: "verify".into(),
            app_secret: None,
            allowed_numbers: vec!["+1".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.phone_number_id, "12345");
        assert_eq!(parsed.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn whatsapp_config_deserializes_without_allowed_numbers() {
        let json = r#"{"access_token":"tok","phone_number_id":"123","verify_token":"ver"}"#;
        let parsed: WhatsAppConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.allowed_numbers.is_empty());
    }

    #[test]
    fn whatsapp_config_wildcard_allowed() {
        let wc = WhatsAppConfig {
            access_token: "tok".into(),
            phone_number_id: "123".into(),
            verify_token: "ver".into(),
            app_secret: None,
            allowed_numbers: vec!["*".into()],
        };
        let toml_str = toml::to_string(&wc).unwrap();
        let parsed: WhatsAppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.allowed_numbers, vec!["*"]);
    }

    #[test]
    fn channels_config_with_whatsapp() {
        let c = ChannelsConfig {
            cli: true,
            telegram: None,
            discord: None,
            slack: None,
            webhook: None,
            imessage: None,
            matrix: None,
            whatsapp: Some(WhatsAppConfig {
                access_token: "tok".into(),
                phone_number_id: "123".into(),
                verify_token: "ver".into(),
                app_secret: None,
                allowed_numbers: vec!["+1".into()],
            }),
            feishu: None,
        };
        let toml_str = toml::to_string_pretty(&c).unwrap();
        let parsed: ChannelsConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.whatsapp.is_some());
        let wa = parsed.whatsapp.unwrap();
        assert_eq!(wa.phone_number_id, "123");
        assert_eq!(wa.allowed_numbers, vec!["+1"]);
    }

    #[test]
    fn channels_config_default_has_no_whatsapp() {
        let c = ChannelsConfig::default();
        assert!(c.whatsapp.is_none());
    }

    // ══════════════════════════════════════════════════════════
    // SECURITY CHECKLIST TESTS — Gateway config
    // ══════════════════════════════════════════════════════════

    #[test]
    fn checklist_gateway_default_requires_pairing() {
        let g = GatewayConfig::default();
        assert!(g.require_pairing, "Pairing must be required by default");
    }

    #[test]
    fn checklist_gateway_default_blocks_public_bind() {
        let g = GatewayConfig::default();
        assert!(
            !g.allow_public_bind,
            "Public bind must be blocked by default"
        );
    }

    #[test]
    fn checklist_gateway_default_no_tokens() {
        let g = GatewayConfig::default();
        assert!(
            g.paired_tokens.is_empty(),
            "No pre-paired tokens by default"
        );
    }

    #[test]
    fn checklist_gateway_cli_default_host_is_localhost() {
        // The CLI default for --host is 127.0.0.1 (checked in main.rs)
        // Here we verify the config default matches
        let c = Config::default();
        assert!(
            c.gateway.require_pairing,
            "Config default must require pairing"
        );
        assert!(
            !c.gateway.allow_public_bind,
            "Config default must block public bind"
        );
    }

    #[test]
    fn checklist_gateway_serde_roundtrip() {
        let g = GatewayConfig {
            port: 3000,
            host: "127.0.0.1".into(),
            require_pairing: true,
            allow_public_bind: false,
            paired_tokens: vec!["zc_test_token".into()],
        };
        let toml_str = toml::to_string(&g).unwrap();
        let parsed: GatewayConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.require_pairing);
        assert!(!parsed.allow_public_bind);
        assert_eq!(parsed.paired_tokens, vec!["zc_test_token"]);
    }

    #[test]
    fn checklist_gateway_backward_compat_no_gateway_section() {
        // Old configs without [gateway] should get secure defaults
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.gateway.require_pairing,
            "Missing [gateway] must default to require_pairing=true"
        );
        assert!(
            !parsed.gateway.allow_public_bind,
            "Missing [gateway] must default to allow_public_bind=false"
        );
    }

    #[test]
    fn checklist_autonomy_default_is_workspace_scoped() {
        let a = AutonomyConfig::default();
        assert!(a.workspace_only, "Default autonomy must be workspace_only");
        assert!(
            a.forbidden_paths.contains(&"/etc".to_string()),
            "Must block /etc"
        );
        assert!(
            a.forbidden_paths.contains(&"/proc".to_string()),
            "Must block /proc"
        );
        assert!(
            a.forbidden_paths.contains(&"~/.ssh".to_string()),
            "Must block ~/.ssh"
        );
    }

    // ══════════════════════════════════════════════════════════
    // SECRETS CONFIG TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn secrets_config_default_encrypts() {
        let s = SecretsConfig::default();
        assert!(s.encrypt, "Encryption must be enabled by default");
    }

    #[test]
    fn secrets_config_serde_roundtrip() {
        let s = SecretsConfig { encrypt: false };
        let toml_str = toml::to_string(&s).unwrap();
        let parsed: SecretsConfig = toml::from_str(&toml_str).unwrap();
        assert!(!parsed.encrypt);
    }

    #[test]
    fn secrets_config_backward_compat_missing_section() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(
            parsed.secrets.encrypt,
            "Missing [secrets] must default to encrypt=true"
        );
    }

    #[test]
    fn config_default_has_secrets_and_browser() {
        let c = Config::default();
        assert!(c.secrets.encrypt);
        assert!(!c.browser.enabled);
        assert!(c.browser.allowed_domains.is_empty());
    }

    #[test]
    fn browser_config_default_disabled() {
        let b = BrowserConfig::default();
        assert!(!b.enabled);
        assert!(b.allowed_domains.is_empty());
    }

    #[test]
    fn browser_config_serde_roundtrip() {
        let b = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into(), "docs.example.com".into()],
            session_name: None,
        };
        let toml_str = toml::to_string(&b).unwrap();
        let parsed: BrowserConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.allowed_domains.len(), 2);
        assert_eq!(parsed.allowed_domains[0], "example.com");
    }

    #[test]
    fn browser_config_backward_compat_missing_section() {
        let minimal = r#"
workspace_dir = "/tmp/ws"
config_path = "/tmp/config.toml"
default_temperature = 0.7
"#;
        let parsed: Config = toml::from_str(minimal).unwrap();
        assert!(!parsed.browser.enabled);
        assert!(parsed.browser.allowed_domains.is_empty());
    }

    // ── Environment variable overrides (Docker support) ─────────

    #[test]
    fn env_override_api_key() {
        let mut config = Config::default();
        assert!(config.api_key.is_none());

        std::env::set_var("ZERO_BOT_API_KEY", "sk-test-env-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-test-env-key"));

        std::env::remove_var("ZERO_BOT_API_KEY");
    }

    #[test]
    fn env_override_api_key_fallback() {
        let mut config = Config::default();

        std::env::remove_var("ZERO_BOT_API_KEY");
        std::env::set_var("API_KEY", "sk-fallback-key");
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("sk-fallback-key"));

        std::env::remove_var("API_KEY");
    }

    #[test]
    fn env_override_provider() {
        let mut config = Config::default();

        std::env::set_var("ZERO_BOT_PROVIDER", "anthropic");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("anthropic"));

        std::env::remove_var("ZERO_BOT_PROVIDER");
    }

    #[test]
    fn env_override_provider_fallback() {
        let mut config = Config::default();

        std::env::remove_var("ZERO_BOT_PROVIDER");
        std::env::set_var("PROVIDER", "openai");
        config.apply_env_overrides();
        assert_eq!(config.default_provider.as_deref(), Some("openai"));

        std::env::remove_var("PROVIDER");
    }

    #[test]
    fn env_override_model() {
        let mut config = Config::default();

        std::env::set_var("ZERO_BOT_MODEL", "gpt-4o");
        config.apply_env_overrides();
        assert_eq!(config.default_model.as_deref(), Some("gpt-4o"));

        std::env::remove_var("ZERO_BOT_MODEL");
    }

    #[test]
    fn env_override_workspace() {
        let mut config = Config::default();

        std::env::set_var("ZERO_BOT_WORKSPACE", "/custom/workspace");
        config.apply_env_overrides();
        assert_eq!(config.workspace_dir, PathBuf::from("/custom/workspace"));

        std::env::remove_var("ZERO_BOT_WORKSPACE");
    }

    #[test]
    fn env_override_empty_values_ignored() {
        let mut config = Config::default();
        let original_provider = config.default_provider.clone();

        std::env::set_var("ZERO_BOT_PROVIDER", "");
        config.apply_env_overrides();
        assert_eq!(config.default_provider, original_provider);

        std::env::remove_var("ZERO_BOT_PROVIDER");
    }

    #[test]
    fn env_override_gateway_port() {
        let mut config = Config::default();
        assert_eq!(config.gateway.port, 3000);

        std::env::set_var("ZERO_BOT_GATEWAY_PORT", "8080");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 8080);

        std::env::remove_var("ZERO_BOT_GATEWAY_PORT");
    }

    #[test]
    fn env_override_port_fallback() {
        let mut config = Config::default();

        std::env::remove_var("ZERO_BOT_GATEWAY_PORT");
        std::env::set_var("PORT", "9000");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, 9000);

        std::env::remove_var("PORT");
    }

    #[test]
    fn env_override_gateway_host() {
        let mut config = Config::default();
        assert_eq!(config.gateway.host, "127.0.0.1");

        std::env::set_var("ZERO_BOT_GATEWAY_HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("ZERO_BOT_GATEWAY_HOST");
    }

    #[test]
    fn env_override_host_fallback() {
        let mut config = Config::default();

        std::env::remove_var("ZERO_BOT_GATEWAY_HOST");
        std::env::set_var("HOST", "0.0.0.0");
        config.apply_env_overrides();
        assert_eq!(config.gateway.host, "0.0.0.0");

        std::env::remove_var("HOST");
    }

    #[test]
    fn env_override_temperature() {
        let mut config = Config::default();

        std::env::set_var("ZERO_BOT_TEMPERATURE", "0.5");
        config.apply_env_overrides();
        assert!((config.default_temperature - 0.5).abs() < f64::EPSILON);

        std::env::remove_var("ZERO_BOT_TEMPERATURE");
    }

    #[test]
    fn env_override_temperature_out_of_range_ignored() {
        // Clean up any leftover env vars from other tests
        std::env::remove_var("ZERO_BOT_TEMPERATURE");
        
        let mut config = Config::default();
        let original_temp = config.default_temperature;

        // Temperature > 2.0 should be ignored
        std::env::set_var("ZERO_BOT_TEMPERATURE", "3.0");
        config.apply_env_overrides();
        assert!(
            (config.default_temperature - original_temp).abs() < f64::EPSILON,
            "Temperature 3.0 should be ignored (out of range)"
        );

        std::env::remove_var("ZERO_BOT_TEMPERATURE");
    }

    #[test]
    fn env_override_invalid_port_ignored() {
        let mut config = Config::default();
        let original_port = config.gateway.port;

        std::env::set_var("PORT", "not_a_number");
        config.apply_env_overrides();
        assert_eq!(config.gateway.port, original_port);

        std::env::remove_var("PORT");
    }

    #[test]
    fn gateway_config_default_values() {
        let g = GatewayConfig::default();
        assert_eq!(g.port, 3000);
        assert_eq!(g.host, "127.0.0.1");
        assert!(g.require_pairing);
        assert!(!g.allow_public_bind);
        assert!(g.paired_tokens.is_empty());
    }

    // ══════════════════════════════════════════════════════════
    // CODECODER JSON CONFIG INTEGRATION TESTS
    // ══════════════════════════════════════════════════════════

    #[test]
    fn strip_json_comments_single_line() {
        let input = r#"{
  "key": "value", // this is a comment
  "other": "data"
}"#;
        let output = super::strip_json_comments(input);
        assert!(!output.contains("//"));
        assert!(output.contains(r#""key": "value""#));
        assert!(output.contains(r#""other": "data""#));
    }

    #[test]
    fn strip_json_comments_multi_line() {
        let input = r#"{
  /* multi-line
     comment */
  "key": "value"
}"#;
        let output = super::strip_json_comments(input);
        assert!(!output.contains("/*"));
        assert!(!output.contains("*/"));
        assert!(output.contains(r#""key": "value""#));
    }

    #[test]
    fn strip_json_comments_preserves_strings() {
        let input = r#"{"url": "https://example.com/path"}"#;
        let output = super::strip_json_comments(input);
        assert_eq!(output, input);
    }

    #[test]
    fn strip_json_comments_slash_in_string_preserved() {
        let input = r#"{"path": "a/b/c"}"#;
        let output = super::strip_json_comments(input);
        assert_eq!(output, input);
    }

    #[test]
    fn resolve_env_vars_basic() {
        std::env::set_var("TEST_VAR_123", "test_value");
        let input = "prefix_{env:TEST_VAR_123}_suffix";
        let output = super::resolve_env_vars(input);
        assert_eq!(output, "prefix_test_value_suffix");
        std::env::remove_var("TEST_VAR_123");
    }

    #[test]
    fn resolve_env_vars_missing_var() {
        std::env::remove_var("NONEXISTENT_VAR_XYZ");
        let input = "prefix_{env:NONEXISTENT_VAR_XYZ}_suffix";
        let output = super::resolve_env_vars(input);
        assert_eq!(output, "prefix__suffix");
    }

    #[test]
    fn resolve_env_vars_multiple() {
        std::env::set_var("TEST_A_123", "aaa");
        std::env::set_var("TEST_B_123", "bbb");
        let input = "{env:TEST_A_123}/{env:TEST_B_123}";
        let output = super::resolve_env_vars(input);
        assert_eq!(output, "aaa/bbb");
        std::env::remove_var("TEST_A_123");
        std::env::remove_var("TEST_B_123");
    }

    #[test]
    fn zerobot_json_config_basic_parse() {
        let json = r#"{
            "default_provider": "openrouter",
            "default_model": "claude-sonnet-4",
            "default_temperature": 0.5
        }"#;
        let parsed: super::ZeroBotJsonConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.default_provider.as_deref(), Some("openrouter"));
        assert_eq!(parsed.default_model.as_deref(), Some("claude-sonnet-4"));
        assert!((parsed.default_temperature.unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn zerobot_json_config_with_gateway() {
        let json = r#"{
            "gateway": {
                "port": 8080,
                "host": "0.0.0.0"
            }
        }"#;
        let parsed: super::ZeroBotJsonConfig = serde_json::from_str(json).unwrap();
        let gateway = parsed.gateway.unwrap();
        assert_eq!(gateway.port, Some(8080));
        assert_eq!(gateway.host.as_deref(), Some("0.0.0.0"));
    }

    #[test]
    fn zerobot_json_config_with_channels() {
        let json = r#"{
            "channels": {
                "cli": true,
                "telegram": {
                    "bot_token": "123:ABC",
                    "allowed_users": ["user1", "user2"]
                }
            }
        }"#;
        let parsed: super::ZeroBotJsonConfig = serde_json::from_str(json).unwrap();
        let channels = parsed.channels.unwrap();
        assert_eq!(channels.cli, Some(true));
        let telegram = channels.telegram.unwrap();
        assert_eq!(telegram.bot_token, "123:ABC");
        assert_eq!(telegram.allowed_users, vec!["user1", "user2"]);
    }

    #[test]
    fn zerobot_json_config_with_autonomy() {
        let json = r#"{
            "autonomy": {
                "level": "full",
                "workspace_only": false,
                "max_actions_per_hour": 100
            }
        }"#;
        let parsed: super::ZeroBotJsonConfig = serde_json::from_str(json).unwrap();
        let autonomy = parsed.autonomy.unwrap();
        assert_eq!(autonomy.level.as_deref(), Some("full"));
        assert_eq!(autonomy.workspace_only, Some(false));
        assert_eq!(autonomy.max_actions_per_hour, Some(100));
    }

    #[test]
    fn zerobot_json_config_minimal_uses_defaults() {
        let json = "{}";
        let parsed: super::ZeroBotJsonConfig = serde_json::from_str(json).unwrap();
        assert!(parsed.default_provider.is_none());
        assert!(parsed.gateway.is_none());
        assert!(parsed.channels.is_none());
    }

    #[test]
    fn config_from_json_applies_defaults() {
        let json_config = super::ZeroBotJsonConfig {
            default_provider: Some("anthropic".into()),
            default_model: None,
            default_temperature: None,
            workspace_dir: None,
            observability: None,
            autonomy: None,
            runtime: None,
            reliability: None,
            heartbeat: None,
            memory: None,
            gateway: None,
            tunnel: None,
            channels: None,
            browser: None,
            identity: None,
            codecoder: None,
            session: None,
            tts: None,
            mcp: None,
        };

        let zerobot_dir = PathBuf::from("/tmp/test-zerobot");
        let config = Config::from_json_config(
            json_config,
            Some("sk-test-key".into()),
            PathBuf::from("/tmp/config.json"),
            &zerobot_dir,
        );

        assert_eq!(config.default_provider.as_deref(), Some("anthropic"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(config.workspace_dir, zerobot_dir.join("workspace"));
        // Defaults should be applied
        assert_eq!(config.gateway.port, 3000);
        assert!(config.gateway.require_pairing);
        assert_eq!(config.autonomy.level, AutonomyLevel::Supervised);
    }

    #[test]
    fn config_from_json_overrides_defaults() {
        let json_config = super::ZeroBotJsonConfig {
            default_provider: Some("openai".into()),
            default_model: Some("gpt-4o".into()),
            default_temperature: Some(0.9),
            workspace_dir: Some("/custom/workspace".into()),
            observability: Some(super::ZeroBotJsonObservability {
                backend: Some("log".into()),
            }),
            autonomy: Some(super::ZeroBotJsonAutonomy {
                level: Some("full".into()),
                workspace_only: Some(false),
                allowed_commands: None,
                forbidden_paths: None,
                max_actions_per_hour: Some(50),
                max_cost_per_day_cents: None,
            }),
            runtime: None,
            reliability: None,
            heartbeat: None,
            memory: None,
            gateway: Some(super::ZeroBotJsonGateway {
                port: Some(9000),
                host: Some("0.0.0.0".into()),
                require_pairing: Some(false),
                allow_public_bind: None,
            }),
            tunnel: None,
            channels: None,
            browser: None,
            identity: None,
            codecoder: None,
            session: None,
            tts: None,
            mcp: None,
        };

        let zerobot_dir = PathBuf::from("/tmp/test");
        let config = Config::from_json_config(
            json_config,
            None,
            PathBuf::from("/tmp/config.json"),
            &zerobot_dir,
        );

        assert_eq!(config.default_provider.as_deref(), Some("openai"));
        assert_eq!(config.default_model.as_deref(), Some("gpt-4o"));
        assert!((config.default_temperature - 0.9).abs() < f64::EPSILON);
        assert_eq!(config.workspace_dir, PathBuf::from("/custom/workspace"));
        assert_eq!(config.observability.backend, "log");
        assert_eq!(config.autonomy.level, AutonomyLevel::Full);
        assert!(!config.autonomy.workspace_only);
        assert_eq!(config.autonomy.max_actions_per_hour, 50);
        assert_eq!(config.gateway.port, 9000);
        assert_eq!(config.gateway.host, "0.0.0.0");
        assert!(!config.gateway.require_pairing);
    }
}
