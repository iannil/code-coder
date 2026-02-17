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

// ============================================================================
// Provider Types
// ============================================================================

export interface ProviderModel {
  id: string
  name?: string
  providerID: string
  context?: number
  output?: number
  cost?: {
    input: number
    output: number
  }
  status?: "stable" | "preview" | "deprecated"
  variants?: Record<string, Record<string, unknown>>
}

export interface ProviderInfo {
  id: string
  name: string
  source: "env" | "config" | "custom" | "api"
  env: string[]
  options?: Record<string, unknown>
  models: Record<string, ProviderModel>
}

export interface ProviderListResponse {
  all: ProviderInfo[]
  default: Record<string, string>
  connected: string[]
}

export interface ProviderAuthMethod {
  type: "oauth" | "api"
  label: string
}

export interface ModelSelection {
  providerID: string
  modelID: string
}

// ============================================================================
// MCP Types
// ============================================================================

export type McpStatusType = "connected" | "disabled" | "failed" | "needs_auth" | "needs_client_registration"

export interface McpStatusConnected {
  status: "connected"
}

export interface McpStatusDisabled {
  status: "disabled"
}

export interface McpStatusFailed {
  status: "failed"
  error: string
}

export interface McpStatusNeedsAuth {
  status: "needs_auth"
}

export interface McpStatusNeedsClientRegistration {
  status: "needs_client_registration"
  error: string
}

export type McpStatus =
  | McpStatusConnected
  | McpStatusDisabled
  | McpStatusFailed
  | McpStatusNeedsAuth
  | McpStatusNeedsClientRegistration

export interface McpTool {
  name: string
  description: string
}

export interface McpResource {
  name: string
  uri: string
  description?: string
  mimeType?: string
  client: string
}

export interface McpAuthStatus {
  name: string
  supportsOAuth: boolean
  authStatus: "authenticated" | "expired" | "not_authenticated"
}

// ============================================================================
// Document Types (P2)
// ============================================================================

export type DocumentStatus = "planning" | "writing" | "reviewing" | "completed"
export type ChapterStatus = "pending" | "drafting" | "completed" | "revision"
export type EntityType = "character" | "location" | "concept" | "item" | "event"

export interface DocumentOutlineChapter {
  id: string
  title: string
  description: string
  estimatedWords: number
  subsections?: string[]
}

export interface DocumentOutline {
  title: string
  description?: string
  chapters: DocumentOutlineChapter[]
}

export interface DocumentStyleGuide {
  tone?: string
  voice?: string
  pov?: string
  tense?: string
  audience?: string
  notes?: string
}

export interface DocumentGlobalSummary {
  plot: string
  themes: string[]
  mainCharacters: string[]
  setting: string
}

export interface DocumentMetadata {
  id: string
  projectID: string
  title: string
  description?: string
  status: DocumentStatus
  targetWords: number
  currentWords: number
  createdAt: number
  updatedAt: number
  outline: DocumentOutline
  styleGuide?: DocumentStyleGuide
  globalSummary?: DocumentGlobalSummary
  volumes: string[]
}

export interface DocumentChapter {
  id: string
  documentID: string
  outlineID: string
  title: string
  status: ChapterStatus
  content: string
  summary?: string
  wordCount: number
  createdAt: number
  updatedAt: number
  volumeID?: string
  mentionedEntityIDs: string[]
}

export interface DocumentEntity {
  id: string
  type: EntityType
  name: string
  aliases: string[]
  description: string
  firstAppearedChapterID: string
  attributes: Record<string, string>
  relationships: Array<{
    targetEntityID: string
    type: string
    description: string
  }>
  createdAt: number
  updatedAt: number
}

export interface DocumentVolume {
  id: string
  documentID: string
  title: string
  description?: string
  summary?: string
  startChapterID: string
  endChapterID: string
  order: number
  createdAt: number
  updatedAt: number
}

export interface DocumentStats {
  totalChapters: number
  completedChapters: number
  pendingChapters: number
  totalWords: number
  targetWords: number
  progress: number
  estimatedRemaining: number
}

// ============================================================================
// Memory Types (P2)
// ============================================================================

export type DailyEntryType = "task" | "decision" | "learning" | "note" | "preference" | "context"

export interface DailyEntry {
  timestamp: string
  type: DailyEntryType
  content: string
  metadata?: Record<string, any>
}

export type MemoryCategory = "user_preferences" | "project_context" | "key_decisions" | "lessons_learned"

export interface MemorySection {
  category: MemoryCategory
  title: string
  content: string
}

export interface MemorySummary {
  longTermSize: number
  dailyNotesCount: number
  lastUpdated: number
}

export interface ConsolidationStats {
  lastRun?: number
  totalProcessed: number
  entriesExtracted: number
}

// ============================================================================
// Hooks Types (P2)
// ============================================================================

export type HookLifecycle = "PreToolUse" | "PostToolUse" | "PreResponse" | "Stop"
export type HookActionType =
  | "scan"
  | "check_env"
  | "check_style"
  | "notify_only"
  | "scan_content"
  | "run_command"
  | "analyze_changes"
  | "scan_files"

export interface HookAction {
  type: HookActionType
  patterns?: string[]
  message?: string
  block?: boolean
  command?: string
  async?: boolean
  variable?: string
  command_pattern?: string
  file_pattern?: string
  on_output?: Record<string, string>
}

export interface HookDefinition {
  pattern?: string
  description?: string
  command_pattern?: string
  file_pattern?: string
  actions: HookAction[]
}

export interface HookEntry {
  lifecycle: HookLifecycle
  name: string
  definition: HookDefinition
  source: string
}

export interface HookSettings {
  enabled?: boolean
  blocking_mode?: "interactive" | "silent" | "strict"
  log_level?: "debug" | "info" | "warn" | "error"
}

export interface HookLocation {
  path: string
  scope: "global" | "project"
  description: string
  exists: boolean
}

export interface HookActionTypeInfo {
  type: HookActionType
  description: string
  params: string[]
}

// ============================================================================
// LSP Types (P2)
// ============================================================================

export type LspServerStatus = "connected" | "error"

export interface LspStatus {
  id: string
  name: string
  root: string
  status: LspServerStatus
}

export interface LspRange {
  start: { line: number; character: number }
  end: { line: number; character: number }
}

export interface LspDiagnostic {
  severity: 1 | 2 | 3 | 4 // error, warning, info, hint
  range: LspRange
  message: string
  source?: string
  code?: string | number
  pretty?: string
}

export interface LspFileDiagnostics {
  filePath: string
  diagnostics: LspDiagnostic[]
}

export interface LspConfig {
  enabled: boolean
  servers?: Record<
    string,
    {
      command: string[]
      extensions?: string[]
      disabled?: boolean
      env?: Record<string, string>
    }
  >
}

export interface LspSymbol {
  name: string
  kind: number
  location: {
    uri: string
    range: LspRange
  }
}

export interface LspDocumentSymbol {
  name: string
  detail?: string
  kind: number
  range: LspRange
  selectionRange: LspRange
}

export interface LspLocation {
  uri: string
  range: LspRange
}

// ============================================================================
// Task Types (Async Task Management)
// ============================================================================

export type TaskStatus = "pending" | "running" | "awaiting_approval" | "completed" | "failed"

export interface TaskContext {
  userID: string
  platform: string
  chatHistory?: unknown[]
  source: "remote"
}

export interface TaskInfo {
  id: string
  sessionID: string
  status: TaskStatus
  agent: string
  prompt: string
  context: TaskContext
  output?: string
  error?: string
  createdAt: string
  updatedAt: string
}

export interface CreateTaskInput {
  agent: string
  prompt: string
  context: TaskContext
  sessionID?: string
  model?: string
}

export interface InteractTaskInput {
  action: "approve" | "reject"
  reason?: string
  reply?: "once" | "always" | "reject"
}

export type TaskEventType = "thought" | "tool_use" | "output" | "confirmation" | "finish" | "progress"

export interface ConfirmationRequest {
  requestID: string
  tool: string
  description: string
  args: unknown
  actions: string[]
}

export interface TaskEventBase {
  type: TaskEventType
}

export interface ThoughtTaskEvent extends TaskEventBase {
  type: "thought"
  data: string
}

export interface ToolUseTaskEvent extends TaskEventBase {
  type: "tool_use"
  data: {
    tool: string
    args: unknown
    result?: unknown
  }
}

export interface OutputTaskEvent extends TaskEventBase {
  type: "output"
  data: string
}

export interface ConfirmationTaskEvent extends TaskEventBase {
  type: "confirmation"
  data: ConfirmationRequest
}

export interface FinishTaskEvent extends TaskEventBase {
  type: "finish"
  data: {
    success: boolean
    output?: string
    error?: string
  }
}

export interface ProgressTaskEvent extends TaskEventBase {
  type: "progress"
  data: {
    stage: string
    message: string
    percentage?: number
  }
}

export type TaskEvent =
  | ThoughtTaskEvent
  | ToolUseTaskEvent
  | OutputTaskEvent
  | ConfirmationTaskEvent
  | FinishTaskEvent
  | ProgressTaskEvent
