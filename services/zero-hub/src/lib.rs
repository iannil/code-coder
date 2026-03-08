//! Zero Hub - Unified service hub for the Zero ecosystem.
//!
//! This crate combines three core services:
//! - **Gateway**: Authentication, routing, quotas, and security sandbox
//! - **Channels**: IM adapters for Telegram, Discord, Slack, Feishu, etc.
//! - **Workflow**: Webhooks, cron scheduling, Git integration, and workflow orchestration
//!
//! ## Architecture
//!
//! The Hub provides a unified entry point for all service functionality:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        zero-hub                             │
//! │  ┌───────────┐  ┌────────────┐  ┌─────────────┐            │
//! │  │  gateway  │  │  channels  │  │  workflow   │            │
//! │  │   :4430   │  │   :4431    │  │   :4432     │            │
//! │  └───────────┘  └────────────┘  └─────────────┘            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Feature Flags
//!
//! - `full`: Enable all services
//! - `gateway`: Gateway service only
//! - `channels`: Channels service with IM adapters
//! - `workflow`: Workflow service with scheduling
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zero_hub::{gateway, channels, workflow};
//!
//! // Build individual routers
//! let gateway_router = gateway::build_router(&config);
//! let channels_router = channels::build_channels_router(&config);
//! let workflow_service = workflow::WorkflowService::new(config);
//!
//! // Or start all services together
//! zero_hub::start_all_services(&config).await?;
//! ```

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

// Service modules
pub mod gateway;
pub mod channels;
pub mod workflow;

use zero_common::config::Config;

// Re-export commonly used types from gateway
pub use gateway::{
    build_router as build_gateway_router,
    start_server as start_gateway_server,
    PairingState,
    WebhookState as GatewayWebhookState,
    RbacState,
    ParallelRequest,
    ParallelResponse,
    ContextState,
    RoutingPolicy,
    RoutingPolicyConfig,
};

// Re-export provider types from gateway
pub use gateway::provider::{
    Provider,
    ProviderError,
    ProviderRegistry,
    AnthropicProvider,
    OpenAIProvider,
    GeminiProvider,
    OllamaProvider,
    OpenRouterProvider,
    CompatibleProvider,
    ResilientProvider,
    ResilienceConfig,
    create_registry,
    create_full_registry,
};

// Re-export commonly used types from channels
pub use channels::{
    build_channels_router,
    start_server as start_channels_server,
    ChannelMessage,
    ChannelType,
    MessageContent,
    OutgoingMessage,
    OutboundRouter,
    CodeCoderBridge,
    TelegramChannel,
    FeishuChannel,
    DiscordChannel,
    SlackChannel,
    EmailChannel,
    Channel,
    ChannelError,
    ChannelResult,
    SseTaskClient,
    TaskDispatcher,
    EventConsumer,
    ProgressHandler,
};

// Re-export commonly used types from workflow
pub use workflow::{
    WorkflowService,
    Scheduler,
    TaskInfo,
    Workflow,
    WorkflowExecutor,
    WorkflowResult,
    WebhookEvent,
    WebhookState as WorkflowWebhookState,
    ReviewBridge,
    TicketBridge,
    MonitorBridge,
    EconomicDataBridge,
    RiskMonitor,
    TradingReviewSystem,
    HandExecutor,
    HandsScheduler,
};

/// Hub configuration for running multiple services.
#[derive(Debug, Clone)]
pub struct HubConfig {
    /// Enable gateway service
    pub gateway_enabled: bool,
    /// Enable channels service
    pub channels_enabled: bool,
    /// Enable workflow service
    pub workflow_enabled: bool,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            gateway_enabled: true,
            channels_enabled: true,
            workflow_enabled: true,
        }
    }
}

/// Start all enabled services concurrently.
///
/// This function spawns each enabled service as a separate Tokio task
/// and runs them concurrently. Use this for the unified hub deployment.
pub async fn start_all_services(config: &Config, hub_config: &HubConfig) -> anyhow::Result<()> {
    use tokio::task::JoinSet;

    let mut tasks = JoinSet::new();

    if hub_config.gateway_enabled {
        let config = config.clone();
        tasks.spawn(async move {
            tracing::info!("Starting Gateway service");
            gateway::start_server(&config).await
        });
    }

    if hub_config.channels_enabled {
        let config = config.clone();
        tasks.spawn(async move {
            tracing::info!("Starting Channels service");
            channels::start_server(&config).await
        });
    }

    if hub_config.workflow_enabled {
        let config = config.clone();
        tasks.spawn(async move {
            tracing::info!("Starting Workflow service");
            let service = workflow::WorkflowService::new(config);
            service.start().await
        });
    }

    // Wait for all services (they run indefinitely unless there's an error)
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) => tracing::info!("Service completed successfully"),
            Ok(Err(e)) => tracing::error!(error = %e, "Service failed"),
            Err(e) => tracing::error!(error = %e, "Service task panicked"),
        }
    }

    Ok(())
}

/// Build a combined router with all services mounted on different paths.
///
/// This is useful for deployments that want a single HTTP server
/// serving all services on different path prefixes.
pub fn build_combined_router(config: &Config) -> axum::Router {
    use axum::Router;
    use tower_http::cors::{Any, CorsLayer};

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build individual routers
    let gateway_router = gateway::build_router(config);

    let (channels_router, _rx, _outbound, _email, _telegram, _tx) = channels::build_channels_router(config);

    let workflow_service = workflow::WorkflowService::new(config.clone());
    let workflow_router = workflow_service.build_router();

    // Combine under path prefixes
    Router::new()
        .nest("/gateway", gateway_router)
        .nest("/channels", channels_router)
        .nest("/workflow", workflow_router)
        .layer(cors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_config_default() {
        let config = HubConfig::default();
        assert!(config.gateway_enabled);
        assert!(config.channels_enabled);
        assert!(config.workflow_enabled);
    }
}
