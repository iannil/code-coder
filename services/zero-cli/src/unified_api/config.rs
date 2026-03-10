//! Configuration Routes
//!
//! Handles configuration retrieval and updates.
//! Configuration is loaded from ~/.codecoder/config.json and related files.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub success: bool,
    pub config: ConfigData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigData {
    /// Core settings
    pub core: CoreConfig,
    /// LLM provider settings (without secrets)
    pub providers: Vec<ProviderInfo>,
    /// Channel settings (without secrets)
    pub channels: ChannelConfig,
    /// Workspace path
    pub workspace_dir: String,
    /// Server version
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub default_model: Option<String>,
    pub default_temperature: Option<f64>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub models: Vec<String>,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub telegram: ChannelStatus,
    pub discord: ChannelStatus,
    pub slack: ChannelStatus,
    pub email: ChannelStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub enabled: bool,
    pub configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    /// Core settings to update
    pub core: Option<CoreConfigUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct CoreConfigUpdate {
    pub default_model: Option<String>,
    pub default_temperature: Option<f64>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub updated_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateConfigRequest {
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ValidateConfigResponse {
    pub success: bool,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/config - Get current configuration
pub async fn get_config(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    // Load configuration from zero-core
    let core_config = match zero_core::common::config::Config::load() {
        Ok(config) => CoreConfig {
            default_model: Some(config.llm.default.clone()),
            default_temperature: None, // Would need to be added to LlmConfig
            max_tokens: None,          // Would need to be added to LlmConfig
        },
        Err(_) => CoreConfig {
            default_model: Some("anthropic/claude-opus-4-5".to_string()),
            default_temperature: Some(0.7),
            max_tokens: Some(32000),
        },
    };

    // Load provider info (from config files, not secrets)
    let providers = get_provider_info();

    // Load channel status
    let channels = get_channel_status();

    let config_data = ConfigData {
        core: core_config,
        providers,
        channels,
        workspace_dir: state.workspace_dir.display().to_string(),
        version: state.version.to_string(),
    };

    Json(ConfigResponse {
        success: true,
        config: config_data,
    })
}

/// PUT /api/v1/config - Update configuration
pub async fn update_config(
    State(_state): State<Arc<UnifiedApiState>>,
    Json(request): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let mut updated_fields = vec![];

    // Load existing config
    let mut config = zero_core::common::config::Config::load().unwrap_or_default();

    // Apply updates
    if let Some(core) = request.core {
        if let Some(model) = core.default_model {
            config.llm.default = model;
            updated_fields.push("default_model".to_string());
        }
        // Note: temperature and max_tokens would need to be added to LlmConfig
    }

    // Save config
    match config.save() {
        Ok(_) => Json(UpdateConfigResponse {
            success: true,
            updated_fields,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to save config: {}", e),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/config/validate - Validate configuration
pub async fn validate_config(
    State(_state): State<Arc<UnifiedApiState>>,
    Json(request): Json<ValidateConfigRequest>,
) -> impl IntoResponse {
    let mut errors = vec![];
    let mut warnings = vec![];

    // Validate structure
    if let Some(obj) = request.config.as_object() {
        // Check core settings
        if let Some(core) = obj.get("core").and_then(|v| v.as_object()) {
            // Validate temperature
            if let Some(temp) = core.get("default_temperature").and_then(|v| v.as_f64()) {
                if !(0.0..=2.0).contains(&temp) {
                    errors.push("default_temperature must be between 0 and 2".to_string());
                }
            }

            // Validate max_tokens
            if let Some(tokens) = core.get("max_tokens").and_then(|v| v.as_u64()) {
                if tokens == 0 {
                    errors.push("max_tokens must be greater than 0".to_string());
                } else if tokens > 200000 {
                    warnings.push("max_tokens is very high, may cause issues".to_string());
                }
            }
        }
    } else {
        errors.push("Config must be a JSON object".to_string());
    }

    let valid = errors.is_empty();

    Json(ValidateConfigResponse {
        success: true,
        valid,
        errors,
        warnings,
    })
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

fn get_provider_info() -> Vec<ProviderInfo> {
    // This would read from providers.json or config
    // For now, return known providers
    vec![
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            enabled: true,
            models: vec![
                "claude-opus-4-5".to_string(),
                "claude-sonnet-4-5".to_string(),
                "claude-haiku-4".to_string(),
            ],
            has_key: std::env::var("ANTHROPIC_API_KEY").is_ok(),
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            enabled: true,
            models: vec![
                "gpt-4.5-turbo".to_string(),
                "gpt-4o".to_string(),
                "o1".to_string(),
                "o3-mini".to_string(),
            ],
            has_key: std::env::var("OPENAI_API_KEY").is_ok(),
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            enabled: true,
            models: vec![
                "gemini-2.5-pro".to_string(),
                "gemini-2.5-flash".to_string(),
            ],
            has_key: std::env::var("GOOGLE_API_KEY").is_ok(),
        },
    ]
}

fn get_channel_status() -> ChannelConfig {
    // This would read from channels.json
    // For now, return default status
    ChannelConfig {
        telegram: ChannelStatus {
            enabled: false,
            configured: std::env::var("TELEGRAM_BOT_TOKEN").is_ok(),
        },
        discord: ChannelStatus {
            enabled: false,
            configured: std::env::var("DISCORD_BOT_TOKEN").is_ok(),
        },
        slack: ChannelStatus {
            enabled: false,
            configured: std::env::var("SLACK_BOT_TOKEN").is_ok(),
        },
        email: ChannelStatus {
            enabled: false,
            configured: std::env::var("SMTP_HOST").is_ok(),
        },
    }
}
