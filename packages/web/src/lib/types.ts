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
  projectID?: string
  directory?: string
  agent?: string
  model?: string
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

// ============================================================================
// Infrastructure Types (Zero-Bot)
// ============================================================================

// Channel Types
export type ChannelType = "cli" | "telegram" | "discord" | "slack" | "matrix" | "whatsapp" | "imessage" | "email" | "feishu"
export type ChannelHealth = "healthy" | "degraded" | "unhealthy"

export interface ChannelConfig {
  token?: string
  webhookUrl?: string
  botId?: string
  apiKey?: string
  [key: string]: unknown
}

export interface ChannelStatus {
  name: string
  type: ChannelType
  enabled: boolean
  health: ChannelHealth
  config: ChannelConfig
  lastPing?: number
  error?: string
}

// Gateway Types
export interface GatewayEndpoint {
  path: string
  method: string
  description?: string
}

export interface GatewayRequest {
  id: string
  method: string
  path: string
  status: number
  timestamp: number
  duration: number
}

export interface GatewayStatus {
  running: boolean
  host: string
  port: number
  uptime?: number
  endpoints: GatewayEndpoint[]
  requestCount: number
  recentRequests: GatewayRequest[]
}

// Cron Types
export type CronJobStatus = "success" | "failed" | "running" | "pending"

export interface CronJob {
  id: string
  name: string
  expression: string
  command: string
  enabled: boolean
  nextRun?: number
  lastRun?: number
  lastStatus?: CronJobStatus
  lastError?: string
}

export interface CronHistory {
  id: string
  jobId: string
  jobName: string
  startTime: number
  endTime?: number
  status: CronJobStatus
  output?: string
  error?: string
}

// Tunnel Types
export type TunnelType = "cloudflare" | "ngrok" | "tailscale" | "custom" | "none"

export interface TunnelStatus {
  type: TunnelType
  connected: boolean
  publicUrl?: string
  localUrl?: string
  latency?: number
  error?: string
  startedAt?: number
}

// ============================================================================
// Project Types
// ============================================================================

export interface ProjectInfo {
  id: string
  worktree: string
  name?: string
  icon?: {
    url?: string
    override?: string
    color?: string
  }
  vcs?: "git" | { branch?: string }
  sandboxes: string[]
  time: {
    created: number
    updated: number
    initialized?: number
  }
}

export interface ProjectCreateInput {
  directory: string
  name?: string
}

export interface DirectoryEntry {
  name: string
  path: string
  isDirectory: boolean
}

export interface DirectoryListResponse {
  path: string
  directories: DirectoryEntry[]
  parent: string | null
}

// ============================================================================
// Credential Types
// ============================================================================

export type CredentialType = "api_key" | "oauth" | "login" | "bearer_token"

export interface OAuthCredential {
  clientId: string
  clientSecret?: string
  accessToken?: string
  refreshToken?: string
  expiresAt?: number
  scope?: string
}

export interface LoginCredential {
  username: string
  password: string
  totpSecret?: string
  sessionPath?: string
  sessionUpdatedAt?: number
}

export interface CredentialEntry {
  id: string
  type: CredentialType
  name: string
  service: string
  apiKey?: string
  oauth?: OAuthCredential
  login?: LoginCredential
  patterns: string[]
  createdAt: number
  updatedAt: number
}

export interface CredentialSummary {
  id: string
  type: CredentialType
  name: string
  service: string
  patterns: string[]
  createdAt: string
  updatedAt: string
}

export interface CredentialCreateInput {
  type: CredentialType
  name: string
  service: string
  apiKey?: string
  oauth?: OAuthCredential
  login?: LoginCredential
  patterns: string[]
}

// ============================================================================
// Metering Types (Admin Dashboard)
// ============================================================================

export interface MeteringUsageResponse {
  total_users: number
  active_users_24h: number
  tokens_used_24h: number
  tokens_used_30d: number
  requests_24h: number
  requests_30d: number
}

export interface MeteringQuota {
  user_id: string
  daily_input_limit: number
  daily_output_limit: number
  monthly_input_limit: number
  monthly_output_limit: number
}

export interface MeteringUserReport {
  user_id: string
  name: string
  email?: string
  role: string
  daily_usage: {
    input_tokens: number
    output_tokens: number
    requests: number
  }
  monthly_usage: {
    input_tokens: number
    output_tokens: number
    requests: number
  }
  quota: MeteringQuota
  percentage_used: number
  last_active?: string
}

export interface MeteringQuotasResponse {
  default: MeteringQuota
  users: MeteringQuota[]
}

export interface MeteringQuotaUpdate {
  daily_input_limit?: number
  daily_output_limit?: number
  monthly_input_limit?: number
  monthly_output_limit?: number
}

// ============================================================================
// Registry Types (Agent Discovery)
// ============================================================================

export interface RegistryAgentCapability {
  id: string
  name: string
  description: string
  primary: boolean
}

export interface RegistryAgentTrigger {
  type: "keyword" | "pattern" | "event" | "context"
  value: string
  priority: number
  description?: string
}

export interface RegistryAgentExample {
  title: string
  input: string
  output: string
  tags: string[]
}

export type RegistryAgentCategory = "engineering" | "content" | "analysis" | "philosophy" | "system" | "custom"

export interface RegistryAgentMetadata {
  name: string
  displayName?: string
  shortDescription?: string
  longDescription?: string
  category: RegistryAgentCategory
  capabilities: RegistryAgentCapability[]
  triggers: RegistryAgentTrigger[]
  examples: RegistryAgentExample[]
  tags: string[]
  author?: string
  version: string
  builtin: boolean
  icon?: string
  recommended: boolean
}

export interface RegistrySearchResult {
  agent: RegistryAgentMetadata
  score: number
  matches: Array<{
    key: string
    value: string
    indices: ReadonlyArray<[number, number]>
  }>
}

export interface RegistryCategory {
  name: string
  count: number
  label: string
}

export interface AgentRecommendation {
  recommended: RegistryAgentMetadata | null
  alternates: RegistryAgentMetadata[]
}

// ============================================================================
// Chat Types (ZeroBot Bridge)
// ============================================================================

export interface ChatInput {
  message: string
  conversation_id?: string
  agent?: string
  user_id: string
  channel: string
}

export interface ChatResponse {
  message: string
  conversation_id: string
  agent: string
  usage?: {
    input_tokens: number
    output_tokens: number
    total_tokens: number
  }
}

// ============================================================================
// Compare Types (A/B Testing)
// ============================================================================

export interface CompareModelResult {
  model: string
  provider: string
  model_id: string
  content: string
  tokens: {
    input: number
    output: number
    total: number
  }
  latency_ms: number
  error?: string
}

export interface CompareResponse {
  id: string
  results: CompareModelResult[]
  total_tokens: number
  total_latency_ms: number
}

export interface CompareHistoryItem {
  id: string
  timestamp: number
  prompt: string
  models: string[]
  total_tokens: number
  total_latency_ms: number
  votes: Record<string, number>
  vote_count: number
  avg_rating: Record<string, number>
}

export interface CompareHistoryEntry extends CompareHistoryItem {
  system?: string
  results: CompareModelResult[]
  ratings: Record<string, number[]>
}

export interface CompareHistoryListResponse {
  items: CompareHistoryItem[]
  total: number
  limit: number
  offset: number
}

export interface CompareVoteRequest {
  model: string
  rating?: number
  user_id?: string
}

export interface CompareVoteResponse {
  id: string
  votes: Record<string, number>
  avg_rating: Record<string, number>
  message: string
}

export interface CompareInput {
  models: string[]
  prompt: string
  system?: string
  max_tokens?: number
  temperature?: number
}

export interface CompareModelInfo {
  id: string
  provider: string
  name: string
  capabilities: {
    reasoning: boolean
    toolcall: boolean
  }
}

// ============================================================================
// Executive Dashboard Types
// ============================================================================

export interface ExecutiveTrendDataPoint {
  date: string
  input_tokens: number
  output_tokens: number
  total_tokens: number
  requests: number
  cost_usd: number
}

export interface ExecutiveTrendsResponse {
  period: string
  days: number
  trends: ExecutiveTrendDataPoint[]
  totals: {
    input_tokens: number
    output_tokens: number
    total_tokens: number
    requests: number
    cost_usd: number
  }
}

export interface ExecutiveTeamUsage {
  team_id: string
  team_name: string
  member_count: number
  tokens_used: number
  requests: number
  percentage: number
  top_users: Array<{
    user_id: string
    name: string
    tokens: number
  }>
}

export interface ExecutiveTeamsResponse {
  teams: ExecutiveTeamUsage[]
  totals: {
    tokens: number
    requests: number
    members: number
  }
  team_count: number
}

export interface ExecutiveProjectActivity {
  project_id: string
  project_name: string
  commits_today: number
  commits_week: number
  active_contributors: number
  last_commit?: string
  ai_sessions: number
}

export interface ExecutiveActivityResponse {
  projects: ExecutiveProjectActivity[]
  totals: {
    commits_today: number
    commits_week: number
    ai_sessions: number
  }
  project_count: number
}

export interface ExecutiveSummary {
  period: string
  total_cost_usd: number
  cost_change_percent: number
  total_tokens: number
  total_requests: number
  active_users: number
  active_projects: number
  top_models: Array<{
    model: string
    usage_percent: number
    cost_usd: number
  }>
  alerts: Array<{
    type: "warning" | "critical" | "info"
    message: string
    metric?: string
    value?: number
    threshold?: number
  }>
}

// ============================================================================
// Knowledge Base Types (RAG)
// ============================================================================

export interface KnowledgeDocument {
  id: string
  filename: string
  chunk_count: number
  created_at: string
  size_bytes: number
  metadata?: Record<string, string>
}

export interface KnowledgeSearchResult {
  content: string
  score: number
  document_id: string
  chunk_index: number
  filename: string
  heading?: string
}

export interface KnowledgeUploadRequest {
  content: string
  filename: string
  mime_type?: string
  metadata?: Record<string, string>
}

export interface KnowledgeUploadResponse {
  id: string
  filename: string
  chunk_count: number
  size_bytes: number
  has_embeddings: boolean
  duplicate?: boolean
  message?: string
}

export interface KnowledgeSearchRequest {
  query: string
  limit?: number
  min_score?: number
  document_id?: string
}

export interface KnowledgeSearchResponse {
  results: KnowledgeSearchResult[]
  total: number
  query: string
  search_mode: "hybrid" | "keyword"
}

export interface KnowledgeHealthResponse {
  status: "healthy" | "degraded"
  document_count: number
  chunk_count: number
  embedding_count: number
  embedding_enabled: boolean
  search_mode: "hybrid" | "keyword"
  db_path: string
  error?: string
}

// ============================================================================
// Budget & Cost Control Types
// ============================================================================

export type BudgetPeriod = "daily" | "weekly" | "monthly"
export type BudgetAlertSeverity = "info" | "warning" | "critical"

export interface BudgetThreshold {
  /** Percentage of budget at which to trigger alert (0-100) */
  percentage: number
  /** Severity level for this threshold */
  severity: BudgetAlertSeverity
  /** Whether to send notification */
  notify: boolean
  /** Notification channels (email, slack, etc.) */
  channels?: string[]
}

export interface BudgetConfig {
  id: string
  name: string
  /** Budget period */
  period: BudgetPeriod
  /** Budget amount in USD */
  budget_usd: number
  /** Current spend in USD */
  current_spend_usd: number
  /** Alert thresholds */
  thresholds: BudgetThreshold[]
  /** Team/department ID (optional, for department-level budgets) */
  team_id?: string
  /** Whether this budget is active */
  enabled: boolean
  /** Reset day for monthly budgets (1-28) */
  reset_day?: number
  /** Created timestamp */
  created_at: string
  /** Updated timestamp */
  updated_at: string
}

export interface BudgetAlert {
  id: string
  budget_id: string
  budget_name: string
  severity: BudgetAlertSeverity
  message: string
  threshold_percentage: number
  current_percentage: number
  current_spend_usd: number
  budget_usd: number
  triggered_at: string
  acknowledged: boolean
  acknowledged_by?: string
  acknowledged_at?: string
}

export interface BudgetSummary {
  total_budget_usd: number
  total_spend_usd: number
  percentage_used: number
  period: BudgetPeriod
  active_alerts: number
  budgets: Array<{
    id: string
    name: string
    budget_usd: number
    spend_usd: number
    percentage: number
    status: "ok" | "warning" | "critical"
  }>
}

export interface BudgetCreateInput {
  name: string
  period: BudgetPeriod
  budget_usd: number
  thresholds: BudgetThreshold[]
  team_id?: string
  enabled?: boolean
  reset_day?: number
}

export interface BudgetUpdateInput {
  name?: string
  budget_usd?: number
  thresholds?: BudgetThreshold[]
  enabled?: boolean
  reset_day?: number
}

// ============================================================================
// DLP (Data Leakage Prevention) Types
// ============================================================================

export type DlpRuleType = "regex" | "keyword" | "pattern"
export type DlpAction = "block" | "redact" | "warn" | "log"

export interface DlpRule {
  id: string
  name: string
  description?: string
  type: DlpRuleType
  pattern: string
  action: DlpAction
  enabled: boolean
  /** Categories this rule belongs to */
  categories: string[]
  /** Replacement text for redaction */
  replacement?: string
  /** Priority (lower = higher priority) */
  priority: number
  /** Match count */
  match_count: number
  created_at: string
  updated_at: string
}

export interface DlpWhitelistEntry {
  id: string
  pattern: string
  description?: string
  created_at: string
}

export interface DlpIncident {
  id: string
  rule_id: string
  rule_name: string
  action_taken: DlpAction
  content_preview: string
  user_id?: string
  request_id?: string
  triggered_at: string
}

export interface DlpConfig {
  enabled: boolean
  default_action: DlpAction
  log_incidents: boolean
  notify_on_block: boolean
  notification_channels: string[]
}

export interface DlpSummary {
  total_rules: number
  active_rules: number
  incidents_24h: number
  incidents_7d: number
  top_triggered_rules: Array<{
    rule_id: string
    rule_name: string
    count: number
  }>
}
