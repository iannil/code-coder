/**
 * HTTP Client for Rust Daemon REST API
 *
 * Provides typed access to the daemon's HTTP endpoints.
 *
 * @module sdk/client
 */

import type {
  AgentListResponse,
  AgentDetailResponse,
  DispatchAgentRequest,
  SessionListResponse,
  SessionInfo,
  GearStatusResponse,
  GearStatus,
  ObserverStatusResponse,
  TokenUsage,
  ProviderInfo,
  ProviderListResponseExtended,
} from "./types"

export interface HttpClientConfig {
  /** Base URL (default: http://127.0.0.1:4402) */
  baseUrl?: string
  /** Request timeout in ms (default: 30000) */
  timeout?: number
  /** Default headers */
  headers?: Record<string, string>
}

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

/**
 * HTTP client for the Rust daemon REST API.
 *
 * @example
 * ```typescript
 * const client = new HttpClient({ baseUrl: "http://127.0.0.1:4402" })
 *
 * // List agents
 * const agents = await client.listAgents()
 *
 * // Get observer status
 * const status = await client.getObserverStatus()
 * ```
 */
export class HttpClient {
  private readonly config: Required<HttpClientConfig>

  constructor(config: HttpClientConfig = {}) {
    this.config = {
      baseUrl: config.baseUrl ?? "http://127.0.0.1:4402",
      timeout: config.timeout ?? 30000,
      headers: config.headers ?? {},
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Agents
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * List all available agents.
   */
  async listAgents(): Promise<AgentListResponse> {
    return this.get<AgentListResponse>("/api/v1/agents")
  }

  /**
   * Get agent details.
   */
  async getAgent(name: string): Promise<AgentDetailResponse> {
    return this.get<AgentDetailResponse>(`/api/v1/agents/${encodeURIComponent(name)}`)
  }

  /**
   * Get agent prompt.
   */
  async getAgentPrompt(name: string): Promise<{ success: boolean; name: string; prompt: string }> {
    return this.get(`/api/v1/agents/${encodeURIComponent(name)}/prompt`)
  }

  /**
   * Dispatch an agent task (non-streaming).
   */
  async dispatchAgent(request: DispatchAgentRequest): Promise<{
    success: boolean
    request_id: string
    response: string
    usage?: TokenUsage
    error?: string
  }> {
    return this.post("/api/v1/agents/dispatch", { ...request, stream: false })
  }

  /**
   * Dispatch an agent task with SSE streaming.
   * Returns an EventSource for consuming events.
   */
  dispatchAgentStream(
    request: DispatchAgentRequest,
    onEvent: (event: { type: string; data: unknown }) => void,
    onError?: (error: Error) => void
  ): { close: () => void } {
    const url = `${this.config.baseUrl}/api/v1/agents/dispatch`

    // Create abort controller for cancellation
    const controller = new AbortController()

    // Use fetch with streaming
    fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "text/event-stream",
        ...this.config.headers,
      },
      body: JSON.stringify({ ...request, stream: true }),
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`)
        }

        const reader = response.body?.getReader()
        if (!reader) {
          throw new Error("No response body")
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
            if (line.startsWith("data: ")) {
              try {
                const data = JSON.parse(line.slice(6))
                onEvent(data)
              } catch {
                // Skip malformed events
              }
            }
          }
        }
      })
      .catch((error) => {
        if (error.name !== "AbortError") {
          onError?.(error)
        }
      })

    return {
      close: () => controller.abort(),
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Sessions
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * List sessions.
   */
  async listSessions(limit = 50, offset = 0): Promise<SessionListResponse> {
    return this.get<SessionListResponse>(`/api/v1/sessions?limit=${limit}&offset=${offset}`)
  }

  /**
   * Get session details.
   */
  async getSession(id: string): Promise<{ success: boolean; session: SessionInfo }> {
    return this.get(`/api/v1/sessions/${encodeURIComponent(id)}`)
  }

  /**
   * Create a new session.
   */
  async createSession(options?: {
    title?: string
    project_id?: string
    agent?: string
  }): Promise<{ success: boolean; session: SessionInfo }> {
    return this.post("/api/v1/sessions", options ?? {})
  }

  /**
   * Delete a session.
   */
  async deleteSession(id: string): Promise<{ success: boolean }> {
    return this.delete(`/api/v1/sessions/${encodeURIComponent(id)}`)
  }

  /**
   * Get session messages.
   */
  async getSessionMessages(
    id: string
  ): Promise<{ success: boolean; messages: Array<{ role: string; content: string; timestamp: string }> }> {
    return this.get(`/api/v1/sessions/${encodeURIComponent(id)}/messages`)
  }

  /**
   * Send a message to a session.
   */
  async sendSessionMessage(
    id: string,
    message: {
      content: string
      agent?: string
      model?: string
      system?: string[]
    }
  ): Promise<{ success: boolean; message_id: string }> {
    return this.post(`/api/v1/sessions/${encodeURIComponent(id)}/messages`, message)
  }

  /**
   * Update session properties.
   */
  async updateSession(id: string, updates: {
    title?: string
    agent?: string
  }): Promise<{ success: boolean; session: SessionInfo }> {
    return this.patch(`/api/v1/sessions/${encodeURIComponent(id)}`, updates)
  }

  /**
   * Rename a session (convenience method).
   */
  async renameSession(id: string, title: string): Promise<{ success: boolean; session: SessionInfo }> {
    return this.updateSession(id, { title })
  }

  /**
   * Fork a session (create a child session from a specific point).
   */
  async forkSession(
    id: string,
    options?: {
      message_id?: string
      title?: string
    }
  ): Promise<{ success: boolean; session: SessionInfo }> {
    return this.post(`/api/v1/sessions/${encodeURIComponent(id)}/fork`, options ?? {})
  }

  /**
   * Compact session history (summarize old messages).
   */
  async compactSession(id: string): Promise<{ success: boolean }> {
    return this.post(`/api/v1/sessions/${encodeURIComponent(id)}/compact`, {})
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Providers
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * List all providers with their models.
   * Returns connected providers, all available providers, and default model per provider.
   */
  async listProviders(): Promise<ProviderListResponseExtended> {
    return this.get<ProviderListResponseExtended>("/api/v1/providers")
  }

  /**
   * Get a specific provider's details.
   */
  async getProvider(id: string): Promise<{ success: boolean; provider: ProviderInfo }> {
    return this.get(`/api/v1/providers/${encodeURIComponent(id)}`)
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Gear Control
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Get current gear status.
   */
  async getGearStatus(): Promise<GearStatusResponse> {
    return this.get<GearStatusResponse>("/api/v1/gear/current")
  }

  /**
   * Switch gear.
   */
  async switchGear(gear: GearStatus["gear"], reason?: string): Promise<{ success: boolean }> {
    return this.post("/api/v1/gear/switch", { gear, reason })
  }

  /**
   * Set dial values (Manual mode).
   */
  async setDials(dials: GearStatus["dials"]): Promise<{ success: boolean }> {
    return this.post("/api/v1/gear/dials", dials)
  }

  /**
   * Get gear presets.
   */
  async getGearPresets(): Promise<{ success: boolean; presets: Record<string, unknown> }> {
    return this.get("/api/v1/gear/presets")
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Observer
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Get observer status.
   */
  async getObserverStatus(): Promise<ObserverStatusResponse> {
    return this.get<ObserverStatusResponse>("/api/v1/observer/status")
  }

  /**
   * Start observer network.
   */
  async startObserver(): Promise<{ success: boolean }> {
    return this.post("/api/v1/observer/start", {})
  }

  /**
   * Stop observer network.
   */
  async stopObserver(): Promise<{ success: boolean }> {
    return this.post("/api/v1/observer/stop", {})
  }

  /**
   * Get world model.
   */
  async getWorldModel(): Promise<{ success: boolean; data: unknown }> {
    return this.get("/api/v1/observer/world-model")
  }

  /**
   * Get consensus state.
   */
  async getConsensus(): Promise<{ success: boolean; data: unknown }> {
    return this.get("/api/v1/observer/consensus")
  }

  /**
   * Get active patterns.
   */
  async getPatterns(): Promise<{ success: boolean; patterns: unknown[] }> {
    return this.get("/api/v1/observer/patterns")
  }

  /**
   * Get active anomalies.
   */
  async getAnomalies(): Promise<{ success: boolean; anomalies: unknown[] }> {
    return this.get("/api/v1/observer/anomalies")
  }

  /**
   * Subscribe to observer events via SSE.
   */
  subscribeToObserverEvents(
    onEvent: (event: unknown) => void,
    onError?: (error: Error) => void
  ): { close: () => void } {
    const eventSource = new EventSource(`${this.config.baseUrl}/api/v1/observer/events`)

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        onEvent(data)
      } catch {
        // Skip malformed events
      }
    }

    eventSource.onerror = () => {
      onError?.(new Error("SSE connection error"))
    }

    return {
      close: () => eventSource.close(),
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Config
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Get current configuration.
   */
  async getConfig(): Promise<{
    success: boolean
    config: {
      core: { default_model?: string; default_temperature?: number; max_tokens?: number }
      providers: Array<{ id: string; name: string; enabled: boolean; models: string[]; has_key: boolean }>
      workspace_dir: string
      version: string
    }
  }> {
    return this.get("/api/v1/config")
  }

  /**
   * Update configuration.
   */
  async updateConfig(updates: {
    core?: {
      default_model?: string
      default_temperature?: number
      max_tokens?: number
    }
  }): Promise<{ success: boolean; updated_fields: string[] }> {
    return this.put("/api/v1/config", updates)
  }

  /**
   * Validate configuration.
   */
  async validateConfig(config: Record<string, unknown>): Promise<{
    success: boolean
    valid: boolean
    errors: string[]
    warnings: string[]
  }> {
    return this.post("/api/v1/config/validate", { config })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Tools
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute a tool.
   */
  async executeTool(tool: string, params: Record<string, unknown>): Promise<{ success: boolean; result?: unknown; error?: string }> {
    return this.post(`/api/v1/tools/${encodeURIComponent(tool)}/execute`, params)
  }

  /**
   * List available tools.
   */
  async listTools(): Promise<{ success: boolean; tools: Array<{ name: string; description: string }> }> {
    return this.get("/api/v1/tools")
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Health
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Check daemon health.
   */
  async health(): Promise<{ status: string; version: string }> {
    return this.get("/health")
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Internal HTTP Methods
  // ──────────────────────────────────────────────────────────────────────────

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method: "GET",
      headers: {
        Accept: "application/json",
        ...this.config.headers,
      },
      signal: AbortSignal.timeout(this.config.timeout),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return response.json()
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        ...this.config.headers,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(this.config.timeout),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return response.json()
  }

  private async patch<T>(path: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method: "PATCH",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        ...this.config.headers,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(this.config.timeout),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return response.json()
  }

  private async put<T>(path: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        ...this.config.headers,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(this.config.timeout),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return response.json()
  }

  private async delete<T>(path: string): Promise<T> {
    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method: "DELETE",
      headers: {
        Accept: "application/json",
        ...this.config.headers,
      },
      signal: AbortSignal.timeout(this.config.timeout),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    return response.json()
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Singleton Instance
// ══════════════════════════════════════════════════════════════════════════════

let defaultHttpClient: HttpClient | null = null

/**
 * Get the default HTTP client singleton.
 */
export function getHttpClient(config?: HttpClientConfig): HttpClient {
  if (!defaultHttpClient) {
    defaultHttpClient = new HttpClient(config)
  }
  return defaultHttpClient
}

/**
 * Reset the default HTTP client.
 */
export function resetHttpClient(): void {
  defaultHttpClient = null
}
