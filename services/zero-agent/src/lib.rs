//! Zero Agent - AI agent execution engine.
//!
//! Provides the core agent loop that:
//! - Receives user messages
//! - Calls LLM with system prompt and conversation history
//! - Parses tool calls from responses
//! - Executes tools and feeds results back
//! - Returns final text response
//!
//! ## Example
//!
//! ```ignore
//! use zero_agent::{AgentExecutor, ToolContext};
//! use zero_tools::Tool;
//!
//! let executor = AgentExecutor::new(
//!     provider,
//!     tools,
//!     system_prompt,
//!     model,
//!     temperature,
//! );
//!
//! let response = executor.execute("Hello!").await?;
//! ```

pub mod confirmation;
pub mod context;
pub mod executor;
pub mod provider;

pub use confirmation::{
    get_confirmation_registry, get_notification_sink, handle_confirmation_response,
    handle_confirmation_response_with_type, init_confirmation_registry, notify,
    request_confirmation_and_wait, set_notification_sink, ConfirmationHandler,
    ConfirmationRegistry, ConfirmationResponse, NotificationSink, PendingConfirmation,
};
pub use context::ToolContext;
pub use executor::{AgentExecutor, ToolCall};
pub use provider::Provider;
