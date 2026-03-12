//! Zero Server - Unified HTTP/WebSocket API server
//!
//! This module provides HTTP/WebSocket endpoints for CodeCoder:
//! - `/api/v1/session` - Session management
//! - `/api/v1/tools` - Tool execution
//! - `/api/v1/mcp` - MCP protocol endpoints
//! - `/ws` - WebSocket for real-time streaming
//!
//! Originally from `zero-server` crate, now merged into `zero-cli`.

pub mod api;

pub use api::{create_router, AppState};

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use zero_core::common::config::Config;

/// Default ports for each service (legacy, for backward compatibility)
/// Note: Unified mode uses DAEMON_PORT (4402) with path prefixes
pub const DEFAULT_GATEWAY_PORT: u16 = 4430;
pub const DEFAULT_CHANNELS_PORT: u16 = 4431;
pub const DEFAULT_WORKFLOW_PORT: u16 = 4432;
pub const DEFAULT_API_PORT: u16 = 4435;

/// Unified daemon port (preferred)
pub const DAEMON_PORT: u16 = 4402;

/// Start the unified Zero Server
pub async fn start_server(config: &Config, unified: bool) -> Result<()> {
    if unified {
        start_unified(config).await
    } else {
        start_multi_port(config).await
    }
}

/// Start all services on separate ports (backward compatible mode).
async fn start_multi_port(config: &Config) -> Result<()> {
    let bind_addr: std::net::IpAddr = config.bind_address().parse()?;

    // Create handles for each service
    let gateway_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = zero_hub::gateway::start_server(&config).await {
                tracing::error!(service = "gateway", error = %e, "Gateway service failed");
            }
        })
    };

    let channels_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = zero_hub::channels::start_server(&config).await {
                tracing::error!(service = "channels", error = %e, "Channels service failed");
            }
        })
    };

    let workflow_handle = {
        let config = config.clone();
        tokio::spawn(async move {
            let service = zero_hub::workflow::WorkflowService::new(config);
            if let Err(e) = service.start().await {
                tracing::error!(service = "workflow", error = %e, "Workflow service failed");
            }
        })
    };

    let api_handle = {
        let bind_addr = bind_addr;
        tokio::spawn(async move {
            if let Err(e) = start_api_server(bind_addr).await {
                tracing::error!(service = "api", error = %e, "API service failed");
            }
        })
    };

    tracing::info!(
        gateway_port = config.gateway_port(),
        channels_port = config.channels_port(),
        workflow_port = config.workflow_port(),
        api_port = DEFAULT_API_PORT,
        "All services started"
    );

    // Wait for shutdown signal
    shutdown_signal().await;

    tracing::info!("Shutting down Zero Server...");

    gateway_handle.abort();
    channels_handle.abort();
    workflow_handle.abort();
    api_handle.abort();

    Ok(())
}

/// Start all services on a single port with path prefixes.
async fn start_unified(config: &Config) -> Result<()> {
    let bind_addr: std::net::IpAddr = config.bind_address().parse()?;
    let port = config.gateway_port();
    let addr = SocketAddr::from((bind_addr, port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create API state
    let api_state = Arc::new(AppState::new()?);

    // Build unified router with path prefixes
    let router = Router::new()
        .nest("/gateway", zero_hub::gateway::build_router(config))
        .nest("/channels", build_channels_router(config))
        .nest("/workflow", build_workflow_router(config))
        .nest("/api", create_router(api_state))
        .layer(cors);

    tracing::info!(
        port = port,
        "Starting unified Zero Server on single port"
    );
    tracing::info!("Routes:");
    tracing::info!("  /gateway/* → Authentication, routing, quotas");
    tracing::info!("  /channels/* → IM channel adapters");
    tracing::info!("  /workflow/* → Webhooks, cron, workflows");
    tracing::info!("  /api/* → HTTP/WebSocket API");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Build channels router without starting background tasks.
fn build_channels_router(config: &Config) -> Router {
    let (router, _rx, _outbound, _email, _telegram, _tx) =
        zero_hub::channels::build_channels_router(config);
    router
}

/// Build workflow router without starting background tasks.
fn build_workflow_router(config: &Config) -> Router {
    let service = zero_hub::workflow::WorkflowService::new(config.clone());
    service.build_router()
}

/// Start the standalone API server.
async fn start_api_server(bind_addr: std::net::IpAddr) -> Result<()> {
    let port = std::env::var("ZERO_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_API_PORT);

    let addr = SocketAddr::from((bind_addr, port));
    let state = Arc::new(AppState::new()?);
    let router = create_router(state);

    tracing::info!(port = port, "Starting Zero API server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Run the unified API server (simplified, no daemon overhead)
///
/// This function starts the unified API server with:
/// - Session management
/// - Agent dispatch
/// - Tool execution
/// - SSE streaming chat
/// - WebSocket support
///
/// Unlike the daemon, this doesn't start background workers or manage
/// external services. It's ideal for development or single-process deployment.
pub async fn run_api_server(config: crate::config::Config, host: &str, port: u16) -> Result<()> {
    use crate::memory::Memory;
    use crate::security::SecurityPolicy;
    use crate::session::store::SessionStore;
    use crate::tools::ToolRegistry;
    use crate::unified_api::state::{
        AnthropicProvider, ApiConfig, GoogleProvider, OpenAIProvider, StreamingProvider,
    };
    use crate::unified_api::{build_router, UnifiedApiState};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tower_http::cors::{Any, CorsLayer};

    // Initialize security policy
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    // Initialize memory
    let mem: Arc<dyn Memory> = Arc::from(crate::memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // Initialize tool registry
    let vault_path = config
        .config_path
        .parent()
        .map_or_else(|| config.workspace_dir.clone(), std::path::Path::to_path_buf);

    let registry = ToolRegistry::with_native_tools(
        &security,
        mem.clone(),
        &config.browser,
        &config.codecoder,
        &config.vault,
        &vault_path,
    );

    // Connect MCP servers if configured
    if !config.mcp.servers.is_empty() {
        match registry.connect_mcp_servers(&config.mcp).await {
            Ok(()) => {
                let count = registry.mcp_tool_count().await;
                if count > 0 {
                    tracing::info!("MCP: {} tools loaded", count);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect some MCP servers: {}", e);
            }
        }
    }

    let tools = Arc::new(RwLock::new(registry));

    // Initialize session store
    let db_path = config.workspace_dir.join("sessions.db");
    let sessions = match SessionStore::new(&db_path) {
        Ok(store) => Arc::new(store),
        Err(e) => {
            tracing::warn!("Failed to open session store: {}, using temp", e);
            let temp_db = std::env::temp_dir().join("codecoder_sessions.db");
            Arc::new(SessionStore::new(&temp_db)?)
        }
    };

    // Find prompts directory
    let prompts_dir = find_prompts_dir(&config.workspace_dir);
    tracing::info!("Prompts directory: {}", prompts_dir.display());

    // Create LLM provider if API key available
    let llm_provider: Option<Arc<dyn StreamingProvider>> =
        config.api_key.as_ref().map(|api_key| {
            let provider_id = config
                .default_provider
                .as_ref()
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| detect_provider(api_key));

            let provider: Arc<dyn StreamingProvider> = match provider_id.as_str() {
                "openai" | "openai-compatible" => Arc::new(OpenAIProvider::new(api_key)),
                "google" | "gemini" => Arc::new(GoogleProvider::new(api_key)),
                _ => Arc::new(AnthropicProvider::new(api_key)),
            };

            tracing::info!("LLM provider: {} (streaming enabled)", provider.name());
            provider
        });

    if llm_provider.is_none() {
        tracing::warn!("No API key configured, agent dispatch will be unavailable");
    }

    // Create unified API state
    let state = if let Some(provider) = llm_provider {
        Arc::new(UnifiedApiState::with_provider(
            sessions,
            tools,
            prompts_dir,
            config.workspace_dir.clone(),
            provider,
        ))
    } else {
        Arc::new(UnifiedApiState::new(
            sessions,
            tools,
            prompts_dir,
            config.workspace_dir.clone(),
        ))
    };

    // Initialize agent registry
    {
        let registry = zero_core::agent::init_global_registry();
        let natives = zero_core::agent::create_builtin_agents();
        registry.register_natives(natives).await;
        tracing::info!(
            "Agent registry: {} built-in agents",
            registry.len().await
        );
    }

    // Load agents from prompt files
    if let Err(e) = state.load_agents().await {
        tracing::warn!("Failed to load agents: {}", e);
    }

    // Load API configuration
    let api_config = ApiConfig::load_from_config_dir();
    if api_config.provider.is_some() || api_config.model.is_some() {
        tracing::info!(
            "Provider config: {} providers, default: {:?}",
            api_config.provider.as_ref().map(|p| p.len()).unwrap_or(0),
            api_config.model
        );
    }

    // Apply config (need to reconstruct Arc)
    let mut state_inner = Arc::try_unwrap(state).unwrap_or_else(|arc| (*arc).clone());
    state_inner.set_config(api_config);
    let state = Arc::new(state_inner);

    // Build router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = build_router(state).layer(cors);

    // Start server
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🚀 CodeCoder API Server                                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  API:       http://{:<43} ║", format!("{host}:{port}"));
    println!("║  WebSocket: ws://{:<44} ║", format!("{host}:{port}/ws"));
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Endpoints:                                                  ║");
    println!("║    POST /api/v1/sessions/:id/chat  - SSE streaming chat      ║");
    println!("║    GET  /api/v1/sessions           - List sessions           ║");
    println!("║    GET  /api/v1/agents             - List agents             ║");
    println!("║    GET  /api/v1/tools              - List tools              ║");
    println!("║    POST /api/v1/tools/:name        - Execute tool            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Press Ctrl+C to stop                                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Serve with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("\n✅ Server stopped gracefully\n");

    Ok(())
}

/// Find prompts directory
fn find_prompts_dir(workspace: &std::path::Path) -> std::path::PathBuf {
    let relative_path = std::path::Path::new("packages/ccode/src/agent/prompt");

    if relative_path.exists() {
        return relative_path.to_path_buf();
    }

    let workspace_prompts = workspace.join(relative_path);
    if workspace_prompts.exists() {
        return workspace_prompts;
    }

    let mut current = std::env::current_dir().unwrap_or_default();
    for _ in 0..5 {
        let candidate = current.join(relative_path);
        if candidate.exists() {
            return candidate;
        }
        if !current.pop() {
            break;
        }
    }

    workspace.join("prompts")
}

/// Detect provider from API key format
fn detect_provider(api_key: &str) -> String {
    if api_key.starts_with("sk-ant-") || api_key.starts_with("sk-proj-") {
        "anthropic".to_string()
    } else if api_key.starts_with("sk-") {
        "openai".to_string()
    } else if api_key.starts_with("AIza") {
        "google".to_string()
    } else {
        "anthropic".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ports() {
        assert_eq!(DEFAULT_GATEWAY_PORT, 4430);
        assert_eq!(DEFAULT_CHANNELS_PORT, 4431);
        assert_eq!(DEFAULT_WORKFLOW_PORT, 4432);
        assert_eq!(DEFAULT_API_PORT, 4435);
    }
}
