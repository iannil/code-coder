/**
 * IPC Protocol Types for zero-cli ↔ TypeScript TUI communication.
 *
 * These types mirror the Rust protocol definitions in
 * services/zero-cli/src/ipc/protocol.rs
 */

// ══════════════════════════════════════════════════════════════════════════════
// JSON-RPC 2.0 Types
// ══════════════════════════════════════════════════════════════════════════════

/** JSON-RPC ID (can be string or number) */
export type IpcId = string | number

/** JSON-RPC 2.0 request */
export interface IpcRequest {
  jsonrpc: "2.0"
  id?: IpcId
  method: string
  params?: unknown
}

/** JSON-RPC 2.0 response */
export interface IpcResponse {
  jsonrpc: "2.0"
  id?: IpcId
  result?: unknown
  error?: IpcError
}

/** JSON-RPC error object */
export interface IpcError {
  code: number
  message: string
  data?: unknown
}

// ══════════════════════════════════════════════════════════════════════════════
// IPC Method Constants
// ══════════════════════════════════════════════════════════════════════════════

export const IpcMethods = {
  // Request methods (TUI → CLI)
  INITIALIZE: "ipc/initialize",
  TOOL_CALL: "ipc/tool_call",
  TOOL_RESULT: "ipc/tool_result",
  CANCEL_GENERATION: "ipc/cancel_generation",
  GET_SESSION: "ipc/get_session",
  LIST_SESSIONS: "ipc/list_sessions",
  COMPACT: "ipc/compact",
  PING: "ipc/ping",
  /** Agent prompt request - initiates LLM call with tool callback support */
  AGENT_PROMPT: "ipc/agent_prompt",

  // Notification methods (CLI → TUI)
  SESSION_UPDATE: "ipc/session_update",
  TOOL_REQUEST: "ipc/tool_request",
  LLM_REQUEST: "ipc/llm_request",
  STREAM_TOKEN: "ipc/stream_token",
  ERROR: "ipc/error",
  /** Agent stream event - streaming response from LLM */
  AGENT_STREAM: "ipc/agent_stream",
} as const

export type IpcMethod = (typeof IpcMethods)[keyof typeof IpcMethods]

// ══════════════════════════════════════════════════════════════════════════════
// Request Parameter Types
// ══════════════════════════════════════════════════════════════════════════════

/** Client information */
export interface ClientInfo {
  name: string
  version: string
}

/** Initialize request params */
export interface InitializeParams {
  /** Optional session ID to resume (if undefined, creates new session) */
  sessionId?: string
  /** Current working directory */
  cwd: string
  /** Client info */
  clientInfo: ClientInfo
}

/** Server information */
export interface ServerInfo {
  name: string
  version: string
}

/** Tool information */
export interface ToolInfo {
  name: string
  description?: string
  inputSchema: Record<string, unknown>
}

/** Message role */
export type MessageRole = "user" | "assistant" | "system"

/** Session message */
export interface SessionMessage {
  id: number
  role: MessageRole
  content: string
  timestamp: number
  tokenEstimate: number
}

/** Initialize result */
export interface InitializeResult {
  /** Session ID (new or resumed) */
  sessionId: string
  /** Server info */
  serverInfo: ServerInfo
  /** Available tools */
  tools: ToolInfo[]
  /** Session history (if resuming) */
  messages: SessionMessage[]
}

/** Tool call request params */
export interface ToolCallParams {
  /** Unique call ID (for correlation) */
  callId: string
  /** Tool name */
  name: string
  /** Tool arguments */
  args: Record<string, unknown>
}

/** Tool content type */
export type ToolContent =
  | { type: "text"; text: string }
  | { type: "image"; data: string; mimeType: string }

/** Tool call result */
export interface ToolCallResult {
  /** Call ID (for correlation) */
  callId: string
  /** Result content */
  content: ToolContent[]
  /** Whether the tool execution resulted in an error */
  isError: boolean
}

/** Tool result notification params */
export interface ToolResultParams {
  /** Call ID (for correlation) */
  callId: string
  /** Result value */
  result: unknown
  /** Error message if any */
  error?: string
}

/** Get session params */
export interface GetSessionParams {
  sessionId: string
}

/** Session info result */
export interface SessionInfo {
  sessionId: string
  messages: SessionMessage[]
  tokenCount: number
  messageCount: number
  createdAt: number
  updatedAt: number
}

/** Session summary (for list) */
export interface SessionSummary {
  sessionId: string
  messageCount: number
  tokenCount: number
  updatedAt: number
}

/** List sessions result */
export interface ListSessionsResult {
  sessions: SessionSummary[]
}

/** Compact session params */
export interface CompactParams {
  sessionId: string
}

/** Compact result */
export interface CompactResult {
  deletedCount: number
  newTokenCount: number
}

// ══════════════════════════════════════════════════════════════════════════════
// Notification Types (CLI → TUI)
// ══════════════════════════════════════════════════════════════════════════════

/** Session update notification */
export interface SessionUpdateNotification {
  sessionId: string
  /** New message added */
  message?: SessionMessage
  /** Updated token count */
  tokenCount: number
}

/** Tool request notification */
export interface ToolRequestNotification {
  /** Unique request ID */
  requestId: string
  /** Tool name */
  name: string
  /** Tool arguments */
  args: unknown
}

/** LLM message format */
export interface LlmMessage {
  role: string
  content: string
}

/** LLM options */
export interface LlmOptions {
  model?: string
  temperature?: number
  maxTokens?: number
  stream: boolean
}

/** LLM request notification */
export interface LlmRequestNotification {
  /** Unique request ID */
  requestId: string
  /** Messages to send to LLM */
  messages: LlmMessage[]
  /** LLM options */
  options: LlmOptions
}

/** Stream token notification */
export interface StreamTokenNotification {
  /** Request ID this token belongs to */
  requestId: string
  /** Token content */
  token: string
  /** Whether this is the final token */
  done: boolean
}

/** Error notification */
export interface ErrorNotification {
  code: number
  message: string
  data?: unknown
}

// ══════════════════════════════════════════════════════════════════════════════
// Agent Prompt Types (Phase 6.1)
// ══════════════════════════════════════════════════════════════════════════════

/** Model information for agent prompt */
export interface ModelInfo {
  /** Provider ID (e.g., "anthropic", "openai") */
  providerId: string
  /** Model ID (e.g., "claude-opus-4-5", "gpt-4o") */
  modelId: string
  /** Optional API key override */
  apiKey?: string
  /** Optional base URL override */
  baseUrl?: string
}

/** Agent prompt request params */
export interface AgentPromptParams {
  /** Session ID */
  sessionId: string
  /** User message */
  message: string
  /** Agent name (e.g., "build", "plan") */
  agent: string
  /** Model to use */
  model: ModelInfo
  /** System prompt parts */
  system?: string[]
  /** Conversation history */
  messages?: AgentMessage[]
  /** Available tools (names) */
  tools?: string[]
}

/** Simplified message for agent context */
export interface AgentMessage {
  role: "user" | "assistant" | "system" | "tool"
  content: string
  toolCalls?: AgentToolCall[]
  toolCallId?: string
}

/** Tool call in message */
export interface AgentToolCall {
  id: string
  name: string
  arguments: unknown
}

/** Agent prompt result */
export interface AgentPromptResult {
  /** Request ID for stream correlation */
  requestId: string
  /** Whether streaming is enabled */
  streaming: boolean
}

/** Agent stream notification */
export interface AgentStreamNotification {
  /** Request ID for correlation */
  requestId: string
  /** Stream event */
  event: AgentStreamEvent
}

/** Agent stream event types */
export type AgentStreamEvent =
  | { type: "start" }
  | { type: "text_delta"; content: string }
  | { type: "reasoning_delta"; content: string }
  | { type: "tool_call_start"; id: string; name: string }
  | { type: "tool_call_delta"; id: string; argumentsDelta: string }
  | { type: "tool_call"; id: string; name: string; arguments: unknown }
  | { type: "tool_result"; id: string; output?: string; error?: string }
  | { type: "finish"; reason: string; usage?: TokenUsage }
  | { type: "error"; code: number; message: string }

/** Token usage statistics */
export interface TokenUsage {
  inputTokens: number
  outputTokens: number
  reasoningTokens?: number
  cacheReadTokens?: number
  cacheWriteTokens?: number
}

// ══════════════════════════════════════════════════════════════════════════════
// Event Types (for TypeScript typing)
// ══════════════════════════════════════════════════════════════════════════════

/** IPC event types for client event emitter */
export interface IpcEvents {
  session_update: [SessionUpdateNotification]
  tool_request: [ToolRequestNotification]
  llm_request: [LlmRequestNotification]
  stream_token: [StreamTokenNotification]
  agent_stream: [AgentStreamNotification]
  error: [ErrorNotification]
  connected: []
  disconnected: []
}

export type IpcEventName = keyof IpcEvents

// ══════════════════════════════════════════════════════════════════════════════
// Error Code Constants
// ══════════════════════════════════════════════════════════════════════════════

export const IpcErrorCodes = {
  // Standard JSON-RPC errors
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,

  // Custom errors
  SESSION_NOT_FOUND: -32001,
  TOOL_ERROR: -32002,
} as const

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

/** Create a JSON-RPC request */
export function createRequest(id: IpcId, method: string, params?: unknown): IpcRequest {
  const request: IpcRequest = {
    jsonrpc: "2.0",
    id,
    method,
  }
  if (params !== undefined) {
    request.params = params
  }
  return request
}

/** Create a JSON-RPC notification (no id) */
export function createNotification(method: string, params?: unknown): IpcRequest {
  const notification: IpcRequest = {
    jsonrpc: "2.0",
    method,
  }
  if (params !== undefined) {
    notification.params = params
  }
  return notification
}

/** Check if a response is an error */
export function isErrorResponse(response: IpcResponse): response is IpcResponse & { error: IpcError } {
  return response.error !== undefined
}

/** Get default IPC socket path */
export function getDefaultSocketPath(): string {
  const home = process.env.HOME || process.env.USERPROFILE || "."
  return `${home}/.codecoder/ipc.sock`
}
