//! IPC Module for zero-cli ↔ TypeScript TUI communication.
//!
//! This module provides Inter-Process Communication between the Rust CLI
//! backend and the TypeScript TUI frontend using JSON-RPC 2.0 over Unix
//! Domain Sockets (or Named Pipes on Windows).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      zero-cli (Rust Binary)                          │
//! │   ┌───────────────────────────────────────────────────────────────┐ │
//! │   │                     IPC Server                                 │ │
//! │   │  • Listens on Unix Socket (~/.codecoder/ipc.sock)            │ │
//! │   │  • Handles JSON-RPC 2.0 requests                              │ │
//! │   │  • Sends notifications to connected clients                   │ │
//! │   └───────────────────────────────────────────────────────────────┘ │
//! │                              ↕ JSON-RPC                             │
//! └─────────────────────────────────────────────────────────────────────┘
//!                               ↕
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  ccode-tui (TypeScript/SolidJS)                      │
//! │   ┌───────────────────────────────────────────────────────────────┐ │
//! │   │                     IPC Client                                 │ │
//! │   │  • Connects to Unix Socket                                    │ │
//! │   │  • Sends requests (initialize, tool_call, etc.)               │ │
//! │   │  • Receives notifications (stream_token, session_update)      │ │
//! │   └───────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Protocol
//!
//! The IPC protocol uses JSON-RPC 2.0 with the following methods:
//!
//! ## Requests (TUI → CLI)
//! - `ipc/initialize` - Initialize session and get tools
//! - `ipc/tool_call` - Execute a tool
//! - `ipc/get_session` - Get session history
//! - `ipc/list_sessions` - List all sessions
//! - `ipc/compact` - Compact session history
//! - `ipc/cancel_generation` - Cancel ongoing generation
//!
//! ## Notifications (CLI → TUI)
//! - `ipc/session_update` - Session state changed
//! - `ipc/tool_request` - Request tool execution
//! - `ipc/llm_request` - Request LLM completion
//! - `ipc/stream_token` - Stream token from LLM
//! - `ipc/error` - Error notification
//!
//! # Usage
//!
//! ```bash
//! # Start the IPC server
//! zero-cli serve-ipc
//!
//! # Start with custom socket path
//! zero-cli serve-ipc --socket /tmp/my-ipc.sock
//! ```

pub mod protocol;
pub mod server;

pub use protocol::{
    // Types
    ClientInfo,
    CompactParams,
    CompactResult,
    ErrorNotification,
    GetSessionParams,
    InitializeParams,
    InitializeResult,
    IpcError,
    IpcId,
    IpcRequest,
    IpcResponse,
    ListSessionsResult,
    LlmMessage,
    LlmOptions,
    LlmRequestNotification,
    ServerInfo,
    SessionInfo,
    SessionSummary,
    SessionUpdateNotification,
    StreamTokenNotification,
    ToolCallParams,
    ToolCallResult,
    ToolContent,
    ToolInfo,
    ToolRequestNotification,
    ToolResultParams,
    // Functions
    create_notification,
    default_socket_path,
    // Method constants
    methods,
};

pub use server::{serve, IpcServer};
