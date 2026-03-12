/**
 * Unified Rust API Client
 *
 * Lightweight TypeScript client for the Rust Unified API server.
 * Replaces local TypeScript implementations with HTTP calls to zero-cli daemon.
 *
 * @module api/rust-client
 */

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

export interface RustClientConfig {
  /** Base URL for the Rust API (default: http://127.0.0.1:4402) */
  baseUrl?: string
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number
  /** Auth token if required */
  authToken?: string
}

const DEFAULT_CONFIG: Required<RustClientConfig> = {
  baseUrl: "http://127.0.0.1:4402",
  timeout: 30000,
  authToken: "",
}

// ─────────────────────────────────────────────────────────────────────────────
// Common Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Session Types
// ─────────────────────────────────────────────────────────────────────────────

export interface SessionTime {
  created: number
  updated: number
  compacting?: number
  archived?: number
}

export interface FileDiff {
  file: string
  before?: string
  after?: string
  additions: number
  deletions: number
}

export interface SessionSummaryInfo {
  additions: number
  deletions: number
  files: number
  diffs?: FileDiff[]
}

export interface SessionPermission {
  allowed_tools?: string[]
  denied_tools?: string[]
  auto_approve?: string[]
  [key: string]: unknown
}

export interface RevertInfo {
  messageID: string
  partID?: string
  snapshot?: string
  diff?: string
}

export interface SessionSummary {
  id: string
  title?: string
  message_count: number
  token_count: number
  time: SessionTime
  created_at: number
  updated_at: number
  project_id?: string
  parent_id?: string
  directory?: string
  summary?: SessionSummaryInfo
  permission?: SessionPermission
  revert?: RevertInfo
}

export interface ToolCall {
  id: string
  name: string
  arguments: unknown
}

export interface SessionMessage {
  id: number
  role: string
  content: string
  timestamp: number
  tool_calls?: ToolCall[]
  tool_call_id?: string
}

export interface SessionDetail extends SessionSummary {
  messages: SessionMessage[]
  agent?: string
}

export interface CreateSessionRequest {
  title?: string
  project_id?: string
  agent?: string
}

export interface UpdateSessionRequest {
  title?: string
  project_id?: string
}

export interface SendMessageRequest {
  content: string
  role?: "user" | "assistant" | "system"
}

export interface ForkSessionRequest {
  from_message_id?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ModelInfo {
  provider_id: string
  model_id: string
  temperature?: number
  max_tokens?: number
}

export interface PermissionInfo {
  rules: Record<string, "allow" | "deny" | "ask" | Record<string, "allow" | "deny" | "ask">>
}

export interface AutoApproveConfig {
  tools?: string[]
  patterns?: string[]
}

export interface ObserverCapability {
  enabled: boolean
  observe?: string[]
}

export interface AgentInfo {
  name: string
  description?: string
  mode: string
  temperature?: number
  color?: string
  hidden: boolean
  model?: ModelInfo
  permission?: PermissionInfo
  auto_approve?: AutoApproveConfig
  observer?: ObserverCapability
  options?: Record<string, unknown>
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

export interface TokenUsage {
  input_tokens: number
  output_tokens: number
  reasoning_tokens: number
  cache_read_tokens?: number
  cache_write_tokens?: number
}

export interface DispatchAgentResponse {
  success: boolean
  request_id: string
  response: string
  usage?: TokenUsage
  error?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ToolInfo {
  name: string
  description: string
  parameters_schema: Record<string, unknown>
  category?: string
  source?: string
}

export interface ExecuteToolRequest {
  params: Record<string, unknown>
}

export interface ExecuteToolResponse {
  success: boolean
  output: string
  error?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory Types
// ─────────────────────────────────────────────────────────────────────────────

export interface DailyNote {
  date: string
  content: string
}

export interface MemoryCategory {
  name: string
  content: string
}

export interface LongTermMemory {
  categories: MemoryCategory[]
}

export interface AppendDailyNoteRequest {
  content: string
  date?: string
}

export interface UpdateCategoryRequest {
  content: string
}

export interface MergeToCategoryRequest {
  content: string
}

export interface MemorySummary {
  daily_count: number
  category_count: number
  total_entries: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Task Types
// ─────────────────────────────────────────────────────────────────────────────

export interface TaskInfo {
  id: string
  type: string
  status: "pending" | "running" | "completed" | "failed"
  created_at: number
  updated_at: number
  result?: unknown
  error?: string
}

export interface CreateTaskRequest {
  type: string
  payload: unknown
}

export interface TaskInteractRequest {
  input: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ProviderInfo {
  id: string
  name: string
  is_default: boolean
  supports_streaming: boolean
  supports_tool_calling: boolean
  models: ModelSummary[]
}

export interface ModelSummary {
  id: string
  name: string
  context_window: number
  max_output?: number
}

export interface ModelDetail extends ModelSummary {
  provider_id: string
  supports_vision: boolean
  supports_function_calling: boolean
  input_cost_per_1k?: number
  output_cost_per_1k?: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ConfigData {
  [key: string]: unknown
}

export interface ValidateConfigResponse {
  valid: boolean
  errors?: string[]
}

// ─────────────────────────────────────────────────────────────────────────────
// Prompt Types
// ─────────────────────────────────────────────────────────────────────────────

export interface PromptInfo {
  name: string
  modified_at?: string
}

export interface PromptDetail {
  name: string
  content: string
  modified_at?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// SSE Chat Event Types
// ─────────────────────────────────────────────────────────────────────────────

export type ChatEventType =
  | "start"
  | "text_delta"
  | "reasoning_delta"
  | "tool_call_start"
  | "tool_call_delta"
  | "tool_call"
  | "tool_result"
  | "finish"
  | "error"

export interface ChatEventBase {
  type: ChatEventType
}

export interface ChatStartEvent extends ChatEventBase {
  type: "start"
}

export interface ChatTextDeltaEvent extends ChatEventBase {
  type: "text_delta"
  content: string
}

export interface ChatReasoningDeltaEvent extends ChatEventBase {
  type: "reasoning_delta"
  content: string
}

export interface ChatToolCallStartEvent extends ChatEventBase {
  type: "tool_call_start"
  id: string
  name: string
}

export interface ChatToolCallDeltaEvent extends ChatEventBase {
  type: "tool_call_delta"
  id: string
  arguments_delta: string
}

export interface ChatToolCallEvent extends ChatEventBase {
  type: "tool_call"
  id: string
  name: string
  arguments: unknown
}

export interface ChatToolResultEvent extends ChatEventBase {
  type: "tool_result"
  id: string
  output?: string
  error?: string
}

export interface ChatFinishEvent extends ChatEventBase {
  type: "finish"
  reason: string
  usage?: TokenUsage
}

export interface ChatErrorEvent extends ChatEventBase {
  type: "error"
  code: number
  message: string
}

export type ChatEvent =
  | ChatStartEvent
  | ChatTextDeltaEvent
  | ChatReasoningDeltaEvent
  | ChatToolCallStartEvent
  | ChatToolCallDeltaEvent
  | ChatToolCallEvent
  | ChatToolResultEvent
  | ChatFinishEvent
  | ChatErrorEvent

// ─────────────────────────────────────────────────────────────────────────────
// Rust API Client
// ─────────────────────────────────────────────────────────────────────────────

export class RustApiClient {
  private config: Required<RustClientConfig>

  constructor(config: RustClientConfig = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Core HTTP Methods
  // ─────────────────────────────────────────────────────────────────────────

  private async request<T>(method: string, path: string, body?: unknown): Promise<ApiResponse<T>> {
    const url = `${this.config.baseUrl}${path}`
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    }

    if (this.config.authToken) {
      headers["Authorization"] = `Bearer ${this.config.authToken}`
    }

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout)

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      })

      const data = await response.json()
      return data as ApiResponse<T>
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") {
        return { success: false, error: "Request timeout" }
      }
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    } finally {
      clearTimeout(timeoutId)
    }
  }

  private async get<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("GET", path)
  }

  private async post<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("POST", path, body)
  }

  private async put<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("PUT", path, body)
  }

  private async patch<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("PATCH", path, body)
  }

  private async delete<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("DELETE", path)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Session Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List all sessions */
  async listSessions(options?: {
    limit?: number
    offset?: number
    project_id?: string
  }): Promise<ApiResponse<{ sessions: SessionSummary[]; total: number }>> {
    const params = new URLSearchParams()
    if (options?.limit) params.set("limit", String(options.limit))
    if (options?.offset) params.set("offset", String(options.offset))
    if (options?.project_id) params.set("project_id", options.project_id)
    const query = params.toString()
    return this.get(`/api/v1/sessions${query ? `?${query}` : ""}`)
  }

  /** Create a new session */
  async createSession(
    request: CreateSessionRequest
  ): Promise<ApiResponse<{ session: SessionDetail }>> {
    return this.post("/api/v1/sessions", request)
  }

  /** Get session by ID */
  async getSession(id: string): Promise<ApiResponse<{ session: SessionDetail }>> {
    return this.get(`/api/v1/sessions/${id}`)
  }

  /** Update session */
  async updateSession(
    id: string,
    request: UpdateSessionRequest
  ): Promise<ApiResponse<{ id: string; message: string }>> {
    return this.patch(`/api/v1/sessions/${id}`, request)
  }

  /** Delete session */
  async deleteSession(
    id: string
  ): Promise<ApiResponse<{ id: string; deleted_messages: number }>> {
    return this.delete(`/api/v1/sessions/${id}`)
  }

  /** Get session messages */
  async getSessionMessages(id: string): Promise<ApiResponse<{ messages: SessionMessage[] }>> {
    return this.get(`/api/v1/sessions/${id}/messages`)
  }

  /** Send a message to session */
  async sendMessage(
    sessionId: string,
    request: SendMessageRequest
  ): Promise<ApiResponse<{ message: SessionMessage }>> {
    return this.post(`/api/v1/sessions/${sessionId}/messages`, request)
  }

  /** Fork a session */
  async forkSession(
    sessionId: string,
    request?: ForkSessionRequest
  ): Promise<ApiResponse<{ session: SessionDetail }>> {
    return this.post(`/api/v1/sessions/${sessionId}/fork`, request ?? {})
  }

  /** Compact session history */
  async compactSession(
    sessionId: string
  ): Promise<ApiResponse<{ deleted_count: number; new_token_count: number }>> {
    return this.post(`/api/v1/sessions/${sessionId}/compact`)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Agent Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List all agents */
  async listAgents(): Promise<ApiResponse<{ agents: AgentInfo[]; total: number }>> {
    return this.get("/api/v1/agents")
  }

  /** Get agent by name */
  async getAgent(name: string): Promise<ApiResponse<{ agent: AgentInfo }>> {
    return this.get(`/api/v1/agents/${name}`)
  }

  /** Dispatch an agent (non-streaming) */
  async dispatchAgent(request: DispatchAgentRequest): Promise<DispatchAgentResponse> {
    const response = await this.post<DispatchAgentResponse>("/api/v1/agents/dispatch", {
      ...request,
      stream: false,
    })
    return response.success && response.data ? response.data : { success: false, request_id: "", response: "", error: response.error }
  }

  /** Get agent's raw prompt */
  async getAgentPrompt(
    name: string
  ): Promise<ApiResponse<{ name: string; prompt: string; modified_at?: string }>> {
    return this.get(`/api/v1/agents/${name}/prompt`)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // SSE Chat Streaming
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Stream chat responses via SSE.
   * Returns an async generator that yields ChatEvent objects.
   */
  async *chat(
    sessionId: string,
    message: string,
    options?: {
      agent?: string
      system?: string[]
      model?: string
      temperature?: number
      max_tokens?: number
      max_iterations?: number
      tool_timeout?: number
    }
  ): AsyncGenerator<ChatEvent, void, unknown> {
    const url = `${this.config.baseUrl}/api/v1/sessions/${sessionId}/chat`
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: "text/event-stream",
    }

    if (this.config.authToken) {
      headers["Authorization"] = `Bearer ${this.config.authToken}`
    }

    const body = JSON.stringify({
      message,
      agent: options?.agent ?? "default",
      system: options?.system ?? [],
      model: options?.model,
      temperature: options?.temperature,
      max_tokens: options?.max_tokens,
      max_iterations: options?.max_iterations ?? 10,
      tool_timeout: options?.tool_timeout ?? 30,
    })

    const response = await fetch(url, {
      method: "POST",
      headers,
      body,
    })

    if (!response.ok) {
      yield {
        type: "error",
        code: response.status,
        message: `HTTP error: ${response.statusText}`,
      }
      return
    }

    const reader = response.body?.getReader()
    if (!reader) {
      yield { type: "error", code: -1, message: "No response body" }
      return
    }

    const decoder = new TextDecoder()
    let buffer = ""

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      buffer += decoder.decode(value, { stream: true })

      // Parse SSE events
      const lines = buffer.split("\n")
      buffer = lines.pop() ?? ""

      for (const line of lines) {
        if (line.startsWith("data:")) {
          const jsonStr = line.slice(5).trim()
          if (jsonStr) {
            try {
              const event = JSON.parse(jsonStr) as ChatEvent
              yield event
            } catch {
              // Skip invalid JSON
            }
          }
        }
      }
    }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Tool Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List all tools */
  async listTools(): Promise<ApiResponse<{ tools: ToolInfo[]; total: number }>> {
    return this.get("/api/v1/tools")
  }

  /** Execute a tool */
  async executeTool(name: string, params: Record<string, unknown>): Promise<ExecuteToolResponse> {
    const response = await this.post<ExecuteToolResponse>(`/api/v1/tools/${name}`, { params })
    return response.success && response.data
      ? response.data
      : { success: false, output: "", error: response.error }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Memory Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List daily note dates */
  async listDailyDates(): Promise<ApiResponse<{ dates: string[] }>> {
    return this.get("/api/v1/memory/daily")
  }

  /** Get daily notes for a specific date */
  async getDailyNotes(date: string): Promise<ApiResponse<DailyNote>> {
    return this.get(`/api/v1/memory/daily/${date}`)
  }

  /** Append to daily notes */
  async appendDailyNote(request: AppendDailyNoteRequest): Promise<ApiResponse<{ success: boolean }>> {
    return this.post("/api/v1/memory/daily", request)
  }

  /** Get long-term memory */
  async getLongTermMemory(): Promise<ApiResponse<LongTermMemory>> {
    return this.get("/api/v1/memory/long-term")
  }

  /** Update a memory category */
  async updateCategory(
    category: string,
    request: UpdateCategoryRequest
  ): Promise<ApiResponse<{ success: boolean }>> {
    return this.put(`/api/v1/memory/category/${category}`, request)
  }

  /** Merge content into a category */
  async mergeToCategory(
    category: string,
    request: MergeToCategoryRequest
  ): Promise<ApiResponse<{ success: boolean }>> {
    return this.post(`/api/v1/memory/category/${category}/merge`, request)
  }

  /** Consolidate daily notes into long-term memory */
  async consolidateMemory(): Promise<ApiResponse<{ consolidated: number }>> {
    return this.post("/api/v1/memory/consolidate")
  }

  /** Get memory summary */
  async getMemorySummary(): Promise<ApiResponse<MemorySummary>> {
    return this.get("/api/v1/memory/summary")
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Task Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List tasks */
  async listTasks(): Promise<ApiResponse<{ tasks: TaskInfo[]; total: number }>> {
    return this.get("/api/v1/tasks")
  }

  /** Create a task */
  async createTask(request: CreateTaskRequest): Promise<ApiResponse<{ task: TaskInfo }>> {
    return this.post("/api/v1/tasks", request)
  }

  /** Get task by ID */
  async getTask(id: string): Promise<ApiResponse<{ task: TaskInfo }>> {
    return this.get(`/api/v1/tasks/${id}`)
  }

  /** Delete task */
  async deleteTask(id: string): Promise<ApiResponse<{ success: boolean }>> {
    return this.delete(`/api/v1/tasks/${id}`)
  }

  /** Interact with task (send input) */
  async interactTask(id: string, request: TaskInteractRequest): Promise<ApiResponse<{ success: boolean }>> {
    return this.post(`/api/v1/tasks/${id}/interact`, request)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Provider Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List configured providers */
  async listProviders(): Promise<ApiResponse<{ providers: ProviderInfo[] }>> {
    return this.get("/api/v1/providers")
  }

  /** List all available providers */
  async listAllProviders(): Promise<ApiResponse<{ providers: ProviderInfo[] }>> {
    return this.get("/api/v1/providers/all")
  }

  /** Get provider by ID */
  async getProvider(id: string): Promise<ApiResponse<{ provider: ProviderInfo }>> {
    return this.get(`/api/v1/providers/${id}`)
  }

  /** Get model details */
  async getModel(providerId: string, modelId: string): Promise<ApiResponse<{ model: ModelDetail }>> {
    return this.get(`/api/v1/providers/${providerId}/models/${modelId}`)
  }

  /** Get default model */
  async getDefaultModel(): Promise<ApiResponse<{ model: ModelDetail }>> {
    return this.get("/api/v1/providers/default-model")
  }

  /** Get small/fast model for a provider */
  async getSmallModel(providerId: string): Promise<ApiResponse<{ model: ModelDetail }>> {
    return this.get(`/api/v1/providers/${providerId}/small-model`)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Configuration
  // ─────────────────────────────────────────────────────────────────────────

  /** Get configuration */
  async getConfig(): Promise<ApiResponse<ConfigData>> {
    return this.get("/api/v1/config")
  }

  /** Update configuration */
  async updateConfig(config: ConfigData): Promise<ApiResponse<{ success: boolean }>> {
    return this.put("/api/v1/config", config)
  }

  /** Validate configuration */
  async validateConfig(config: ConfigData): Promise<ApiResponse<ValidateConfigResponse>> {
    return this.post("/api/v1/config/validate", config)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Prompt Management
  // ─────────────────────────────────────────────────────────────────────────

  /** List prompts */
  async listPrompts(): Promise<ApiResponse<{ prompts: PromptInfo[] }>> {
    return this.get("/api/v1/prompts")
  }

  /** Get prompt by name */
  async getPrompt(name: string): Promise<ApiResponse<PromptDetail>> {
    return this.get(`/api/v1/prompts/${name}`)
  }

  /** Reload prompts from disk */
  async reloadPrompts(): Promise<ApiResponse<{ reloaded: number }>> {
    return this.post("/api/v1/prompts/reload")
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let clientInstance: RustApiClient | null = null

/**
 * Get or create the Rust API client singleton.
 */
export function getRustClient(config?: RustClientConfig): RustApiClient {
  if (!clientInstance || config) {
    clientInstance = new RustApiClient(config)
  }
  return clientInstance
}

/**
 * Reset the client singleton (useful for testing).
 */
export function resetRustClient(): void {
  clientInstance = null
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Check if the Rust API server is available.
 */
export async function isRustApiAvailable(): Promise<boolean> {
  const client = getRustClient()
  const response = await client.listAgents()
  return response.success
}

/**
 * Quick chat helper - creates session if needed and streams response.
 */
export async function quickChat(
  message: string,
  options?: {
    sessionId?: string
    agent?: string
    onDelta?: (content: string) => void
  }
): Promise<{ response: string; sessionId: string; usage?: TokenUsage }> {
  const client = getRustClient()

  // Create session if not provided
  let sessionId = options?.sessionId
  if (!sessionId) {
    const createResponse = await client.createSession({ agent: options?.agent })
    if (!createResponse.success || !createResponse.data) {
      throw new Error(createResponse.error ?? "Failed to create session")
    }
    sessionId = createResponse.data.session.id
  }

  // Stream chat
  let fullResponse = ""
  let usage: TokenUsage | undefined

  for await (const event of client.chat(sessionId, message, { agent: options?.agent })) {
    switch (event.type) {
      case "text_delta":
        fullResponse += event.content
        options?.onDelta?.(event.content)
        break
      case "finish":
        usage = event.usage
        break
      case "error":
        throw new Error(`Chat error (${event.code}): ${event.message}`)
    }
  }

  return { response: fullResponse, sessionId, usage }
}
