// Local type definitions to replace @codecoder-ai/sdk
// These types are based on the SDK but defined locally

import type { MessageV2 } from "@/session/message-v2"
import type { Session } from "@/session"
import type { Permission } from "@/permission"
import type { Provider } from "@/provider/provider"
import type { Agent } from "@/agent/agent"
import type { MCP } from "@/mcp"
import type { LSP } from "@/lsp"
import type { Format } from "@/format"
import type { Todo } from "@/session/todo"
import type { SessionStatus } from "@/session/status"
import type { Question } from "@/question"
import type { Command } from "@/command"

// Re-export core types from internal modules
export type { Session } from "@/session"
export type { Permission } from "@/permission"

// Message types
export type Message = MessageV2.Info
export type SessionMessage = MessageV2.Info
export type SessionMessageWithParts = MessageV2.WithParts

// Part types
export type Part = MessageV2.Part
export type TextPart = MessageV2.TextPart
export type ToolPart = MessageV2.ToolPart
export type ReasoningPart = MessageV2.ReasoningPart
export type AgentPart = { type: "agent"; name: string; source?: { value: string; start: number; end: number } }
export type FilePart = MessageV2.FilePart

// Message types
export type UserMessage = { role: "user" } & MessageV2.Info
export type AssistantMessage = { role: "assistant" } & MessageV2.Info
export type SystemMessage = { role: "system" } & MessageV2.Info

// Re-export namespaces for their types
export type { Agent } from "@/agent/agent"
export type { Command } from "@/command"
export type { MCP } from "@/mcp"
export type { LSP } from "@/lsp"
export type { Format } from "@/format"
export type { Todo } from "@/session/todo"
export type { SessionStatus } from "@/session/status"
export type { Question } from "@/question"
export type { Provider } from "@/provider/provider"

// Type aliases for commonly used namespace types (for SDK compatibility)
export type AgentInfo = Agent.Info
export type CommandInfo = Command.Info
export type QuestionRequest = Question.Request
export type QuestionAnswer = Question.Answer
export type QuestionInfo = Question.Info
export type LspStatus = LSP.Status
export type McpStatus = MCP.Status
export type McpResource = MCP.Resource
export type FormatterStatus = Format.Status
export type TodoInfo = Todo.Info
export type SessionStatusInfo = SessionStatus.Info
export type ProviderInfo = Provider.Info

// Config type (simplified - actual type is more complex)
export type Config = Record<string, any>

// Provider list response
export type ProviderListResponse = {
  all: Provider.Info[]
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
  metadata: Record<string, any>
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
  [key: string]: string
}

// Event types - these are Bus events
export type Event = {
  type: string
  properties: any
}

// Client interface (not used anymore but kept for compatibility)
export interface CodecoderClient {
  url: string
  client: any
  event: any
}

export interface SessionMessageResponse {
  data: SessionMessage
}

export interface SessionListResponse {
  data: Session.Info[]
}

export interface SessionResponse {
  data: Session.Info
}
