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
  /** Legacy: creation timestamp in seconds */
  created_at: number
  /** Legacy: update timestamp in seconds */
  updated_at: number
  /** Time information (TS-compatible format) */
  time: SessionTime
  title?: string
  project_id?: string
  agent?: string
  message_count: number
  token_count?: number
  /** Parent session ID for forked sessions */
  parent_id?: string
  /** Directory where session was created */
  directory?: string
}

/**
 * Time information for sessions (compatible with TS Session.Info.time)
 */
export interface SessionTime {
  /** Creation timestamp (milliseconds since epoch) */
  created: number
  /** Last update timestamp (milliseconds since epoch) */
  updated: number
  /** Compacting timestamp if session is being compacted */
  compacting?: number
  /** Archive timestamp if session is archived */
  archived?: number
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

// ══════════════════════════════════════════════════════════════════════════════
// Extended Session Types (from @/session)
// These types bridge the TUI to the SDK, providing compatibility during migration.
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Extended session info from TypeScript Session module.
 * Maps to Session.Info in @/session/index.ts
 */
export interface SessionInfoExtended {
  id: string
  slug: string
  projectID: string
  directory: string
  parentID?: string
  summary?: {
    additions: number
    deletions: number
    files: number
    diffs?: FileDiff[]
  }
  title: string
  version: string
  time: {
    created: number
    updated: number
    compacting?: number
    archived?: number
  }
  permission?: PermissionRuleset
  revert?: {
    messageID: string
    partID?: string
    snapshot?: string
    diff?: string
  }
}

/**
 * Permission ruleset for session access control
 */
export interface PermissionRuleset {
  [key: string]: unknown
}

/**
 * File diff information from snapshots
 */
export interface FileDiff {
  file: string
  before: string
  after: string
  additions: number
  deletions: number
}

// ══════════════════════════════════════════════════════════════════════════════
// Message Types (from @/session/message-v2)
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Base part structure
 */
interface MessagePartBase {
  id: string
  sessionID: string
  messageID: string
}

/**
 * Text part of a message
 */
export interface TextPart extends MessagePartBase {
  type: "text"
  text: string
  synthetic?: boolean
  ignored?: boolean
  time?: {
    start: number
    end?: number
  }
  metadata?: Record<string, unknown>
}

/**
 * Reasoning part (for models with extended thinking)
 */
export interface ReasoningPart extends MessagePartBase {
  type: "reasoning"
  text: string
  metadata?: Record<string, unknown>
  time: {
    start: number
    end?: number
  }
}

/**
 * File part (attachments)
 */
export interface FilePart extends MessagePartBase {
  type: "file"
  mime: string
  filename?: string
  url: string
  source?: FilePartSource
}

export type FilePartSource =
  | { type: "file"; path: string; text: { value: string; start: number; end: number } }
  | { type: "symbol"; path: string; range: { start: { line: number; character: number }; end: { line: number; character: number } }; name: string; kind: number; text: { value: string; start: number; end: number } }
  | { type: "resource"; clientName: string; uri: string; text: { value: string; start: number; end: number } }

/**
 * Tool call state
 */
export type ToolState =
  | { status: "pending"; input: Record<string, unknown>; raw: string }
  | { status: "running"; input: Record<string, unknown>; title?: string; metadata?: Record<string, unknown>; time: { start: number } }
  | { status: "completed"; input: Record<string, unknown>; output: string; title: string; metadata: Record<string, unknown>; time: { start: number; end: number; compacted?: number }; attachments?: FilePart[] }
  | { status: "error"; input: Record<string, unknown>; error: string; metadata?: Record<string, unknown>; time: { start: number; end: number } }

/**
 * Tool part of a message
 */
export interface ToolPart extends MessagePartBase {
  type: "tool"
  callID: string
  tool: string
  state: ToolState
  metadata?: Record<string, unknown>
}

/**
 * Step start/finish parts for tracking API calls
 */
export interface StepStartPart extends MessagePartBase {
  type: "step-start"
  snapshot?: string
}

export interface StepFinishPart extends MessagePartBase {
  type: "step-finish"
  reason: string
  snapshot?: string
  cost: number
  tokens: {
    input: number
    output: number
    reasoning: number
    cache: { read: number; write: number }
  }
}

/**
 * Union type for all message parts
 */
export type MessagePart =
  | TextPart
  | ReasoningPart
  | ToolPart
  | FilePart
  | StepStartPart
  | StepFinishPart
  | { type: "snapshot"; id: string; sessionID: string; messageID: string; snapshot: string }
  | { type: "patch"; id: string; sessionID: string; messageID: string; hash: string; files: string[] }
  | { type: "agent"; id: string; sessionID: string; messageID: string; name: string; source?: { value: string; start: number; end: number } }
  | { type: "subtask"; id: string; sessionID: string; messageID: string; prompt: string; description: string; agent: string; model?: { providerID: string; modelID: string }; command?: string }
  | { type: "compaction"; id: string; sessionID: string; messageID: string; auto: boolean }
  | { type: "retry"; id: string; sessionID: string; messageID: string; attempt: number; error: APIErrorInfo; time: { created: number } }
  | { type: "decision"; id: string; sessionID: string; messageID: string; tool: string; action: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip"; score: CLOSEScore; reasoning: string; riskLevel?: "low" | "medium" | "high" | "critical" }

/**
 * API error information
 */
export interface APIErrorInfo {
  name: string
  message: string
  statusCode?: number
  isRetryable: boolean
  responseHeaders?: Record<string, string>
  responseBody?: string
  metadata?: Record<string, string>
}

/**
 * CLOSE framework score (Convergence, Leverage, Optionality, Surplus, Evolution)
 */
export interface CLOSEScore {
  total: number
  convergence: number
  leverage: number
  optionality: number
  surplus: number
  evolution: number
}

/**
 * User message info
 */
export interface UserMessageInfo {
  id: string
  sessionID: string
  role: "user"
  time: { created: number }
  summary?: {
    title?: string
    body?: string
    diffs: FileDiff[]
  }
  agent: string
  model: { providerID: string; modelID: string }
  system?: string
  tools?: Record<string, boolean>
  variant?: string
}

/**
 * Assistant message info
 */
export interface AssistantMessageInfo {
  id: string
  sessionID: string
  role: "assistant"
  time: { created: number; completed?: number }
  error?: { name: string; [key: string]: unknown }
  parentID: string
  modelID: string
  providerID: string
  mode: string
  agent: string
  path: { cwd: string; root: string }
  summary?: boolean
  cost: number
  tokens: {
    input: number
    output: number
    reasoning: number
    cache: { read: number; write: number }
  }
  finish?: string
}

/**
 * Message info (user or assistant)
 */
export type MessageInfo = UserMessageInfo | AssistantMessageInfo

/**
 * Message with its parts
 */
export interface MessageWithParts {
  info: MessageInfo
  parts: MessagePart[]
}

// ══════════════════════════════════════════════════════════════════════════════
// Autonomous Mode Types (from @/autonomous)
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Autonomous mode state machine states
 */
export type AutonomousState =
  | "IDLE"
  | "PLANNING"
  | "EXECUTING"
  | "TESTING"
  | "REVIEWING"
  | "PAUSED"
  | "COMPLETED"
  | "FAILED"
  | "ROLLING_BACK"

/**
 * Autonomy level (craziness level)
 */
export type AutonomyLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

/**
 * Session metrics summary
 */
export interface SessionMetrics {
  sessionId: string
  startTime: number
  endTime?: number
  duration: number
  tasks: {
    total: number
    completed: number
    failed: number
    skipped: number
  }
  decisions: {
    total: number
    approved: number
    paused: number
    blocked: number
    averageScore: number
  }
  resources: {
    tokensUsed: number
    costUSD: number
    filesChanged: number
  }
  tests: {
    run: number
    passed: number
    failed: number
    passRate: number
  }
  tdd: {
    cycles: number
    redPassed: number
    greenPassed: number
    refactorPassed: number
  }
  safety: {
    rollbacks: number
    loopsDetected: number
    warnings: number
  }
  states: {
    transitions: number
    finalState: AutonomousState
  }
}

/**
 * Quality score breakdown
 */
export interface QualityScoreBreakdown {
  overall: number
  testCoverage: number
  codeQuality: number
  decisionQuality: number
  efficiency: number
  safety: number
}

/**
 * Craziness score breakdown
 */
export interface CrazinessScoreBreakdown {
  overall: number
  level: AutonomyLevel
  autonomy: number
  selfCorrection: number
  speed: number
  riskTaking: number
}

/**
 * Resource usage tracking
 */
export interface ResourceUsage {
  tokens: number
  cost: number
  time: number
  files: number
  actions: number
}

/**
 * Safety integration status
 */
export interface SafetyStatus {
  resources: {
    usage: ResourceUsage
    remaining: {
      tokens?: number
      cost?: number
      time?: number
      files?: number
      actions?: number
    }
    surplusRatio: number
    warnings: number
  }
  loops: {
    stateLoops: number
    toolLoops: number
    decisionHesitations: number
    loopsBroken: number
  }
  rollbacks: {
    count: number
    canRetry: boolean
  }
  safe: boolean
}

/**
 * Simplified autonomous event types for TUI consumption
 */
export type AutonomousEventType =
  | "state_changed"
  | "session_started"
  | "session_completed"
  | "session_failed"
  | "session_paused"
  | "decision_made"
  | "task_created"
  | "task_started"
  | "task_completed"
  | "task_failed"
  | "phase_started"
  | "phase_completed"
  | "metrics_updated"
  | "safety_triggered"
  | "resource_warning"

export interface AutonomousEventData {
  type: AutonomousEventType
  sessionId: string
  timestamp: number
  data: Record<string, unknown>
}

// ══════════════════════════════════════════════════════════════════════════════
// Provider Types (from @/provider)
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Provider model capabilities
 */
export interface ModelCapabilities {
  temperature: boolean
  reasoning: boolean
  attachment: boolean
  toolcall: boolean
  input: {
    text: boolean
    audio: boolean
    image: boolean
    video: boolean
    pdf: boolean
  }
  output: {
    text: boolean
    audio: boolean
    image: boolean
    video: boolean
    pdf: boolean
  }
  interleaved: boolean | { field: "reasoning_content" | "reasoning_details" }
}

/**
 * Model cost information
 */
export interface ModelCost {
  input: number
  output: number
  cache: {
    read: number
    write: number
  }
  experimentalOver200K?: {
    input: number
    output: number
    cache: {
      read: number
      write: number
    }
  }
}

/**
 * Model information
 */
export interface ModelInfo {
  id: string
  providerID: string
  name: string
  family?: string
  api: {
    id: string
    url: string
    npm: string
  }
  capabilities: ModelCapabilities
  cost: ModelCost
  limit: {
    context: number
    input?: number
    output: number
  }
  status: "alpha" | "beta" | "deprecated" | "active"
  options: Record<string, unknown>
  headers: Record<string, string>
  release_date: string
  variants?: Record<string, Record<string, unknown>>
}

/**
 * Provider information
 */
export interface ProviderInfo {
  id: string
  name: string
  source: "env" | "config" | "custom" | "api"
  env: string[]
  key?: string
  options: Record<string, unknown>
  models: Record<string, ModelInfo>
}

/**
 * Provider list response (extended)
 */
export interface ProviderListResponseExtended {
  success: boolean
  all: ProviderInfo[]
  default: Record<string, string>
  connected: string[]
}

// ══════════════════════════════════════════════════════════════════════════════
// Agent Mode Types (from @/agent/mode)
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Mode definition
 */
export interface Mode {
  id: string
  name: string
  description: string
  primaryAgent: string
  alternativePrimaries: string[]
  capabilities: string[]
  color?: string
  icon?: string
}

// ══════════════════════════════════════════════════════════════════════════════
// Utility Functions
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Parse a model string (e.g., "anthropic/claude-sonnet-4") into provider and model IDs
 */
export function parseModel(model: string): { providerID: string; modelID: string } {
  const [providerID, ...rest] = model.split("/")
  return {
    providerID,
    modelID: rest.join("/"),
  }
}

/**
 * Check if a session title is the default generated title
 */
export function isDefaultTitle(title: string): boolean {
  const parentTitlePrefix = "New session - "
  const childTitlePrefix = "Child session - "
  return new RegExp(
    `^(${parentTitlePrefix}|${childTitlePrefix})\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}\\.\\d{3}Z$`,
  ).test(title)
}

/**
 * Parse mode:capability notation (e.g., "@build:security-review")
 */
export function parseModeCapability(input: string): { mode: string; capability: string } | null {
  const match = input.match(/^@?(\w+):(\w+(?:-\w+)*)$/)
  if (!match) return null
  const [, mode, capability] = match
  return { mode, capability }
}
