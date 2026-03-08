//! Google Gemini provider for zero-gateway.
//!
//! Supports:
//! - Direct API key (GEMINI_API_KEY or GOOGLE_API_KEY env var)
//! - Gemini CLI OAuth tokens (~/.gemini/oauth_creds.json)

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use directories::UserDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Gemini provider supporting multiple authentication methods.
pub struct GeminiProvider {
    api_key: Option<String>,
    client: Client,
}

// ══════════════════════════════════════════════════════════════════════════════
// API REQUEST/RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f64,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<i64>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<i64>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<i64>,
}

// ══════════════════════════════════════════════════════════════════════════════
// GEMINI CLI TOKEN STRUCTURES
// ══════════════════════════════════════════════════════════════════════════════

/// OAuth token stored by Gemini CLI in `~/.gemini/oauth_creds.json`
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeminiCliOAuthCreds {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expiry: Option<String>,
}

impl GeminiProvider {
    /// Create a new Gemini provider.
    ///
    /// Authentication priority:
    /// 1. Explicit API key passed in
    /// 2. `GEMINI_API_KEY` environment variable
    /// 3. `GOOGLE_API_KEY` environment variable
    /// 4. Gemini CLI OAuth tokens (`~/.gemini/oauth_creds.json`)
    pub fn new(api_key: Option<&str>) -> Self {
        let resolved_key = api_key
            .map(String::from)
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(Self::try_load_gemini_cli_token);

        Self {
            api_key: resolved_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// Try to load OAuth access token from Gemini CLI's cached credentials.
    fn try_load_gemini_cli_token() -> Option<String> {
        let gemini_dir = Self::gemini_cli_dir()?;
        let creds_path = gemini_dir.join("oauth_creds.json");

        if !creds_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&creds_path).ok()?;
        let creds: GeminiCliOAuthCreds = serde_json::from_str(&content).ok()?;

        // Check if token is expired (basic check)
        if let Some(ref expiry) = creds.expiry {
            if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expiry) {
                if expiry_time < chrono::Utc::now() {
                    tracing::debug!("Gemini CLI OAuth token expired, skipping");
                    return None;
                }
            }
        }

        creds.access_token
    }

    /// Get the Gemini CLI config directory (~/.gemini)
    fn gemini_cli_dir() -> Option<PathBuf> {
        UserDirs::new().map(|u| u.home_dir().join(".gemini"))
    }

    /// Check if Gemini CLI is configured and has valid credentials
    pub fn has_cli_credentials() -> bool {
        Self::try_load_gemini_cli_token().is_some()
    }

    /// Check if any Gemini authentication is available
    pub fn has_any_auth() -> bool {
        std::env::var("GEMINI_API_KEY").is_ok()
            || std::env::var("GOOGLE_API_KEY").is_ok()
            || Self::has_cli_credentials()
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn models(&self) -> Vec<&str> {
        vec![
            "gemini-2.0-flash",
            "gemini-1.5-pro",
            "gemini-1.5-flash",
            "gemini-1.5-flash-8b",
        ]
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("gemini-")
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();

        let api_key = self.api_key.as_ref().ok_or_else(|| ProviderError {
            provider: "gemini".into(),
            model: request.model.clone(),
            message: "Gemini API key not found. Set GEMINI_API_KEY env var or run `gemini` CLI to authenticate.".into(),
            status_code: None,
        })?;

        // Build system instruction if provided
        let system_instruction = request.system.as_ref().map(|sys| Content {
            role: None,
            parts: vec![Part { text: sys.clone() }],
        });

        // Convert messages to Gemini format
        let contents: Vec<Content> = request
            .messages
            .iter()
            .map(|msg| Content {
                role: Some(match msg.role.as_str() {
                    "assistant" => "model".to_string(),
                    other => other.to_string(),
                }),
                parts: vec![Part {
                    text: msg.content.clone(),
                }],
            })
            .collect();

        let gemini_request = GenerateContentRequest {
            contents,
            system_instruction,
            generation_config: GenerationConfig {
                temperature: request.temperature.unwrap_or(0.7),
                max_output_tokens: request.max_tokens.unwrap_or(8192),
            },
        };

        // Gemini API endpoint
        let model_name = if request.model.starts_with("models/") {
            request.model.clone()
        } else {
            format!("models/{}", request.model)
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{model_name}:generateContent?key={api_key}"
        );

        let response = self
            .client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| ProviderError {
                provider: "gemini".into(),
                model: request.model.clone(),
                message: format!("Request failed: {}", e),
                status_code: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: "gemini".into(),
                model: request.model.clone(),
                message: format!("API error ({}): {}", status.as_u16(), error_text),
                status_code: Some(status.as_u16()),
            });
        }

        let result: GenerateContentResponse = response.json().await.map_err(|e| ProviderError {
            provider: "gemini".into(),
            model: request.model.clone(),
            message: format!("Failed to parse response: {}", e),
            status_code: None,
        })?;

        // Check for API error in response body
        if let Some(err) = result.error {
            return Err(ProviderError {
                provider: "gemini".into(),
                model: request.model.clone(),
                message: format!("API error: {}", err.message),
                status_code: None,
            });
        }

        // Extract text from response
        let candidate = result
            .candidates
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| ProviderError {
                provider: "gemini".into(),
                model: request.model.clone(),
                message: "No response from Gemini".into(),
                status_code: None,
            })?;

        let content = candidate
            .content
            .parts
            .into_iter()
            .next()
            .and_then(|p| p.text)
            .unwrap_or_default();

        // Extract token usage
        let usage = result.usage_metadata.map_or(TokenUsage::default(), |u| {
            TokenUsage {
                input_tokens: u.prompt_token_count.unwrap_or(0),
                output_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
            }
        });

        Ok(ChatResponse {
            provider: "gemini".into(),
            model: request.model,
            content,
            usage,
            finish_reason: candidate.finish_reason,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_creates_without_key() {
        let provider = GeminiProvider::new(None);
        // Should not panic, just have no key (unless env vars are set)
        assert!(provider.api_key.is_none() || provider.api_key.is_some());
    }

    #[test]
    fn provider_creates_with_key() {
        let provider = GeminiProvider::new(Some("test-api-key"));
        assert!(provider.api_key.is_some());
        assert_eq!(provider.api_key.as_deref(), Some("test-api-key"));
    }

    #[test]
    fn supports_gemini_models() {
        let provider = GeminiProvider::new(Some("key"));
        assert!(provider.supports_model("gemini-2.0-flash"));
        assert!(provider.supports_model("gemini-1.5-pro"));
        assert!(!provider.supports_model("gpt-4"));
    }

    #[test]
    fn provider_name_is_gemini() {
        let provider = GeminiProvider::new(Some("key"));
        assert_eq!(provider.name(), "gemini");
    }

    #[test]
    fn gemini_cli_dir_returns_path() {
        let dir = GeminiProvider::gemini_cli_dir();
        if UserDirs::new().is_some() {
            assert!(dir.is_some());
            assert!(dir.unwrap().ends_with(".gemini"));
        }
    }
}
