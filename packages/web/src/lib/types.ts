/**
 * Shared TypeScript types for the CodeCoder Web API
 * These types match the backend API structure in packages/ccode/src/api/server/
 */

// ============================================================================
// UI Types
// ============================================================================

export interface BreadcrumbItem {
  label: string
  href?: string
  onClick?: () => void
}

// ============================================================================
// API Response Wrappers
// ============================================================================

export interface ApiResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

export interface ApiError {
  code: string
  message: string
  details?: unknown
}

// ============================================================================
// Session Types
// ============================================================================

export interface SessionInfo {
  id: string
  slug: string
  projectID: string
  directory: string
  parentID?: string
  summary?: SessionSummary
  title: string
  version: string
  time: SessionTime
  permission?: PermissionRuleset
  revert?: SessionRevert
}

export interface SessionSummary {
  additions: number
  deletions: number
  files: number
  diffs?: FileDiff[]
}

export interface FileDiff {
  path: string
  status: "added" | "modified" | "deleted" | "renamed"
  additions?: number
  deletions?: number
}

export interface SessionTime {
  created: number
  updated: number
  compacting?: number
  archived?: number
}

export interface SessionRevert {
  messageID: string
  partID?: string
  snapshot?: string
  diff?: string
}

export interface PermissionRuleset {
  [key: string]: boolean | string[]
}

export type SessionStatus = "idle" | "busy" | "retry"

export interface SessionStatusInfo {
  type: SessionStatus
  attempt?: number
  message?: string
  next?: number
}

// ============================================================================
// Message Types
// ============================================================================

export interface MessageInfo {
  id: string
  sessionID: string
  role: "user" | "assistant"
  time: MessageTime
  // User message fields
  summary?: MessageSummary
  agent?: string
  model?: MessageModel
  system?: string
  tools?: Record<string, boolean>
  variant?: string
  // Assistant message fields
  error?: MessageError
  parentID?: string
  modelID?: string
  providerID?: string
  mode?: string
  path?: MessagePath
  cost?: number
  tokens?: MessageTokens
  finish?: string
}

export interface MessageTime {
  created: number
  completed?: number
}

export interface MessageSummary {
  title?: string
  body?: string
  diffs: FileDiff[]
}

export interface MessageModel {
  providerID: string
  modelID: string
}

export interface MessagePath {
  cwd: string
  root: string
}

export interface MessageTokens {
  input: number
  output: number
  reasoning: number
  cache: {
    read: number
    write: number
  }
}

export interface MessageError {
  name: string
  message?: string
  [key: string]: any
}

export interface MessageWithParts {
  info: MessageInfo
  parts: MessagePart[]
}

// Message Parts
export type MessagePart =
  | TextPart
  | ReasoningPart
  | FilePart
  | ToolPart
  | AgentPart
  | SubtaskPart
  | RetryPart
  | CompactionPart
  | DecisionPart
  | StepStartPart
  | StepFinishPart
  | SnapshotPart
  | PatchPart

export interface PartBase {
  id: string
  sessionID: string
  messageID: string
}

export interface TextPart extends PartBase {
  type: "text"
  text: string
  synthetic?: boolean
  ignored?: boolean
  time?: { start: number; end?: number }
  metadata?: Record<string, any>
}

export interface ReasoningPart extends PartBase {
  type: "reasoning"
  text: string
  time: { start: number; end?: number }
  metadata?: Record<string, any>
}

export interface FilePart extends PartBase {
  type: "file"
  mime: string
  filename?: string
  url: string
  source?: FilePartSource
}

export interface FilePartSource {
  type: "file" | "symbol" | "resource"
  path?: string
  range?: { start: { line: number }; end: { line: number } }
  name?: string
  kind?: number
  clientName?: string
  uri?: string
  text: { value: string; start: number; end: number }
}

export interface ToolPart extends PartBase {
  type: "tool"
  callID: string
  tool: string
  state: ToolState
  metadata?: Record<string, any>
}

export type ToolState = ToolStatePending | ToolStateRunning | ToolStateCompleted | ToolStateError

export interface ToolStatePending {
  status: "pending"
  input: Record<string, any>
  raw: string
}

export interface ToolStateRunning {
  status: "running"
  input: Record<string, any>
  title?: string
  metadata?: Record<string, any>
  time: { start: number }
}

export interface ToolStateCompleted {
  status: "completed"
  input: Record<string, any>
  output: string
  title: string
  metadata: Record<string, any>
  time: { start: number; end: number; compacted?: number }
  attachments?: FilePart[]
}

export interface ToolStateError {
  status: "error"
  input: Record<string, any>
  error: string
  metadata?: Record<string, any>
  time: { start: number; end: number }
}

export interface AgentPart extends PartBase {
  type: "agent"
  name: string
  source?: { value: string; start: number; end: number }
}

export interface SubtaskPart extends PartBase {
  type: "subtask"
  prompt: string
  description: string
  agent: string
  model?: { providerID: string; modelID: string }
  command?: string
}

export interface RetryPart extends PartBase {
  type: "retry"
  attempt: number
  error: MessageError
  time: { created: number }
}

export interface CompactionPart extends PartBase {
  type: "compaction"
  auto: boolean
}

export interface DecisionPart extends PartBase {
  type: "decision"
  tool: string
  action: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip"
  score: {
    total: number
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
  }
  reasoning: string
  riskLevel?: "low" | "medium" | "high" | "critical"
}

export interface StepStartPart extends PartBase {
  type: "step-start"
  snapshot?: string
}

export interface StepFinishPart extends PartBase {
  type: "step-finish"
  reason: string
  snapshot?: string
  cost: number
  tokens: MessageTokens
}

export interface SnapshotPart extends PartBase {
  type: "snapshot"
  snapshot: string
}

export interface PatchPart extends PartBase {
  type: "patch"
  hash: string
  files: string[]
}

// ============================================================================
// Permission Types
// ============================================================================

export interface PermissionInfo {
  id: string
  type: string
  pattern?: string | string[]
  sessionID: string
  messageID: string
  callID?: string
  message: string
  metadata: Record<string, any>
  time: { created: number }
}

export type PermissionResponse = "once" | "always" | "reject"

export interface PermissionRespondInput {
  reply: PermissionResponse
  message?: string
}

// ============================================================================
// Config Types
// ============================================================================

export type ConfigValue = string | number | boolean | null | ConfigObject | ConfigArray
export interface ConfigObject { [key: string]: ConfigValue }
export interface ConfigArray extends Array<ConfigValue> {}

export type ConfigData = Record<string, ConfigValue>

// ============================================================================
// Agent Types
// ============================================================================

export interface AgentInfo {
  id: string
  name: string
  description?: string
  category?: string
  permission?: PermissionRuleset
  system?: string[]
  [key: string]: any
}

// ============================================================================
// File Types
// ============================================================================

export interface FileInfo {
  path: string
  name?: string
  type?: string
}

// ============================================================================
// Event Types (SSE)
// ============================================================================

export interface SSEEvent {
  event?: string
  data: string
  id?: string
  retry?: number
}

export type SSEEventType = "message" | "status" | "error" | "permission" | "progress"

export interface SSEMessageEvent {
  type: "message"
  sessionID: string
  messageID: string
  part?: MessagePart
}

export interface SSEStatusEvent {
  type: "status"
  sessionID: string
  status: SessionStatusInfo
}

export interface SSEErrorEvent {
  type: "error"
  sessionID?: string
  error: {
    name: string
    message: string
    [key: string]: any
  }
}

export interface SSEPermissionEvent {
  type: "permission"
  permission: PermissionInfo
}

export interface SSEProgressEvent {
  type: "progress"
  sessionID: string
  messageID?: string
  progress: {
    current: number
    total: number
    message?: string
  }
}

export type SSEDataEvent = SSEMessageEvent | SSEStatusEvent | SSEErrorEvent | SSEPermissionEvent | SSEProgressEvent

// ============================================================================
// Health Check Types
// ============================================================================

export interface HealthResponse {
  status: string
  version: string
  uptime: number
}

export interface ApiDiscoveryResponse {
  endpoints: string[]
  version: string
}

// ============================================================================
// Request Types
// ============================================================================

export interface SessionListQuery {
  limit?: number
  search?: string
}

export interface SessionCreateInput {
  title?: string
  parentID?: string
}

export interface SessionMessagesQuery {
  limit?: number
}

export interface MessageSendInput {
  agent?: string
  model?: string
  variant?: string
  parts: Array<{ type: string; text?: string; url?: string; filename?: string; mime?: string }>
}

export interface SessionForkInput {
  messageID?: string
}

export interface FileSearchQuery {
  q?: string
}

export interface EventChannelsResponse {
  success: boolean
  data: {
    channels: string[]
  }
}
