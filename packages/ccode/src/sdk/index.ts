/**
 * Rust Daemon SDK
 *
 * A lightweight TypeScript SDK for communicating with the zero-cli Rust daemon.
 * Provides both WebSocket (real-time) and HTTP (REST) interfaces.
 *
 * ## Quick Start
 *
 * ### WebSocket (Recommended for Agent Execution)
 *
 * ```typescript
 * import { getWebSocketClient } from "./sdk"
 *
 * const ws = getWebSocketClient({ url: "ws://127.0.0.1:4402/ws" })
 * await ws.connect()
 *
 * // Execute an agent with streaming
 * await ws.executeAgent(
 *   {
 *     session_id: "my-session",
 *     agent: "build",
 *     message: "Help me implement this feature",
 *   },
 *   (event) => {
 *     if (event.type === "text") {
 *       process.stdout.write(event.content)
 *     }
 *   }
 * )
 * ```
 *
 * ### HTTP Client (for Simple Requests)
 *
 * ```typescript
 * import { getHttpClient } from "./sdk"
 *
 * const http = getHttpClient({ baseUrl: "http://127.0.0.1:4402" })
 *
 * // List agents
 * const { agents } = await http.listAgents()
 *
 * // Get observer status
 * const { data } = await http.getObserverStatus()
 *
 * // Switch gear
 * await http.switchGear("S", "Need high autonomy mode")
 * ```
 *
 * @module sdk
 */

// Types
export * from "./types"

// WebSocket Client
export { WebSocketClient, getWebSocketClient, resetWebSocketClient } from "./websocket"
export type { WebSocketClientConfig } from "./websocket"

// HTTP Client
export { HttpClient, getHttpClient, resetHttpClient } from "./client"
export type { HttpClientConfig } from "./client"

// Adapter (for gradual migration from TS modules to SDK)
export {
  adaptSessionInfo,
  adaptSessionList,
  isSdkModeEnabled,
  configureAdapter,
  getAdapterConfig,
} from "./adapter"
export type { SessionInfoLegacy, AdapterConfig } from "./adapter"

// Agent Bridge (for migrating from deprecated Agent module)
export { AgentBridge, getAgentBridge, resetAgentBridge } from "./agent-bridge"
export type { AgentInfo as BridgeAgentInfo, StreamEvent, ExecuteOptions } from "./agent-bridge"

// NAPI Bindings (for direct Rust function calls)
export { NAPI } from "./napi"
export type {
  DailyEntryType,
  MemoryCategory,
  MemorySection,
  MemoryContext,
  MarkdownMemoryConfig,
} from "./napi"

// HITL Client (Human-in-the-Loop approval system)
export {
  HitLClient,
  HitLApiError,
  getHitLClient,
  createHitLClient,
  resetHitLClient,
  listPendingApprovals,
  approveRequest,
  rejectRequest,
  isHitLServiceHealthy,
  getApprovalTypeName,
  getRiskLevelColor,
  getRiskLevelIcon,
  getStatusDisplay,
  formatApprovalSummary,
} from "./hitl"
export type {
  RiskLevel,
  ApprovalType,
  ApprovalStatus,
  ApprovalRequest,
  CreateApprovalRequest,
  ApprovalResponse,
  ListPendingResponse,
  DecideRequest,
  HitLClientConfig,
} from "./hitl"

// ══════════════════════════════════════════════════════════════════════════════
// SDK Helper Functions
// These replace commonly used patterns from deprecated modules (agent/agent.ts,
// provider/provider.ts, session/index.ts) with SDK-based alternatives.
// ══════════════════════════════════════════════════════════════════════════════

import { HttpClient, getHttpClient as getClient } from "./client"
import type { AgentInfo, ProviderListResponseExtended } from "./types"

/**
 * Get the default agent name.
 * Reads from config or falls back to first available primary agent.
 *
 * @example
 * ```typescript
 * const defaultAgent = await getDefaultAgentName()
 * // Returns "build" or the configured default agent
 * ```
 */
export async function getDefaultAgentName(client?: HttpClient): Promise<string> {
  const http = client ?? getClient()
  const response = await http.listAgents()
  const primaryAgents = response.agents.filter(
    (a) => a.mode === "primary" && !a.hidden
  )
  if (primaryAgents.length === 0) {
    throw new Error("No primary visible agent found")
  }
  // Return first primary visible agent (typically "build")
  return primaryAgents[0].name
}

/**
 * Get agent info by name.
 *
 * @example
 * ```typescript
 * const agent = await getAgentByName("build")
 * console.log(agent?.description)
 * ```
 */
export async function getAgentByName(
  name: string,
  client?: HttpClient
): Promise<AgentInfo | undefined> {
  const http = client ?? getClient()
  try {
    const response = await http.getAgent(name)
    return response.agent
  } catch {
    return undefined
  }
}

/**
 * List all agents with optional filtering.
 *
 * @example
 * ```typescript
 * const agents = await listAgentsFiltered({ hidden: false, mode: "primary" })
 * ```
 */
export async function listAgentsFiltered(
  options?: {
    hidden?: boolean
    mode?: "primary" | "subagent" | "all"
  },
  client?: HttpClient
): Promise<AgentInfo[]> {
  const http = client ?? getClient()
  const response = await http.listAgents()
  let agents = response.agents

  if (options?.hidden !== undefined) {
    agents = agents.filter((a) => a.hidden === options.hidden)
  }
  if (options?.mode && options.mode !== "all") {
    agents = agents.filter((a) => a.mode === options.mode)
  }

  return agents
}

/**
 * Get provider info with models.
 *
 * @example
 * ```typescript
 * const { providers, defaultModel } = await getProviderInfo()
 * const anthropic = providers.find(p => p.id === "anthropic")
 * ```
 */
export async function getProviderInfo(
  client?: HttpClient
): Promise<{
  providers: ProviderListResponseExtended["all"]
  defaultModel: Record<string, string>
  connected: string[]
}> {
  const http = client ?? getClient()
  const response = await http.listProviders()
  return {
    providers: response.all,
    defaultModel: response.default,
    connected: response.connected,
  }
}

/**
 * Check if a provider is connected (has valid credentials).
 *
 * @example
 * ```typescript
 * const hasAnthropic = await isProviderConnected("anthropic")
 * ```
 */
export async function isProviderConnected(
  providerId: string,
  client?: HttpClient
): Promise<boolean> {
  const http = client ?? getClient()
  const response = await http.listProviders()
  return response.connected.includes(providerId)
}

// Memory Adapter (for future migration - not currently used by consumers)
// Consumers still use @/memory-markdown directly
// See docs/progress/2026-03-11-rust-migration.md for migration plan

// ══════════════════════════════════════════════════════════════════════════════
// WebSocket-to-Bus Bridge
// Allows TUI to use WebSocket executeAgent() while receiving Bus events
// ══════════════════════════════════════════════════════════════════════════════

import { getWebSocketClient, WebSocketClient } from "./websocket"
import type { AgentStreamEvent } from "./types"

/**
 * Input format matching LocalSession.PromptInput
 */
export interface WebSocketPromptInput {
  sessionID: string
  agent?: string
  model?: string
  variant?: string
  parts: Array<{ type: string; text?: string; url?: string; filename?: string; mime?: string }>
}

/**
 * Event emitter interface for publishing Bus-compatible events
 */
export interface BusEventPublisher {
  publishPartUpdated: (part: unknown, delta?: string) => void
  publishMessageUpdated: (info: unknown) => void
  publishStepStart: (part: unknown) => void
  publishStepFinish: (part: unknown) => void
  publishError: (error: { code: string; message: string }) => void
}

/**
 * Create message part factories for generating Bus-compatible parts
 */
function createPartFactories(sessionID: string, messageID: string) {
  let partCounter = 0
  const nextPartId = () => `part-${messageID}-${++partCounter}`

  return {
    textPart: (text: string, delta?: string) => ({
      id: delta ? `text-${messageID}` : nextPartId(), // Reuse ID for streaming
      sessionID,
      messageID,
      type: "text" as const,
      text,
      time: { start: Date.now() },
    }),

    reasoningPart: (text: string, delta?: string) => ({
      id: delta ? `reasoning-${messageID}` : nextPartId(),
      sessionID,
      messageID,
      type: "reasoning" as const,
      text,
      time: { start: Date.now() },
    }),

    toolPartPending: (callID: string, tool: string, input: Record<string, unknown>) => ({
      id: nextPartId(),
      sessionID,
      messageID,
      type: "tool" as const,
      callID,
      tool,
      state: {
        status: "pending" as const,
        input,
        raw: JSON.stringify(input),
      },
    }),

    toolPartRunning: (callID: string, tool: string, input: Record<string, unknown>) => ({
      id: `tool-${callID}`,
      sessionID,
      messageID,
      type: "tool" as const,
      callID,
      tool,
      state: {
        status: "running" as const,
        input,
        title: tool,
        time: { start: Date.now() },
      },
    }),

    toolPartCompleted: (
      callID: string,
      tool: string,
      input: Record<string, unknown>,
      output: string
    ) => ({
      id: `tool-${callID}`,
      sessionID,
      messageID,
      type: "tool" as const,
      callID,
      tool,
      state: {
        status: "completed" as const,
        input,
        output,
        title: tool,
        metadata: {},
        time: { start: Date.now(), end: Date.now() },
      },
    }),

    toolPartError: (
      callID: string,
      tool: string,
      input: Record<string, unknown>,
      error: string
    ) => ({
      id: `tool-${callID}`,
      sessionID,
      messageID,
      type: "tool" as const,
      callID,
      tool,
      state: {
        status: "error" as const,
        input,
        error,
        time: { start: Date.now(), end: Date.now() },
      },
    }),

    stepStartPart: (snapshot?: string) => ({
      id: nextPartId(),
      sessionID,
      messageID,
      type: "step-start" as const,
      snapshot,
    }),

    stepFinishPart: (reason: string, tokens: { input: number; output: number }) => ({
      id: nextPartId(),
      sessionID,
      messageID,
      type: "step-finish" as const,
      reason,
      cost: 0,
      tokens: {
        input: tokens.input,
        output: tokens.output,
        reasoning: 0,
        cache: { read: 0, write: 0 },
      },
    }),
  }
}

/**
 * Execute agent via WebSocket and publish events to Bus.
 *
 * This bridges the WebSocket streaming API to the Bus event system,
 * allowing TUI components to work unchanged while using the Rust daemon.
 *
 * @param input - Prompt input in LocalSession.PromptInput format
 * @param publisher - Event publisher for Bus events
 * @param wsClient - Optional WebSocket client (uses singleton if not provided)
 *
 * @example
 * ```typescript
 * import { Bus } from "@/bus"
 * import { MessageV2 } from "@/session/message-v2"
 *
 * await promptViaWebSocket(
 *   {
 *     sessionID: "sess-123",
 *     agent: "build",
 *     parts: [{ type: "text", text: "Help me fix this bug" }],
 *   },
 *   {
 *     publishPartUpdated: (part, delta) =>
 *       Bus.publish(MessageV2.Event.PartUpdated, { part, delta }),
 *     publishMessageUpdated: (info) =>
 *       Bus.publish(MessageV2.Event.Updated, { info }),
 *     publishStepStart: (part) =>
 *       Bus.publish(MessageV2.Event.PartUpdated, { part }),
 *     publishStepFinish: (part) =>
 *       Bus.publish(MessageV2.Event.PartUpdated, { part }),
 *     publishError: (error) =>
 *       console.error("Agent error:", error),
 *   }
 * )
 * ```
 */
export async function promptViaWebSocket(
  input: WebSocketPromptInput,
  publisher: BusEventPublisher,
  wsClient?: WebSocketClient
): Promise<{ messageID: string; content?: string }> {
  const client = wsClient ?? getWebSocketClient()

  // Ensure WebSocket is connected
  if (!client.isConnected) {
    await client.connect()
  }

  // Generate message ID for this prompt
  const messageID = `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`

  // Create part factories with consistent IDs
  const parts = createPartFactories(input.sessionID, messageID)

  // Track tool calls for mapping results back
  const toolCalls = new Map<string, { tool: string; input: Record<string, unknown> }>()

  // Track accumulated text for return value
  let accumulatedText = ""
  let currentTextPart: ReturnType<typeof parts.textPart> | null = null
  let currentReasoningPart: ReturnType<typeof parts.reasoningPart> | null = null

  // Convert parts array to message string (simple text concatenation for now)
  const message = input.parts
    .filter((p) => p.type === "text" && p.text)
    .map((p) => p.text)
    .join("\n")

  // Execute agent with streaming
  const result = await client.executeAgent(
    {
      session_id: input.sessionID,
      agent: input.agent ?? "build", // Default to "build" if not specified
      message,
      model: input.model,
    },
    (event: AgentStreamEvent) => {
      switch (event.type) {
        case "start":
          // Publish step start
          publisher.publishStepStart(parts.stepStartPart())
          break

        case "text":
          // Stream text content
          accumulatedText += event.content
          if (!currentTextPart) {
            currentTextPart = parts.textPart(event.content, event.content)
          } else {
            currentTextPart = { ...currentTextPart, text: accumulatedText }
          }
          publisher.publishPartUpdated(currentTextPart, event.content)
          break

        case "reasoning":
          // Stream reasoning content
          if (!currentReasoningPart) {
            currentReasoningPart = parts.reasoningPart(event.content, event.content)
          } else {
            currentReasoningPart = {
              ...currentReasoningPart,
              text: currentReasoningPart.text + event.content,
            }
          }
          publisher.publishPartUpdated(currentReasoningPart, event.content)
          break

        case "tool_call":
          // Tool execution starting
          toolCalls.set(event.toolCallId, {
            tool: event.tool,
            input: event.arguments,
          })
          publisher.publishPartUpdated(
            parts.toolPartRunning(event.toolCallId, event.tool, event.arguments)
          )
          break

        case "tool_result":
          // Tool execution completed
          const call = toolCalls.get(event.toolCallId)
          if (call) {
            if (event.error) {
              publisher.publishPartUpdated(
                parts.toolPartError(event.toolCallId, call.tool, call.input, event.error)
              )
            } else {
              publisher.publishPartUpdated(
                parts.toolPartCompleted(
                  event.toolCallId,
                  call.tool,
                  call.input,
                  event.output ?? ""
                )
              )
            }
          }
          break

        case "complete":
          // Agent finished
          publisher.publishStepFinish(
            parts.stepFinishPart(event.reason, {
              input: event.usage?.input_tokens ?? 0,
              output: event.usage?.output_tokens ?? 0,
            })
          )
          break

        case "error":
          publisher.publishError({ code: String(event.code), message: event.message })
          break

        case "cancelled":
          publisher.publishError({ code: "CANCELLED", message: "Agent execution cancelled" })
          break
      }
    }
  )

  return {
    messageID,
    content: accumulatedText || undefined,
  }
}

/**
 * Create a Bus event publisher that integrates with the existing Bus system.
 *
 * Usage:
 * ```typescript
 * import { Bus } from "@/bus"
 * import { MessageV2 } from "@/session/message-v2"
 * import { createBusPublisher, promptViaWebSocket } from "@/sdk"
 *
 * const publisher = createBusPublisher(Bus, MessageV2.Event)
 * await promptViaWebSocket(input, publisher)
 * ```
 */
export function createBusPublisher(
  Bus: { publish: (event: { type: string }, data: unknown) => void },
  MessageEvent: {
    PartUpdated: { type: string }
    Updated: { type: string }
  }
): BusEventPublisher {
  return {
    publishPartUpdated: (part, delta) =>
      Bus.publish(MessageEvent.PartUpdated, { part, delta }),
    publishMessageUpdated: (info) =>
      Bus.publish(MessageEvent.Updated, { info }),
    publishStepStart: (part) =>
      Bus.publish(MessageEvent.PartUpdated, { part }),
    publishStepFinish: (part) =>
      Bus.publish(MessageEvent.PartUpdated, { part }),
    publishError: (error) => {
      // Errors are logged but not published as Bus events
      // The TUI handles errors via the agent_error callback
      console.error("[SDK] Agent error:", error.code, error.message)
    },
  }
}
