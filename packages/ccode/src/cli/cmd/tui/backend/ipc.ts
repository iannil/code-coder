/**
 * IPC Backend Adapter
 *
 * Implements the TuiBackend interface using JSON-RPC 2.0 over Unix Domain Socket
 * to communicate with the zero-cli Rust backend.
 *
 * ## How it works
 *
 * ```
 * ┌─────────────────────────────────────────────────────────────────────────────┐
 * │                         IpcBackend                                          │
 * │   ┌─────────────────────────────────────────────────────────────────────┐   │
 * │   │                      IpcClient                                       │   │
 * │   │  • Connects to ~/.codecoder/ipc.sock                                │   │
 * │   │  • Auto-starts zero-cli if not running                              │   │
 * │   │  • Auto-reconnects on disconnect                                    │   │
 * │   └─────────────────────────────────────────────────────────────────────┘   │
 * │                              ↕ JSON-RPC 2.0                                 │
 * │   ┌─────────────────────────────────────────────────────────────────────┐   │
 * │   │                    zero-cli serve-ipc                                │   │
 * │   │  • Tool execution                                                   │   │
 * │   │  • Session management                                               │   │
 * │   │  • LLM coordination (Phase 6.1)                                     │   │
 * │   └─────────────────────────────────────────────────────────────────────┘   │
 * └─────────────────────────────────────────────────────────────────────────────┘
 * ```
 *
 * ## Current Limitations
 *
 * This adapter provides a bridge layer for gradual migration. Currently:
 * - Some LocalAPI methods are not yet implemented in zero-cli
 * - The adapter returns mock/stub responses for unimplemented methods
 * - As zero-cli gains capabilities, these stubs will be replaced with real calls
 */

import {
  IpcClient,
  createIpcClient,
  type IpcClientOptions,
  type AgentPromptParams,
  type AgentStreamEvent,
  type AgentStreamNotification,
} from "@/ipc"
import { Log } from "@/util/log"
import type { Event } from "@/types"
import type { TuiBackend, EventSource, RpcClient, IpcBackendOptions } from "./index"

// ══════════════════════════════════════════════════════════════════════════════
// Event Mapping
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Maps IPC notifications to TUI Event format.
 *
 * The IPC protocol uses different notification types than the internal Bus events.
 * This function bridges the gap until zero-cli implements full event parity.
 */
function mapIpcNotificationToEvent(
  type: string,
  notification: unknown,
): Event | null {
  // Map known notification types to Event format
  switch (type) {
    case "session_update": {
      const n = notification as { sessionId: string; message?: unknown; tokenCount: number }
      return {
        type: "session.update" as Event["type"],
        properties: {
          sessionID: n.sessionId,
          message: n.message,
          tokenCount: n.tokenCount,
        },
      } as Event
    }
    case "stream_token": {
      const n = notification as { requestId: string; token: string; done: boolean }
      return {
        type: "session.stream" as Event["type"],
        properties: {
          requestID: n.requestId,
          token: n.token,
          done: n.done,
        },
      } as Event
    }
    case "tool_request": {
      const n = notification as { requestId: string; name: string; args: unknown }
      return {
        type: "tool.request" as Event["type"],
        properties: {
          requestID: n.requestId,
          name: n.name,
          args: n.args,
        },
      } as Event
    }
    case "agent_stream": {
      const n = notification as AgentStreamNotification
      return mapAgentStreamEvent(n)
    }
    case "error": {
      const n = notification as { code: number; message: string; data?: unknown }
      return {
        type: "error" as Event["type"],
        properties: {
          code: n.code,
          message: n.message,
          data: n.data,
        },
      } as Event
    }
    default:
      Log.Default.debug("Unknown IPC notification type", { type, notification })
      return null
  }
}

/**
 * Maps AgentStreamNotification to TUI Event format.
 *
 * Phase 6.1: Converts Rust agent loop events to TUI-compatible events.
 */
function mapAgentStreamEvent(notification: AgentStreamNotification): Event | null {
  const event = notification.event
  const requestId = notification.requestId

  switch (event.type) {
    case "start":
      return {
        type: "session.stream.start" as Event["type"],
        properties: { requestID: requestId },
      } as Event

    case "text_delta":
      return {
        type: "session.stream" as Event["type"],
        properties: {
          requestID: requestId,
          token: event.content,
          done: false,
        },
      } as Event

    case "reasoning_delta":
      return {
        type: "session.reasoning" as Event["type"],
        properties: {
          requestID: requestId,
          content: event.content,
        },
      } as Event

    case "tool_call_start":
      return {
        type: "tool.start" as Event["type"],
        properties: {
          requestID: requestId,
          callID: event.id,
          tool: event.name,
        },
      } as Event

    case "tool_call":
      return {
        type: "tool.call" as Event["type"],
        properties: {
          requestID: requestId,
          callID: event.id,
          tool: event.name,
          args: event.arguments,
        },
      } as Event

    case "tool_result":
      return {
        type: "tool.result" as Event["type"],
        properties: {
          requestID: requestId,
          callID: event.id,
          output: event.output,
          error: event.error,
        },
      } as Event

    case "finish":
      return {
        type: "session.stream.finish" as Event["type"],
        properties: {
          requestID: requestId,
          reason: event.reason,
          usage: event.usage,
        },
      } as Event

    case "error":
      return {
        type: "error" as Event["type"],
        properties: {
          code: event.code,
          message: event.message,
        },
      } as Event

    default:
      return null
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// IpcBackend Class
// ══════════════════════════════════════════════════════════════════════════════

/**
 * IPC Backend Implementation
 *
 * Wraps the IpcClient to provide the TuiBackend interface for zero-cli communication.
 */
export class IpcBackend implements TuiBackend {
  readonly mode = "ipc" as const
  private client: IpcClient
  private eventHandlers: Set<(event: Event) => void> = new Set()
  private _events: EventSource
  private _rpc: RpcClient

  constructor(client: IpcClient) {
    this.client = client

    // Set up notification handlers to map IPC events to TUI events
    this.setupNotificationHandlers()

    // Create EventSource adapter
    this._events = {
      on: (handler) => {
        this.eventHandlers.add(handler)
        return () => {
          this.eventHandlers.delete(handler)
        }
      },
    }

    // Create RpcClient adapter
    // Note: This bridges the Worker RPC format to IPC format
    this._rpc = {
      call: async (input) => {
        return this.handleRpcCall(input.namespace, input.method, input.args)
      },
      on: (_event, _handler) => {
        // IPC notifications are handled separately via setupNotificationHandlers
        // This method is kept for interface compatibility
        return () => {}
      },
    }
  }

  get events(): EventSource {
    return this._events
  }

  get rpc(): RpcClient {
    return this._rpc
  }

  private setupNotificationHandlers(): void {
    // Map IPC notifications to TUI events
    const notificationTypes = [
      "session_update",
      "tool_request",
      "llm_request",
      "stream_token",
      "agent_stream", // Phase 6.1: Agent loop streaming events
      "error",
    ] as const

    for (const type of notificationTypes) {
      this.client.on(type, (notification) => {
        const event = mapIpcNotificationToEvent(type, notification)
        if (event) {
          this.emitEvent(event)
        }
      })
    }

    // Handle connection events
    this.client.on("connected", () => {
      Log.Default.info("IPC backend connected to zero-cli")
    })

    this.client.on("disconnected", () => {
      Log.Default.warn("IPC backend disconnected from zero-cli")
    })
  }

  private emitEvent(event: Event): void {
    for (const handler of this.eventHandlers) {
      try {
        handler(event)
      } catch (err) {
        Log.Default.error("Error in event handler", { error: err })
      }
    }
  }

  /**
   * Handle RPC calls by mapping to IPC methods or providing stubs.
   *
   * As zero-cli implements more methods, this will route to actual IPC calls.
   * Currently, most methods return stub responses for development.
   */
  private async handleRpcCall(
    namespace: string,
    method: string,
    args: unknown[],
  ): Promise<unknown> {
    const fullMethod = `${namespace}.${method}`

    // Map known methods to IPC calls
    switch (fullMethod) {
      // Session methods
      case "session.list":
        return this.handleSessionList()
      case "session.get":
        return this.handleSessionGet(args[0] as { sessionID: string })
      case "session.compact":
        return this.handleSessionCompact(args[0] as { sessionID: string })

      // Agent methods (Phase 6.1)
      case "agent.prompt":
        return this.handleAgentPrompt(args[0] as AgentPromptParams)

      // Config methods (stub for now)
      case "config.get":
        return this.stubConfigGet()
      case "config.providers":
        return this.stubConfigProviders()

      // Path methods (local data)
      case "path.get":
        return {
          home: process.env.HOME || ".",
          state: `${process.env.HOME}/.codecoder`,
          config: `${process.env.HOME}/.codecoder`,
          worktree: process.cwd(),
          directory: process.cwd(),
        }

      // VCS methods (stub)
      case "vcs.get":
        return { branch: "main" }

      // App methods (stub)
      case "app.agents":
        return []
      case "app.skills":
        return []

      // Command methods (stub)
      case "command.list":
        return []

      // LSP methods (stub)
      case "lsp.status":
        return { status: "unavailable", reason: "IPC mode" }

      // Formatter methods (stub)
      case "formatter.status":
        return { status: "unavailable", reason: "IPC mode" }

      // MCP methods (stub)
      case "mcp.status":
        return { status: "unavailable", reason: "IPC mode" }
      case "mcp.resources":
        return { resources: [] }

      // Find methods (stub)
      case "find.files":
        return { files: [] }

      default:
        Log.Default.warn("Unimplemented IPC method", { namespace, method })
        throw new Error(`Method ${fullMethod} not implemented in IPC mode`)
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Session Method Handlers
  // ────────────────────────────────────────────────────────────────────────────

  private async handleSessionList(): Promise<unknown> {
    const result = await this.client.listSessions()
    return result.sessions.map((s) => ({
      id: s.sessionId,
      messageCount: s.messageCount,
      tokenCount: s.tokenCount,
      updatedAt: s.updatedAt,
    }))
  }

  private async handleSessionGet(input: { sessionID: string }): Promise<unknown> {
    const result = await this.client.getSession(input.sessionID)
    return {
      id: result.sessionId,
      messages: result.messages,
      tokenCount: result.tokenCount,
      messageCount: result.messageCount,
      createdAt: result.createdAt,
      updatedAt: result.updatedAt,
    }
  }

  private async handleSessionCompact(input: { sessionID: string }): Promise<unknown> {
    const result = await this.client.compact(input.sessionID)
    return {
      deletedCount: result.deletedCount,
      newTokenCount: result.newTokenCount,
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Agent Method Handlers (Phase 6.1)
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Handle agent prompt via Rust agent loop.
   *
   * This method sends the prompt to the Rust backend, which handles:
   * - LLM API calls with streaming
   * - Tool call parsing
   * - Stream event notifications
   *
   * Tool execution callbacks are handled via the IPC notification system.
   */
  private async handleAgentPrompt(params: AgentPromptParams): Promise<unknown> {
    Log.Default.info("Starting agent prompt via IPC", {
      sessionId: params.sessionId,
      agent: params.agent,
      model: params.model.modelId,
    })

    // The client.agentPrompt returns an async generator
    // For RPC compatibility, we collect events and return a summary
    // The actual streaming happens via agent_stream notifications

    const events: AgentStreamEvent[] = []
    let text = ""
    let usage = null

    for await (const event of this.client.agentPrompt(params)) {
      events.push(event)

      // Accumulate text
      if (event.type === "text_delta") {
        text += event.content
      }

      // Capture usage
      if (event.type === "finish" && event.usage) {
        usage = event.usage
      }
    }

    return {
      text,
      usage,
      eventCount: events.length,
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Stub Methods (to be replaced with real implementations)
  // ────────────────────────────────────────────────────────────────────────────

  private stubConfigGet(): unknown {
    return {
      model: "claude-3-5-sonnet-latest",
      theme: "dark",
    }
  }

  private stubConfigProviders(): unknown {
    return {
      providers: [],
      default: null,
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // TuiBackend Interface Methods
  // ────────────────────────────────────────────────────────────────────────────

  async reload(): Promise<void> {
    Log.Default.info("IPC backend reload requested")
    // In IPC mode, reload means reconnecting to zero-cli
    // The CLI handles its own state management
  }

  async shutdown(): Promise<void> {
    Log.Default.info("IPC backend shutting down")
    await this.client.close()
  }

  isConnected(): boolean {
    return this.client.connected
  }

  /**
   * Initialize a session via IPC.
   *
   * This should be called after creating the backend to set up the session.
   */
  async initializeSession(options: { sessionId?: string; cwd?: string } = {}): Promise<void> {
    await this.client.initialize(options)
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Factory Function
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Create an IPC backend instance.
 *
 * @param options - IPC configuration options
 * @returns Promise resolving to initialized IpcBackend
 *
 * @example
 * ```typescript
 * const backend = await createIpcBackend({
 *   socketPath: "~/.codecoder/ipc.sock",
 *   autoStart: true,
 * })
 *
 * // Initialize session
 * await backend.initializeSession({ cwd: process.cwd() })
 *
 * // Subscribe to events
 * backend.events.on((event) => {
 *   console.log("Event:", event.type)
 * })
 *
 * // Make API calls
 * const sessions = await backend.rpc.call({
 *   namespace: "session",
 *   method: "list",
 *   args: []
 * })
 *
 * // Cleanup
 * await backend.shutdown()
 * ```
 */
export async function createIpcBackend(options?: IpcBackendOptions): Promise<IpcBackend> {
  const clientOptions: IpcClientOptions = {
    socketPath: options?.socketPath,
    cliBinary: options?.cliBinary,
    autoStart: options?.autoStart ?? true,
    timeout: options?.timeout ?? 30000,
  }

  Log.Default.info("Creating IPC backend", { options: clientOptions })

  const client = await createIpcClient(clientOptions)
  return new IpcBackend(client)
}
