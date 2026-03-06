/**
 * IPC Client for TypeScript TUI ↔ zero-cli communication.
 *
 * Provides a high-level API for the TUI to communicate with the Rust CLI backend.
 * Handles connection management, automatic reconnection, and event dispatch.
 */

import { spawn, type ChildProcess } from "child_process"
import { EventEmitter } from "events"
import { IpcProtocol, isSocketReady, waitForSocket } from "./protocol"
import type {
  InitializeParams,
  InitializeResult,
  ToolCallParams,
  ToolCallResult,
  GetSessionParams,
  SessionInfo,
  ListSessionsResult,
  CompactParams,
  CompactResult,
  ToolResultParams,
  IpcEvents,
  IpcEventName,
  SessionMessage,
  ToolInfo,
  AgentPromptParams,
  AgentPromptResult,
  AgentStreamNotification,
  AgentStreamEvent,
} from "./types"
import { IpcMethods, getDefaultSocketPath } from "./types"

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

export interface IpcClientOptions {
  /** Path to IPC socket (default: ~/.codecoder/ipc.sock) */
  socketPath?: string
  /** Path to zero-cli binary (default: "zero-cli") */
  cliBinary?: string
  /** Auto-start CLI if not running (default: true) */
  autoStart?: boolean
  /** Auto-reconnect on disconnect (default: true) */
  autoReconnect?: boolean
  /** Reconnect delay in ms (default: 1000) */
  reconnectDelay?: number
  /** Maximum reconnect attempts (default: 5) */
  maxReconnectAttempts?: number
  /** Request timeout in ms (default: 30000) */
  timeout?: number
}

type IpcEventHandler<T extends IpcEventName> = (...args: IpcEvents[T]) => void

// ══════════════════════════════════════════════════════════════════════════════
// IpcClient Class
// ══════════════════════════════════════════════════════════════════════════════

/**
 * High-level IPC client for TUI communication.
 *
 * Features:
 * - Auto-start CLI process if not running
 * - Auto-reconnect on disconnect
 * - Event-based notification handling
 * - Type-safe request/response methods
 */
export class IpcClient extends EventEmitter {
  private protocol: IpcProtocol
  private cliProcess: ChildProcess | null = null
  private options: Required<IpcClientOptions>
  private reconnectAttempts = 0
  private reconnecting = false

  // Session state
  private _sessionId: string | null = null
  private _tools: ToolInfo[] = []
  private _messages: SessionMessage[] = []

  constructor(options?: IpcClientOptions) {
    super()

    this.options = {
      socketPath: options?.socketPath ?? getDefaultSocketPath(),
      cliBinary: options?.cliBinary ?? "zero-cli",
      autoStart: options?.autoStart ?? true,
      autoReconnect: options?.autoReconnect ?? true,
      reconnectDelay: options?.reconnectDelay ?? 1000,
      maxReconnectAttempts: options?.maxReconnectAttempts ?? 5,
      timeout: options?.timeout ?? 30000,
    }

    this.protocol = new IpcProtocol({ timeout: this.options.timeout })
    this.setupProtocolListeners()
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Getters
  // ──────────────────────────────────────────────────────────────────────────

  /** Current session ID */
  get sessionId(): string | null {
    return this._sessionId
  }

  /** Available tools */
  get tools(): ToolInfo[] {
    return this._tools
  }

  /** Session messages */
  get messages(): SessionMessage[] {
    return this._messages
  }

  /** Whether connected to CLI */
  get connected(): boolean {
    return this.protocol.connected
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Connection Management
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Connect to the IPC server.
   * If autoStart is enabled and server is not running, starts the CLI.
   */
  async connect(): Promise<void> {
    // Check if server is already running
    const serverReady = await isSocketReady(this.options.socketPath)

    if (!serverReady && this.options.autoStart) {
      await this.startCli()
    }

    await this.protocol.connect(this.options.socketPath)
    this.reconnectAttempts = 0
    this.emit("connected")
  }

  /** Start the CLI process */
  private async startCli(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.cliProcess = spawn(this.options.cliBinary, ["serve-ipc", "--socket", this.options.socketPath], {
        stdio: ["ignore", "pipe", "pipe"],
        detached: false,
      })

      this.cliProcess.on("error", (err) => {
        reject(new Error(`Failed to start CLI: ${err.message}`))
      })

      this.cliProcess.on("exit", (code) => {
        if (code !== 0) {
          this.emit("error", { code: -32603, message: `CLI exited with code ${code}` })
        }
        this.cliProcess = null
      })

      // Wait for socket to become available
      waitForSocket(this.options.socketPath, { timeout: 10000 })
        .then(resolve)
        .catch(reject)
    })
  }

  /** Handle reconnection */
  private async reconnect(): Promise<void> {
    if (this.reconnecting) return
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.emit("error", {
        code: -32603,
        message: `Max reconnect attempts (${this.options.maxReconnectAttempts}) exceeded`,
      })
      return
    }

    this.reconnecting = true
    this.reconnectAttempts++

    await new Promise((r) => setTimeout(r, this.options.reconnectDelay))

    try {
      await this.connect()
      this.reconnecting = false

      // Re-initialize session if we had one
      if (this._sessionId) {
        await this.initialize({ sessionId: this._sessionId, cwd: process.cwd() })
      }
    } catch {
      this.reconnecting = false
      // Will retry on next disconnect event
    }
  }

  /** Close the connection and stop CLI if we started it */
  async close(): Promise<void> {
    this.options.autoReconnect = false // Prevent reconnection
    this.protocol.close()

    if (this.cliProcess) {
      this.cliProcess.kill("SIGTERM")
      this.cliProcess = null
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Protocol Event Handling
  // ──────────────────────────────────────────────────────────────────────────

  private setupProtocolListeners(): void {
    this.protocol.on("session_update", (notification) => {
      if (notification.message) {
        this._messages = [...this._messages, notification.message]
      }
      this.emit("session_update", notification)
    })

    this.protocol.on("tool_request", (notification) => {
      this.emit("tool_request", notification)
    })

    this.protocol.on("llm_request", (notification) => {
      this.emit("llm_request", notification)
    })

    this.protocol.on("stream_token", (notification) => {
      this.emit("stream_token", notification)
    })

    this.protocol.on("error", (notification) => {
      this.emit("error", notification)
    })

    this.protocol.on("disconnected", () => {
      this.emit("disconnected")
      if (this.options.autoReconnect) {
        this.reconnect()
      }
    })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // API Methods
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Initialize a session.
   * Creates a new session or resumes an existing one.
   */
  async initialize(options: { sessionId?: string; cwd?: string } = {}): Promise<InitializeResult> {
    const params: InitializeParams = {
      sessionId: options.sessionId,
      cwd: options.cwd ?? process.cwd(),
      clientInfo: {
        name: "ccode-tui",
        version: "1.0.0", // TODO: Get from package.json
      },
    }

    const result = await this.protocol.request<InitializeResult>(IpcMethods.INITIALIZE, params)

    this._sessionId = result.sessionId
    this._tools = result.tools
    this._messages = result.messages

    return result
  }

  /**
   * Execute a tool.
   */
  async callTool(name: string, args: Record<string, unknown>): Promise<ToolCallResult> {
    const callId = `call-${Date.now()}-${Math.random().toString(36).slice(2)}`

    const params: ToolCallParams = {
      callId,
      name,
      args,
    }

    return await this.protocol.request<ToolCallResult>(IpcMethods.TOOL_CALL, params)
  }

  /**
   * Send tool result back to CLI.
   * Used when the TUI executes a tool via LLM.
   */
  sendToolResult(callId: string, result: unknown, error?: string): void {
    const params: ToolResultParams = {
      callId,
      result,
      error,
    }

    this.protocol.notify(IpcMethods.TOOL_RESULT, params)
  }

  /**
   * Get session details.
   */
  async getSession(sessionId: string): Promise<SessionInfo> {
    const params: GetSessionParams = { sessionId }
    return await this.protocol.request<SessionInfo>(IpcMethods.GET_SESSION, params)
  }

  /**
   * List all sessions.
   */
  async listSessions(): Promise<ListSessionsResult> {
    return await this.protocol.request<ListSessionsResult>(IpcMethods.LIST_SESSIONS)
  }

  /**
   * Compact session history.
   */
  async compact(sessionId?: string): Promise<CompactResult> {
    const params: CompactParams = {
      sessionId: sessionId ?? this._sessionId ?? "",
    }
    return await this.protocol.request<CompactResult>(IpcMethods.COMPACT, params)
  }

  /**
   * Cancel ongoing generation.
   */
  async cancelGeneration(): Promise<void> {
    await this.protocol.request(IpcMethods.CANCEL_GENERATION)
  }

  /**
   * Ping the server.
   */
  async ping(): Promise<boolean> {
    try {
      await this.protocol.request(IpcMethods.PING)
      return true
    } catch {
      return false
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Agent Prompt (Phase 6.1)
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Tool executor callback type.
   * Called when the LLM requests a tool execution.
   * Should return the tool output or throw an error.
   */
  private _toolExecutor:
    | ((name: string, args: Record<string, unknown>) => Promise<{ output: string; isError?: boolean }>)
    | null = null

  /**
   * Set the tool executor for agent prompt tool callbacks.
   */
  setToolExecutor(
    executor: (name: string, args: Record<string, unknown>) => Promise<{ output: string; isError?: boolean }>
  ): void {
    this._toolExecutor = executor
  }

  /**
   * Send an agent prompt and stream the response.
   *
   * This method:
   * 1. Sends the prompt to the Rust backend
   * 2. Streams text deltas, reasoning, and tool calls
   * 3. Executes tool calls via the registered tool executor
   * 4. Sends tool results back to continue the loop
   *
   * @param params Agent prompt parameters
   * @returns Async generator yielding stream events
   */
  async *agentPrompt(params: AgentPromptParams): AsyncGenerator<AgentStreamEvent, void, unknown> {
    // Send the prompt request
    const result = await this.protocol.request<AgentPromptResult>(IpcMethods.AGENT_PROMPT, params)
    const requestId = result.requestId

    // Create a promise that resolves when we receive each event
    const eventQueue: AgentStreamEvent[] = []
    let resolveNext: ((value: AgentStreamEvent | null) => void) | null = null
    let finished = false

    // Handler for agent stream events
    const handleAgentStream = (notification: AgentStreamNotification) => {
      if (notification.requestId !== requestId) return

      const event = notification.event
      eventQueue.push(event)

      // Check if this is a finish or error event
      if (event.type === "finish" || event.type === "error") {
        finished = true
      }

      // If there's a waiting consumer, resolve with the event
      if (resolveNext) {
        const resolve = resolveNext
        resolveNext = null
        resolve(eventQueue.shift() ?? null)
      }
    }

    // Subscribe to events
    this.on("agent_stream", handleAgentStream)

    try {
      while (!finished || eventQueue.length > 0) {
        // Get next event
        let event: AgentStreamEvent | null = eventQueue.shift() ?? null

        if (!event) {
          // Wait for next event
          event = await new Promise<AgentStreamEvent | null>((resolve) => {
            resolveNext = resolve

            // Timeout after 5 minutes
            setTimeout(() => {
              if (resolveNext === resolve) {
                resolveNext = null
                resolve(null)
              }
            }, 300000)
          })
        }

        if (!event) break

        // Handle tool calls - execute and send results back
        if (event.type === "tool_call" && this._toolExecutor) {
          const toolCallId = event.id
          const toolName = event.name
          const toolArgs = (event.arguments ?? {}) as Record<string, unknown>

          try {
            const result = await this._toolExecutor(toolName, toolArgs)

            // Send tool result back
            const resultParams: ToolResultParams = {
              callId: toolCallId,
              result: result.output,
              error: result.isError ? result.output : undefined,
            }
            this.protocol.notify(IpcMethods.TOOL_RESULT, resultParams)

            // Yield the tool call event
            yield event
          } catch (error) {
            // Send error as tool result
            const errorMsg = error instanceof Error ? error.message : String(error)
            const resultParams: ToolResultParams = {
              callId: toolCallId,
              result: errorMsg,
              error: errorMsg,
            }
            this.protocol.notify(IpcMethods.TOOL_RESULT, resultParams)

            // Yield the tool call event anyway
            yield event
          }
        } else {
          // Yield non-tool-call events
          yield event
        }

        // Check if we're done
        if (event.type === "finish" || event.type === "error") {
          break
        }
      }
    } finally {
      // Unsubscribe
      this.off("agent_stream", handleAgentStream)
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Type-safe Event Methods
  // ──────────────────────────────────────────────────────────────────────────

  override on<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.on(event, listener as (...args: unknown[]) => void)
  }

  override once<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.once(event, listener as (...args: unknown[]) => void)
  }

  override off<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.off(event, listener as (...args: unknown[]) => void)
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Factory Function
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Create and connect an IPC client.
 * Convenience function that creates a client and connects in one call.
 */
export async function createIpcClient(options?: IpcClientOptions): Promise<IpcClient> {
  const client = new IpcClient(options)
  await client.connect()
  return client
}
