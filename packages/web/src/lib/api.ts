/**
 * API Client for CodeCoder Web
 * Provides methods to interact with the CodeCoder HTTP API
 */

import type {
  ApiResponse,
  SessionInfo,
  SessionCreateInput,
  SessionListQuery,
  MessageWithParts,
  MessageSendInput,
  SessionMessagesQuery,
  PermissionInfo,
  PermissionRespondInput,
  ConfigData,
  FileInfo,
  HealthResponse,
  ApiDiscoveryResponse,
  EventChannelsResponse,
} from "./types"

// ============================================================================
// API Client Configuration
// ============================================================================

export interface ApiClientConfig {
  baseUrl?: string
  apiKey?: string
  timeout?: number
  headers?: Record<string, string>
}

const DEFAULT_CONFIG: Required<Omit<ApiClientConfig, "apiKey" | "headers">> = {
  baseUrl: "/api",
  timeout: 30000,
}

// ============================================================================
// API Error Classes
// ============================================================================

export class ApiError extends Error {
  constructor(
    public readonly statusCode: number,
    public readonly code: string,
    message: string,
    public readonly details?: unknown,
  ) {
    super(message)
    this.name = "ApiError"
  }
}

export class NetworkError extends Error {
  constructor(message: string, public readonly cause?: Error) {
    super(message)
    this.name = "NetworkError"
  }
}

export class TimeoutError extends Error {
  constructor(timeout: number) {
    super(`Request timed out after ${timeout}ms`)
    this.name = "TimeoutError"
  }
}

// ============================================================================
// API Client Class
// ============================================================================

export class ApiClient {
  private config: Required<ApiClientConfig>

  constructor(config: ApiClientConfig = {}) {
    this.config = {
      ...DEFAULT_CONFIG,
      apiKey: config.apiKey ?? "",
      headers: {
        "Content-Type": "application/json",
        ...config.headers,
      },
    }
  }

  // ========================================================================
  // Private Methods
  // ========================================================================

  private get baseUrl(): string {
    return this.config.baseUrl.replace(/\/+$/, "")
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    options: RequestInit = {},
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`
    const headers: HeadersInit = { ...this.config.headers }

    if (this.config.apiKey) {
      headers["Authorization"] = `Bearer ${this.config.apiKey}`
      headers["X-API-Key"] = this.config.apiKey
    }

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout)

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
        ...options,
      })

      clearTimeout(timeoutId)

      if (!response.ok) {
        let errorMessage = `HTTP ${response.status}: ${response.statusText}`
        let errorDetails: unknown

        try {
          const errorData = (await response.json()) as ApiResponse
          errorMessage = errorData.error ?? errorMessage
          errorDetails = errorData
        } catch {
          // Ignore JSON parse errors
        }

        throw new ApiError(response.status, `HTTP_${response.status}`, errorMessage, errorDetails)
      }

      // Handle 204 No Content
      if (response.status === 204) {
        return undefined as T
      }

      const data = (await response.json()) as ApiResponse<T>
      return data.data as T
    } catch (error) {
      clearTimeout(timeoutId)

      if (error instanceof ApiError) {
        throw error
      }

      if (error instanceof Error && error.name === "AbortError") {
        throw new TimeoutError(this.config.timeout)
      }

      throw new NetworkError(
        error instanceof Error ? error.message : "Unknown network error",
        error instanceof Error ? error : undefined,
      )
    }
  }

  private get<T>(path: string, options?: RequestInit): Promise<T> {
    return this.request<T>("GET", path, undefined, options)
  }

  private post<T>(path: string, body?: unknown, options?: RequestInit): Promise<T> {
    return this.request<T>("POST", path, body, options)
  }

  private put<T>(path: string, body?: unknown, options?: RequestInit): Promise<T> {
    return this.request<T>("PUT", path, body, options)
  }

  private delete<T>(path: string, options?: RequestInit): Promise<T> {
    return this.request<T>("DELETE", path, undefined, options)
  }

  // ========================================================================
  // Health Check
  // ========================================================================

  /**
   * Check API health status
   */
  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>("/health")
  }

  /**
   * Discover available API endpoints
   */
  async discover(): Promise<ApiDiscoveryResponse> {
    return this.get<ApiDiscoveryResponse>("/")
  }

  // ========================================================================
  // Sessions
  // ========================================================================

  /**
   * List all sessions with optional filtering
   */
  async listSessions(query?: SessionListQuery): Promise<SessionInfo[]> {
    const params = new URLSearchParams()
    if (query?.limit) params.append("limit", String(query.limit))
    if (query?.search) params.append("search", query.search)
    const queryString = params.toString()
    const path = queryString ? `/sessions?${queryString}` : "/sessions"
    return this.get<SessionInfo[]>(path)
  }

  /**
   * Get a specific session by ID
   */
  async getSession(id: string): Promise<SessionInfo> {
    return this.get<SessionInfo>(`/sessions/${id}`)
  }

  /**
   * Create a new session
   */
  async createSession(input: SessionCreateInput): Promise<SessionInfo> {
    return this.post<SessionInfo>("/sessions", input)
  }

  /**
   * Delete a session
   */
  async deleteSession(id: string): Promise<void> {
    return this.delete<void>(`/sessions/${id}`)
  }

  /**
   * Get child sessions of a session
   */
  async getSessionChildren(id: string): Promise<SessionInfo[]> {
    return this.get<SessionInfo[]>(`/sessions/${id}/children`)
  }

  /**
   * Fork a session at a specific message
   */
  async forkSession(id: string, input?: { messageID?: string }): Promise<SessionInfo> {
    return this.post<SessionInfo>(`/sessions/${id}/fork`, input)
  }

  // ========================================================================
  // Messages
  // ========================================================================

  /**
   * Get messages for a session
   */
  async getSessionMessages(id: string, query?: SessionMessagesQuery): Promise<MessageWithParts[]> {
    const params = new URLSearchParams()
    if (query?.limit) params.append("limit", String(query.limit))
    const queryString = params.toString()
    const path = queryString ? `/sessions/${id}/messages?${queryString}` : `/sessions/${id}/messages`
    return this.get<MessageWithParts[]>(path)
  }

  /**
   * Send a message to a session
   */
  async sendMessage(id: string, input: MessageSendInput): Promise<{ messageID: string }> {
    return this.post<{ messageID: string }>(`/sessions/${id}/messages`, input)
  }

  // ========================================================================
  // Config
  // ========================================================================

  /**
   * Get current configuration
   */
  async getConfig(): Promise<ConfigData> {
    return this.get<ConfigData>("/config")
  }

  /**
   * Update configuration
   */
  async updateConfig(updates: Partial<ConfigData>): Promise<ConfigData> {
    return this.put<ConfigData>("/config", updates)
  }

  // ========================================================================
  // Permissions
  // ========================================================================

  /**
   * List all pending permissions
   */
  async listPermissions(): Promise<PermissionInfo[]> {
    return this.get<PermissionInfo[]>("/permissions")
  }

  /**
   * Respond to a permission request (legacy)
   */
  async respondPermission(id: string, input: PermissionRespondInput): Promise<void> {
    return this.post<void>(`/permissions/${id}/respond`, input)
  }

  /**
   * Reply to a permission request (next-generation)
   */
  async replyPermission(id: string, input: PermissionRespondInput): Promise<void> {
    return this.post<void>(`/permissions/${id}/reply`, input)
  }

  // ========================================================================
  // Files
  // ========================================================================

  /**
   * Search for files
   */
  async findFiles(query?: string): Promise<FileInfo[]> {
    const params = new URLSearchParams()
    if (query) params.append("q", query)
    const queryString = params.toString()
    const path = queryString ? `/files?${queryString}` : "/files"
    return this.get<FileInfo[]>(path)
  }

  /**
   * Search files using cache
   */
  async findFilesCache(query?: string): Promise<FileInfo[]> {
    const params = new URLSearchParams()
    if (query) params.append("q", query)
    const queryString = params.toString()
    const path = queryString ? `/files/cache?${queryString}` : "/files/cache"
    return this.get<FileInfo[]>(path)
  }

  // ========================================================================
  // Events
  // ========================================================================

  /**
   * List available event channels
   */
  async listEventChannels(): Promise<string[]> {
    const response = await this.get<EventChannelsResponse>("/events/channels")
    return response.data.channels
  }
}

// ============================================================================
// Default Client Instance
// ============================================================================

let defaultClient: ApiClient | null = null

export function setDefaultClient(config: ApiClientConfig): void {
  defaultClient = new ApiClient(config)
}

export function getClient(): ApiClient {
  if (!defaultClient) {
    defaultClient = new ApiClient()
  }
  return defaultClient
}

// ============================================================================
// Convenience Functions (using default client)
// ============================================================================

export const api = {
  health: () => getClient().health(),
  discover: () => getClient().discover(),
  listSessions: (query?: SessionListQuery) => getClient().listSessions(query),
  getSession: (id: string) => getClient().getSession(id),
  createSession: (input: SessionCreateInput) => getClient().createSession(input),
  deleteSession: (id: string) => getClient().deleteSession(id),
  getSessionChildren: (id: string) => getClient().getSessionChildren(id),
  forkSession: (id: string, input?: { messageID?: string }) => getClient().forkSession(id, input),
  getSessionMessages: (id: string, query?: SessionMessagesQuery) => getClient().getSessionMessages(id, query),
  sendMessage: (id: string, input: MessageSendInput) => getClient().sendMessage(id, input),
  getConfig: () => getClient().getConfig(),
  updateConfig: (updates: Partial<ConfigData>) => getClient().updateConfig(updates),
  listPermissions: () => getClient().listPermissions(),
  respondPermission: (id: string, input: PermissionRespondInput) => getClient().respondPermission(id, input),
  replyPermission: (id: string, input: PermissionRespondInput) => getClient().replyPermission(id, input),
  findFiles: (query?: string) => getClient().findFiles(query),
  findFilesCache: (query?: string) => getClient().findFilesCache(query),
  listEventChannels: () => getClient().listEventChannels(),
}
