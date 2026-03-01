//! Network configuration.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_defaults() {
        let config = NetworkConfig::default();
        assert_eq!(config.bind, "127.0.0.1");
        assert!(config.public_url.is_none());
    }
}
