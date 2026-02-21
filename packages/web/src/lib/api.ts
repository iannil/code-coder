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
  ProviderInfo,
  ProviderListResponse,
  ProviderModel,
  ProviderAuthMethod,
  McpStatus,
  McpTool,
  McpResource,
  McpAuthStatus,
  // P2 Types
  DocumentMetadata,
  DocumentChapter,
  DocumentEntity,
  DocumentVolume,
  DocumentStats,
  DailyEntry,
  MemorySection,
  MemorySummary,
  ConsolidationStats,
  HookEntry,
  HookSettings,
  HookLocation,
  HookActionTypeInfo,
  LspStatus,
  LspFileDiagnostics,
  LspConfig,
  LspSymbol,
  LspDocumentSymbol,
  LspLocation,
  // Task Types
  TaskInfo,
  CreateTaskInput,
  InteractTaskInput,
  // Channel Types
  ChannelStatus,
  // Project Types
  ProjectInfo,
  ProjectCreateInput,
  DirectoryListResponse,
  // Credential Types
  CredentialSummary,
  CredentialEntry,
  CredentialCreateInput,
  // Metering Types
  MeteringUsageResponse,
  MeteringUserReport,
  MeteringQuotasResponse,
  MeteringQuota,
  MeteringQuotaUpdate,
  // Registry Types
  RegistryAgentMetadata,
  RegistrySearchResult,
  RegistryCategory,
  AgentRecommendation,
  // Chat Types
  ChatInput,
  ChatResponse,
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

  private patch<T>(path: string, body?: unknown, options?: RequestInit): Promise<T> {
    return this.request<T>("PATCH", path, body, options)
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
   * Update a session (rename)
   */
  async updateSession(id: string, input: { title: string }): Promise<SessionInfo> {
    return this.patch<SessionInfo>(`/sessions/${id}`, input)
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

  // ========================================================================
  // Providers
  // ========================================================================

  /**
   * List all available providers with connection status
   */
  async listProviders(): Promise<ProviderListResponse> {
    return this.get<ProviderListResponse>("/providers")
  }

  /**
   * List only connected providers
   */
  async listConnectedProviders(): Promise<ProviderInfo[]> {
    return this.get<ProviderInfo[]>("/providers/connected")
  }

  /**
   * Get authentication methods for all providers
   */
  async getProviderAuthMethods(): Promise<Record<string, ProviderAuthMethod[]>> {
    return this.get<Record<string, ProviderAuthMethod[]>>("/providers/auth")
  }

  /**
   * Get a specific provider by ID
   */
  async getProvider(providerId: string): Promise<ProviderInfo> {
    return this.get<ProviderInfo>(`/providers/${providerId}`)
  }

  /**
   * Get models for a specific provider
   */
  async getProviderModels(providerId: string): Promise<ProviderModel[]> {
    return this.get<ProviderModel[]>(`/providers/${providerId}/models`)
  }

  // ========================================================================
  // MCP (Model Context Protocol)
  // ========================================================================

  /**
   * Get status of all configured MCP servers
   */
  async getMcpStatus(): Promise<Record<string, McpStatus>> {
    return this.get<Record<string, McpStatus>>("/mcp/status")
  }

  /**
   * Get all available MCP tools from connected servers
   */
  async getMcpTools(): Promise<McpTool[]> {
    return this.get<McpTool[]>("/mcp/tools")
  }

  /**
   * Get all available MCP resources from connected servers
   */
  async getMcpResources(): Promise<Record<string, McpResource>> {
    return this.get<Record<string, McpResource>>("/mcp/resources")
  }

  /**
   * Connect (enable) an MCP server
   */
  async connectMcp(name: string): Promise<{ name: string; status: McpStatus }> {
    return this.post<{ name: string; status: McpStatus }>(`/mcp/${name}/connect`)
  }

  /**
   * Disconnect (disable) an MCP server
   */
  async disconnectMcp(name: string): Promise<{ name: string; status: McpStatus }> {
    return this.post<{ name: string; status: McpStatus }>(`/mcp/${name}/disconnect`)
  }

  /**
   * Toggle an MCP server's enabled state
   */
  async toggleMcp(name: string): Promise<{ name: string; status: McpStatus }> {
    return this.post<{ name: string; status: McpStatus }>(`/mcp/${name}/toggle`)
  }

  /**
   * Get authentication status for an MCP server
   */
  async getMcpAuthStatus(name: string): Promise<McpAuthStatus> {
    return this.get<McpAuthStatus>(`/mcp/${name}/auth-status`)
  }

  /**
   * Start OAuth authentication flow for an MCP server
   */
  async startMcpAuth(name: string): Promise<{ authorizationUrl: string }> {
    return this.post<{ authorizationUrl: string }>(`/mcp/${name}/auth/start`)
  }

  /**
   * Complete OAuth authentication with authorization code
   */
  async finishMcpAuth(name: string, code: string): Promise<{ name: string; status: McpStatus }> {
    return this.post<{ name: string; status: McpStatus }>(`/mcp/${name}/auth/finish`, { code })
  }

  // ========================================================================
  // Documents (P2)
  // ========================================================================

  /**
   * List all documents
   */
  async listDocuments(): Promise<DocumentMetadata[]> {
    return this.get<DocumentMetadata[]>("/documents")
  }

  /**
   * Get a specific document
   */
  async getDocument(id: string): Promise<DocumentMetadata> {
    return this.get<DocumentMetadata>(`/documents/${id}`)
  }

  /**
   * Create a new document
   */
  async createDocument(input: {
    title: string
    description?: string
    targetWords: number
  }): Promise<DocumentMetadata> {
    return this.post<DocumentMetadata>("/documents", input)
  }

  /**
   * Update a document
   */
  async updateDocument(id: string, input: Partial<DocumentMetadata>): Promise<DocumentMetadata> {
    return this.put<DocumentMetadata>(`/documents/${id}`, input)
  }

  /**
   * Delete a document
   */
  async deleteDocument(id: string): Promise<void> {
    return this.delete<void>(`/documents/${id}`)
  }

  /**
   * Get document statistics
   */
  async getDocumentStats(id: string): Promise<DocumentStats> {
    return this.get<DocumentStats>(`/documents/${id}/stats`)
  }

  /**
   * Export document
   */
  async exportDocument(id: string, format: "markdown" | "html" = "markdown"): Promise<string> {
    const response = await fetch(`${this.baseUrl}/documents/${id}/export?format=${format}`)
    return response.text()
  }

  /**
   * List chapters for a document
   */
  async listChapters(documentId: string): Promise<DocumentChapter[]> {
    return this.get<DocumentChapter[]>(`/documents/${documentId}/chapters`)
  }

  /**
   * Get a specific chapter
   */
  async getChapter(documentId: string, chapterId: string): Promise<DocumentChapter> {
    return this.get<DocumentChapter>(`/documents/${documentId}/chapters/${chapterId}`)
  }

  /**
   * Update a chapter
   */
  async updateChapter(
    documentId: string,
    chapterId: string,
    input: Partial<DocumentChapter>,
  ): Promise<DocumentChapter> {
    return this.put<DocumentChapter>(`/documents/${documentId}/chapters/${chapterId}`, input)
  }

  /**
   * List entities for a document
   */
  async listEntities(documentId: string): Promise<DocumentEntity[]> {
    return this.get<DocumentEntity[]>(`/documents/${documentId}/entities`)
  }

  /**
   * Create an entity
   */
  async createEntity(
    documentId: string,
    input: {
      name: string
      type: string
      description: string
      firstAppearedChapterID: string
      aliases?: string[]
      attributes?: Record<string, string>
    },
  ): Promise<DocumentEntity> {
    return this.post<DocumentEntity>(`/documents/${documentId}/entities`, input)
  }

  /**
   * Update an entity
   */
  async updateEntity(
    documentId: string,
    entityId: string,
    input: Partial<DocumentEntity>,
  ): Promise<DocumentEntity> {
    return this.put<DocumentEntity>(`/documents/${documentId}/entities/${entityId}`, input)
  }

  /**
   * Delete an entity
   */
  async deleteEntity(documentId: string, entityId: string): Promise<void> {
    return this.delete<void>(`/documents/${documentId}/entities/${entityId}`)
  }

  /**
   * List volumes for a document
   */
  async listVolumes(documentId: string): Promise<DocumentVolume[]> {
    return this.get<DocumentVolume[]>(`/documents/${documentId}/volumes`)
  }

  /**
   * Create a volume
   */
  async createVolume(
    documentId: string,
    input: {
      title: string
      description?: string
      startChapterID: string
      endChapterID: string
    },
  ): Promise<DocumentVolume> {
    return this.post<DocumentVolume>(`/documents/${documentId}/volumes`, input)
  }

  // ========================================================================
  // Memory (P2)
  // ========================================================================

  /**
   * List daily note dates
   */
  async listDailyDates(): Promise<string[]> {
    return this.get<string[]>("/memory/daily")
  }

  /**
   * Get daily notes for a specific date
   */
  async getDailyNotes(date: string): Promise<string[]> {
    return this.get<string[]>(`/memory/daily/${date}`)
  }

  /**
   * Append a note to today's daily notes
   */
  async appendDailyNote(input: {
    type: string
    content: string
    metadata?: Record<string, any>
  }): Promise<DailyEntry> {
    return this.post<DailyEntry>("/memory/daily", input)
  }

  /**
   * Get long-term memory content
   */
  async getLongTermMemory(): Promise<{ content: string }> {
    return this.get<{ content: string }>("/memory/long-term")
  }

  /**
   * Get memory sections
   */
  async getMemorySections(): Promise<MemorySection[]> {
    return this.get<MemorySection[]>("/memory/sections")
  }

  /**
   * Update a memory category
   */
  async updateMemoryCategory(category: string, content: string): Promise<void> {
    return this.put<void>(`/memory/category/${category}`, { content })
  }

  /**
   * Merge content into a memory category
   */
  async mergeToMemoryCategory(category: string, content: string): Promise<void> {
    return this.post<void>(`/memory/category/${category}/merge`, { content })
  }

  /**
   * Get consolidation statistics
   */
  async getConsolidationStats(): Promise<ConsolidationStats> {
    return this.get<ConsolidationStats>("/memory/consolidation/stats")
  }

  /**
   * Trigger memory consolidation
   */
  async triggerConsolidation(options?: {
    days?: number
    preserveOriginal?: boolean
    minImportance?: number
  }): Promise<any> {
    return this.post<any>("/memory/consolidation", options)
  }

  /**
   * Get memory summary
   */
  async getMemorySummary(): Promise<MemorySummary> {
    return this.get<MemorySummary>("/memory/summary")
  }

  // ========================================================================
  // Hooks (P2)
  // ========================================================================

  /**
   * List all configured hooks
   */
  async listHooks(): Promise<HookEntry[]> {
    return this.get<HookEntry[]>("/hooks")
  }

  /**
   * Get hooks by lifecycle
   */
  async getHooksByLifecycle(lifecycle: string): Promise<HookEntry[]> {
    return this.get<HookEntry[]>(`/hooks/${lifecycle}`)
  }

  /**
   * Get hooks settings
   */
  async getHooksSettings(): Promise<HookSettings> {
    return this.get<HookSettings>("/hooks/settings")
  }

  /**
   * Get hook configuration file locations
   */
  async getHookLocations(): Promise<HookLocation[]> {
    return this.get<HookLocation[]>("/hooks/locations")
  }

  /**
   * Get available hook action types
   */
  async getHookActionTypes(): Promise<HookActionTypeInfo[]> {
    return this.get<HookActionTypeInfo[]>("/hooks/action-types")
  }

  // ========================================================================
  // LSP (P2)
  // ========================================================================

  /**
   * Get LSP server status
   */
  async getLspStatus(): Promise<LspStatus[]> {
    return this.get<LspStatus[]>("/lsp/status")
  }

  /**
   * Get LSP diagnostics
   */
  async getLspDiagnostics(): Promise<LspFileDiagnostics[]> {
    return this.get<LspFileDiagnostics[]>("/lsp/diagnostics")
  }

  /**
   * Get LSP configuration
   */
  async getLspConfig(): Promise<LspConfig> {
    return this.get<LspConfig>("/lsp/config")
  }

  /**
   * Check if LSP is available for a file
   */
  async checkLspAvailable(filePath: string): Promise<{ available: boolean; filePath: string }> {
    return this.get<{ available: boolean; filePath: string }>(`/lsp/available?file=${encodeURIComponent(filePath)}`)
  }

  /**
   * Initialize LSP
   */
  async initLsp(): Promise<{ initialized: boolean; status: LspStatus[] }> {
    return this.post<{ initialized: boolean; status: LspStatus[] }>("/lsp/init")
  }

  /**
   * Touch a file to trigger LSP analysis
   */
  async touchLspFile(filePath: string, waitForDiagnostics?: boolean): Promise<void> {
    return this.post<void>("/lsp/touch", { filePath, waitForDiagnostics })
  }

  /**
   * Get hover information for a position
   */
  async getLspHover(
    file: string,
    line: number,
    character: number,
  ): Promise<any[]> {
    return this.post<any[]>("/lsp/hover", { file, line, character })
  }

  /**
   * Go to definition
   */
  async getLspDefinition(
    file: string,
    line: number,
    character: number,
  ): Promise<LspLocation[]> {
    return this.post<LspLocation[]>("/lsp/definition", { file, line, character })
  }

  /**
   * Find references
   */
  async getLspReferences(
    file: string,
    line: number,
    character: number,
  ): Promise<LspLocation[]> {
    return this.post<LspLocation[]>("/lsp/references", { file, line, character })
  }

  /**
   * Get workspace symbols
   */
  async getLspWorkspaceSymbols(query?: string): Promise<LspSymbol[]> {
    return this.post<LspSymbol[]>("/lsp/workspace-symbols", { query })
  }

  /**
   * Get document symbols
   */
  async getLspDocumentSymbols(uri: string): Promise<LspDocumentSymbol[]> {
    return this.post<LspDocumentSymbol[]>("/lsp/document-symbols", { uri })
  }

  // ========================================================================
  // Tasks (Async Task Management)
  // ========================================================================

  /**
   * List all tasks
   */
  async listTasks(): Promise<TaskInfo[]> {
    return this.get<TaskInfo[]>("/v1/tasks")
  }

  /**
   * Get a specific task
   */
  async getTask(id: string): Promise<TaskInfo> {
    return this.get<TaskInfo>(`/v1/tasks/${id}`)
  }

  /**
   * Create a new task
   */
  async createTask(input: CreateTaskInput): Promise<TaskInfo> {
    return this.post<TaskInfo>("/v1/tasks", input)
  }

  /**
   * Interact with a task (approve/reject)
   */
  async interactTask(id: string, input: InteractTaskInput): Promise<TaskInfo> {
    return this.post<TaskInfo>(`/v1/tasks/${id}/interact`, input)
  }

  /**
   * Delete a task
   */
  async deleteTask(id: string): Promise<void> {
    return this.delete<void>(`/v1/tasks/${id}`)
  }

  // ========================================================================
  // Channels (ZeroBot Integration)
  // ========================================================================

  /**
   * List all configured channels with status
   */
  async listChannels(): Promise<ChannelStatus[]> {
    return this.get<ChannelStatus[]>("/channels")
  }

  /**
   * Get specific channel status
   */
  async getChannel(name: string): Promise<ChannelStatus> {
    return this.get<ChannelStatus>(`/channels/${name}`)
  }

  /**
   * Check channel health
   */
  async checkChannelHealth(name: string): Promise<ChannelStatus> {
    return this.post<ChannelStatus>(`/channels/${name}/health`)
  }

  // ========================================================================
  // Directories
  // ========================================================================

  /**
   * List directories at a path
   */
  async listDirectories(path?: string): Promise<DirectoryListResponse> {
    const params = new URLSearchParams()
    if (path) params.append("path", path)
    const queryString = params.toString()
    const apiPath = queryString ? `/directories?${queryString}` : "/directories"
    return this.get<DirectoryListResponse>(apiPath)
  }

  // ========================================================================
  // Projects
  // ========================================================================

  /**
   * List all projects
   */
  async listProjects(): Promise<ProjectInfo[]> {
    return this.get<ProjectInfo[]>("/projects")
  }

  /**
   * Get a specific project
   */
  async getProject(id: string): Promise<ProjectInfo> {
    return this.get<ProjectInfo>(`/projects/${id}`)
  }

  /**
   * Create a new project
   */
  async createProject(input: ProjectCreateInput): Promise<ProjectInfo> {
    return this.post<ProjectInfo>("/projects", input)
  }

  /**
   * Update a project
   */
  async updateProject(
    id: string,
    input: { name?: string; icon?: { url?: string; override?: string; color?: string } },
  ): Promise<ProjectInfo> {
    return this.patch<ProjectInfo>(`/projects/${id}`, input)
  }

  /**
   * Delete a project
   */
  async deleteProject(id: string): Promise<void> {
    return this.delete<void>(`/projects/${id}`)
  }

  /**
   * Get sessions for a project
   */
  async getProjectSessions(id: string): Promise<SessionInfo[]> {
    return this.get<SessionInfo[]>(`/projects/${id}/sessions`)
  }

  // ========================================================================
  // Credentials
  // ========================================================================

  /**
   * List all credentials (without sensitive data)
   */
  async listCredentials(): Promise<CredentialSummary[]> {
    return this.get<CredentialSummary[]>("/credentials")
  }

  /**
   * Get a specific credential (includes sensitive data)
   */
  async getCredential(id: string): Promise<CredentialEntry> {
    return this.get<CredentialEntry>(`/credentials/${id}`)
  }

  /**
   * Add a new credential
   */
  async addCredential(input: CredentialCreateInput): Promise<{ id: string }> {
    return this.post<{ id: string }>("/credentials", input)
  }

  /**
   * Update an existing credential
   */
  async updateCredential(id: string, input: Partial<CredentialCreateInput>): Promise<void> {
    return this.put<void>(`/credentials/${id}`, input)
  }

  /**
   * Delete a credential
   */
  async deleteCredential(id: string): Promise<void> {
    return this.delete<void>(`/credentials/${id}`)
  }

  // ========================================================================
  // Metering (Admin Dashboard)
  // ========================================================================

  /**
   * Get overall usage statistics
   */
  async getMeteringUsage(): Promise<MeteringUsageResponse> {
    return this.get<MeteringUsageResponse>("/v1/metering/usage")
  }

  /**
   * Get per-user usage breakdown
   */
  async getMeteringUsers(): Promise<MeteringUserReport[]> {
    return this.get<MeteringUserReport[]>("/v1/metering/users")
  }

  /**
   * Get quota configurations
   */
  async getMeteringQuotas(): Promise<MeteringQuotasResponse> {
    return this.get<MeteringQuotasResponse>("/v1/metering/quotas")
  }

  /**
   * Update quota for a specific user
   */
  async updateMeteringQuota(userId: string, quota: MeteringQuotaUpdate): Promise<MeteringQuota> {
    return this.put<MeteringQuota>(`/v1/metering/quotas/${userId}`, quota)
  }

  // ========================================================================
  // Registry (Agent Discovery)
  // ========================================================================

  /**
   * List all registered agents with metadata
   */
  async getRegistryAgents(category?: string): Promise<RegistryAgentMetadata[]> {
    const params = category ? `?category=${encodeURIComponent(category)}` : ""
    return this.get<RegistryAgentMetadata[]>(`/v1/registry/agents${params}`)
  }

  /**
   * Get metadata for a specific agent
   */
  async getRegistryAgent(name: string): Promise<RegistryAgentMetadata> {
    return this.get<RegistryAgentMetadata>(`/v1/registry/agents/${encodeURIComponent(name)}`)
  }

  /**
   * Get recommended agent based on user intent
   */
  async recommendAgent(intent: string): Promise<AgentRecommendation> {
    return this.post<AgentRecommendation>("/v1/registry/recommend", { intent })
  }

  /**
   * Search agents by query
   */
  async searchRegistryAgents(query: string, limit?: number): Promise<RegistrySearchResult[]> {
    const params = new URLSearchParams({ q: query })
    if (limit) params.append("limit", String(limit))
    return this.get<RegistrySearchResult[]>(`/v1/registry/search?${params}`)
  }

  /**
   * List available agent categories
   */
  async getRegistryCategories(): Promise<RegistryCategory[]> {
    return this.get<RegistryCategory[]>("/v1/registry/categories")
  }

  /**
   * Get recommended agents for new users
   */
  async getRecommendedAgents(): Promise<RegistryAgentMetadata[]> {
    return this.get<RegistryAgentMetadata[]>("/v1/registry/recommended")
  }

  // ========================================================================
  // Chat (ZeroBot Bridge)
  // ========================================================================

  /**
   * Send a chat message and receive a response
   */
  async chat(input: ChatInput): Promise<ChatResponse> {
    return this.post<ChatResponse>("/v1/chat", input)
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
  updateSession: (id: string, input: { title: string }) => getClient().updateSession(id, input),
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
  // Provider APIs
  listProviders: () => getClient().listProviders(),
  listConnectedProviders: () => getClient().listConnectedProviders(),
  getProviderAuthMethods: () => getClient().getProviderAuthMethods(),
  getProvider: (providerId: string) => getClient().getProvider(providerId),
  getProviderModels: (providerId: string) => getClient().getProviderModels(providerId),
  // MCP APIs
  getMcpStatus: () => getClient().getMcpStatus(),
  getMcpTools: () => getClient().getMcpTools(),
  getMcpResources: () => getClient().getMcpResources(),
  connectMcp: (name: string) => getClient().connectMcp(name),
  disconnectMcp: (name: string) => getClient().disconnectMcp(name),
  toggleMcp: (name: string) => getClient().toggleMcp(name),
  getMcpAuthStatus: (name: string) => getClient().getMcpAuthStatus(name),
  startMcpAuth: (name: string) => getClient().startMcpAuth(name),
  finishMcpAuth: (name: string, code: string) => getClient().finishMcpAuth(name, code),
  // Document APIs (P2)
  listDocuments: () => getClient().listDocuments(),
  getDocument: (id: string) => getClient().getDocument(id),
  createDocument: (input: { title: string; description?: string; targetWords: number }) =>
    getClient().createDocument(input),
  updateDocument: (id: string, input: Partial<DocumentMetadata>) => getClient().updateDocument(id, input),
  deleteDocument: (id: string) => getClient().deleteDocument(id),
  getDocumentStats: (id: string) => getClient().getDocumentStats(id),
  exportDocument: (id: string, format?: "markdown" | "html") => getClient().exportDocument(id, format),
  listChapters: (documentId: string) => getClient().listChapters(documentId),
  getChapter: (documentId: string, chapterId: string) => getClient().getChapter(documentId, chapterId),
  updateChapter: (documentId: string, chapterId: string, input: Partial<DocumentChapter>) =>
    getClient().updateChapter(documentId, chapterId, input),
  listEntities: (documentId: string) => getClient().listEntities(documentId),
  createEntity: (
    documentId: string,
    input: {
      name: string
      type: string
      description: string
      firstAppearedChapterID: string
      aliases?: string[]
      attributes?: Record<string, string>
    },
  ) => getClient().createEntity(documentId, input),
  updateEntity: (documentId: string, entityId: string, input: Partial<DocumentEntity>) =>
    getClient().updateEntity(documentId, entityId, input),
  deleteEntity: (documentId: string, entityId: string) => getClient().deleteEntity(documentId, entityId),
  listVolumes: (documentId: string) => getClient().listVolumes(documentId),
  createVolume: (
    documentId: string,
    input: { title: string; description?: string; startChapterID: string; endChapterID: string },
  ) => getClient().createVolume(documentId, input),
  // Memory APIs (P2)
  listDailyDates: () => getClient().listDailyDates(),
  getDailyNotes: (date: string) => getClient().getDailyNotes(date),
  appendDailyNote: (input: { type: string; content: string; metadata?: Record<string, any> }) =>
    getClient().appendDailyNote(input),
  getLongTermMemory: () => getClient().getLongTermMemory(),
  getMemorySections: () => getClient().getMemorySections(),
  updateMemoryCategory: (category: string, content: string) => getClient().updateMemoryCategory(category, content),
  mergeToMemoryCategory: (category: string, content: string) => getClient().mergeToMemoryCategory(category, content),
  getConsolidationStats: () => getClient().getConsolidationStats(),
  triggerConsolidation: (options?: { days?: number; preserveOriginal?: boolean; minImportance?: number }) =>
    getClient().triggerConsolidation(options),
  getMemorySummary: () => getClient().getMemorySummary(),
  // Hooks APIs (P2)
  listHooks: () => getClient().listHooks(),
  getHooksByLifecycle: (lifecycle: string) => getClient().getHooksByLifecycle(lifecycle),
  getHooksSettings: () => getClient().getHooksSettings(),
  getHookLocations: () => getClient().getHookLocations(),
  getHookActionTypes: () => getClient().getHookActionTypes(),
  // LSP APIs (P2)
  getLspStatus: () => getClient().getLspStatus(),
  getLspDiagnostics: () => getClient().getLspDiagnostics(),
  getLspConfig: () => getClient().getLspConfig(),
  checkLspAvailable: (filePath: string) => getClient().checkLspAvailable(filePath),
  initLsp: () => getClient().initLsp(),
  touchLspFile: (filePath: string, waitForDiagnostics?: boolean) =>
    getClient().touchLspFile(filePath, waitForDiagnostics),
  getLspHover: (file: string, line: number, character: number) => getClient().getLspHover(file, line, character),
  getLspDefinition: (file: string, line: number, character: number) =>
    getClient().getLspDefinition(file, line, character),
  getLspReferences: (file: string, line: number, character: number) =>
    getClient().getLspReferences(file, line, character),
  getLspWorkspaceSymbols: (query?: string) => getClient().getLspWorkspaceSymbols(query),
  getLspDocumentSymbols: (uri: string) => getClient().getLspDocumentSymbols(uri),
  // Task APIs
  listTasks: () => getClient().listTasks(),
  getTask: (id: string) => getClient().getTask(id),
  createTask: (input: CreateTaskInput) => getClient().createTask(input),
  interactTask: (id: string, input: InteractTaskInput) => getClient().interactTask(id, input),
  deleteTask: (id: string) => getClient().deleteTask(id),
  // Channel APIs (ZeroBot Integration)
  listChannels: () => getClient().listChannels(),
  getChannel: (name: string) => getClient().getChannel(name),
  checkChannelHealth: (name: string) => getClient().checkChannelHealth(name),
  // Directory APIs
  listDirectories: (path?: string) => getClient().listDirectories(path),
  // Project APIs
  listProjects: () => getClient().listProjects(),
  getProject: (id: string) => getClient().getProject(id),
  createProject: (input: ProjectCreateInput) => getClient().createProject(input),
  updateProject: (id: string, input: { name?: string; icon?: { url?: string; override?: string; color?: string } }) =>
    getClient().updateProject(id, input),
  deleteProject: (id: string) => getClient().deleteProject(id),
  getProjectSessions: (id: string) => getClient().getProjectSessions(id),
  // Credential APIs
  listCredentials: () => getClient().listCredentials(),
  getCredential: (id: string) => getClient().getCredential(id),
  addCredential: (input: CredentialCreateInput) => getClient().addCredential(input),
  updateCredential: (id: string, input: Partial<CredentialCreateInput>) => getClient().updateCredential(id, input),
  deleteCredential: (id: string) => getClient().deleteCredential(id),
  // Metering APIs
  getMeteringUsage: () => getClient().getMeteringUsage(),
  getMeteringUsers: () => getClient().getMeteringUsers(),
  getMeteringQuotas: () => getClient().getMeteringQuotas(),
  updateMeteringQuota: (userId: string, quota: MeteringQuotaUpdate) => getClient().updateMeteringQuota(userId, quota),
  // Registry APIs
  getRegistryAgents: (category?: string) => getClient().getRegistryAgents(category),
  getRegistryAgent: (name: string) => getClient().getRegistryAgent(name),
  recommendAgent: (intent: string) => getClient().recommendAgent(intent),
  searchRegistryAgents: (query: string, limit?: number) => getClient().searchRegistryAgents(query, limit),
  getRegistryCategories: () => getClient().getRegistryCategories(),
  getRecommendedAgents: () => getClient().getRecommendedAgents(),
  // Chat API
  chat: (input: ChatInput) => getClient().chat(input),
}
