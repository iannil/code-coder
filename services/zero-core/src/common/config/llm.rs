//! LLM (Large Language Model) configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

// Default value functions

fn default_llm_model() -> String {
    "anthropic/claude-sonnet-4-20250514".into()
}

fn default_llm_retries() -> u32 {
    2
}

fn default_llm_backoff_ms() -> u64 {
    1000
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_defaults() {
        let config = LlmConfig::default();
        assert!(config.default.contains("claude"));
        assert_eq!(config.retries, 2);
        assert_eq!(config.backoff_ms, 1000);
    }

    #[test]
    fn test_ollama_defaults() {
        let config = LlmOllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.default_model, "llama3");
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_provider_settings() {
        let settings = ProviderSettings {
            default: Some("openai/gpt-4".to_string()),
            retries: Some(3),
            backoff_ms: Some(2000),
            fallbacks: vec!["anthropic/claude-3-opus".to_string()],
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: ProviderSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default, Some("openai/gpt-4".to_string()));
    }
}
