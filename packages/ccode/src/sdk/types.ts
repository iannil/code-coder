/**
 * Rust Daemon SDK Types
 *
 * These types mirror the Rust WebSocket protocol exactly.
 * Generated from services/zero-cli/src/server/api/routes/ws.rs
 *
 * @module sdk/types
 */

// ══════════════════════════════════════════════════════════════════════════════
// Client → Server Messages
// ══════════════════════════════════════════════════════════════════════════════

export type WsClientMessage =
  | { type: "ping" }
  | AgentRequest
  | AgentCancel
  | ToolRequest
  | ConfirmationResponse
  | SessionSubscribe
  | SessionUnsubscribe
  | ObserverSubscribe
  | ObserverUnsubscribe

export interface AgentRequest {
  type: "agent_request"
  /** Unique request ID (for correlation) */
  id: string
  /** Session ID */
  session_id: string
  /** Agent name (e.g., "build", "plan") */
  agent: string
  /** User message */
  message: string
  /** Additional system prompts */
  system?: string[]
  /** Temperature override */
  temperature?: number
  /** Max tokens */
  max_tokens?: number
  /** Model override */
  model?: string
}

export interface AgentCancel {
  type: "agent_cancel"
  /** Request ID to cancel */
  id: string
}

export interface ToolRequest {
  type: "tool_request"
  id: string
  tool: string
  params: Record<string, unknown>
}

export interface ConfirmationResponse {
  type: "confirmation_response"
  /** Confirmation request ID */
  id: string
  /** User's decision */
  approved: boolean
  /** Optional user comment */
  comment?: string
}

export interface SessionSubscribe {
  type: "session_subscribe"
  session_id: string
}

export interface SessionUnsubscribe {
  type: "session_unsubscribe"
  session_id: string
}

export interface ObserverSubscribe {
  type: "observer_subscribe"
}

export interface ObserverUnsubscribe {
  type: "observer_unsubscribe"
}

// ══════════════════════════════════════════════════════════════════════════════
// Server → Client Messages
// ══════════════════════════════════════════════════════════════════════════════

export type WsServerMessage =
  | { type: "pong" }
  | Connected
  | AgentStart
  | AgentText
  | AgentReasoning
  | AgentToolCall
  | AgentToolResult
  | AgentComplete
  | AgentError
  | AgentCancelled
  | ToolResponse
  | ConfirmationRequest
  | SessionEvent
  | ObserverEvent
  | ErrorMessage

export interface Connected {
  type: "connected"
  /** Connection ID */
  connection_id: string
  /** Server version */
  version: string
}

export interface AgentStart {
  type: "agent_start"
  /** Request ID */
  id: string
}

export interface AgentText {
  type: "agent_text"
  /** Request ID */
  id: string
  /** Text content delta */
  content: string
}

export interface AgentReasoning {
  type: "agent_reasoning"
  /** Request ID */
  id: string
  /** Reasoning content delta */
  content: string
}

export interface AgentToolCall {
  type: "agent_tool_call"
  /** Request ID */
  id: string
  /** Tool call ID */
  tool_call_id: string
  /** Tool name */
  tool: string
  /** Tool arguments */
  arguments: Record<string, unknown>
}

export interface AgentToolResult {
  type: "agent_tool_result"
  /** Request ID */
  id: string
  /** Tool call ID */
  tool_call_id: string
  /** Tool output (if success) */
  output?: string
  /** Error message (if failed) */
  error?: string
}

export interface AgentComplete {
  type: "agent_complete"
  /** Request ID */
  id: string
  /** Stop reason */
  reason: string
  /** Token usage */
  usage?: TokenUsage
}

export interface AgentError {
  type: "agent_error"
  /** Request ID */
  id: string
  /** Error code */
  code: number
  /** Error message */
  message: string
}

export interface AgentCancelled {
  type: "agent_cancelled"
  /** Request ID */
  id: string
}

export interface ToolResponse {
  type: "tool_response"
  id: string
  success: boolean
  result?: unknown
  error?: string
}

export interface ConfirmationRequest {
  type: "confirmation_request"
  /** Confirmation ID */
  id: string
  /** Tool name */
  tool: string
  /** Tool arguments */
  arguments: Record<string, unknown>
  /** Risk level */
  risk_level: string
  /** Explanation of why confirmation is needed */
  reason: string
}

export interface SessionEvent {
  type: "session_event"
  session_id: string
  event_type: string
  data: unknown
}

export interface ObserverEvent {
  type: "observer_event"
  event_type: string
  data: unknown
}

export interface ErrorMessage {
  type: "error"
  /** Error code */
  code: number
  /** Error message */
  message: string
}

// ══════════════════════════════════════════════════════════════════════════════
// Common Types
// ══════════════════════════════════════════════════════════════════════════════

export interface TokenUsage {
  input_tokens: number
  output_tokens: number
  reasoning_tokens: number
  cache_read_tokens?: number
  cache_write_tokens?: number
}

// ══════════════════════════════════════════════════════════════════════════════
// HTTP API Types
// ══════════════════════════════════════════════════════════════════════════════

export interface AgentInfo {
  name: string
  description?: string
  mode: string
  temperature?: number
  color?: string
  hidden: boolean
}

export interface AgentListResponse {
  success: boolean
  agents: AgentInfo[]
  total: number
}

export interface AgentDetailResponse {
  success: boolean
  agent: AgentInfo
}

export interface DispatchAgentRequest {
  session_id: string
  agent: string
  message: string
  system?: string[]
  temperature?: number
  max_tokens?: number
  model?: string
  stream?: boolean
  max_iterations?: number
  tool_timeout?: number
}

export interface DispatchAgentResponse {
  success: boolean
  request_id: string
  streaming: boolean
}

export interface SessionInfo {
  id: string
  created_at: string
  updated_at: string
  title?: string
  project_id?: string
  agent?: string
  message_count: number
}

export interface SessionListResponse {
  success: boolean
  sessions: SessionInfo[]
  total: number
}

export interface GearStatus {
  gear: "P" | "N" | "D" | "S" | "M"
  dials: {
    observe: number
    decide: number
    act: number
  }
  auto_switch: boolean
}

export interface GearStatusResponse {
  success: boolean
  status: GearStatus
}

export interface ObserverStatus {
  running: boolean
  enabled: boolean
  consensus_confidence: number
  active_patterns: number
  active_anomalies: number
  active_opportunities: number
  world_model_updated_at?: string
}

export interface ObserverStatusResponse {
  success: boolean
  data: ObserverStatus
}

// ══════════════════════════════════════════════════════════════════════════════
// Event Types for Callbacks
// ══════════════════════════════════════════════════════════════════════════════

export type AgentEventHandler = (event: AgentStreamEvent) => void

export type AgentStreamEvent =
  | { type: "start"; id: string }
  | { type: "text"; id: string; content: string }
  | { type: "reasoning"; id: string; content: string }
  | { type: "tool_call"; id: string; toolCallId: string; tool: string; arguments: Record<string, unknown> }
  | { type: "tool_result"; id: string; toolCallId: string; output?: string; error?: string }
  | { type: "complete"; id: string; reason: string; usage?: TokenUsage }
  | { type: "error"; id: string; code: number; message: string }
  | { type: "cancelled"; id: string }
