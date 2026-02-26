//! Zero Gateway - Authentication, routing, quotas, and security sandbox.
//!
//! This crate provides the gateway service for the Zero ecosystem:
//! - JWT and API Key authentication
//! - Pairing-based authentication for device pairing
//! - Webhook endpoints for external integrations
//! - Request routing and load balancing
//! - Token quota management and metering
//! - Security sandbox and audit logging
//! - User management with RBAC
//!
//! ## Architecture
//!
//! The gateway sits between clients and the CodeCoder API:
//! ```text
//! Client → Gateway (auth → quota check → metering) → CodeCoder
//!                         ↓
//!                   Record usage
//! ```
//!
//! ## Authentication Modes
//!
//! The gateway supports multiple authentication modes:
//! - `pairing`: Device pairing with one-time codes (for local/trusted environments)
//! - `jwt`: JWT token authentication (for multi-user environments)
//! - `both`: Both pairing and JWT are accepted

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod auth;
pub mod context;
pub mod hitl;
pub mod metering;
pub mod pairing;
pub mod parallel;
pub mod provider;
pub mod proxy;
pub mod quota;
pub mod rbac;
pub mod routes;
pub mod routing_policy;
pub mod sandbox;
pub mod user;
pub mod webhook;

pub use parallel::{ParallelRequest, ParallelResponse, ParallelState, parallel_routes};
pub use pairing::{PairingState, pairing_routes};
pub use context::{ContextState, context_routes, SearchQuery, SearchResponse, ContextEntry, IngestRequest, IngestResponse};
pub use provider::{
    AnthropicProvider, AuthStyle, ChatRequest, ChatResponse, CompatibleProvider, GeminiProvider,
    OllamaProvider, OpenAIProvider, OpenRouterProvider, Provider, ProviderError, ProviderRegistry,
    ResilienceConfig, ResilientProvider, create_full_registry, create_registry,
};
pub use rbac::{RbacState, require_access};
pub use routing_policy::{RoutingDecision, RoutingPolicy, RoutingPolicyConfig, SensitivityLevel};
pub use webhook::{WebhookState, webhook_routes};

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use zero_common::config::Config;

/// Build the gateway router with all routes and middleware.
pub fn build_router(config: &Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build base routes
    let router = routes::build_all_routes(config);

    // Add pairing routes if auth.mode includes pairing
    let router = if config.auth.mode == "pairing" || config.auth.mode == "both" {
        let pairing_state = PairingState::new(
            config.gateway.require_pairing,
            &config.gateway.paired_tokens,
        );
        router.merge(pairing_routes(pairing_state))
    } else {
        router
    };

    // Add webhook routes
    let webhook_state = WebhookState::new(&config.codecoder_endpoint());
    let router = router.merge(webhook_routes(webhook_state));

    router.layer(cors)
}

/// Build the gateway router using legacy separate route builders (for backward compatibility).
pub fn build_router_legacy(config: &Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(routes::auth_routes())
        .merge(routes::proxy_routes(config))
        .merge(routes::health_routes())
        .layer(cors)
}

/// Start the gateway server.
pub async fn start_server(config: &Config) -> anyhow::Result<()> {
    let addr = SocketAddr::from((
        config.bind_address().parse::<std::net::IpAddr>()?,
        config.gateway_port(),
    ));

    let router = build_router(config);

    tracing::info!("Starting Zero Gateway on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
