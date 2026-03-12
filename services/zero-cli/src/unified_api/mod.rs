//! Unified API Module
//!
//! This module provides the unified HTTP API that merges the TypeScript API Server
//! functionality into the Rust daemon. It handles:
//!
//! - Sessions: CRUD operations for conversation sessions
//! - Agents: Agent dispatching and prompt management
//! - Memory: Dual-layer memory system (daily notes + long-term)
//! - Tasks: Async task management for external integrations
//! - Config: Configuration management
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Unified API Server (:4402)                              │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │   /health, /status        - Daemon management (existing)                    │
//! │   /gateway/*              - Gateway routes (from zero-hub)                  │
//! │   /channels/*             - IM channel routes (from zero-hub)               │
//! │   /workflow/*             - Workflow routes (from zero-hub)                 │
//! │                                                                              │
//! │   /api/v1/sessions/*      - Session management (NEW)                        │
//! │   /api/v1/agents/*        - Agent dispatching (NEW)                         │
//! │   /api/v1/memory/*        - Memory system (NEW)                             │
//! │   /api/v1/tasks/*         - Task management (NEW)                           │
//! │   /api/v1/config/*        - Configuration (NEW)                             │
//! │   /api/v1/prompts/*       - Prompt hot-loading (NEW)                        │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Migration Strategy
//!
//! This module implements the Phase 1 API Server merge:
//! 1. Core routes are implemented in Rust
//! 2. Complex business logic routes call TS layer via IPC
//! 3. Prompt files are hot-loaded from TS source

pub mod agents;
pub mod config;
pub mod definitions;
pub mod gear;
pub mod memory;
pub mod observer;
pub mod prompts;
pub mod providers;
pub mod sessions;
pub mod state;
pub mod tasks;
pub mod websocket;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

pub use state::UnifiedApiState;

/// Build the unified API router with all routes
pub fn build_router(state: Arc<UnifiedApiState>) -> Router {
    Router::new()
        // WebSocket route for real-time streaming
        .route("/ws", get(websocket::ws_handler))
        // Session routes
        .route("/api/v1/sessions", get(sessions::list_sessions))
        .route("/api/v1/sessions", post(sessions::create_session))
        .route("/api/v1/sessions/:id", get(sessions::get_session))
        .route("/api/v1/sessions/:id", patch(sessions::update_session))
        .route("/api/v1/sessions/:id", delete(sessions::delete_session))
        .route(
            "/api/v1/sessions/:id/messages",
            get(sessions::get_session_messages),
        )
        .route(
            "/api/v1/sessions/:id/messages",
            post(sessions::send_session_message),
        )
        .route(
            "/api/v1/sessions/:id/fork",
            post(sessions::fork_session),
        )
        .route(
            "/api/v1/sessions/:id/compact",
            post(sessions::compact_session),
        )
        // Agent routes
        .route("/api/v1/agents", get(agents::list_agents))
        .route("/api/v1/agents/:name", get(agents::get_agent))
        .route("/api/v1/agents/dispatch", post(agents::dispatch_agent))
        .route("/api/v1/agents/:name/prompt", get(agents::get_agent_prompt))
        // Provider routes
        .route("/api/v1/providers", get(providers::list_providers))
        .route("/api/v1/providers/all", get(providers::list_all_providers))
        .route(
            "/api/v1/providers/default-model",
            get(providers::get_default_model),
        )
        .route("/api/v1/providers/:id", get(providers::get_provider))
        .route(
            "/api/v1/providers/:provider_id/models/:model_id",
            get(providers::get_model),
        )
        .route(
            "/api/v1/providers/:provider_id/small-model",
            get(providers::get_small_model),
        )
        // Agent definition routes (new)
        .route(
            "/api/v1/definitions/agents",
            get(definitions::list_agent_definitions),
        )
        .route(
            "/api/v1/definitions/agents",
            post(definitions::create_agent_definition),
        )
        .route(
            "/api/v1/definitions/agents/:name",
            get(definitions::get_agent_definition),
        )
        .route(
            "/api/v1/definitions/agents/:name",
            put(definitions::update_agent_definition),
        )
        // Memory routes
        .route("/api/v1/memory/daily", get(memory::list_daily_dates))
        .route("/api/v1/memory/daily/:date", get(memory::get_daily_notes))
        .route("/api/v1/memory/daily", post(memory::append_daily_note))
        .route("/api/v1/memory/long-term", get(memory::get_long_term))
        .route(
            "/api/v1/memory/category/:category",
            put(memory::update_category),
        )
        .route(
            "/api/v1/memory/category/:category/merge",
            post(memory::merge_to_category),
        )
        .route("/api/v1/memory/consolidate", post(memory::consolidate))
        .route("/api/v1/memory/summary", get(memory::get_summary))
        // Task routes
        .route("/api/v1/tasks", get(tasks::list_tasks))
        .route("/api/v1/tasks", post(tasks::create_task))
        .route("/api/v1/tasks/:id", get(tasks::get_task))
        .route("/api/v1/tasks/:id", delete(tasks::delete_task))
        .route("/api/v1/tasks/:id/events", get(tasks::stream_task_events))
        .route("/api/v1/tasks/:id/interact", post(tasks::interact_task))
        // Config routes
        .route("/api/v1/config", get(config::get_config))
        .route("/api/v1/config", put(config::update_config))
        .route("/api/v1/config/validate", post(config::validate_config))
        // Prompt routes (hot-loading)
        .route("/api/v1/prompts", get(prompts::list_prompts))
        .route("/api/v1/prompts/:name", get(prompts::get_prompt))
        .route("/api/v1/prompts/reload", post(prompts::reload_prompts))
        // Gear control routes
        .route("/api/v1/gear/current", get(gear::get_gear_current))
        .route("/api/v1/gear/switch", post(gear::switch_gear))
        .route("/api/v1/gear/dials", post(gear::set_dials))
        .route("/api/v1/gear/dial", post(gear::set_single_dial))
        .route("/api/v1/gear/presets", get(gear::get_gear_presets))
        .route("/api/v1/gear/presets/:gear", get(gear::get_gear_preset))
        .route("/api/v1/gear/close", get(gear::get_close_evaluation))
        .route("/api/v1/gear/close", post(gear::evaluate_close))
        .route("/api/v1/gear/auto-switch", post(gear::set_auto_switch))
        // Observer routes
        .route("/api/v1/observer/start", post(observer::start_observer))
        .route("/api/v1/observer/stop", post(observer::stop_observer))
        .route("/api/v1/observer/status", get(observer::get_status))
        .route("/api/v1/observer/events", get(observer::stream_events))
        .route("/api/v1/observer/world-model", get(observer::get_world_model))
        .route("/api/v1/observer/consensus", get(observer::get_consensus))
        .route("/api/v1/observer/patterns", get(observer::get_patterns))
        .route("/api/v1/observer/anomalies", get(observer::get_anomalies))
        .route("/api/v1/observer/opportunities", get(observer::get_opportunities))
        .route("/api/v1/observer/ingest", post(observer::ingest_observations))
        // Watcher routes
        .route("/api/v1/observer/watchers", get(observer::list_watchers))
        .route("/api/v1/observer/watchers/:id", get(observer::get_watcher))
        .route("/api/v1/observer/watchers/:id/start", post(observer::start_watcher))
        .route("/api/v1/observer/watchers/:id/stop", post(observer::stop_watcher))
        .with_state(state)
}

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
