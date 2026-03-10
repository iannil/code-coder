//! HTTP Client Factory with Unified Timeout Management.
//!
//! This module provides factory functions for building `reqwest::Client` instances
//! with proper timeout configuration. It ensures all HTTP clients in the Zero ecosystem
//! have consistent and appropriate timeouts.
//!
//! # Problem
//!
//! Before this module, ~60 places used `reqwest::Client::new()` with no timeout,
//! leading to hung requests and timeout failures, especially for LLM API calls.
//!
//! # Solution
//!
//! Use `build_client()` with a `ClientCategory` to get a properly configured client:
//!
//! ```rust,ignore
//! use zero_core::common::http_client::{build_client, ClientCategory};
//! use zero_core::common::Config;
//!
//! let config = Config::default();
//! let client = build_client(&config.timeout, ClientCategory::Llm);
//! ```
//!
//! # Categories
//!
//! - `Llm`: AI provider calls (300s default) - Claude, OpenAI, etc.
//! - `Notification`: IM/webhook calls (15s default) - Telegram, Discord, Slack
//! - `Api`: External API calls (30s default) - Market data, GitHub, etc.
//! - `General`: Default timeout (30s) - Fallback for unspecified use cases

use std::time::Duration;

use reqwest::Client;

use super::config::TimeoutConfig;

/// HTTP client usage categories with appropriate timeout defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCategory {
    /// LLM/AI provider API calls.
    /// These can take 30-180+ seconds for complex analysis.
    /// Default: 300 seconds (5 minutes)
    Llm,

    /// Notification and webhook calls.
    /// IM services (Telegram, Discord, Slack) and webhook endpoints.
    /// Default: 15 seconds
    Notification,

    /// External API calls.
    /// Third-party services like market data, GitHub, etc.
    /// Default: 30 seconds
    Api,

    /// General purpose HTTP requests.
    /// Fallback for unspecified use cases.
    /// Default: 30 seconds
    General,
}

/// Build an HTTP client with timeout based on usage category.
///
/// This is the primary factory function for creating HTTP clients in the Zero ecosystem.
///
/// # Arguments
///
/// * `config` - Timeout configuration from the unified config
/// * `category` - The type of HTTP usage (determines timeout)
///
/// # Returns
///
/// A configured `reqwest::Client` with appropriate timeouts.
///
/// # Example
///
/// ```rust,ignore
/// let config = Config::default();
///
/// // For LLM API calls (e.g., Claude, OpenAI)
/// let llm_client = build_client(&config.timeout, ClientCategory::Llm);
///
/// // For sending Telegram/Discord notifications
/// let notify_client = build_client(&config.timeout, ClientCategory::Notification);
///
/// // For external API calls (e.g., market data)
/// let api_client = build_client(&config.timeout, ClientCategory::Api);
/// ```
pub fn build_client(config: &TimeoutConfig, category: ClientCategory) -> Client {
    let timeout_secs = category_timeout(config, category);
    build_client_with_timeout(config, timeout_secs)
}

/// Build an HTTP client with an explicit timeout override.
///
/// Use this when you need a specific timeout that differs from the category defaults,
/// such as when a component has its own timeout configuration.
///
/// # Arguments
///
/// * `config` - Timeout configuration (used for connect timeout)
/// * `timeout_secs` - Request timeout in seconds
///
/// # Returns
///
/// A configured `reqwest::Client`.
///
/// # Example
///
/// ```rust,ignore
/// let config = Config::default();
///
/// // Use component-specific timeout (e.g., trading.macro_agent.timeout_secs)
/// let client = build_client_with_timeout(&config.timeout, 180);
/// ```
pub fn build_client_with_timeout(config: &TimeoutConfig, timeout_secs: u64) -> Client {
    build_client_internal(config.connect_secs, timeout_secs)
}

/// Build a blocking HTTP client with timeout based on usage category.
///
/// Use this for synchronous code that cannot use async (e.g., wizard/setup flows).
///
/// # Arguments
///
/// * `config` - Timeout configuration
/// * `category` - The type of HTTP usage (determines timeout)
///
/// # Returns
///
/// A configured blocking `reqwest::blocking::Client`.
#[cfg(feature = "blocking")]
pub fn build_blocking_client(
    config: &TimeoutConfig,
    category: ClientCategory,
) -> reqwest::blocking::Client {
    let timeout_secs = category_timeout(config, category);
    build_blocking_client_with_timeout(config, timeout_secs)
}

/// Build a blocking HTTP client with an explicit timeout override.
#[cfg(feature = "blocking")]
pub fn build_blocking_client_with_timeout(
    config: &TimeoutConfig,
    timeout_secs: u64,
) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(config.connect_secs))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| {
            // Fallback: create client with hardcoded defaults
            reqwest::blocking::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to build fallback HTTP client")
        })
}

/// Get the timeout value for a given category.
///
/// # Arguments
///
/// * `config` - Timeout configuration
/// * `category` - Client category
///
/// # Returns
///
/// Timeout in seconds for the specified category.
pub fn category_timeout(config: &TimeoutConfig, category: ClientCategory) -> u64 {
    match category {
        ClientCategory::Llm => config.llm_secs,
        ClientCategory::Notification => config.notification_secs,
        ClientCategory::Api => config.api_secs,
        ClientCategory::General => config.default_secs,
    }
}

/// Internal function to build the actual client.
fn build_client_internal(connect_secs: u64, timeout_secs: u64) -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(connect_secs))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| {
            // Fallback: create client with hardcoded defaults
            // This should rarely happen, but provides a safety net
            Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to build fallback HTTP client")
        })
}

/// Create a default client with category-appropriate timeout.
///
/// This is a convenience function that uses default TimeoutConfig.
/// Prefer `build_client()` with actual config when available.
pub fn default_client(category: ClientCategory) -> Client {
    build_client(&TimeoutConfig::default(), category)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_timeout_returns_correct_values() {
        let config = TimeoutConfig::default();

        assert_eq!(category_timeout(&config, ClientCategory::Llm), 300);
        assert_eq!(category_timeout(&config, ClientCategory::Notification), 15);
        assert_eq!(category_timeout(&config, ClientCategory::Api), 30);
        assert_eq!(category_timeout(&config, ClientCategory::General), 30);
    }

    #[test]
    fn test_category_timeout_respects_custom_config() {
        let config = TimeoutConfig {
            default_secs: 60,
            connect_secs: 5,
            llm_secs: 600,
            notification_secs: 30,
            api_secs: 45,
            shell_secs: 180,
        };

        assert_eq!(category_timeout(&config, ClientCategory::Llm), 600);
        assert_eq!(category_timeout(&config, ClientCategory::Notification), 30);
        assert_eq!(category_timeout(&config, ClientCategory::Api), 45);
        assert_eq!(category_timeout(&config, ClientCategory::General), 60);
    }

    #[test]
    fn test_build_client_succeeds() {
        let config = TimeoutConfig::default();
        let client = build_client(&config, ClientCategory::General);
        // If we get here without panic, the client was built successfully
        drop(client);
    }

    #[test]
    fn test_build_client_with_timeout_succeeds() {
        let config = TimeoutConfig::default();
        let client = build_client_with_timeout(&config, 120);
        drop(client);
    }

    #[test]
    fn test_default_client_succeeds() {
        let client = default_client(ClientCategory::Llm);
        drop(client);
    }

    #[test]
    fn test_all_categories_can_build_clients() {
        let config = TimeoutConfig::default();

        for category in [
            ClientCategory::Llm,
            ClientCategory::Notification,
            ClientCategory::Api,
            ClientCategory::General,
        ] {
            let client = build_client(&config, category);
            drop(client);
        }
    }
}
