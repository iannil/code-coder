//! Service port configuration.

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServicePortConfig {
    /// Port number for the service
    #[serde(default)]
    pub port: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_services_defaults() {
        let config = ServicesConfig::default();
        assert!(config.codecoder.port.is_none());
        assert!(config.gateway.port.is_none());
    }

    #[test]
    fn test_service_port_config() {
        let config = ServicePortConfig { port: Some(4400) };
        assert_eq!(config.port, Some(4400));
    }
}
