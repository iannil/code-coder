//! Zero Agent - AI agent execution engine.
//!
//! Provides the core agent loop that:
//! - Receives user messages
//! - Calls LLM with system prompt and conversation history
//! - Parses tool calls from responses
//! - Executes tools and feeds results back
//! - Returns final text response
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        AgentRegistry                                │
//! │              (configuration storage and lookup)                     │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         AgentLoader                                 │
//! │        (YAML config + Markdown prompt file loading)                 │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        AgentExecutor                                │
//! │              (tool-calling loop with LLM)                           │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use zero_core::agent::{AgentExecutor, AgentRegistry, ToolContext};
//!
//! // Initialize registry with built-in agents
//! let registry = AgentRegistry::new();
//! registry.register_natives(create_builtin_agents()).await;
//!
//! // Get agent configuration
//! let config = registry.get("build").await.unwrap();
//!
//! // Create executor
//! let executor = AgentExecutor::new(
//!     provider,
//!     tools,
//!     config.prompt_content.unwrap_or_default(),
//!     model,
//!     config.model.map(|m| m.temperature).flatten().unwrap_or(0.7),
//! );
//!
//! let response = executor.execute("Hello!").await?;
//! ```

pub mod builtin_prompts;
pub mod confirmation;
pub mod context;
pub mod executor;
pub mod loader;
pub mod provider;
pub mod registry;
pub mod streaming;

pub use confirmation::{
    get_confirmation_registry, get_notification_sink, handle_confirmation_response,
    handle_confirmation_response_with_type, init_confirmation_registry, notify,
    request_confirmation_and_wait, set_notification_sink, ConfirmationRegistry,
    ConfirmationResponse, NotificationSink, PendingConfirmation,
};
pub use context::ToolContext;
pub use executor::{AgentExecutor, ConfiguredExecutor, ToolCall, ToolRisk};
pub use loader::{
    AgentConfig, AgentLoader, AgentMode, AutoApproveConfig, LoaderError, LoaderPaths,
    ModelConfig, ObserverCapability, PermissionAction, PermissionConfig, PermissionRule,
    PermissionValue, RiskThreshold, ThinkingMode, WatcherType,
};
pub use provider::Provider;
pub use registry::{
    create_builtin_agents, get_global_registry, init_and_load, init_global_registry,
    AgentRegistry, RegistryError,
};
pub use streaming::{
    AnthropicProvider, ContentPart, GoogleProvider, Message, OpenAIProvider, Role, StreamEvent,
    StreamRequest, StreamingProvider, ToolDef, Usage,
};

pub use builtin_prompts::{get_builtin_prompt, list_builtin_agents};
