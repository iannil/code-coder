//! Parallel inference - send requests to multiple LLM providers concurrently.

use crate::provider::{ChatRequest, ChatResponse, ProviderError, ProviderRegistry};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinSet;

/// State for parallel inference routes.
#[derive(Clone)]
pub struct ParallelState {
    pub registry: Arc<ProviderRegistry>,
}

impl ParallelState {
    /// Create a new parallel state with the given registry.
    pub fn new(registry: ProviderRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }
}

/// Request for parallel inference.
#[derive(Debug, Deserialize)]
pub struct ParallelRequest {
    /// Models to query (e.g., ["claude-sonnet-4", "gpt-4o"])
    pub models: Vec<String>,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate (applies to all models)
    #[serde(default)]
    pub max_tokens: Option<i64>,
    /// Temperature (0.0 - 1.0)
    #[serde(default)]
    pub temperature: Option<f64>,
    /// System prompt
    #[serde(default)]
    pub system: Option<String>,
}

/// Message in the conversation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Response from parallel inference.
#[derive(Debug, Serialize)]
pub struct ParallelResponse {
    /// Results from each model
    pub results: Vec<ModelResult>,
    /// Total tokens used across all models
    pub total_tokens: i64,
    /// Total latency in milliseconds (max of all models)
    pub total_latency_ms: u64,
}

/// Result from a single model.
#[derive(Debug, Serialize)]
pub struct ModelResult {
    /// Model name
    pub model: String,
    /// Provider name
    pub provider: String,
    /// Response content
    pub content: String,
    /// Token usage
    pub tokens: TokenInfo,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Token usage info.
#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub input: i64,
    pub output: i64,
    pub total: i64,
}

/// Handle parallel inference request.
pub async fn parallel_chat(
    State(state): State<ParallelState>,
    Json(request): Json<ParallelRequest>,
) -> impl IntoResponse {
    if request.models.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "At least one model is required"
            })),
        );
    }

    if request.models.len() > 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Maximum 5 models allowed per request"
            })),
        );
    }

    // Build chat requests for each model
    let chat_requests: Vec<(String, ChatRequest)> = request
        .models
        .iter()
        .map(|model| {
            (
                model.clone(),
                ChatRequest {
                    model: model.clone(),
                    messages: request
                        .messages
                        .iter()
                        .map(|m| crate::provider::Message {
                            role: m.role.clone(),
                            content: m.content.clone(),
                        })
                        .collect(),
                    max_tokens: request.max_tokens,
                    temperature: request.temperature,
                    system: request.system.clone(),
                },
            )
        })
        .collect();

    // Execute requests in parallel
    let mut join_set: JoinSet<(String, Result<ChatResponse, ProviderError>)> = JoinSet::new();

    for (model, chat_request) in chat_requests {
        let registry = Arc::clone(&state.registry);
        let model_clone = model.clone();

        join_set.spawn(async move {
            let result = if let Some(provider) = registry.get_for_model(&model_clone) {
                provider.chat(chat_request).await
            } else {
                Err(ProviderError {
                    provider: "unknown".into(),
                    model: model_clone.clone(),
                    message: format!("No provider found for model: {}", model_clone),
                    status_code: None,
                })
            };
            (model_clone, result)
        });
    }

    // Collect results
    let mut results = Vec::new();
    let mut total_tokens: i64 = 0;
    let mut max_latency: u64 = 0;

    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok((model, Ok(response))) => {
                total_tokens += response.usage.total_tokens;
                max_latency = max_latency.max(response.latency_ms);

                results.push(ModelResult {
                    model,
                    provider: response.provider,
                    content: response.content,
                    tokens: TokenInfo {
                        input: response.usage.input_tokens,
                        output: response.usage.output_tokens,
                        total: response.usage.total_tokens,
                    },
                    latency_ms: response.latency_ms,
                    error: None,
                });
            }
            Ok((model, Err(e))) => {
                results.push(ModelResult {
                    model: model.clone(),
                    provider: e.provider.clone(),
                    content: String::new(),
                    tokens: TokenInfo {
                        input: 0,
                        output: 0,
                        total: 0,
                    },
                    latency_ms: 0,
                    error: Some(e.message),
                });
            }
            Err(join_error) => {
                // Task panicked or was cancelled
                results.push(ModelResult {
                    model: "unknown".into(),
                    provider: "unknown".into(),
                    content: String::new(),
                    tokens: TokenInfo {
                        input: 0,
                        output: 0,
                        total: 0,
                    },
                    latency_ms: 0,
                    error: Some(format!("Task failed: {}", join_error)),
                });
            }
        }
    }

    // Sort results by model name for consistent output
    results.sort_by(|a, b| a.model.cmp(&b.model));

    let response = ParallelResponse {
        results,
        total_tokens,
        total_latency_ms: max_latency,
    };

    (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
}

/// Build parallel inference routes.
pub fn parallel_routes(state: ParallelState) -> axum::Router {
    use axum::routing::post;

    axum::Router::new()
        .route("/api/v1/parallel", post(parallel_chat))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_request_deserialization() {
        let json = r#"{
            "models": ["claude-sonnet-4", "gpt-4o"],
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 1000
        }"#;

        let request: ParallelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.models.len(), 2);
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.max_tokens, Some(1000));
    }

    #[test]
    fn test_parallel_response_serialization() {
        let response = ParallelResponse {
            results: vec![
                ModelResult {
                    model: "claude-sonnet-4".into(),
                    provider: "anthropic".into(),
                    content: "Hello from Claude!".into(),
                    tokens: TokenInfo {
                        input: 10,
                        output: 5,
                        total: 15,
                    },
                    latency_ms: 500,
                    error: None,
                },
                ModelResult {
                    model: "gpt-4o".into(),
                    provider: "openai".into(),
                    content: "Hello from GPT!".into(),
                    tokens: TokenInfo {
                        input: 12,
                        output: 6,
                        total: 18,
                    },
                    latency_ms: 450,
                    error: None,
                },
            ],
            total_tokens: 33,
            total_latency_ms: 500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("claude-sonnet-4"));
        assert!(json.contains("gpt-4o"));
        assert!(json.contains("33"));
    }

    #[test]
    fn test_parallel_state_creation() {
        use crate::provider::ProviderRegistry;

        let registry = ProviderRegistry::new();
        let state = ParallelState::new(registry);
        assert!(state.registry.list_providers().is_empty());
    }
}
