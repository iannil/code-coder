//! NAPI bindings for config module
//!
//! Exposes ConfigLoader to Node.js/TypeScript for high-performance config loading.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::foundation::config::{
    AgentConfig as RustAgentConfig, CommandConfig as RustCommandConfig, Config as RustConfig,
    ConfigLoader as RustConfigLoader, ProviderConfig as RustProviderConfig,
    SecretsConfig as RustSecretsConfig, ServerConfig as RustServerConfig,
};

// ============================================================================
// Type Definitions
// ============================================================================

/// Provider configuration
#[napi(object)]
pub struct NapiProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub organization: Option<String>,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}

impl From<RustProviderConfig> for NapiProviderConfig {
    fn from(p: RustProviderConfig) -> Self {
        Self {
            api_key: p.api_key,
            base_url: p.base_url,
            organization: p.organization,
            whitelist: p.whitelist,
            blacklist: p.blacklist,
        }
    }
}

impl From<NapiProviderConfig> for RustProviderConfig {
    fn from(p: NapiProviderConfig) -> Self {
        Self {
            api_key: p.api_key,
            base_url: p.base_url,
            organization: p.organization,
            whitelist: p.whitelist,
            blacklist: p.blacklist,
            models: HashMap::new(),
            options: None,
            extra: HashMap::new(),
        }
    }
}

/// Agent configuration
#[napi(object)]
pub struct NapiAgentConfig {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub prompt: Option<String>,
    pub description: Option<String>,
    pub disable: Option<bool>,
    pub mode: Option<String>,
    pub hidden: Option<bool>,
    pub color: Option<String>,
    pub steps: Option<u32>,
}

impl From<RustAgentConfig> for NapiAgentConfig {
    fn from(a: RustAgentConfig) -> Self {
        Self {
            model: a.model,
            temperature: a.temperature,
            top_p: a.top_p,
            prompt: a.prompt,
            description: a.description,
            disable: a.disable,
            mode: a.mode,
            hidden: a.hidden,
            color: a.color,
            steps: a.steps,
        }
    }
}

impl From<NapiAgentConfig> for RustAgentConfig {
    fn from(a: NapiAgentConfig) -> Self {
        Self {
            model: a.model,
            temperature: a.temperature,
            top_p: a.top_p,
            prompt: a.prompt,
            description: a.description,
            disable: a.disable,
            mode: a.mode,
            hidden: a.hidden,
            color: a.color,
            steps: a.steps,
            permission: None,
            options: None,
            extra: HashMap::new(),
        }
    }
}

/// Command configuration
#[napi(object)]
pub struct NapiCommandConfig {
    pub template: String,
    pub description: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub subtask: Option<bool>,
}

impl From<RustCommandConfig> for NapiCommandConfig {
    fn from(c: RustCommandConfig) -> Self {
        Self {
            template: c.template,
            description: c.description,
            agent: c.agent,
            model: c.model,
            subtask: c.subtask,
        }
    }
}

impl From<NapiCommandConfig> for RustCommandConfig {
    fn from(c: NapiCommandConfig) -> Self {
        Self {
            template: c.template,
            description: c.description,
            agent: c.agent,
            model: c.model,
            subtask: c.subtask,
        }
    }
}

/// Server configuration
#[napi(object)]
pub struct NapiServerConfig {
    pub port: Option<u32>,
    pub hostname: Option<String>,
    pub mdns: Option<bool>,
    pub cors: Vec<String>,
    pub api_key: Option<String>,
}

impl From<RustServerConfig> for NapiServerConfig {
    fn from(s: RustServerConfig) -> Self {
        Self {
            port: s.port.map(|p| p as u32),
            hostname: s.hostname,
            mdns: s.mdns,
            cors: s.cors,
            api_key: s.api_key,
        }
    }
}

/// Secrets configuration
#[napi(object)]
pub struct NapiSecretsConfig {
    pub llm: HashMap<String, Option<String>>,
    pub channels: HashMap<String, Option<String>>,
    pub external: HashMap<String, Option<String>>,
}

impl From<RustSecretsConfig> for NapiSecretsConfig {
    fn from(s: RustSecretsConfig) -> Self {
        Self {
            llm: s.llm,
            channels: s.channels,
            external: s.external,
        }
    }
}

/// Main configuration (simplified for NAPI)
#[napi(object)]
pub struct NapiConfig {
    /// Theme name
    pub theme: Option<String>,
    /// Log level
    pub log_level: Option<String>,
    /// Model string
    pub model: Option<String>,
    /// Small model string
    pub small_model: Option<String>,
    /// Default agent
    pub default_agent: Option<String>,
    /// Username
    pub username: Option<String>,
    /// Disabled providers
    pub disabled_providers: Vec<String>,
    /// Enabled providers
    pub enabled_providers: Vec<String>,
    /// Instructions
    pub instructions: Vec<String>,
}

impl From<&RustConfig> for NapiConfig {
    fn from(c: &RustConfig) -> Self {
        Self {
            theme: c.theme.clone(),
            log_level: c.log_level.clone(),
            model: c.model.clone(),
            small_model: c.small_model.clone(),
            default_agent: c.default_agent.clone(),
            username: c.username.clone(),
            disabled_providers: c.disabled_providers.clone(),
            enabled_providers: c.enabled_providers.clone(),
            instructions: c.instructions.clone(),
        }
    }
}

/// Config load result with directories
#[napi(object)]
pub struct NapiConfigLoadResult {
    /// The loaded configuration as JSON string
    pub config_json: String,
    /// Directories that were scanned
    pub directories: Vec<String>,
}

// ============================================================================
// ConfigLoaderHandle
// ============================================================================

/// Handle to a configuration loader for high-performance operations
#[napi]
pub struct ConfigLoaderHandle {
    inner: Arc<Mutex<RustConfigLoader>>,
    config: Arc<Mutex<Option<RustConfig>>>,
}

/// Create a new config loader
#[napi]
pub fn create_config_loader(paths: Option<Vec<String>>) -> ConfigLoaderHandle {
    let loader = match paths {
        Some(p) => RustConfigLoader::with_paths(p.into_iter().map(PathBuf::from).collect()),
        None => RustConfigLoader::new(),
    };

    ConfigLoaderHandle {
        inner: Arc::new(Mutex::new(loader)),
        config: Arc::new(Mutex::new(None)),
    }
}

#[napi]
impl ConfigLoaderHandle {
    /// Get the config directory path
    #[napi]
    pub fn config_dir(&self) -> String {
        let loader = self.inner.lock().unwrap();
        loader.config_dir().to_string_lossy().to_string()
    }

    /// Get the home directory path
    #[napi]
    pub fn home_dir(&self) -> String {
        let loader = self.inner.lock().unwrap();
        loader.home_dir().to_string_lossy().to_string()
    }

    /// Add a search path
    #[napi]
    pub fn add_path(&self, path: String) {
        let mut loader = self.inner.lock().unwrap();
        loader.add_path(PathBuf::from(path));
    }

    /// Load configuration from a single file
    #[napi]
    pub fn load_file(&self, path: String) -> Result<String> {
        let loader = self.inner.lock().unwrap();
        let config = loader
            .load_jsonc_file(&PathBuf::from(path))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let json = serde_json::to_string(&config).map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(json)
    }

    /// Parse JSONC content
    #[napi]
    pub fn parse_jsonc(&self, content: String) -> Result<String> {
        let loader = self.inner.lock().unwrap();
        let config = loader
            .parse_raw(&content)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let json = serde_json::to_string(&config).map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(json)
    }

    /// Load merged configuration from all paths
    #[napi]
    pub fn load_merged(&self) -> Result<String> {
        let loader = self.inner.lock().unwrap();
        let config = loader
            .load_merged()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        // Cache the config
        *self.config.lock().unwrap() = Some(config.clone());

        let json = serde_json::to_string(&config).map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(json)
    }

    /// Get simplified config object
    #[napi]
    pub fn get_config(&self) -> Result<NapiConfig> {
        let config_guard = self.config.lock().unwrap();
        let config = config_guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Config not loaded. Call load_merged first."))?;

        Ok(NapiConfig::from(config))
    }

    /// Get provider configurations
    #[napi]
    pub fn get_providers(&self) -> Result<HashMap<String, NapiProviderConfig>> {
        let config_guard = self.config.lock().unwrap();
        let config = config_guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Config not loaded. Call load_merged first."))?;

        Ok(config
            .provider
            .iter()
            .map(|(k, v)| (k.clone(), NapiProviderConfig::from(v.clone())))
            .collect())
    }

    /// Get agent configurations
    #[napi]
    pub fn get_agents(&self) -> Result<HashMap<String, NapiAgentConfig>> {
        let config_guard = self.config.lock().unwrap();
        let config = config_guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Config not loaded. Call load_merged first."))?;

        Ok(config
            .agent
            .iter()
            .map(|(k, v)| (k.clone(), NapiAgentConfig::from(v.clone())))
            .collect())
    }

    /// Get command configurations
    #[napi]
    pub fn get_commands(&self) -> Result<HashMap<String, NapiCommandConfig>> {
        let config_guard = self.config.lock().unwrap();
        let config = config_guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Config not loaded. Call load_merged first."))?;

        Ok(config
            .command
            .iter()
            .map(|(k, v)| (k.clone(), NapiCommandConfig::from(v.clone())))
            .collect())
    }

    /// Get secrets configuration
    #[napi]
    pub fn get_secrets(&self) -> Result<Option<NapiSecretsConfig>> {
        let config_guard = self.config.lock().unwrap();
        let config = config_guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Config not loaded. Call load_merged first."))?;

        Ok(config.secrets.as_ref().map(|s| NapiSecretsConfig::from(s.clone())))
    }

    /// Scan directory for .codecoder directories
    #[napi]
    pub fn scan_directory(&self, start: String, stop: Option<String>) -> Vec<String> {
        let loader = self.inner.lock().unwrap();
        let stop_path = stop.as_ref().map(|s| PathBuf::from(s));
        loader
            .scan_directory(&PathBuf::from(start), stop_path.as_deref())
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// Find config files in a directory
    #[napi]
    pub fn find_config_files(&self, dir: String) -> Vec<String> {
        let loader = self.inner.lock().unwrap();
        loader
            .find_config_files(&PathBuf::from(dir))
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// Save configuration to file
    #[napi]
    pub fn save(&self, config_json: String) -> Result<()> {
        let loader = self.inner.lock().unwrap();
        let config: RustConfig =
            serde_json::from_str(&config_json).map_err(|e| Error::from_reason(e.to_string()))?;

        loader.save(&config).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load secrets from secrets.json
    #[napi]
    pub fn load_secrets(&self) -> Result<HashMap<String, String>> {
        let loader = self.inner.lock().unwrap();
        loader
            .load_secrets()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Merge two configs (base + source), returns JSON string
    #[napi]
    pub fn merge_configs(&self, base_json: String, source_json: String) -> Result<String> {
        let loader = self.inner.lock().unwrap();
        let base: RustConfig =
            serde_json::from_str(&base_json).map_err(|e| Error::from_reason(e.to_string()))?;
        let source: RustConfig =
            serde_json::from_str(&source_json).map_err(|e| Error::from_reason(e.to_string()))?;

        let merged = loader.merge_configs(base, source);
        serde_json::to_string(&merged).map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_config_loader() {
        let handle = create_config_loader(None);
        let dir = handle.config_dir();
        assert!(dir.contains(".codecoder"));
    }

    #[test]
    fn test_parse_jsonc() {
        let handle = create_config_loader(None);
        let content = r#"{ "theme": "dark" }"#;
        let result = handle.parse_jsonc(content.to_string());
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("dark"));
    }
}
