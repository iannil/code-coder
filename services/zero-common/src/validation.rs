//! Configuration validation for Zero services.
//!
//! Provides validation logic for configuration fields to ensure
//! all required values are present and within valid ranges.

use std::str::FromStr;
use thiserror::Error;

use crate::config::{
    ChannelsConfig, CodeCoderConfig, Config, GatewayConfig, MemoryConfig, ObservabilityConfig,
    WorkflowConfig,
};

/// Configuration validation error.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid port {port}: must be between 1 and 65535")]
    InvalidPort { port: u16, field: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Configuration conflict: {reason}")]
    Conflict { reason: String },

    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<ValidationError>),
}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Trait for validatable configuration sections.
pub trait Validate {
    /// Validate this configuration section.
    fn validate(&self) -> ValidationResult<()>;
}

impl Config {
    /// Validate the entire configuration.
    pub fn validate(&self) -> ValidationResult<()> {
        let mut errors = Vec::new();

        // Validate gateway config
        if let Err(e) = self.gateway.validate() {
            errors.push(e);
        }

        // Validate channels config
        if let Err(e) = self.channels.validate() {
            errors.push(e);
        }

        // Validate workflow config
        if let Err(e) = self.workflow.validate() {
            errors.push(e);
        }

        // Validate codecoder config
        if let Err(e) = self.codecoder.validate() {
            errors.push(e);
        }

        // Validate observability config
        if let Err(e) = self.observability.validate() {
            errors.push(e);
        }

        // Validate memory config
        if let Err(e) = self.memory.validate() {
            errors.push(e);
        }

        // Check for port conflicts
        if let Err(e) = self.check_port_conflicts() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.remove(0))
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }

    /// Check for port conflicts between services.
    fn check_port_conflicts(&self) -> ValidationResult<()> {
        let mut ports: Vec<(u16, &str)> = vec![
            (self.gateway.port, "gateway.port"),
            (self.channels.port, "channels.port"),
            (self.workflow.port, "workflow.port"),
            (self.codecoder.port, "codecoder.port"),
        ];

        // Add workflow webhook port if specified
        if let Some(port) = self.workflow.webhook.port {
            ports.push((port, "workflow.webhook.port"));
        }

        // Add trading port if trading config exists
        if let Some(ref trading) = self.trading {
            ports.push((trading.port, "trading.port"));
        }

        // Check for duplicates
        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                if ports[i].0 == ports[j].0 {
                    return Err(ValidationError::Conflict {
                        reason: format!(
                            "Port {} is used by both {} and {}",
                            ports[i].0, ports[i].1, ports[j].1
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Load and validate configuration.
    pub fn load_and_validate() -> anyhow::Result<Self> {
        let config = Self::load()?;
        config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(config)
    }
}

impl Validate for GatewayConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Port validation
        if self.port == 0 {
            return Err(ValidationError::InvalidPort {
                port: self.port,
                field: "gateway.port".into(),
            });
        }

        // Host validation
        if self.host.is_empty() {
            return Err(ValidationError::MissingField {
                field: "gateway.host".into(),
            });
        }

        // Token expiry validation
        if self.token_expiry_secs == 0 {
            return Err(ValidationError::InvalidValue {
                field: "gateway.token_expiry_secs".into(),
                reason: "must be greater than 0".into(),
            });
        }

        // Rate limit validation
        if self.rate_limiting && self.rate_limit_rpm == 0 {
            return Err(ValidationError::InvalidValue {
                field: "gateway.rate_limit_rpm".into(),
                reason: "must be greater than 0 when rate limiting is enabled".into(),
            });
        }

        Ok(())
    }
}

impl Validate for ChannelsConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Port validation
        if self.port == 0 {
            return Err(ValidationError::InvalidPort {
                port: self.port,
                field: "channels.port".into(),
            });
        }

        // Host validation
        if self.host.is_empty() {
            return Err(ValidationError::MissingField {
                field: "channels.host".into(),
            });
        }

        // Validate Telegram if enabled
        if let Some(ref telegram) = self.telegram {
            if telegram.enabled && telegram.bot_token.as_ref().map_or(true, |t| t.is_empty()) {
                return Err(ValidationError::MissingField {
                    field: "channels.telegram.bot_token".into(),
                });
            }
        }

        // Validate Discord if enabled
        if let Some(ref discord) = self.discord {
            if discord.enabled && discord.bot_token.is_empty() {
                return Err(ValidationError::MissingField {
                    field: "channels.discord.bot_token".into(),
                });
            }
        }

        // Validate Slack if enabled
        if let Some(ref slack) = self.slack {
            if slack.enabled {
                if slack.bot_token.is_empty() {
                    return Err(ValidationError::MissingField {
                        field: "channels.slack.bot_token".into(),
                    });
                }
                if slack.app_token.is_empty() {
                    return Err(ValidationError::MissingField {
                        field: "channels.slack.app_token".into(),
                    });
                }
            }
        }

        // Validate Feishu if enabled
        if let Some(ref feishu) = self.feishu {
            if feishu.enabled {
                if feishu.app_id.is_empty() {
                    return Err(ValidationError::MissingField {
                        field: "channels.feishu.app_id".into(),
                    });
                }
                if feishu.app_secret.is_empty() {
                    return Err(ValidationError::MissingField {
                        field: "channels.feishu.app_secret".into(),
                    });
                }
            }
        }

        // Validate TTS if configured
        if let Some(ref tts) = self.tts {
            if tts.provider.is_empty() {
                return Err(ValidationError::MissingField {
                    field: "channels.tts.provider".into(),
                });
            }
            // API key is required for cloud providers
            let cloud_providers = ["openai", "elevenlabs", "azure"];
            if cloud_providers.contains(&tts.provider.to_lowercase().as_str())
                && tts.api_key.is_none()
            {
                return Err(ValidationError::MissingField {
                    field: format!("channels.tts.api_key (required for {})", tts.provider),
                });
            }
        }

        // Validate STT if configured
        if let Some(ref stt) = self.stt {
            if stt.provider.is_empty() {
                return Err(ValidationError::MissingField {
                    field: "channels.stt.provider".into(),
                });
            }
            // API key is required for cloud providers
            let cloud_providers = ["openai", "azure", "google"];
            if cloud_providers.contains(&stt.provider.to_lowercase().as_str())
                && stt.api_key.is_none()
            {
                return Err(ValidationError::MissingField {
                    field: format!("channels.stt.api_key (required for {})", stt.provider),
                });
            }
        }

        Ok(())
    }
}

impl Validate for WorkflowConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Validate cron tasks
        for task in &self.cron.tasks {
            if task.id.is_empty() {
                return Err(ValidationError::MissingField {
                    field: "workflow.cron.tasks[].id".into(),
                });
            }
            if task.expression.is_empty() {
                return Err(ValidationError::MissingField {
                    field: format!("workflow.cron.tasks[{}].expression", task.id),
                });
            }
            if task.command.is_empty() {
                return Err(ValidationError::MissingField {
                    field: format!("workflow.cron.tasks[{}].command", task.id),
                });
            }

            // Validate cron expression format
            if cron::Schedule::from_str(&task.expression).is_err() {
                return Err(ValidationError::InvalidValue {
                    field: format!("workflow.cron.tasks[{}].expression", task.id),
                    reason: format!("invalid cron expression: {}", task.expression),
                });
            }
        }

        // Validate webhook port if specified
        if let Some(port) = self.webhook.port {
            if port == 0 {
                return Err(ValidationError::InvalidPort {
                    port,
                    field: "workflow.webhook.port".into(),
                });
            }
        }

        Ok(())
    }
}

impl Validate for CodeCoderConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Validate host is not empty when enabled
        if self.enabled && self.host.is_empty() {
            return Err(ValidationError::MissingField {
                field: "codecoder.host".into(),
            });
        }

        if self.timeout_secs == 0 {
            return Err(ValidationError::InvalidValue {
                field: "codecoder.timeout_secs".into(),
                reason: "must be greater than 0".into(),
            });
        }

        // Validate port is valid
        if self.enabled && self.port == 0 {
            return Err(ValidationError::InvalidValue {
                field: "codecoder.port".into(),
                reason: "must be greater than 0".into(),
            });
        }

        Ok(())
    }
}

impl Validate for ObservabilityConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(ValidationError::InvalidValue {
                field: "observability.log_level".into(),
                reason: format!(
                    "must be one of: {}",
                    valid_levels.join(", ")
                ),
            });
        }

        // Validate log format
        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&self.log_format.to_lowercase().as_str()) {
            return Err(ValidationError::InvalidValue {
                field: "observability.log_format".into(),
                reason: format!(
                    "must be one of: {}",
                    valid_formats.join(", ")
                ),
            });
        }

        Ok(())
    }
}

impl Validate for MemoryConfig {
    fn validate(&self) -> ValidationResult<()> {
        // Validate backend type
        let valid_backends = ["sqlite", "postgres"];
        if !valid_backends.contains(&self.backend.to_lowercase().as_str()) {
            return Err(ValidationError::InvalidValue {
                field: "memory.backend".into(),
                reason: format!(
                    "must be one of: {}",
                    valid_backends.join(", ")
                ),
            });
        }

        // Check required fields based on backend
        if self.backend.to_lowercase().as_str() == "postgres"
            && self.connection_string.is_none()
        {
            return Err(ValidationError::MissingField {
                field: "memory.connection_string (required for postgres backend)".into(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_valid_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_gateway_port() {
        let mut config = Config::default();
        config.gateway.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = Config::default();
        config.observability.log_level = "invalid".into();
        let result = config.validate();
        assert!(result.is_err());
        if let Err(ValidationError::InvalidValue { field, .. }) = result {
            assert_eq!(field, "observability.log_level");
        }
    }

    #[test]
    fn test_port_conflict() {
        let mut config = Config::default();
        config.gateway.port = 4402;
        config.channels.port = 4402;
        let result = config.validate();
        assert!(result.is_err());
        if let Err(ValidationError::Conflict { reason }) = result {
            assert!(reason.contains("4402"));
        }
    }

    #[test]
    fn test_telegram_missing_token() {
        let mut config = Config::default();
        config.channels.telegram = Some(TelegramConfig {
            enabled: true,
            bot_token: Some("".into()),
            allowed_users: vec![],
            allowed_chats: vec![],
            trading_chat_id: None,
        });
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_postgres_missing_connection_string() {
        let mut config = Config::default();
        config.memory.backend = "postgres".into();
        config.memory.connection_string = None;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_codecoder_host() {
        let mut config = Config::default();
        config.codecoder.host = "".into();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_codecoder_port() {
        let mut config = Config::default();
        config.codecoder.port = 0;
        let result = config.validate();
        assert!(result.is_err());
    }
}
