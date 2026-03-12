//! Provider API endpoints
//!
//! Provides HTTP endpoints for provider and model metadata access.
//! These endpoints support the ProviderBridge SDK for gradual migration.
//!
//! Note: This module provides READ-ONLY access to provider/model configuration.
//! The actual LLM execution (getLanguage, streaming) remains in the TypeScript layer
//! because it requires AI SDK integration that can't be serialized over HTTP.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::unified_api::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub temperature: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub toolcall: bool,
    pub input: ModalityCapabilities,
    pub output: ModalityCapabilities,
    #[serde(default)]
    pub interleaved: InterleavedCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityCapabilities {
    pub text: bool,
    pub audio: bool,
    pub image: bool,
    pub video: bool,
    pub pdf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InterleavedCapability {
    Enabled(bool),
    Field { field: String },
}

impl Default for InterleavedCapability {
    fn default() -> Self {
        InterleavedCapability::Enabled(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache: CacheCost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCost {
    pub read: f64,
    pub write: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimit {
    pub context: u64,
    pub input: Option<u64>,
    pub output: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelApi {
    pub id: String,
    pub url: String,
    pub npm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub name: String,
    pub family: Option<String>,
    pub api: ModelApi,
    pub capabilities: ModelCapabilities,
    pub cost: ModelCost,
    pub limit: ModelLimit,
    pub status: String,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub release_date: String,
    #[serde(default)]
    pub variants: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub env: Vec<String>,
    pub key: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub models: HashMap<String, ModelInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProvidersListResponse {
    pub success: bool,
    pub providers: HashMap<String, ProviderInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProvidersListAllResponse {
    pub success: bool,
    pub all: Vec<ProviderInfo>,
    pub default: HashMap<String, String>,
    pub connected: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderDetailResponse {
    pub success: bool,
    pub provider: ProviderInfo,
}

#[derive(Debug, Serialize)]
pub struct ModelDetailResponse {
    pub success: bool,
    pub model: ModelInfo,
}

#[derive(Debug, Serialize)]
pub struct DefaultModelResponse {
    pub success: bool,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// List connected providers
///
/// GET /api/v1/providers
pub async fn list_providers(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    // For now, return empty - will be populated from config
    // TODO: Load providers from config file
    let providers = load_providers_from_config(&state).await;

    Json(ProvidersListResponse {
        success: true,
        providers,
    })
}

/// List all providers (connected and unconnected)
///
/// GET /api/v1/providers/all
pub async fn list_all_providers(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    let providers = load_providers_from_config(&state).await;
    let connected: Vec<String> = providers.keys().cloned().collect();

    // Build default model map
    let mut default_models: HashMap<String, String> = HashMap::new();
    for (provider_id, provider) in &providers {
        if let Some(first_model) = provider.models.keys().next() {
            default_models.insert(provider_id.clone(), first_model.clone());
        }
    }

    Json(ProvidersListAllResponse {
        success: true,
        all: providers.values().cloned().collect(),
        default: default_models,
        connected,
    })
}

/// Get a specific provider
///
/// GET /api/v1/providers/:id
pub async fn get_provider(
    State(state): State<Arc<UnifiedApiState>>,
    Path(provider_id): Path<String>,
) -> impl IntoResponse {
    let providers = load_providers_from_config(&state).await;

    match providers.get(&provider_id) {
        Some(provider) => (
            StatusCode::OK,
            Json(ProviderDetailResponse {
                success: true,
                provider: provider.clone(),
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Provider '{}' not found", provider_id),
            }),
        )
            .into_response(),
    }
}

/// Get a specific model
///
/// GET /api/v1/providers/:provider_id/models/:model_id
pub async fn get_model(
    State(state): State<Arc<UnifiedApiState>>,
    Path((provider_id, model_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let providers = load_providers_from_config(&state).await;

    let provider = match providers.get(&provider_id) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Provider '{}' not found", provider_id),
                }),
            )
                .into_response()
        }
    };

    match provider.models.get(&model_id) {
        Some(model) => (
            StatusCode::OK,
            Json(ModelDetailResponse {
                success: true,
                model: model.clone(),
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Model '{}' not found in provider '{}'", model_id, provider_id),
            }),
        )
            .into_response(),
    }
}

/// Get the default model
///
/// GET /api/v1/providers/default-model
pub async fn get_default_model(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    // Check config for default model
    let config = &state.config;

    // Try to get from config.model
    if let Some(model_str) = &config.model {
        let parts: Vec<&str> = model_str.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Json(DefaultModelResponse {
                success: true,
                provider_id: parts[0].to_string(),
                model_id: parts[1].to_string(),
            })
            .into_response();
        }
    }

    // Fall back to first provider's first model
    let providers = load_providers_from_config(&state).await;

    for (provider_id, provider) in &providers {
        if let Some(model_id) = provider.models.keys().next() {
            return Json(DefaultModelResponse {
                success: true,
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
            })
            .into_response();
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            success: false,
            error: "No default model configured and no providers available".to_string(),
        }),
    )
        .into_response()
}

/// Get small/fast model for a provider
///
/// GET /api/v1/providers/:provider_id/small-model
pub async fn get_small_model(
    State(state): State<Arc<UnifiedApiState>>,
    Path(provider_id): Path<String>,
) -> impl IntoResponse {
    let providers = load_providers_from_config(&state).await;

    let provider = match providers.get(&provider_id) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Provider '{}' not found", provider_id),
                }),
            )
                .into_response()
        }
    };

    // Priority list for small models
    let small_model_priority = [
        "claude-haiku-4-5",
        "claude-haiku-4.5",
        "3-5-haiku",
        "3.5-haiku",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gpt-5-nano",
        "gpt-5-mini",
    ];

    // Find first matching small model
    for priority in &small_model_priority {
        for (model_id, model) in &provider.models {
            if model_id.contains(priority) {
                return Json(ModelDetailResponse {
                    success: true,
                    model: model.clone(),
                })
                .into_response();
            }
        }
    }

    // Fall back to first model
    if let Some(model) = provider.models.values().next() {
        return Json(ModelDetailResponse {
            success: true,
            model: model.clone(),
        })
        .into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            success: false,
            error: format!("No models available for provider '{}'", provider_id),
        }),
    )
        .into_response()
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

/// Load providers from config file
///
/// This reads the codecoder.json config and extracts provider information.
/// In the future, this will also load from models.dev database.
async fn load_providers_from_config(state: &UnifiedApiState) -> HashMap<String, ProviderInfo> {
    let mut providers = HashMap::new();

    // Load from config.provider section if available
    if let Some(provider_config) = &state.config.provider {
        for (provider_id, config) in provider_config {
            // Skip _settings - it's global settings, not a provider
            if provider_id == "_settings" {
                continue;
            }

            let mut models = HashMap::new();

            // Load models from config
            if let Some(model_configs) = config.get("models").and_then(|m| m.as_object()) {
                for (model_id, model_config) in model_configs {
                    let model = parse_model_config(provider_id, model_id, model_config);
                    models.insert(model_id.clone(), model);
                }
            }

            let provider = ProviderInfo {
                id: provider_id.clone(),
                name: config
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or(provider_id)
                    .to_string(),
                source: "config".to_string(),
                env: config
                    .get("env")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                key: config
                    .get("options")
                    .and_then(|o| o.get("apiKey"))
                    .and_then(|k| k.as_str())
                    .map(String::from),
                options: config
                    .get("options")
                    .and_then(|o| o.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                models,
            };

            providers.insert(provider_id.clone(), provider);
        }
    }

    // Also check llm.providers for backward compatibility
    if let Some(llm_config) = &state.config.llm {
        if let Some(llm_providers) = llm_config.get("providers").and_then(|p| p.as_object()) {
            for (provider_id, config) in llm_providers {
                if providers.contains_key(provider_id) {
                    continue; // Already loaded from config.provider
                }

                let provider = ProviderInfo {
                    id: provider_id.clone(),
                    name: provider_id.clone(),
                    source: "config".to_string(),
                    env: vec![],
                    key: None,
                    options: config
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    models: HashMap::new(),
                };

                providers.insert(provider_id.clone(), provider);
            }
        }
    }

    providers
}

/// Parse a model configuration from JSON
fn parse_model_config(
    provider_id: &str,
    model_id: &str,
    config: &serde_json::Value,
) -> ModelInfo {
    let obj = config.as_object();

    ModelInfo {
        id: model_id.to_string(),
        provider_id: provider_id.to_string(),
        name: obj
            .and_then(|o| o.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or(model_id)
            .to_string(),
        family: obj
            .and_then(|o| o.get("family"))
            .and_then(|f| f.as_str())
            .map(String::from),
        api: ModelApi {
            id: obj
                .and_then(|o| o.get("id"))
                .and_then(|i| i.as_str())
                .unwrap_or(model_id)
                .to_string(),
            url: obj
                .and_then(|o| o.get("api"))
                .and_then(|a| a.as_str())
                .unwrap_or("")
                .to_string(),
            npm: obj
                .and_then(|o| o.get("provider"))
                .and_then(|p| p.get("npm"))
                .and_then(|n| n.as_str())
                .unwrap_or("@ai-sdk/openai-compatible")
                .to_string(),
        },
        capabilities: ModelCapabilities {
            temperature: obj
                .and_then(|o| o.get("temperature"))
                .and_then(|t| t.as_bool())
                .unwrap_or(true),
            reasoning: obj
                .and_then(|o| o.get("reasoning"))
                .and_then(|r| r.as_bool())
                .unwrap_or(false),
            attachment: obj
                .and_then(|o| o.get("attachment"))
                .and_then(|a| a.as_bool())
                .unwrap_or(false),
            toolcall: obj
                .and_then(|o| o.get("tool_call"))
                .and_then(|t| t.as_bool())
                .unwrap_or(true),
            input: ModalityCapabilities {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            output: ModalityCapabilities {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::default(),
        },
        cost: ModelCost {
            input: obj
                .and_then(|o| o.get("cost"))
                .and_then(|c| c.get("input"))
                .and_then(|i| i.as_f64())
                .unwrap_or(0.0),
            output: obj
                .and_then(|o| o.get("cost"))
                .and_then(|c| c.get("output"))
                .and_then(|o| o.as_f64())
                .unwrap_or(0.0),
            cache: CacheCost {
                read: obj
                    .and_then(|o| o.get("cost"))
                    .and_then(|c| c.get("cache_read"))
                    .and_then(|r| r.as_f64())
                    .unwrap_or(0.0),
                write: obj
                    .and_then(|o| o.get("cost"))
                    .and_then(|c| c.get("cache_write"))
                    .and_then(|w| w.as_f64())
                    .unwrap_or(0.0),
            },
        },
        limit: ModelLimit {
            context: obj
                .and_then(|o| o.get("limit"))
                .and_then(|l| l.get("context"))
                .and_then(|c| c.as_u64())
                .unwrap_or(128000),
            input: obj
                .and_then(|o| o.get("limit"))
                .and_then(|l| l.get("input"))
                .and_then(|i| i.as_u64()),
            output: obj
                .and_then(|o| o.get("limit"))
                .and_then(|l| l.get("output"))
                .and_then(|o| o.as_u64())
                .unwrap_or(8192),
        },
        status: obj
            .and_then(|o| o.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("active")
            .to_string(),
        options: obj
            .and_then(|o| o.get("options"))
            .and_then(|o| o.as_object())
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default(),
        headers: obj
            .and_then(|o| o.get("headers"))
            .and_then(|h| h.as_object())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default(),
        release_date: obj
            .and_then(|o| o.get("release_date"))
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string(),
        variants: HashMap::new(),
    }
}
