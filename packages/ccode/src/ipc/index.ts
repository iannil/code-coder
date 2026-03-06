/**
 * IPC Module for TypeScript TUI ↔ zero-cli communication.
 *
 * This module provides Inter-Process Communication between the TypeScript TUI
 * frontend and the Rust CLI backend using JSON-RPC 2.0 over Unix Domain Sockets.
 *
 * ## Architecture
 *
 * ```
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                  ccode-tui (TypeScript/SolidJS)                      │
 * │   ┌───────────────────────────────────────────────────────────────┐ │
 * │   │                     IpcClient                                  │ │
 * │   │  • Connects to Unix Socket (~/.codecoder/ipc.sock)            │ │
 * │   │  • Auto-starts zero-cli if not running                        │ │
 * │   │  • Auto-reconnects on disconnect                              │ │
 * │   │  • Type-safe request/response API                             │ │
 * │   └───────────────────────────────────────────────────────────────┘ │
 * │                              ↕ JSON-RPC                             │
 * └─────────────────────────────────────────────────────────────────────┘
 *                               ↕
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                      zero-cli (Rust Binary)                          │
 * │   ┌───────────────────────────────────────────────────────────────┐ │
 * │   │                     IPC Server                                 │ │
 * │   │  • Listens on Unix Socket                                     │ │
 * │   │  • Handles tool execution                                     │ │
 * │   │  • Manages sessions                                           │ │
 * │   │  • Sends notifications                                        │ │
 * │   └───────────────────────────────────────────────────────────────┘ │
 * └─────────────────────────────────────────────────────────────────────┘
 * ```
 *
 * ## Usage
 *
 * ```typescript
 * import { createIpcClient } from "./ipc"
 *
 * // Create and connect client
 * const client = await createIpcClient()
 *
 * // Initialize session
 * const session = await client.initialize({ cwd: process.cwd() })
 * console.log("Session:", session.sessionId)
 * console.log("Tools:", session.tools.map(t => t.name))
 *
 * // Execute a tool
 * const result = await client.callTool("shell", { command: "ls -la" })
 * console.log("Result:", result.content)
 *
 * // Listen for notifications
 * client.on("stream_token", (notification) => {
 *   process.stdout.write(notification.token)
 * })
 *
 * client.on("error", (err) => {
 *   console.error("Error:", err.message)
 * })
 *
 * // Cleanup
 * await client.close()
 * ```
 */

// Types
export type {
  IpcId,
  IpcRequest,
  IpcResponse,
  IpcError,
  IpcMethod,
  IpcEvents,
  IpcEventName,
  ClientInfo,
  ServerInfo,
  ToolInfo,
  MessageRole,
  SessionMessage,
  InitializeParams,
  InitializeResult,
  ToolCallParams,
  ToolCallResult,
  ToolContent,
  ToolResultParams,
  GetSessionParams,
  SessionInfo,
  SessionSummary,
  ListSessionsResult,
  CompactParams,
  CompactResult,
  SessionUpdateNotification,
  ToolRequestNotification,
  LlmMessage,
  LlmOptions,
  LlmRequestNotification,
  StreamTokenNotification,
  ErrorNotification,
  // Phase 6.1: Agent prompt types
  ModelInfo,
  AgentPromptParams,
  AgentMessage,
  AgentToolCall,
  AgentPromptResult,
  AgentStreamNotification,
  AgentStreamEvent,
  TokenUsage,
} from "./types"

// Constants
export { IpcMethods, IpcErrorCodes, getDefaultSocketPath, createRequest, createNotification, isErrorResponse } from "./types"

// Protocol
export { IpcProtocol, isSocketReady, waitForSocket } from "./protocol"

// Client
export { IpcClient, createIpcClient, type IpcClientOptions } from "./client"
