/**
 * Agent Bridge SDK
 *
 * Provides a bridge between the deprecated TypeScript Agent module and the
 * new Rust daemon API. This allows existing code to gradually migrate without
 * breaking changes.
 *
 * ## Usage
 *
 * ```typescript
 * import { AgentBridge } from "@/sdk/agent-bridge"
 *
 * // Initialize with daemon URL (defaults to http://localhost:4402)
 * const bridge = new AgentBridge()
 *
 * // List agents (same interface as deprecated Agent.list())
 * const agents = await bridge.list()
 *
 * // Get specific agent
 * const build = await bridge.get("build")
 *
 * // Execute agent with streaming
 * await bridge.execute({
 *   sessionId: "session-123",
 *   agent: "build",
 *   message: "Hello",
 *   onEvent: (event) => console.log(event)
 * })
 * ```
 *
 * ## Migration Path
 *
 * 1. Replace `Agent.list()` with `bridge.list()`
 * 2. Replace `Agent.get(name)` with `bridge.get(name)`
 * 3. Replace direct LLM calls with `bridge.execute()`
 *
 * @see services/zero-cli/src/unified_api/agents.rs for Rust implementation
 */

import { Config } from "@/config/config"

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

export interface AgentInfo {
  name: string
  description?: string
  mode: "primary" | "subagent" | "all"
  temperature?: number
  color?: string
  hidden: boolean
  // New fields from Rust API
  model?: {
    provider_id: string
    model_id: string
    temperature?: number
    max_tokens?: number
  }
  permission?: {
    rules: Record<string, "allow" | "deny" | "ask" | Record<string, "allow" | "deny" | "ask">>
  }
  options?: Record<string, unknown>
}

/**
 * Permission rule in TS format (compatible with PermissionNext.Rule)
 */
export interface PermissionRule {
  permission: string
  pattern: string
  action: "allow" | "deny" | "ask"
}

/**
 * Convert Rust API permission format to TS Ruleset format
 *
 * Rust format: { rules: { "doom_loop": "ask", "read": { "*.env": "ask" } } }
 * TS format: [{ permission: "doom_loop", pattern: "*", action: "ask" }, ...]
 */
export function convertPermissionToRuleset(
  permission: AgentInfo["permission"]
): PermissionRule[] {
  if (!permission?.rules) return []

  const ruleset: PermissionRule[] = []

  for (const [key, value] of Object.entries(permission.rules)) {
    if (typeof value === "string") {
      // Simple action: { "doom_loop": "ask" }
      ruleset.push({
        permission: key,
        pattern: "*",
        action: value as "allow" | "deny" | "ask",
      })
    } else if (typeof value === "object") {
      // Nested patterns: { "read": { "*.env": "ask", "*": "allow" } }
      for (const [pattern, action] of Object.entries(value)) {
        ruleset.push({
          permission: key,
          pattern,
          action: action as "allow" | "deny" | "ask",
        })
      }
    }
  }

  return ruleset
}

/**
 * Type compatible with Agent.Info from @/agent/agent
 * Used as the return type for toAgentInfo conversion
 *
 * @deprecated Use this type during migration. Will be renamed to AgentInfo
 * once @/agent/agent is removed.
 */
export interface ConvertedAgentInfo {
  name: string
  description?: string
  mode: "primary" | "subagent" | "all"
  native?: boolean
  hidden?: boolean
  topP?: number
  temperature?: number
  color?: string
  permission: PermissionRule[]
  model?: { providerID: string; modelID: string }
  prompt?: string
  options: Record<string, unknown>
  steps?: number
  autoApprove?: Record<string, unknown>
  observerCapability?: {
    canWatch: ("code" | "world" | "self" | "meta")[]
    contributeToConsensus: boolean
    reportToMeta: boolean
  }
}

/**
 * Type alias for Agent.Info compatibility
 * Import this instead of Agent.Info from @/agent/agent
 */
export type AgentInfoType = ConvertedAgentInfo

/**
 * Convert AgentInfo from Rust API to Agent.Info format expected by LLM.stream
 *
 * Field mappings:
 * - permission: { rules: Record } → PermissionRule[]
 * - model: { provider_id, model_id } → { providerID, modelID }
 */
export function toAgentInfo(agent: AgentInfo): ConvertedAgentInfo {
  return {
    name: agent.name,
    description: agent.description,
    mode: agent.mode,
    temperature: agent.temperature,
    color: agent.color,
    hidden: agent.hidden,
    permission: convertPermissionToRuleset(agent.permission),
    model: agent.model
      ? { providerID: agent.model.provider_id, modelID: agent.model.model_id }
      : undefined,
    options: agent.options ?? {},
  }
}

export interface AgentListResponse {
  success: boolean
  agents: AgentInfo[]
  total: number
}

export interface AgentDetailResponse {
  success: boolean
  agent: AgentInfo
}

export interface AgentPromptResponse {
  success: boolean
  name: string
  prompt: string
  modifiedAt?: string
}

export interface StreamEvent {
  type:
    | "start"
    | "text_delta"
    | "reasoning_delta"
    | "tool_call_start"
    | "tool_call_delta"
    | "tool_call"
    | "finish"
    | "error"
  content?: string
  id?: string
  name?: string
  arguments?: unknown
  argumentsDelta?: string
  reason?: string
  usage?: {
    inputTokens: number
    outputTokens: number
    reasoningTokens?: number
  }
  code?: number
  message?: string
}

export interface ExecuteOptions {
  sessionId: string
  agent: string
  message: string
  system?: string[]
  temperature?: number
  maxTokens?: number
  model?: string
  maxIterations?: number
  toolTimeout?: number
  onEvent?: (event: StreamEvent) => void
}

// ══════════════════════════════════════════════════════════════════════════════
// Agent Bridge Implementation
// ══════════════════════════════════════════════════════════════════════════════

export class AgentBridge {
  private baseUrl: string
  private wsUrl: string

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || "http://localhost:4402"
    this.wsUrl = this.baseUrl.replace(/^http/, "ws") + "/ws"
  }

  /**
   * Initialize bridge with config-based URL
   */
  static async create(): Promise<AgentBridge> {
    const cfg = await Config.get()
    // Use daemon port from config, falling back to default 4402
    const port = (cfg.ports as Record<string, { port?: number }> | undefined)?.daemon?.port || 4402
    const baseUrl = `http://localhost:${port}`
    return new AgentBridge(baseUrl)
  }

  /**
   * Check if the daemon is running
   */
  async isHealthy(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/health`, {
        signal: AbortSignal.timeout(2000),
      })
      return res.ok
    } catch {
      return false
    }
  }

  /**
   * List all visible agents
   * Compatible with: Agent.list()
   */
  async list(): Promise<AgentInfo[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/agents`)
    if (!res.ok) {
      throw new Error(`Failed to list agents: ${res.status} ${res.statusText}`)
    }
    const data: AgentListResponse = await res.json()
    return data.agents
  }

  /**
   * Get a specific agent by name
   * Compatible with: Agent.get(name)
   */
  async get(name: string): Promise<AgentInfo | undefined> {
    const res = await fetch(`${this.baseUrl}/api/v1/agents/${encodeURIComponent(name)}`)
    if (res.status === 404) {
      return undefined
    }
    if (!res.ok) {
      throw new Error(`Failed to get agent: ${res.status} ${res.statusText}`)
    }
    const data: AgentDetailResponse = await res.json()
    return data.agent
  }

  /**
   * Get agent permission as TS Ruleset format
   * Compatible with: agent.permission in TS Agent module
   */
  async getPermissionRuleset(name: string): Promise<PermissionRule[]> {
    const agent = await this.get(name)
    if (!agent) return []
    return convertPermissionToRuleset(agent.permission)
  }

  /**
   * Get the system prompt for an agent
   */
  async getPrompt(name: string): Promise<string | undefined> {
    const res = await fetch(`${this.baseUrl}/api/v1/agents/${encodeURIComponent(name)}/prompt`)
    if (res.status === 404) {
      return undefined
    }
    if (!res.ok) {
      throw new Error(`Failed to get prompt: ${res.status} ${res.statusText}`)
    }
    const data: AgentPromptResponse = await res.json()
    return data.prompt
  }

  /**
   * Get the default agent name
   * Compatible with: Agent.defaultAgent()
   */
  async defaultAgent(): Promise<string> {
    const agents = await this.list()
    const primary = agents.find((a) => a.mode === "primary" && !a.hidden)
    if (!primary) {
      throw new Error("No primary visible agent found")
    }
    return primary.name
  }

  /**
   * Execute an agent with streaming response
   * This is the main entry point for running agent tasks.
   */
  async execute(options: ExecuteOptions): Promise<void> {
    const {
      sessionId,
      agent,
      message,
      system = [],
      temperature,
      maxTokens,
      model,
      maxIterations = 10,
      toolTimeout = 30,
      onEvent,
    } = options

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(this.wsUrl)

      ws.onopen = () => {
        // Send agent dispatch request
        ws.send(
          JSON.stringify({
            type: "agent_dispatch",
            session_id: sessionId,
            agent,
            message,
            system,
            temperature,
            max_tokens: maxTokens,
            model,
            max_iterations: maxIterations,
            tool_timeout: toolTimeout,
          })
        )
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as StreamEvent
          onEvent?.(data)

          // Check for completion
          if (data.type === "finish" || data.type === "error") {
            ws.close()
            if (data.type === "error") {
              reject(new Error(data.message || "Agent execution failed"))
            } else {
              resolve()
            }
          }
        } catch (err) {
          console.error("Failed to parse WebSocket message:", err)
        }
      }

      ws.onerror = (error) => {
        reject(new Error(`WebSocket error: ${error}`))
      }

      ws.onclose = (event) => {
        if (!event.wasClean) {
          reject(new Error(`WebSocket closed unexpectedly: ${event.code}`))
        }
      }
    })
  }

  /**
   * Execute an agent and collect the full response (non-streaming)
   */
  async executeSync(options: Omit<ExecuteOptions, "onEvent">): Promise<{
    text: string
    toolCalls: Array<{ id: string; name: string; arguments: unknown }>
    usage?: StreamEvent["usage"]
  }> {
    let text = ""
    const toolCalls: Array<{ id: string; name: string; arguments: unknown }> = []
    let usage: StreamEvent["usage"] | undefined

    await this.execute({
      ...options,
      onEvent: (event) => {
        switch (event.type) {
          case "text_delta":
            text += event.content || ""
            break
          case "tool_call":
            if (event.id && event.name) {
              toolCalls.push({
                id: event.id,
                name: event.name,
                arguments: event.arguments,
              })
            }
            break
          case "finish":
            usage = event.usage
            break
        }
      },
    })

    return { text, toolCalls, usage }
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Singleton Instance
// ══════════════════════════════════════════════════════════════════════════════

let _instance: AgentBridge | null = null

/**
 * Get the shared AgentBridge instance
 */
export async function getAgentBridge(): Promise<AgentBridge> {
  if (!_instance) {
    _instance = await AgentBridge.create()
  }
  return _instance
}

/**
 * Reset the shared instance (for testing)
 */
export function resetAgentBridge(): void {
  _instance = null
}
