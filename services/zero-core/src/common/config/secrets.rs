//! Secrets configuration for API keys and credentials.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    #[serde(default)]
    pub uniapi: Option<String>,
    #[serde(default)]
    pub volces: Option<String>,
    #[serde(default, rename = "zhipu-ai")]
    pub zhipu_ai: Option<String>,
    /// Additional custom providers not explicitly defined
    #[serde(flatten)]
    pub other: HashMap<String, String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_default() {
        let config = SecretsConfig::default();
        assert!(config.llm.anthropic.is_none());
        assert!(config.channels.telegram_bot_token.is_none());
        assert!(config.external.lixin.is_none());
    }

    #[test]
    fn test_secrets_serialization() {
        let mut config = SecretsConfig::default();
        config.llm.anthropic = Some("sk-test".to_string());

        let json = serde_json::to_string(&config).unwrap();
        let parsed: SecretsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.llm.anthropic, Some("sk-test".to_string()));
    }
}
