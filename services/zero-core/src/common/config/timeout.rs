//! Timeout configuration for HTTP clients and task execution.

use serde::{Deserialize, Serialize};

/// Unified timeout configuration for all HTTP clients and task execution.
///
/// This provides consistent timeout settings across the Zero ecosystem,
/// preventing timeout failures from misconfigured or missing timeout values.
///
/// # Priority Order
///
/// Component-level config > Category timeout > Global default > Hardcoded fallback
///
/// Example: `trading.macro_agent.timeout_secs` > `timeout.llm_secs` > `timeout.default_secs` > 30s
///
/// # Example Configuration
///
/// ```json
/// {
///   "timeout": {
///     "default_secs": 30,
///     "connect_secs": 10,
///     "llm_secs": 300,
///     "notification_secs": 15,
///     "api_secs": 30,
///     "shell_secs": 120
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default timeout for general HTTP requests (seconds).
    /// Used when no specific category applies.
    #[serde(default = "default_timeout_default_secs")]
    pub default_secs: u64,

    /// TCP connection timeout (seconds).
    /// Applied to all HTTP clients for initial connection establishment.
    #[serde(default = "default_timeout_connect_secs")]
    pub connect_secs: u64,

    /// LLM API call timeout (seconds).
    /// Used for calls to AI providers (Claude, OpenAI, etc.) which can be slow.
    #[serde(default = "default_timeout_llm_secs")]
    pub llm_secs: u64,

    /// Notification/webhook timeout (seconds).
    /// Used for Telegram, Discord, Slack, and other notification services.
    #[serde(default = "default_timeout_notification_secs")]
    pub notification_secs: u64,

    /// External API timeout (seconds).
    /// Used for third-party APIs (market data, GitHub, etc.).
    #[serde(default = "default_timeout_api_secs")]
    pub api_secs: u64,

    /// Shell command execution timeout (seconds).
    /// Used for shell tool and subprocess execution.
    #[serde(default = "default_timeout_shell_secs")]
    pub shell_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_secs: default_timeout_default_secs(),
            connect_secs: default_timeout_connect_secs(),
            llm_secs: default_timeout_llm_secs(),
            notification_secs: default_timeout_notification_secs(),
            api_secs: default_timeout_api_secs(),
            shell_secs: default_timeout_shell_secs(),
        }
    }
}

/// Default general timeout: 30 seconds
pub fn default_timeout_default_secs() -> u64 {
    30
}

/// Default TCP connect timeout: 10 seconds
pub fn default_timeout_connect_secs() -> u64 {
    10
}

/// Default LLM timeout: 300 seconds (5 minutes)
/// LLM calls often take 30-180 seconds, especially for complex analysis
pub fn default_timeout_llm_secs() -> u64 {
    300
}

/// Default notification timeout: 15 seconds
pub fn default_timeout_notification_secs() -> u64 {
    15
}

/// Default external API timeout: 30 seconds
pub fn default_timeout_api_secs() -> u64 {
    30
}

/// Default shell timeout: 120 seconds (2 minutes)
pub fn default_timeout_shell_secs() -> u64 {
    120
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_defaults() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_secs, 30);
        assert_eq!(config.connect_secs, 10);
        assert_eq!(config.llm_secs, 300);
        assert_eq!(config.notification_secs, 15);
        assert_eq!(config.api_secs, 30);
        assert_eq!(config.shell_secs, 120);
    }

    #[test]
    fn test_timeout_serialization() {
        let config = TimeoutConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TimeoutConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.llm_secs, config.llm_secs);
    }
}
