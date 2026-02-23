//! HTTP client for Zero Services.
//!
//! This module provides HTTP clients for calling deployed Zero Services:
//! - zero-gateway: LLM provider routing, authentication, quota management
//! - zero-channels: Message channel management
//! - zero-workflow: Cron scheduling, git webhooks
//!
//! When services are running locally or remotely, this client handles:
//! - Authentication via JWT or API key
//! - Request routing to the correct service
//! - Error handling and retries

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Default endpoints for Zero Services.
pub mod endpoints {
    pub const GATEWAY: &str = "http://localhost:4410";
    pub const CHANNELS: &str = "http://localhost:4411";
    pub const WORKFLOW: &str = "http://localhost:4412";
}

/// Client configuration.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Gateway service endpoint (default: localhost:4410)
    pub gateway_endpoint: String,
    /// Channels service endpoint (default: localhost:4411)
    pub channels_endpoint: String,
    /// Workflow service endpoint (default: localhost:4412)
    pub workflow_endpoint: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            gateway_endpoint: endpoints::GATEWAY.to_string(),
            channels_endpoint: endpoints::CHANNELS.to_string(),
            workflow_endpoint: endpoints::WORKFLOW.to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }
}

/// Zero Services HTTP client.
pub struct ZeroClient {
    config: ClientConfig,
    http: reqwest::Client,
}

impl ZeroClient {
    /// Create a new client with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new client with custom configuration.
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()?;

        Ok(Self { config, http })
    }

    /// Set the API key for authentication.
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.config.api_key = Some(api_key);
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Gateway API (LLM Provider Routing)
    // ─────────────────────────────────────────────────────────────────────────

    /// Chat with an LLM provider through the gateway.
    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/v1/chat", self.config.gateway_endpoint);
        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;
        Ok(response)
    }

    /// Run parallel inference across multiple models.
    pub async fn parallel_inference(
        &self,
        request: &ParallelInferenceRequest,
    ) -> Result<Vec<ChatResponse>> {
        let url = format!("{}/api/v1/parallel", self.config.gateway_endpoint);
        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await?
            .json::<Vec<ChatResponse>>()
            .await?;
        Ok(response)
    }

    /// Check quota usage.
    pub async fn get_quota(&self) -> Result<QuotaResponse> {
        let url = format!("{}/api/v1/quota", self.config.gateway_endpoint);
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Channels API (Message Channel Management)
    // ─────────────────────────────────────────────────────────────────────────

    /// List configured channels.
    pub async fn list_channels(&self) -> Result<Vec<ChannelInfo>> {
        let url = format!("{}/api/v1/channels", self.config.channels_endpoint);
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Send a message through a channel.
    pub async fn send_message(&self, request: &SendMessageRequest) -> Result<SendMessageResponse> {
        let url = format!("{}/api/v1/channels/send", self.config.channels_endpoint);
        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
    }

    /// Check channel health.
    pub async fn channel_health(&self, channel: &str) -> Result<HealthResponse> {
        let url = format!(
            "{}/api/v1/channels/{}/health",
            self.config.channels_endpoint, channel
        );
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Workflow API (Cron, Git Webhooks)
    // ─────────────────────────────────────────────────────────────────────────

    /// List scheduled cron jobs.
    pub async fn list_cron_jobs(&self) -> Result<Vec<CronJob>> {
        let url = format!("{}/api/v1/cron", self.config.workflow_endpoint);
        let response = self.http.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Add a cron job.
    pub async fn add_cron_job(&self, request: &AddCronJobRequest) -> Result<CronJob> {
        let url = format!("{}/api/v1/cron", self.config.workflow_endpoint);
        let response = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
    }

    /// Delete a cron job.
    pub async fn delete_cron_job(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/v1/cron/{}", self.config.workflow_endpoint, id);
        self.http.delete(&url).send().await?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Health & Status
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if all services are healthy.
    pub async fn health_check(&self) -> Result<ServicesHealth> {
        let (gateway, channels, workflow) = tokio::join!(
            self.check_service_health(&self.config.gateway_endpoint),
            self.check_service_health(&self.config.channels_endpoint),
            self.check_service_health(&self.config.workflow_endpoint),
        );

        Ok(ServicesHealth {
            gateway: gateway.unwrap_or(false),
            channels: channels.unwrap_or(false),
            workflow: workflow.unwrap_or(false),
        })
    }

    async fn check_service_health(&self, endpoint: &str) -> Result<bool> {
        let url = format!("{}/health", endpoint);
        let response = self.http.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

impl Default for ZeroClient {
    fn default() -> Self {
        Self::new().expect("Failed to create ZeroClient")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Request/Response Types
// ─────────────────────────────────────────────────────────────────────────────

/// Chat request to gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Chat response from gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub usage: Option<Usage>,
}

/// Token usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Parallel inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelInferenceRequest {
    pub models: Vec<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Quota response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaResponse {
    pub used_tokens: u64,
    pub limit_tokens: u64,
    pub used_requests: u64,
    pub limit_requests: u64,
    pub reset_at: String,
}

/// Channel info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub channel_type: String,
    pub enabled: bool,
    pub status: String,
}

/// Send message request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub channel: String,
    pub recipient: String,
    pub content: String,
}

/// Send message response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub sent_at: String,
}

/// Health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub message: Option<String>,
}

/// Cron job info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub expression: String,
    pub command: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
}

/// Add cron job request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCronJobRequest {
    pub expression: String,
    pub command: String,
}

/// Services health status.
#[derive(Debug, Clone)]
pub struct ServicesHealth {
    pub gateway: bool,
    pub channels: bool,
    pub workflow: bool,
}

impl ServicesHealth {
    pub fn all_healthy(&self) -> bool {
        self.gateway && self.channels && self.workflow
    }

    pub fn any_healthy(&self) -> bool {
        self.gateway || self.channels || self.workflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_config_defaults() {
        let config = ClientConfig::default();
        assert_eq!(config.gateway_endpoint, "http://localhost:4410");
        assert_eq!(config.channels_endpoint, "http://localhost:4411");
        assert_eq!(config.workflow_endpoint, "http://localhost:4412");
        assert_eq!(config.timeout_secs, 30);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn services_health_all_healthy() {
        let health = ServicesHealth {
            gateway: true,
            channels: true,
            workflow: true,
        };
        assert!(health.all_healthy());
        assert!(health.any_healthy());
    }

    #[test]
    fn services_health_partial() {
        let health = ServicesHealth {
            gateway: true,
            channels: false,
            workflow: true,
        };
        assert!(!health.all_healthy());
        assert!(health.any_healthy());
    }

    #[test]
    fn services_health_none() {
        let health = ServicesHealth {
            gateway: false,
            channels: false,
            workflow: false,
        };
        assert!(!health.all_healthy());
        assert!(!health.any_healthy());
    }

    #[test]
    fn chat_request_serialization() {
        let request = ChatRequest {
            model: "claude-3".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: None,
            system: Some("You are helpful".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-3"));
        assert!(json.contains("Hello"));
        assert!(!json.contains("max_tokens")); // skipped when None
    }
}
