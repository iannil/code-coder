/// ─── Config ────────────────────────────────────────────────────────────────
///
/// File-based configuration (codecoder.json) with env-var override.
/// Priority: environment variables > file config > built-in defaults.
///
/// API keys are NEVER stored in the config file — they are read from
/// environment variables only (CODECODER_API_KEY or OPENAI_API_KEY).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// ─── CodeCoderConfig ───────────────────────────────────────────────────────

/// Top-level configuration, all fields optional so partial config files work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeCoderConfig {
    pub llm: LlmConfigSection,
    pub github_token: String,
    pub features: FeatureFlags,
    pub mcp_servers: Vec<crate::mcp::McpServerConfig>,
    /// Scheduled tasks for autonomous daemon mode
    pub scheduled_tasks: Vec<ScheduledTaskConfig>,
    /// Directories to watch for file changes
    pub watch_paths: Vec<String>,
}

/// Configuration for a scheduled task in daemon mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskConfig {
    pub name: String,
    pub prompt: String,
    pub interval_secs: u64,
}

/// LLM-specific config stored in the file.
/// Note: api_key is intentionally absent — it stays in env vars only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfigSection {
    pub api_base: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Feature flags and runtime settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureFlags {
    pub sandbox_enabled: bool,
    pub auto_save_session: bool,
    /// Max tool-call rounds per agent loop (default: 10)
    pub max_tool_rounds: usize,
    /// Permission request timeout in seconds (default: 30)
    pub permission_timeout_secs: u64,
    /// Default command execution timeout in seconds (default: 30)
    pub command_timeout_secs: u64,
    /// Docker sandbox memory limit (default: "128m")
    pub sandbox_memory_limit: String,
    /// LLM HTTP request timeout in seconds (default: 120)
    pub http_timeout_secs: u64,
}

/// ─── Defaults ──────────────────────────────────────────────────────────────

impl Default for CodeCoderConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfigSection::default(),
            github_token: String::new(),
            features: FeatureFlags::default(),
            mcp_servers: Vec::new(),
            scheduled_tasks: Vec::new(),
            watch_paths: Vec::new(),
        }
    }
}

impl Default for LlmConfigSection {
    fn default() -> Self {
        Self {
            api_base: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            auto_save_session: true,
            max_tool_rounds: 10,
            permission_timeout_secs: 30,
            command_timeout_secs: 30,
            sandbox_memory_limit: "128m".into(),
            http_timeout_secs: 120,
        }
    }
}

/// ─── ConfigStore ───────────────────────────────────────────────────────────

pub struct ConfigStore {
    file_path: PathBuf,
    config: CodeCoderConfig,
}

impl ConfigStore {
    /// Load config from `codecoder.json` in the project root, merged with env vars.
    pub fn load(project_root: &str) -> Self {
        let file_path = Path::new(project_root).join("codecoder.json");
        let file_config = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|content| serde_json::from_str::<CodeCoderConfig>(&content).ok())
                .unwrap_or_default()
        } else {
            CodeCoderConfig::default()
        };

        // Merge: env vars override file config
        let config = CodeCoderConfig {
            llm: LlmConfigSection {
                api_base: std::env::var("CODECODER_API_BASE")
                    .or_else(|_| std::env::var("OPENAI_API_BASE"))
                    .unwrap_or(file_config.llm.api_base),
                model: std::env::var("CODECODER_MODEL")
                    .or_else(|_| std::env::var("OPENAI_MODEL"))
                    .unwrap_or(file_config.llm.model),
                max_tokens: std::env::var("CODECODER_MAX_TOKENS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(file_config.llm.max_tokens),
                temperature: std::env::var("CODECODER_TEMPERATURE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(file_config.llm.temperature),
            },
            github_token: std::env::var("GITHUB_TOKEN")
                .ok()
                .unwrap_or(file_config.github_token),
            features: file_config.features,
            mcp_servers: file_config.mcp_servers,
            scheduled_tasks: file_config.scheduled_tasks,
            watch_paths: file_config.watch_paths,
        };

        Self { file_path, config }
    }

    /// Save current config to `codecoder.json`.
    pub fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }

    /// Get a reference to the loaded config.
    pub fn get(&self) -> &CodeCoderConfig {
        &self.config
    }

    /// Get mutable access to the loaded config (for runtime changes).
    pub fn get_mut(&mut self) -> &mut CodeCoderConfig {
        &mut self.config
    }

    /// Build an LlmConfig for the LLM client, reading api_key from env.
    pub fn to_llm_config(&self) -> crate::llm::LlmConfig {
        crate::llm::LlmConfig {
            api_base: self.config.llm.api_base.clone(),
            model: self.config.llm.model.clone(),
            api_key: std::env::var("CODECODER_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
            max_tokens: self.config.llm.max_tokens,
            temperature: self.config.llm.temperature,
        }
    }

    /// Get effective model name.
    pub fn model(&self) -> &str {
        &self.config.llm.model
    }

    /// Set model at runtime (in memory; call save() to persist).
    pub fn set_model(&mut self, model: &str) {
        self.config.llm.model = model.to_string();
    }

    /// Get config formatted for display.
    pub fn format_display(&self) -> String {
        let c = &self.config;
        let mut lines = vec![
            "── CodeCoder Config ──".to_string(),
            format!("  Model:        {}", c.llm.model),
            format!("  API Base:     {}", c.llm.api_base),
            format!("  Max Tokens:   {}", c.llm.max_tokens),
            format!("  Temperature:  {:.1}", c.llm.temperature),
            format!("  API Key Set:  {}", is_env_key_set()),
            format!("  Sandbox:      {}", if c.features.sandbox_enabled { "enabled" } else { "disabled" }),
            format!("  Tool Rounds:  {}", c.features.max_tool_rounds),
            format!("  Cmd Timeout:  {}s", c.features.command_timeout_secs),
            format!("  HTTP Timeout: {}s", c.features.http_timeout_secs),
            format!("  Sandbox Mem:  {}", c.features.sandbox_memory_limit),
            format!("  Config File:  {}", self.file_path.display()),
        ];
        if !c.github_token.is_empty() {
            lines.push(format!("  GitHub Token: set"));
        }
        let mcp_count = c.mcp_servers.len();
        if mcp_count > 0 {
            lines.push(format!("  MCP Servers:  {} configured", mcp_count));
            for s in &c.mcp_servers {
                lines.push(format!("    · {}: {} {}", s.name, s.command, s.args.join(" ")));
            }
        }
        let task_count = c.scheduled_tasks.len();
        if task_count > 0 {
            lines.push(format!("  Schedules:    {} tasks", task_count));
        }
        let watch_count = c.watch_paths.len();
        if watch_count > 0 {
            lines.push(format!("  Watchers:     {} paths", watch_count));
        }
        lines.push(String::new());
        lines.push("Use /config set <key> <value> to change.".to_string());
        lines.push("Keys: model, api_base, max_tokens, temperature".to_string());
        lines.join("\n")
    }
}

/// Check if API key is available in environment.
fn is_env_key_set() -> bool {
    std::env::var("CODECODER_API_KEY").is_ok() || std::env::var("OPENAI_API_KEY").is_ok()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = CodeCoderConfig::default();
        assert_eq!(config.llm.model, "gpt-4o");
        assert_eq!(config.llm.api_base, "https://api.openai.com/v1");
        assert_eq!(config.llm.max_tokens, 4096);
        assert!(config.features.sandbox_enabled);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::load(dir.path().to_str().unwrap());
        assert_eq!(store.model(), "gpt-4o");
    }

    #[test]
    fn test_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // Save default config
        {
            let store = ConfigStore::load(root);
            store.save().unwrap();
        }

        // Reload should read the file
        let _store = ConfigStore::load(root);
        // Model might be overridden by env, but file must exist
        assert!(dir.path().join("codecoder.json").exists());

        // Verify saved JSON is valid by re-parsing it
        let saved_content = std::fs::read_to_string(dir.path().join("codecoder.json")).unwrap();
        let parsed: CodeCoderConfig = serde_json::from_str(&saved_content).unwrap();
        assert_eq!(parsed.llm.api_base, "https://api.openai.com/v1");
    }

    #[test]
    fn test_set_model() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ConfigStore::load(dir.path().to_str().unwrap());
        store.set_model("claude-sonnet-4");
        assert_eq!(store.model(), "claude-sonnet-4");

        store.save().unwrap();

        // Re-load and verify file takes effect when no env override
        let store2 = ConfigStore::load(dir.path().to_str().unwrap());
        let env_model = std::env::var("CODECODER_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .unwrap_or_default();
        if env_model.is_empty() {
            assert_eq!(store2.model(), "claude-sonnet-4");
        }
    }

    #[test]
    fn test_format_display() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::load(dir.path().to_str().unwrap());
        let display = store.format_display();
        assert!(display.contains("Model:"));
        assert!(display.contains("API Base:"));
        assert!(display.contains("Config File:"));
    }

    #[test]
    fn test_partial_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("codecoder.json");

        // Write a partial config (only api_base)
        let partial = r#"{"llm": {"api_base": "http://localhost:11434"}}"#;
        std::fs::write(&config_path, partial).unwrap();

        #[allow(unused_mut)]
        let mut store = ConfigStore::load(dir.path().to_str().unwrap());
        assert_eq!(store.config.llm.api_base, "http://localhost:11434",
            "partial files should fill unspecified fields from defaults");

        // Unset fields use defaults
        assert_eq!(store.config.llm.max_tokens, 4096);
        assert_eq!(store.config.llm.model, "gpt-4o");
    }

    #[test]
    fn test_to_llm_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::load(dir.path().to_str().unwrap());
        let llm_config = store.to_llm_config();
        assert_eq!(llm_config.model, "gpt-4o");
        assert_eq!(llm_config.max_tokens, 4096);
    }
}
