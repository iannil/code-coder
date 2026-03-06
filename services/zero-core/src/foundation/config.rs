//! Configuration loading and management
//!
//! Provides high-performance configuration loading with:
//! - JSONC (JSON with Comments) parsing
//! - Multi-file configuration merging
//! - Environment variable overrides
//! - Directory scanning for `.codecoder` directories

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Core Configuration Types
// ============================================================================

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// JSON schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// API keys and provider configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub provider: HashMap<String, ProviderConfig>,

    /// Model preferences
    #[serde(default)]
    pub models: ModelConfig,

    /// Tool settings
    #[serde(default)]
    pub tools: ToolConfig,

    /// Agent configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agent: HashMap<String, AgentConfig>,

    /// Mode configurations (deprecated, use agent)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mode: HashMap<String, AgentConfig>,

    /// Command configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub command: HashMap<String, CommandConfig>,

    /// MCP server configurations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpConfig>,

    /// Permission settings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionConfig>,

    /// Custom instructions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<String>,

    /// Server configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfig>,

    /// Theme name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Log level
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_level: Option<String>,

    /// Model string (provider/model format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Small model for quick tasks
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_model: Option<String>,

    /// Default agent name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_agent: Option<String>,

    /// Username override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Disabled providers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_providers: Vec<String>,

    /// Enabled providers (exclusive list)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_providers: Vec<String>,

    /// Keybinds configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybinds: Option<KeybindsConfig>,

    /// TUI configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tui: Option<TuiConfig>,

    /// Workspace configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceConfig>,

    /// Compaction settings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionConfig>,

    /// Auto-update settings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<Value>,

    /// Vault configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultConfig>,

    /// Secrets configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<SecretsConfig>,

    /// ZeroBot configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zerobot: Option<Value>,

    /// LLM configuration (Rust services format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<Value>,

    /// Experimental features
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,

    /// Autonomous mode configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autonomous_mode: Option<Value>,

    /// Redis configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redis: Option<Value>,

    /// Task queue configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_queue: Option<Value>,

    /// Network configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Value>,

    /// Services configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services: Option<Value>,

    /// LSP configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<Value>,

    /// Formatter configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatter: Option<Value>,

    /// Watcher configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher: Option<Value>,

    /// Enterprise configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise: Option<Value>,

    /// Additional fields (passthrough)
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    /// API key (environment variable reference or direct value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Base URL override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Organization ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,

    /// Model whitelist
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub whitelist: Vec<String>,

    /// Model blacklist
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blacklist: Vec<String>,

    /// Model configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub models: HashMap<String, Value>,

    /// Provider options
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,

    /// Additional fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default model for general use
    #[serde(default = "default_model")]
    pub default: String,

    /// Model for fast operations
    #[serde(default = "default_fast_model")]
    pub fast: String,

    /// Model for complex reasoning
    #[serde(default = "default_reasoning_model")]
    pub reasoning: String,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_fast_model() -> String {
    "claude-haiku-4-20250514".to_string()
}

fn default_reasoning_model() -> String {
    "claude-opus-4-20250514".to_string()
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
            fast: default_fast_model(),
            reasoning: default_reasoning_model(),
        }
    }
}

/// Tool configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Timeout for shell commands (ms)
    #[serde(default = "default_timeout")]
    pub shell_timeout_ms: u64,

    /// Maximum file size to read
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Whether to use PTY for shell
    #[serde(default)]
    pub use_pty: bool,
}

fn default_timeout() -> u64 {
    120_000
}

fn default_max_file_size() -> u64 {
    50 * 1024 * 1024 // 50MB
}

/// Agent configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Temperature setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Agent description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Disabled flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,

    /// Agent mode (subagent, primary, all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Hidden from autocomplete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,

    /// Color (hex format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Maximum steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,

    /// Permission settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Value>,

    /// Agent options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,

    /// Additional fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Command configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandConfig {
    /// Template content
    pub template: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Agent to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Subtask mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
}

/// MCP configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP server configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<Value>,

    /// Additional MCP servers
    #[serde(flatten)]
    pub servers: HashMap<String, Value>,
}

/// Permission configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Read permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<Value>,

    /// Edit permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<Value>,

    /// Bash permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash: Option<Value>,

    /// Glob permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<Value>,

    /// Grep permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grep: Option<Value>,

    /// Task permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Value>,

    /// Additional permissions
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Server configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Port to listen on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// Hostname to listen on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// Enable mDNS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mdns: Option<bool>,

    /// CORS domains
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cors: Vec<String>,

    /// API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Keybinds configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindsConfig {
    /// Leader key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader: Option<String>,

    /// Additional keybinds
    #[serde(flatten)]
    pub binds: HashMap<String, String>,
}

/// TUI configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TuiConfig {
    /// Scroll speed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_speed: Option<f64>,

    /// Scroll acceleration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_acceleration: Option<Value>,

    /// Diff style
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_style: Option<String>,
}

/// Workspace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Subdirectory mappings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirs: Option<Value>,
}

/// Compaction configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Auto-compact enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,

    /// Prune old outputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prune: Option<bool>,
}

/// Vault configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultConfig {
    /// Vault enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Auto-inject credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_inject: Option<bool>,
}

/// Secrets configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// LLM provider API keys
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub llm: HashMap<String, Option<String>>,

    /// Channel API keys
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub channels: HashMap<String, Option<String>>,

    /// External service API keys
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub external: HashMap<String, Option<String>>,
}

// ============================================================================
// Configuration Loader
// ============================================================================

/// Configuration loader with JSONC support and multi-file merging
pub struct ConfigLoader {
    /// Configuration search paths
    paths: Vec<PathBuf>,

    /// Environment variable prefix for overrides
    env_prefix: String,

    /// Home directory
    home_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with default paths
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".codecoder");

        Self {
            paths: vec![config_dir],
            env_prefix: "CCODE_".to_string(),
            home_dir,
        }
    }

    /// Create a config loader with custom paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            env_prefix: "CCODE_".to_string(),
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
        }
    }

    /// Add a path to search
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.paths.push(path.into());
    }

    /// Get the config directory
    pub fn config_dir(&self) -> &Path {
        self.paths.first().map(|p| p.as_path()).unwrap_or(Path::new("."))
    }

    /// Get the home directory
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    /// Load a single JSONC file
    pub fn load_jsonc_file(&self, path: &Path) -> Result<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        self.parse_jsonc(&content, path)
    }

    /// Parse JSONC content
    pub fn parse_jsonc(&self, content: &str, path: &Path) -> Result<Config> {
        // Replace environment variable references {env:VAR}
        let processed = self.expand_env_vars(content);

        // Parse using json5 (supports comments and trailing commas)
        let config: Config = json5::from_str(&processed)
            .with_context(|| format!("Failed to parse JSONC: {}", path.display()))?;

        Ok(config)
    }

    /// Expand environment variable references in config content
    fn expand_env_vars(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Replace {env:VAR} patterns
        let re = regex::Regex::new(r"\{env:([^}]+)\}").unwrap();
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                env::var(&caps[1]).unwrap_or_default()
            })
            .to_string();

        result
    }

    /// Load configuration from a single path
    pub fn load(&self) -> Result<Config> {
        let config_path = self.config_dir().join("config.json");
        self.load_jsonc_file(&config_path)
    }

    /// Load and merge configuration from multiple files
    pub fn load_merged(&self) -> Result<Config> {
        let mut result = Config::default();

        // Load from each path in order
        for base_path in &self.paths {
            // Try multiple config file names
            for name in &["config.json", "codecoder.json", "codecoder.jsonc", "config.jsonc"] {
                let path = base_path.join(name);
                if path.exists() {
                    let config = self.load_jsonc_file(&path)?;
                    result = self.merge_configs(result, config);
                }
            }
        }

        // Apply environment variable overrides
        self.apply_env_overrides(&mut result);

        Ok(result)
    }

    /// Merge two configs, with source taking precedence
    pub fn merge_configs(&self, mut base: Config, source: Config) -> Config {
        // Merge providers
        for (key, value) in source.provider {
            base.provider.insert(key, value);
        }

        // Merge agents
        for (key, value) in source.agent {
            base.agent.insert(key, value);
        }

        // Merge modes
        for (key, value) in source.mode {
            base.mode.insert(key, value);
        }

        // Merge commands
        for (key, value) in source.command {
            base.command.insert(key, value);
        }

        // Concatenate instructions (don't replace)
        let mut instructions = base.instructions;
        for inst in source.instructions {
            if !instructions.contains(&inst) {
                instructions.push(inst);
            }
        }
        base.instructions = instructions;

        // Override simple fields if present
        if source.schema.is_some() {
            base.schema = source.schema;
        }
        if source.theme.is_some() {
            base.theme = source.theme;
        }
        if source.log_level.is_some() {
            base.log_level = source.log_level;
        }
        if source.model.is_some() {
            base.model = source.model;
        }
        if source.small_model.is_some() {
            base.small_model = source.small_model;
        }
        if source.default_agent.is_some() {
            base.default_agent = source.default_agent;
        }
        if source.username.is_some() {
            base.username = source.username;
        }
        if source.mcp.is_some() {
            base.mcp = source.mcp;
        }
        if source.permission.is_some() {
            base.permission = source.permission;
        }
        if source.server.is_some() {
            base.server = source.server;
        }
        if source.keybinds.is_some() {
            base.keybinds = source.keybinds;
        }
        if source.tui.is_some() {
            base.tui = source.tui;
        }
        if source.workspace.is_some() {
            base.workspace = source.workspace;
        }
        if source.compaction.is_some() {
            base.compaction = source.compaction;
        }
        if source.autoupdate.is_some() {
            base.autoupdate = source.autoupdate;
        }
        if source.vault.is_some() {
            base.vault = source.vault;
        }
        if source.secrets.is_some() {
            base.secrets = source.secrets;
        }
        if source.zerobot.is_some() {
            base.zerobot = source.zerobot;
        }
        if source.llm.is_some() {
            base.llm = source.llm;
        }
        if source.experimental.is_some() {
            base.experimental = source.experimental;
        }
        if source.autonomous_mode.is_some() {
            base.autonomous_mode = source.autonomous_mode;
        }
        if source.redis.is_some() {
            base.redis = source.redis;
        }
        if source.task_queue.is_some() {
            base.task_queue = source.task_queue;
        }
        if source.network.is_some() {
            base.network = source.network;
        }
        if source.services.is_some() {
            base.services = source.services;
        }
        if source.lsp.is_some() {
            base.lsp = source.lsp;
        }
        if source.formatter.is_some() {
            base.formatter = source.formatter;
        }
        if source.watcher.is_some() {
            base.watcher = source.watcher;
        }
        if source.enterprise.is_some() {
            base.enterprise = source.enterprise;
        }

        // Merge disabled/enabled providers
        if !source.disabled_providers.is_empty() {
            base.disabled_providers = source.disabled_providers;
        }
        if !source.enabled_providers.is_empty() {
            base.enabled_providers = source.enabled_providers;
        }

        // Merge extra fields
        for (key, value) in source.extra {
            base.extra.insert(key, value);
        }

        base
    }

    /// Apply environment variable overrides to config
    pub fn apply_env_overrides(&self, config: &mut Config) {
        // LLM API keys
        let key_map: &[(&str, &str)] = &[
            ("ANTHROPIC_API_KEY", "anthropic"),
            ("OPENAI_API_KEY", "openai"),
            ("DEEPSEEK_API_KEY", "deepseek"),
            ("GOOGLE_API_KEY", "google"),
            ("OPENROUTER_API_KEY", "openrouter"),
            ("GROQ_API_KEY", "groq"),
            ("MISTRAL_API_KEY", "mistral"),
            ("XAI_API_KEY", "xai"),
            ("TOGETHER_API_KEY", "together"),
            ("FIREWORKS_API_KEY", "fireworks"),
            ("PERPLEXITY_API_KEY", "perplexity"),
        ];

        for (env_var, provider) in key_map {
            if let Ok(value) = env::var(env_var) {
                let secrets = config.secrets.get_or_insert_with(SecretsConfig::default);
                secrets.llm.insert(provider.to_string(), Some(value));
            }
        }

        // External API keys
        if let Ok(value) = env::var("LIXIN_API_KEY") {
            let secrets = config.secrets.get_or_insert_with(SecretsConfig::default);
            secrets.external.insert("lixin".to_string(), Some(value));
        }
        if let Ok(value) = env::var("ITICK_API_KEY") {
            let secrets = config.secrets.get_or_insert_with(SecretsConfig::default);
            secrets.external.insert("itick".to_string(), Some(value));
        }

        // CCODE_* environment overrides
        if env::var("CCODE_DISABLE_AUTOCOMPACT").is_ok() {
            let compaction = config.compaction.get_or_insert_with(CompactionConfig::default);
            compaction.auto = Some(false);
        }
        if env::var("CCODE_DISABLE_PRUNE").is_ok() {
            let compaction = config.compaction.get_or_insert_with(CompactionConfig::default);
            compaction.prune = Some(false);
        }

        // Get username from system if not set
        if config.username.is_none() {
            config.username = env::var("USER")
                .or_else(|_| env::var("USERNAME"))
                .ok();
        }
    }

    /// Scan a directory for .codecoder config directories
    pub fn scan_directory(&self, start: &Path, stop: Option<&Path>) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut current = start.to_path_buf();

        loop {
            let codecoder_dir = current.join(".codecoder");
            if codecoder_dir.exists() && codecoder_dir.is_dir() {
                result.push(codecoder_dir);
            }

            // Stop at the specified directory
            if let Some(stop_dir) = stop {
                if current == stop_dir {
                    break;
                }
            }

            // Move to parent
            if let Some(parent) = current.parent() {
                if parent == current {
                    break; // Reached root
                }
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        result.reverse(); // Return in order from root to start
        result
    }

    /// Find config files in a directory
    pub fn find_config_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();

        for name in &["config.json", "codecoder.json", "codecoder.jsonc", "config.jsonc"] {
            let path = dir.join(name);
            if path.exists() {
                result.push(path);
            }
        }

        result
    }

    /// Save configuration to file
    pub fn save(&self, config: &Config) -> Result<()> {
        let config_dir = self.config_dir();
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;

        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config: {}", config_path.display()))?;

        Ok(())
    }

    /// Load secrets from secrets.json
    pub fn load_secrets(&self) -> Result<HashMap<String, String>> {
        let secrets_path = self.config_dir().join("secrets.json");

        if !secrets_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&secrets_path)
            .with_context(|| format!("Failed to read secrets: {}", secrets_path.display()))?;

        let secrets: HashMap<String, String> =
            serde_json::from_str(&content).with_context(|| "Failed to parse secrets.json")?;

        Ok(secrets)
    }

    /// Parse a raw JSON/JSONC string to Config
    pub fn parse_raw(&self, content: &str) -> Result<Config> {
        let processed = self.expand_env_vars(content);
        json5::from_str(&processed).context("Failed to parse config")
    }

    /// Convert Config to JSON Value
    pub fn to_value(&self, config: &Config) -> Result<Value> {
        serde_json::to_value(config).context("Failed to convert config to value")
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.provider.is_empty());
        assert_eq!(config.models.default, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.provider.insert(
            "anthropic".to_string(),
            ProviderConfig {
                api_key: Some("test-key".to_string()),
                ..Default::default()
            },
        );

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed.provider.get("anthropic").unwrap().api_key,
            Some("test-key".to_string())
        );
    }

    #[test]
    fn test_jsonc_parsing() {
        let loader = ConfigLoader::new();
        let content = r#"{
            // This is a comment
            "model": "test-model",
            "theme": "dark", // inline comment
        }"#;

        let config = loader.parse_raw(content).unwrap();
        assert_eq!(config.model, Some("test-model".to_string()));
        assert_eq!(config.theme, Some("dark".to_string()));
    }

    #[test]
    fn test_env_var_expansion() {
        let loader = ConfigLoader::new();
        env::set_var("TEST_VAR", "test-value");

        let content = r#"{"model": "{env:TEST_VAR}"}"#;
        let processed = loader.expand_env_vars(content);

        assert!(processed.contains("test-value"));
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_config_merge() {
        let loader = ConfigLoader::new();

        let base = Config {
            theme: Some("light".to_string()),
            instructions: vec!["base-instruction".to_string()],
            ..Default::default()
        };

        let source = Config {
            theme: Some("dark".to_string()),
            instructions: vec!["source-instruction".to_string()],
            model: Some("new-model".to_string()),
            ..Default::default()
        };

        let merged = loader.merge_configs(base, source);

        assert_eq!(merged.theme, Some("dark".to_string()));
        assert_eq!(merged.model, Some("new-model".to_string()));
        assert_eq!(merged.instructions.len(), 2);
        assert!(merged.instructions.contains(&"base-instruction".to_string()));
        assert!(merged.instructions.contains(&"source-instruction".to_string()));
    }

    #[test]
    fn test_config_loader() {
        let dir = tempfile::TempDir::new().unwrap();
        let loader = ConfigLoader::with_paths(vec![dir.path().to_path_buf()]);

        // Should return default when no file exists
        let config = loader.load().unwrap();
        assert!(config.provider.is_empty());

        // Save and reload
        let mut config = Config::default();
        config.models.default = "custom-model".to_string();
        loader.save(&config).unwrap();

        let loaded = loader.load().unwrap();
        assert_eq!(loaded.models.default, "custom-model");
    }

    #[test]
    fn test_scan_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let codecoder_dir = dir.path().join(".codecoder");
        fs::create_dir_all(&codecoder_dir).unwrap();

        let loader = ConfigLoader::new();
        let found = loader.scan_directory(dir.path(), None);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0], codecoder_dir);
    }
}
