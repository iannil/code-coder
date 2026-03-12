/**
 * Local type definitions for CodeCoder TUI
 *
 * This file re-exports types from the SDK for use throughout the codebase.
 * All types are now sourced from the SDK, which mirrors the Rust daemon API.
 *
 * @module types
 */

// ══════════════════════════════════════════════════════════════════════════════
// Re-export core types from SDK
// ══════════════════════════════════════════════════════════════════════════════

export type {
  // Agent types
  AgentInfo,
  AgentListResponse,
  AgentDetailResponse,
  DispatchAgentRequest,
  DispatchAgentResponse,

  // Session types
  SessionInfo,
  SessionInfoExtended,
  SessionTime,
  SessionSummary,
  SessionListResponse,
  RevertInfo,
  PermissionRuleset,
  FileDiff,

  // Message types
  MessageInfo,
  UserMessageInfo,
  AssistantMessageInfo,
  MessageWithParts,
  MessagePart,
  TextPart,
  ReasoningPart,
  ToolPart,
  ToolState,
  FilePart,
  FilePartSource,
  StepStartPart,
  StepFinishPart,
  APIErrorInfo,
  CLOSEScore,

  // Provider types
  ProviderInfo,
  ProviderListResponseExtended,
  ModelInfo,
  ModelCapabilities,
  ModelCost,
  Mode,

  // Token usage
  TokenUsage,

  // Gear types
  GearStatus,
  GearStatusResponse,
  GearPreset,
  DialValues,

  // Observer types
  ObserverStatus,
  ObserverStatusResponse,
  WatcherType,
  WatcherStatus,
  Observation,
  OperatingMode,
  CLOSEDimension,
  CLOSEEvaluation,
  EscalationPriority,
  EscalationStatus,
  EscalationContext,
  Escalation,
  HumanDecision,
  ConsensusSnapshot,
  ModeDecision,
  ModeControllerStats,

  // Autonomous types
  AutonomousState,
  AutonomyLevel,
  SessionMetrics,
  QualityScoreBreakdown,
  CrazinessScoreBreakdown,
  ResourceUsage,
  SafetyStatus,
  AutonomousEventType,
  AutonomousEventData,

  // WebSocket types
  WsClientMessage,
  WsServerMessage,
  AgentRequest,
  AgentCancel,
  AgentStart,
  AgentText,
  AgentReasoning,
  AgentToolCall,
  AgentToolResult,
  AgentComplete,
  AgentError,
  AgentCancelled,
  AgentStreamEvent,
  AgentEventHandler,
  ConfirmationRequest,
  ConfirmationResponse,
  SessionEvent,
  ObserverEvent,
} from "@/sdk/types"

// Re-export utility functions from SDK
export { parseModel, isDefaultTitle, parseModeCapability, getMode, validateCapability, listModes, MODES, DEFAULT_MODE } from "@/sdk/types"
export { GEAR_INFO, GEAR_PRESETS, operatingModeToGear, gearToOperatingMode } from "@/sdk/types"

// Re-export session adapters
export type { SessionInfoLegacy } from "@/sdk/adapter"
export { adaptSessionInfo, adaptSessionList } from "@/sdk/adapter"

// ══════════════════════════════════════════════════════════════════════════════
// Types from modules that are NOT being deleted
// ══════════════════════════════════════════════════════════════════════════════

import type { MCP } from "@/mcp"
import type { LSP } from "@/lsp"
import type { Format } from "@/util/format"

export type { MCP } from "@/mcp"
export type { LSP } from "@/lsp"
export type { Format } from "@/util/format"

// Type aliases for commonly used namespace types
export type LspStatus = LSP.Status
export type McpStatus = MCP.Status
export type McpResource = MCP.Resource
export type FormatterStatus = Format.Status

// ══════════════════════════════════════════════════════════════════════════════
// Backward compatibility type aliases
// These map old type names to SDK equivalents
// ══════════════════════════════════════════════════════════════════════════════

// Session compatibility
import type {
  SessionInfo as SdkSessionInfo,
  SessionInfoExtended as SdkSessionInfoExtended,
  MessageInfo as SdkMessageInfo,
  MessageWithParts as SdkMessageWithParts,
  MessagePart as SdkMessagePart,
  TextPart as SdkTextPart,
  ToolPart as SdkToolPart,
  ReasoningPart as SdkReasoningPart,
  FilePart as SdkFilePart,
  UserMessageInfo as SdkUserMessageInfo,
  AssistantMessageInfo as SdkAssistantMessageInfo,
  AgentInfo as SdkAgentInfo,
  ProviderInfo as SdkProviderInfo,
  ProviderListResponseExtended as SdkProviderListResponse,
} from "@/sdk/types"

/** @deprecated Use SessionInfo from SDK instead */
export type Message = SdkMessageInfo
/** @deprecated Use SessionInfo from SDK instead */
export type SessionMessage = SdkMessageInfo
/** @deprecated Use MessageWithParts from SDK instead */
export type SessionMessageWithParts = SdkMessageWithParts
/** @deprecated Use MessagePart from SDK instead */
export type Part = SdkMessagePart
/** @deprecated Use UserMessageInfo from SDK instead */
export type UserMessage = SdkUserMessageInfo
/** @deprecated Use AssistantMessageInfo from SDK instead */
export type AssistantMessage = SdkAssistantMessageInfo
/** @deprecated Use TextPart & role: "system" instead */
export type SystemMessage = { role: "system"; content: string }
/** @deprecated Use AgentPart from SDK types instead */
export type AgentPart = { type: "agent"; name: string; source?: { value: string; start: number; end: number } }

// TUI-specific config
export interface TuiConfig {
  diff_style?: "unified" | "split" | "minimal" | "stacked"
  scroll_acceleration?: { enabled?: boolean }
  scroll_speed?: number
  message?: string
}

// Experimental features config
export interface ExperimentalConfig {
  disable_paste_summary?: boolean
}

// Plugin config entry
export interface PluginConfigEntry {
  name: string
  status: "connected" | "failed" | "loading"
  error?: string
}

// Config type with proper structure
export interface Config {
  tui?: TuiConfig
  keybinds?: KeybindsConfig
  plugin?: string[]
  experimental?: ExperimentalConfig
  theme?: string
  model?: string
  lsp?: boolean
  [key: string]: unknown
}

// Provider list response (backward compat)
export type ProviderListResponse = {
  all: SdkProviderInfo[]
  default: Record<string, string>
  connected: string[]
}

// Provider auth method types
export type ProviderAuthMethod = ProviderAuthEntry

export interface ProviderAuthEntry {
  providerID: string
  type: "oauth" | "token"
  username?: string
}

export interface ProviderAuthAuthorization {
  providerID: string
  type: "oauth" | "token"
  url?: string
  code?: string
  instructions?: string
}

export interface ProviderAuthStatus {
  [providerID: string]: ProviderAuthEntry
}

// VCS info type
export interface VcsInfo {
  branch: string
}

// Permission types
export interface PermissionRequest {
  id: string
  permission: string
  patterns: string[]
  always: string[]
  sessionID: string
  messageID: string
  callID?: string
  message: string
  metadata: Record<string, unknown>
  time: { created: number }
}

// Path types
export interface Path {
  home: string
  state: string
  config: string
  worktree: string
  directory: string
}

// Keybind types
export interface KeybindsConfig {
  [key: string]: string | undefined
  leader?: string
}

// ══════════════════════════════════════════════════════════════════════════════
// Types that were previously imported from deprecated modules
// These are now defined locally to maintain backward compatibility
// ══════════════════════════════════════════════════════════════════════════════

// Session status info (from @/session/status)
export interface SessionStatusInfo {
  type: "idle" | "busy" | "error" | "retry"
  message?: string
  next?: number
  attempt?: number
  error?: {
    name: string
    message?: string
  }
}

// Todo info (from @/session/todo)
export interface TodoInfo {
  id: string
  content: string
  status: "pending" | "in_progress" | "completed"
  priority?: "low" | "medium" | "high"
  createdAt: number
  updatedAt?: number
}

// Command info (from @/agent/command)
export interface CommandInfo {
  name: string
  description?: string
  aliases?: string[]
  mcp?: boolean
  hidden?: boolean
  args?: Array<{
    name: string
    type: string
    required?: boolean
    description?: string
  }>
}

// Question types (from @/agent/question)
export interface QuestionRequest {
  id: string
  sessionID: string
  messageID: string
  questions: Array<{
    question: string
    header: string
    options: Array<{
      label: string
      description?: string
      markdown?: string
    }>
    multiSelect: boolean
    /** @deprecated Use multiSelect instead */
    multiple?: boolean
    /** Whether custom text input is allowed (default: true) */
    custom?: boolean
  }>
  time: { created: number }
}

/**
 * Answer for a single question - array of selected option labels
 */
export type QuestionAnswer = string[]

export interface QuestionInfo {
  request: QuestionRequest
  answers?: QuestionAnswer[]
  status: "pending" | "answered" | "rejected"
}

// Snapshot types (from @/session/snapshot)
export interface SnapshotDiff {
  file: string
  before: string
  after: string
  additions: number
  deletions: number
}

/** @deprecated Use FileDiff from SDK instead */
export interface Snapshot {
  id: string
  sessionID: string
  messageID: string
  time: { created: number }
  files: SnapshotDiff[]
}

// Event types - these are Bus events
export type Event = {
  type: string
  properties: unknown
}

// Client interface (not used anymore but kept for compatibility)
export interface CodecoderClient {
  url: string
  client: unknown
  event: unknown
}

export interface SessionMessageResponse {
  data: SdkMessageInfo
}

/** @deprecated Use SessionListResponse from SDK instead */
export interface SessionListResponseLegacy {
  data: SdkSessionInfoExtended[]
}

export interface SessionResponse {
  data: SdkSessionInfoExtended
}

/**
 * Session namespace for backward compatibility
 * @deprecated Use SessionInfoExtended directly
 */
export namespace Session {
  export type Info = SdkSessionInfoExtended
}

/**
 * Snapshot namespace for backward compatibility
 * @deprecated Use FileDiff directly
 */
export namespace Snapshot {
  export type FileDiff = import("@/sdk/types").FileDiff
}

// ══════════════════════════════════════════════════════════════════════════════
// Session Event Type Constants
// These replace Session.Event.*.type from @/session for TUI event handling
// ══════════════════════════════════════════════════════════════════════════════

export const SessionEventTypes = {
  Created: "session.created",
  Updated: "session.updated",
  Deleted: "session.deleted",
  Diff: "session.diff",
  Error: "session.error",
} as const

// ══════════════════════════════════════════════════════════════════════════════
// Message Event Type Constants
// These replace MessageV2.Event.*.type from @/session/message-v2 for TUI event handling
// ══════════════════════════════════════════════════════════════════════════════

export const MessageEventTypes = {
  Updated: "message.updated",
  Removed: "message.removed",
  PartUpdated: "message.part.updated",
  PartRemoved: "message.part.removed",
} as const

// ══════════════════════════════════════════════════════════════════════════════
// Bus Event Definitions
// Lightweight event definitions that don't pull in business logic
// ══════════════════════════════════════════════════════════════════════════════

import { BusEvent } from "@/bus/bus-event"
import z from "zod"

/**
 * Message events for TUI - mirrors MessageV2.Event from @/session/message-v2
 * These are standalone definitions that use SDK types instead of importing from session/
 * Uses z.any() for flexibility since the actual data comes from SDK and may vary
 */
export const MessageEvents = {
  Updated: BusEvent.define(
    MessageEventTypes.Updated,
    z.object({
      info: z.any(),
    }),
  ),
  Removed: BusEvent.define(
    MessageEventTypes.Removed,
    z.object({
      sessionID: z.string(),
      messageID: z.string(),
    }),
  ),
  PartUpdated: BusEvent.define(
    MessageEventTypes.PartUpdated,
    z.object({
      part: z.any(),
      delta: z.string().optional(),
    }),
  ),
  PartRemoved: BusEvent.define(
    MessageEventTypes.PartRemoved,
    z.object({
      sessionID: z.string(),
      messageID: z.string(),
      partID: z.string(),
    }),
  ),
}

/**
 * Question events for TUI - mirrors Question.Event from @/agent/question
 */
export const QuestionEventTypes = {
  Asked: "question.asked",
  Replied: "question.replied",
  Rejected: "question.rejected",
} as const

export const QuestionEvents = {
  Asked: BusEvent.define(
    QuestionEventTypes.Asked,
    z.object({
      id: z.string(),
      sessionID: z.string(),
      questions: z.array(z.object({
        question: z.string(),
        header: z.string(),
        options: z.array(z.object({
          label: z.string(),
          description: z.string().optional(),
        })),
        multiple: z.boolean().optional(),
        custom: z.boolean().optional(),
      })),
      tool: z.object({
        messageID: z.string(),
        callID: z.string(),
      }).optional(),
    }),
  ),
  Replied: BusEvent.define(
    QuestionEventTypes.Replied,
    z.object({
      sessionID: z.string(),
      requestID: z.string(),
      answers: z.array(z.array(z.string())),
    }),
  ),
  Rejected: BusEvent.define(
    QuestionEventTypes.Rejected,
    z.object({
      sessionID: z.string(),
      requestID: z.string(),
    }),
  ),
}

/**
 * Command events for TUI - mirrors Command.Event from @/agent/command
 */
export const CommandEventTypes = {
  Executed: "command.executed",
} as const

export const CommandEvents = {
  Executed: BusEvent.define(
    CommandEventTypes.Executed,
    z.object({
      name: z.string(),
      sessionID: z.string(),
      arguments: z.string(),
      messageID: z.string(),
    }),
  ),
}
