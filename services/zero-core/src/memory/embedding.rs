//! Embedding providers for vector search.
//!
//! Supports OpenAI-compatible APIs and custom endpoints.
//!
//! Adapted from zero-memory/src/embeddings.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Embedding provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type: "openai", "custom:<url>", "none"
    pub provider: String,
    /// API key (optional for some providers)
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Embedding dimensions
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "none".to_string(),
            api_key: None,
            model: "text-embedding-3-small".to_string(),
            dimensions: 1536,
        }
    }
}

/// Trait for embedding providers — convert text to vectors
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Embedding dimensions
    fn dimensions(&self) -> usize;

    /// Embed a batch of texts into vectors
    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>>;

    /// Embed a single text
    async fn embed_one(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut results = self.embed(&[text]).await?;
        results
            .pop()
            .ok_or_else(|| anyhow::anyhow!("Empty embedding result"))
    }
}

// ── Noop provider (keyword-only fallback) ────────────────────

/// No-op embedding provider for keyword-only search.
pub struct NoopEmbedding;

#[async_trait]
impl EmbeddingProvider for NoopEmbedding {
    fn name(&self) -> &str {
        "none"
    }

    fn dimensions(&self) -> usize {
        0
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(Vec::new())
    }
}

// ── OpenAI-compatible embedding provider ─────────────────────

/// OpenAI-compatible embedding provider.
pub struct OpenAiEmbedding {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    dims: usize,
}

impl OpenAiEmbedding {
    /// Create a new OpenAI embedding provider.
    pub fn new(base_url: &str, api_key: &str, model: &str, dims: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            dims,
        }
    }

    /// Create from configuration
    pub fn from_config(config: &EmbeddingConfig, base_url: &str) -> Self {
        Self::new(
            base_url,
            config.api_key.as_deref().unwrap_or(""),
            &config.model,
            config.dimensions,
        )
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedding {
    fn name(&self) -> &str {
        "openai"
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Embedding API error {status}: {text}");
        }

        let json: serde_json::Value = resp.json().await?;
        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid embedding response: missing 'data'"))?;

        let mut embeddings = Vec::with_capacity(data.len());
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| anyhow::anyhow!("Invalid embedding item"))?;

            #[allow(clippy::cast_possible_truncation)]
            let vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            embeddings.push(vec);
        }

        Ok(embeddings)
    }
}

// ── Factory ──────────────────────────────────────────────────

/// Create an embedding provider from configuration.
///
/// Supported providers:
/// - `"openai"` - OpenAI embeddings API
/// - `"custom:<base_url>"` - Custom OpenAI-compatible endpoint
/// - Other values - Returns `NoopEmbedding` (keyword-only search)
pub fn create_embedding_provider(
    provider: &str,
    api_key: Option<&str>,
    model: &str,
    dims: usize,
) -> Box<dyn EmbeddingProvider> {
    match provider {
        "openai" => {
            let key = api_key.unwrap_or("");
            Box::new(OpenAiEmbedding::new(
                "https://api.openai.com",
                key,
                model,
                dims,
            ))
        }
        name if name.starts_with("custom:") => {
            let base_url = name.strip_prefix("custom:").unwrap_or("");
            let key = api_key.unwrap_or("");
            Box::new(OpenAiEmbedding::new(base_url, key, model, dims))
        }
        _ => Box::new(NoopEmbedding),
    }
}

/// Create embedding provider from EmbeddingConfig
pub fn create_embedding_provider_from_config(config: &EmbeddingConfig) -> Box<dyn EmbeddingProvider> {
    create_embedding_provider(
        &config.provider,
        config.api_key.as_deref(),
        &config.model,
        config.dimensions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_name() {
        let p = NoopEmbedding;
        assert_eq!(p.name(), "none");
        assert_eq!(p.dimensions(), 0);
    }

    #[tokio::test]
    async fn noop_embed_returns_empty() {
        let p = NoopEmbedding;
        let result = p.embed(&["hello"]).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn factory_none() {
        let p = create_embedding_provider("none", None, "model", 1536);
        assert_eq!(p.name(), "none");
    }

    #[test]
    fn factory_openai() {
        let p = create_embedding_provider("openai", Some("key"), "text-embedding-3-small", 1536);
        assert_eq!(p.name(), "openai");
        assert_eq!(p.dimensions(), 1536);
    }

    #[test]
    fn factory_custom_url() {
        let p = create_embedding_provider("custom:http://localhost:1234", None, "model", 768);
        assert_eq!(p.name(), "openai"); // uses OpenAiEmbedding internally
        assert_eq!(p.dimensions(), 768);
    }

    #[tokio::test]
    async fn noop_embed_one_returns_error() {
        let p = NoopEmbedding;
        let result = p.embed_one("hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn noop_embed_empty_batch() {
        let p = NoopEmbedding;
        let result = p.embed(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn factory_empty_string_returns_noop() {
        let p = create_embedding_provider("", None, "model", 1536);
        assert_eq!(p.name(), "none");
    }

    #[test]
    fn factory_unknown_provider_returns_noop() {
        let p = create_embedding_provider("cohere", None, "model", 1536);
        assert_eq!(p.name(), "none");
    }

    #[test]
    fn openai_trailing_slash_stripped() {
        let p = OpenAiEmbedding::new("https://api.openai.com/", "key", "model", 1536);
        assert_eq!(p.base_url, "https://api.openai.com");
    }

    #[test]
    fn default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "none");
        assert_eq!(config.dimensions, 1536);
    }

    #[test]
    fn from_config() {
        let config = EmbeddingConfig {
            provider: "openai".to_string(),
            api_key: Some("key".to_string()),
            model: "text-embedding-3-small".to_string(),
            dimensions: 1536,
        };
        let p = create_embedding_provider_from_config(&config);
        assert_eq!(p.name(), "openai");
    }
}
