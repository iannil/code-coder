//! Provider system for ZeroBot.
//!
//! This module provides LLM provider abstractions with retry and fallback support.
//! Provider implementations are imported from `zero-gateway` and adapted to the local trait.
//!
//! ## Architecture
//!
//! - `zero_gateway::Provider`: Full-featured trait with `ChatRequest`/`ChatResponse`
//! - `Provider` (this module): Simplified trait with `chat_with_system()` for CLI use
//! - `GatewayProviderAdapter`: Bridges gateway providers to CLI trait
//! - `ResilientProvider` (from zero-gateway): Provides retry + fallback behavior

use async_trait::async_trait;
use std::sync::Arc;

// Re-export gateway provider types for direct access
pub use zero_gateway::{
    AnthropicProvider, AuthStyle, CompatibleProvider, GeminiProvider, OllamaProvider,
    OpenAIProvider, OpenRouterProvider, ResilienceConfig, ResilientProvider,
};

// ============================================================================
// Provider Trait (zero-cli specific, simpler than zero-gateway)
// ============================================================================

/// LLM provider trait for zero-cli.
///
/// This is a simplified interface that works with the CLI's chat-based workflow.
/// For the full-featured API with ChatRequest/ChatResponse, use zero-gateway directly.
///
/// This trait is a superset of `zero_agent::Provider`, allowing CLI providers
/// to be used with `AgentExecutor` for tool-calling workflows.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai").
    fn name(&self) -> &str;

    /// Check if the provider supports a specific model.
    fn supports_model(&self, model: &str) -> bool;

    /// Chat with the LLM using an optional system prompt.
    async fn chat_with_system(
        &self,
        system: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    /// Simple chat without system prompt.
    async fn chat(&self, message: &str, model: &str, temperature: f64) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature)
            .await
    }

    /// Warm up the provider connection.
    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

// ============================================================================
// Adapter: zero-gateway Provider -> zero-cli Provider
// ============================================================================

/// Adapter that wraps a zero-gateway provider to implement the zero-cli Provider trait.
///
/// This adapter also implements `zero_agent::Provider`, allowing it to be used
/// with `AgentExecutor` for tool-calling workflows.
pub struct GatewayProviderAdapter<P: zero_gateway::Provider> {
    inner: P,
}

impl<P: zero_gateway::Provider> GatewayProviderAdapter<P> {
    pub fn new(provider: P) -> Self {
        Self { inner: provider }
    }

    /// Get the inner provider name.
    pub fn provider_name(&self) -> &str {
        self.inner.name()
    }
}

#[async_trait]
impl<P: zero_gateway::Provider + 'static> Provider for GatewayProviderAdapter<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn supports_model(&self, model: &str) -> bool {
        self.inner.supports_model(model)
    }

    async fn chat_with_system(
        &self,
        system: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let request = zero_gateway::ChatRequest {
            model: model.to_string(),
            messages: vec![zero_gateway::provider::Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(4096),
            temperature: Some(temperature),
            system: system.map(ToString::to_string),
        };

        match self.inner.chat(request).await {
            Ok(response) => Ok(response.content),
            Err(e) => anyhow::bail!("{}", e),
        }
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        // Gateway providers don't have a warmup method, just return Ok
        Ok(())
    }
}

/// Implement zero_agent::Provider for GatewayProviderAdapter.
/// This allows CLI providers to be used with AgentExecutor.
#[async_trait]
impl<P: zero_gateway::Provider + 'static> zero_agent::Provider for GatewayProviderAdapter<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn supports_model(&self, model: &str) -> bool {
        self.inner.supports_model(model)
    }

    async fn chat_with_system(
        &self,
        system: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        // Delegate to the Provider trait implementation
        <Self as Provider>::chat_with_system(self, system, message, model, temperature).await
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

const MAX_API_ERROR_CHARS: usize = 200;

fn is_secret_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

fn token_end(input: &str, from: usize) -> usize {
    let mut end = from;
    for (i, c) in input[from..].char_indices() {
        if is_secret_char(c) {
            end = from + i + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

/// Scrub known secret-like token prefixes from provider error strings.
pub fn scrub_secret_patterns(input: &str) -> String {
    const PREFIXES: [&str; 3] = ["sk-", "xoxb-", "xoxp-"];

    let mut scrubbed = input.to_string();

    for prefix in PREFIXES {
        let mut search_from = 0;
        loop {
            let Some(rel) = scrubbed[search_from..].find(prefix) else {
                break;
            };

            let start = search_from + rel;
            let content_start = start + prefix.len();
            let end = token_end(&scrubbed, content_start);

            if end == content_start {
                search_from = content_start;
                continue;
            }

            scrubbed.replace_range(start..end, "[REDACTED]");
            search_from = start + "[REDACTED]".len();
        }
    }

    scrubbed
}

/// Sanitize API error text by scrubbing secrets and truncating length.
pub fn sanitize_api_error(input: &str) -> String {
    let scrubbed = scrub_secret_patterns(input);

    if scrubbed.chars().count() <= MAX_API_ERROR_CHARS {
        return scrubbed;
    }

    let mut end = MAX_API_ERROR_CHARS;
    while end > 0 && !scrubbed.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...", &scrubbed[..end])
}

/// Build a sanitized provider error from a failed HTTP response.
pub async fn api_error(provider: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read provider error body>".to_string());
    let sanitized = sanitize_api_error(&body);
    anyhow::anyhow!("{provider} API error ({status}): {sanitized}")
}

/// Create a gateway provider (returns Arc for use with ResilientProvider).
fn create_gateway_provider(
    name: &str,
    api_key: Option<&str>,
) -> anyhow::Result<Arc<dyn zero_gateway::Provider>> {
    match name {
        // ── Primary providers ───────────────────────────────────
        "openrouter" => Ok(Arc::new(OpenRouterProvider::new(api_key))),
        "anthropic" => Ok(Arc::new(AnthropicProvider::new(api_key.unwrap_or("")))),
        "openai" => Ok(Arc::new(OpenAIProvider::new(api_key.unwrap_or("")))),
        "ollama" => Ok(Arc::new(OllamaProvider::new(api_key.filter(|k| !k.is_empty())))),
        "gemini" | "google" | "google-gemini" => Ok(Arc::new(GeminiProvider::new(api_key))),

        // ── OpenAI-compatible providers ─────────────────────────
        "venice" => Ok(Arc::new(CompatibleProvider::new(
            "Venice",
            "https://api.venice.ai",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "vercel" | "vercel-ai" => Ok(Arc::new(CompatibleProvider::new(
            "Vercel AI Gateway",
            "https://api.vercel.ai",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "cloudflare" | "cloudflare-ai" => Ok(Arc::new(CompatibleProvider::new(
            "Cloudflare AI Gateway",
            "https://gateway.ai.cloudflare.com/v1",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "moonshot" | "kimi" => Ok(Arc::new(CompatibleProvider::new(
            "Moonshot",
            "https://api.moonshot.cn",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "synthetic" => Ok(Arc::new(CompatibleProvider::new(
            "Synthetic",
            "https://api.synthetic.com",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "opencode" | "opencode-zen" => Ok(Arc::new(CompatibleProvider::new(
            "OpenCode Zen",
            "https://api.opencode.ai",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "zai" | "z.ai" => Ok(Arc::new(CompatibleProvider::new(
            "Z.AI",
            "https://api.z.ai",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "glm" | "zhipu" => Ok(Arc::new(CompatibleProvider::new(
            "GLM",
            "https://open.bigmodel.cn/api/paas",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "minimax" => Ok(Arc::new(CompatibleProvider::new(
            "MiniMax",
            "https://api.minimax.chat",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "bedrock" | "aws-bedrock" => Ok(Arc::new(CompatibleProvider::new(
            "Amazon Bedrock",
            "https://bedrock-runtime.us-east-1.amazonaws.com",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),
        "qianfan" | "baidu" => Ok(Arc::new(CompatibleProvider::new(
            "Qianfan",
            "https://aip.baidubce.com",
            api_key,
            AuthStyle::Bearer,
            vec![],
        ))),

        // ── Extended ecosystem ──────────────────────────────────
        "groq" => Ok(Arc::new(CompatibleProvider::groq(api_key))),
        "mistral" => Ok(Arc::new(CompatibleProvider::mistral(api_key))),
        "xai" | "grok" => Ok(Arc::new(CompatibleProvider::xai(api_key))),
        "deepseek" => Ok(Arc::new(CompatibleProvider::deepseek(api_key))),
        "together" | "together-ai" => Ok(Arc::new(CompatibleProvider::together(api_key))),
        "fireworks" | "fireworks-ai" => Ok(Arc::new(CompatibleProvider::fireworks(api_key))),
        "perplexity" => Ok(Arc::new(CompatibleProvider::perplexity(api_key))),
        "cohere" => Ok(Arc::new(CompatibleProvider::cohere(api_key))),

        // ── Custom provider ─────────────────────────────────────
        name if name.starts_with("custom:") => {
            let base_url = name.strip_prefix("custom:").unwrap_or("");
            if base_url.is_empty() {
                anyhow::bail!(
                    "Custom provider requires a URL. Format: custom:https://your-api.com"
                );
            }
            Ok(Arc::new(CompatibleProvider::new(
                "Custom",
                base_url,
                api_key,
                AuthStyle::Bearer,
                vec![],
            )))
        }

        _ => anyhow::bail!(
            "Unknown provider: {name}. Check README for supported providers or run `zero-bot onboard --interactive` to reconfigure.\n\
             Tip: Use \"custom:https://your-api.com\" for any OpenAI-compatible endpoint."
        ),
    }
}

/// Factory: create the right provider from config (wrapped in CLI adapter)
#[allow(clippy::too_many_lines)]
pub fn create_provider(name: &str, api_key: Option<&str>) -> anyhow::Result<Box<dyn Provider>> {
    match name {
        // ── Primary providers ───────────────────────────────────
        "openrouter" => Ok(Box::new(GatewayProviderAdapter::new(
            OpenRouterProvider::new(api_key),
        ))),
        "anthropic" => Ok(Box::new(GatewayProviderAdapter::new(AnthropicProvider::new(
            api_key.unwrap_or(""),
        )))),
        "openai" => Ok(Box::new(GatewayProviderAdapter::new(OpenAIProvider::new(
            api_key.unwrap_or(""),
        )))),
        "ollama" => Ok(Box::new(GatewayProviderAdapter::new(OllamaProvider::new(
            api_key.filter(|k| !k.is_empty()),
        )))),
        "gemini" | "google" | "google-gemini" => Ok(Box::new(GatewayProviderAdapter::new(
            GeminiProvider::new(api_key),
        ))),

        // ── OpenAI-compatible providers ─────────────────────────
        "venice" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new("Venice", "https://api.venice.ai", api_key, AuthStyle::Bearer, vec![]),
        ))),
        "vercel" | "vercel-ai" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Vercel AI Gateway",
                "https://api.vercel.ai",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "cloudflare" | "cloudflare-ai" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Cloudflare AI Gateway",
                "https://gateway.ai.cloudflare.com/v1",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "moonshot" | "kimi" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Moonshot",
                "https://api.moonshot.cn",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "synthetic" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Synthetic",
                "https://api.synthetic.com",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "opencode" | "opencode-zen" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "OpenCode Zen",
                "https://api.opencode.ai",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "zai" | "z.ai" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new("Z.AI", "https://api.z.ai", api_key, AuthStyle::Bearer, vec![]),
        ))),
        "glm" | "zhipu" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "GLM",
                "https://open.bigmodel.cn/api/paas",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "minimax" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "MiniMax",
                "https://api.minimax.chat",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "bedrock" | "aws-bedrock" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Amazon Bedrock",
                "https://bedrock-runtime.us-east-1.amazonaws.com",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),
        "qianfan" | "baidu" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::new(
                "Qianfan",
                "https://aip.baidubce.com",
                api_key,
                AuthStyle::Bearer,
                vec![],
            ),
        ))),

        // ── Extended ecosystem ──────────────────────────────────
        "groq" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::groq(api_key),
        ))),
        "mistral" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::mistral(api_key),
        ))),
        "xai" | "grok" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::xai(api_key),
        ))),
        "deepseek" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::deepseek(api_key),
        ))),
        "together" | "together-ai" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::together(api_key),
        ))),
        "fireworks" | "fireworks-ai" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::fireworks(api_key),
        ))),
        "perplexity" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::perplexity(api_key),
        ))),
        "cohere" => Ok(Box::new(GatewayProviderAdapter::new(
            CompatibleProvider::cohere(api_key),
        ))),

        // ── Custom provider ─────────────────────────────────────
        name if name.starts_with("custom:") => {
            let base_url = name.strip_prefix("custom:").unwrap_or("");
            if base_url.is_empty() {
                anyhow::bail!(
                    "Custom provider requires a URL. Format: custom:https://your-api.com"
                );
            }
            Ok(Box::new(GatewayProviderAdapter::new(
                CompatibleProvider::new("Custom", base_url, api_key, AuthStyle::Bearer, vec![]),
            )))
        }

        _ => anyhow::bail!(
            "Unknown provider: {name}. Check README for supported providers or run `zero-bot onboard --interactive` to reconfigure.\n\
             Tip: Use \"custom:https://your-api.com\" for any OpenAI-compatible endpoint."
        ),
    }
}

/// Create provider chain with retry and fallback behavior.
///
/// Uses zero-gateway's `ResilientProvider` for retry/fallback logic.
pub fn create_resilient_provider(
    primary_name: &str,
    api_key: Option<&str>,
    reliability: &crate::config::ReliabilityConfig,
) -> anyhow::Result<Box<dyn Provider>> {
    let mut gateway_providers: Vec<Arc<dyn zero_gateway::Provider>> = Vec::new();

    // Add primary provider
    gateway_providers.push(create_gateway_provider(primary_name, api_key)?);

    // Add fallback providers
    for fallback in &reliability.fallback_providers {
        if fallback == primary_name
            || gateway_providers
                .iter()
                .any(|p| p.name() == fallback.as_str())
        {
            continue;
        }

        if api_key.is_some() && fallback != "ollama" {
            tracing::warn!(
                fallback_provider = fallback,
                primary_provider = primary_name,
                "Fallback provider will use the primary provider's API key — \
                 this will fail if the providers require different keys"
            );
        }

        match create_gateway_provider(fallback, api_key) {
            Ok(provider) => gateway_providers.push(provider),
            Err(e) => {
                tracing::warn!(
                    fallback_provider = fallback,
                    "Ignoring invalid fallback provider: {e}"
                );
            }
        }
    }

    // Create resilience configuration
    let resilience_config = ResilienceConfig {
        max_retries: reliability.provider_retries,
        base_backoff_ms: reliability.provider_backoff_ms.max(50),
        max_backoff_ms: 10_000,
    };

    // Wrap in ResilientProvider from zero-gateway
    let resilient = ResilientProvider::new(gateway_providers, resilience_config);

    // Adapt to CLI Provider trait
    Ok(Box::new(GatewayProviderAdapter::new(resilient)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Primary providers ────────────────────────────────────

    #[test]
    fn factory_openrouter() {
        assert!(create_provider("openrouter", Some("sk-test")).is_ok());
        assert!(create_provider("openrouter", None).is_ok());
    }

    #[test]
    fn factory_anthropic() {
        assert!(create_provider("anthropic", Some("sk-test")).is_ok());
    }

    #[test]
    fn factory_openai() {
        assert!(create_provider("openai", Some("sk-test")).is_ok());
    }

    #[test]
    fn factory_ollama() {
        assert!(create_provider("ollama", None).is_ok());
    }

    #[test]
    fn factory_gemini() {
        assert!(create_provider("gemini", Some("test-key")).is_ok());
        assert!(create_provider("google", Some("test-key")).is_ok());
        assert!(create_provider("google-gemini", Some("test-key")).is_ok());
        assert!(create_provider("gemini", None).is_ok());
    }

    // ── OpenAI-compatible providers ─────────────────────────

    #[test]
    fn factory_groq() {
        assert!(create_provider("groq", Some("key")).is_ok());
    }

    #[test]
    fn factory_mistral() {
        assert!(create_provider("mistral", Some("key")).is_ok());
    }

    #[test]
    fn factory_xai() {
        assert!(create_provider("xai", Some("key")).is_ok());
        assert!(create_provider("grok", Some("key")).is_ok());
    }

    #[test]
    fn factory_deepseek() {
        assert!(create_provider("deepseek", Some("key")).is_ok());
    }

    #[test]
    fn factory_together() {
        assert!(create_provider("together", Some("key")).is_ok());
        assert!(create_provider("together-ai", Some("key")).is_ok());
    }

    #[test]
    fn factory_fireworks() {
        assert!(create_provider("fireworks", Some("key")).is_ok());
        assert!(create_provider("fireworks-ai", Some("key")).is_ok());
    }

    #[test]
    fn factory_perplexity() {
        assert!(create_provider("perplexity", Some("key")).is_ok());
    }

    #[test]
    fn factory_cohere() {
        assert!(create_provider("cohere", Some("key")).is_ok());
    }

    // ── Custom provider ─────────────────────────────────────

    #[test]
    fn factory_custom_url() {
        let p = create_provider("custom:https://my-llm.example.com", Some("key"));
        assert!(p.is_ok());
    }

    #[test]
    fn factory_custom_localhost() {
        let p = create_provider("custom:http://localhost:1234", Some("key"));
        assert!(p.is_ok());
    }

    #[test]
    fn factory_custom_no_key() {
        let p = create_provider("custom:https://my-llm.example.com", None);
        assert!(p.is_ok());
    }

    #[test]
    fn factory_custom_empty_url_errors() {
        match create_provider("custom:", None) {
            Err(e) => assert!(
                e.to_string().contains("requires a URL"),
                "Expected 'requires a URL', got: {e}"
            ),
            Ok(_) => panic!("Expected error for empty custom URL"),
        }
    }

    // ── Error cases ─────────────────────────────────────────

    #[test]
    fn factory_unknown_provider_errors() {
        let p = create_provider("nonexistent", None);
        assert!(p.is_err());
        let msg = p.err().unwrap().to_string();
        assert!(msg.contains("Unknown provider"));
        assert!(msg.contains("nonexistent"));
    }

    #[test]
    fn factory_empty_name_errors() {
        assert!(create_provider("", None).is_err());
    }

    #[test]
    fn resilient_provider_ignores_duplicate_and_invalid_fallbacks() {
        let reliability = crate::config::ReliabilityConfig {
            provider_retries: 1,
            provider_backoff_ms: 100,
            fallback_providers: vec![
                "openrouter".into(),
                "nonexistent-provider".into(),
                "openai".into(),
                "openai".into(),
            ],
            channel_initial_backoff_secs: 2,
            channel_max_backoff_secs: 60,
            scheduler_poll_secs: 15,
            scheduler_retries: 2,
        };

        let provider = create_resilient_provider("openrouter", Some("sk-test"), &reliability);
        assert!(provider.is_ok());
    }

    #[test]
    fn resilient_provider_errors_for_invalid_primary() {
        let reliability = crate::config::ReliabilityConfig::default();
        let provider = create_resilient_provider("totally-invalid", Some("sk-test"), &reliability);
        assert!(provider.is_err());
    }

    // ── API error sanitization ──────────────────────────────

    #[test]
    fn sanitize_scrubs_sk_prefix() {
        let input = "request failed: sk-1234567890abcdef";
        let out = sanitize_api_error(input);
        assert!(!out.contains("sk-1234567890abcdef"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_scrubs_multiple_prefixes() {
        let input = "keys sk-abcdef xoxb-12345 xoxp-67890";
        let out = sanitize_api_error(input);
        assert!(!out.contains("sk-abcdef"));
        assert!(!out.contains("xoxb-12345"));
        assert!(!out.contains("xoxp-67890"));
    }

    #[test]
    fn sanitize_truncates_long_error() {
        let long = "a".repeat(400);
        let result = sanitize_api_error(&long);
        assert!(result.len() <= 203);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn sanitize_no_secret_no_change() {
        let input = "simple upstream timeout";
        let result = sanitize_api_error(input);
        assert_eq!(result, input);
    }
}
