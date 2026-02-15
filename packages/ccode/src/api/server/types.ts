/**
 * HTTP API Server Type Definitions
 */

import type { Session } from "../../session"
import type { Permission } from "../../permission"

// ============================================================================
// Server Configuration
// ============================================================================

export interface ServerConfig {
  port: number
  hostname: string
  cors?: string[]
  apiKey?: string
}

// ============================================================================
// HTTP Request/Response Types
// ============================================================================

export interface HttpRequest {
  method: string
  url: URL
  headers: Headers
  body?: ReadableStream | null
}

export interface HttpResponse {
  status: number
  headers?: HeadersInit
  body?: BodyInit | ReadableStream<Uint8Array> | null
}

export type RouteHandler = (req: HttpRequest, params: RouteParams) => Promise<HttpResponse>

// ============================================================================
// Route Parameters
// ============================================================================

export type RouteParams = Record<string, string>

// ============================================================================
// Route Definition
// ============================================================================

export interface Route {
  method: HttpMethod
  pattern: string
  handler: RouteHandler
}

export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS"

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
  sessionID: string
  agent?: string
  model?: string
  variant?: string
  parts: Array<{ type: string; value?: string }>
}

// ============================================================================
// Config Types
// ============================================================================

export type ConfigUpdateInput = Record<string, unknown>

// ============================================================================
// Permission Types
// ============================================================================

export interface PermissionRespondInput {
  reply: "once" | "always" | "reject"
  message?: string
}

// ============================================================================
// File Search Types
// ============================================================================

export interface FileSearchQuery {
  q?: string
}

// ============================================================================
// Event Stream Types
// ============================================================================

export interface SSEEvent {
  event?: string
  data: string
  id?: string
  retry?: number
}

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
