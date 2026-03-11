/**
 * WebSocket Client for Rust Daemon
 *
 * Provides a thin wrapper around WebSocket with:
 * - Auto-reconnection
 * - Message type safety
 * - Request/response correlation
 * - Event subscriptions
 *
 * @module sdk/websocket
 */

import type {
  WsClientMessage,
  WsServerMessage,
  AgentRequest,
  AgentEventHandler,
  AgentStreamEvent,
  ConfirmationRequest,
} from "./types"

export interface WebSocketClientConfig {
  /** WebSocket URL (default: ws://127.0.0.1:4402/ws) */
  url?: string
  /** Auto-reconnect on disconnect (default: true) */
  autoReconnect?: boolean
  /** Reconnect delay in ms (default: 1000) */
  reconnectDelay?: number
  /** Max reconnect attempts (default: 10) */
  maxReconnectAttempts?: number
  /** Ping interval in ms (default: 30000) */
  pingInterval?: number
}

type MessageHandler = (msg: WsServerMessage) => void
type ConnectionHandler = () => void
type ErrorHandler = (error: Error) => void

/**
 * WebSocket client for communicating with the Rust daemon.
 *
 * @example
 * ```typescript
 * const client = new WebSocketClient({ url: "ws://127.0.0.1:4402/ws" })
 *
 * await client.connect()
 *
 * // Execute an agent with streaming
 * await client.executeAgent({
 *   session_id: "sess-1",
 *   agent: "build",
 *   message: "Help me fix this bug"
 * }, (event) => {
 *   if (event.type === "text") {
 *     process.stdout.write(event.content)
 *   }
 * })
 *
 * client.close()
 * ```
 */
export class WebSocketClient {
  private ws: WebSocket | null = null
  private readonly config: Required<WebSocketClientConfig>
  private connectionId: string | null = null
  private reconnectAttempts = 0
  private pingTimer: ReturnType<typeof setInterval> | null = null
  private isConnecting = false
  private isClosed = false

  // Event handlers
  private messageHandlers = new Set<MessageHandler>()
  private connectHandlers = new Set<ConnectionHandler>()
  private disconnectHandlers = new Set<ConnectionHandler>()
  private errorHandlers = new Set<ErrorHandler>()

  // Request tracking for correlation
  private pendingRequests = new Map<
    string,
    {
      resolve: (value: unknown) => void
      reject: (error: Error) => void
    }
  >()

  // Agent execution tracking
  private agentHandlers = new Map<string, AgentEventHandler>()

  // Confirmation handlers
  private confirmationHandlers = new Set<(req: ConfirmationRequest) => void>()

  constructor(config: WebSocketClientConfig = {}) {
    this.config = {
      url: config.url ?? "ws://127.0.0.1:4402/ws",
      autoReconnect: config.autoReconnect ?? true,
      reconnectDelay: config.reconnectDelay ?? 1000,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
      pingInterval: config.pingInterval ?? 30000,
    }
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Connection Management
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Connect to the WebSocket server.
   * Returns a promise that resolves when connected.
   */
  async connect(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return
    }

    if (this.isConnecting) {
      // Wait for existing connection attempt
      return new Promise((resolve, reject) => {
        const onConnect = () => {
          this.offConnect(onConnect)
          this.offError(onError)
          resolve()
        }
        const onError = (e: Error) => {
          this.offConnect(onConnect)
          this.offError(onError)
          reject(e)
        }
        this.onConnect(onConnect)
        this.onError(onError)
      })
    }

    this.isConnecting = true
    this.isClosed = false

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.config.url)

        this.ws.onopen = () => {
          this.isConnecting = false
          this.reconnectAttempts = 0
          this.startPing()
          // Note: we don't resolve here - wait for "connected" message
        }

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data as string, resolve)
        }

        this.ws.onerror = (event) => {
          this.isConnecting = false
          const error = new Error("WebSocket error")
          this.errorHandlers.forEach((h) => h(error))
          reject(error)
        }

        this.ws.onclose = () => {
          this.isConnecting = false
          this.stopPing()
          this.connectionId = null
          this.disconnectHandlers.forEach((h) => h())

          if (this.config.autoReconnect && !this.isClosed) {
            this.scheduleReconnect()
          }
        }
      } catch (error) {
        this.isConnecting = false
        reject(error)
      }
    })
  }

  /**
   * Close the WebSocket connection.
   */
  close(): void {
    this.isClosed = true
    this.stopPing()
    this.ws?.close()
    this.ws = null
    this.connectionId = null
  }

  /**
   * Check if connected.
   */
  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN
  }

  /**
   * Get the connection ID (assigned by server).
   */
  getConnectionId(): string | null {
    return this.connectionId
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Event Subscriptions
  // ──────────────────────────────────────────────────────────────────────────

  onMessage(handler: MessageHandler): void {
    this.messageHandlers.add(handler)
  }

  offMessage(handler: MessageHandler): void {
    this.messageHandlers.delete(handler)
  }

  onConnect(handler: ConnectionHandler): void {
    this.connectHandlers.add(handler)
  }

  offConnect(handler: ConnectionHandler): void {
    this.connectHandlers.delete(handler)
  }

  onDisconnect(handler: ConnectionHandler): void {
    this.disconnectHandlers.add(handler)
  }

  offDisconnect(handler: ConnectionHandler): void {
    this.disconnectHandlers.delete(handler)
  }

  onError(handler: ErrorHandler): void {
    this.errorHandlers.add(handler)
  }

  offError(handler: ErrorHandler): void {
    this.errorHandlers.delete(handler)
  }

  onConfirmation(handler: (req: ConfirmationRequest) => void): void {
    this.confirmationHandlers.add(handler)
  }

  offConfirmation(handler: (req: ConfirmationRequest) => void): void {
    this.confirmationHandlers.delete(handler)
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Agent Execution
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute an agent with streaming callbacks.
   *
   * @param request - Agent execution request
   * @param onEvent - Callback for streaming events
   * @returns Promise that resolves when agent completes
   */
  async executeAgent(
    request: Omit<AgentRequest, "type" | "id">,
    onEvent?: AgentEventHandler
  ): Promise<{ reason: string; usage?: import("./types").TokenUsage }> {
    const id = `agent-${Date.now()}-${Math.random().toString(36).slice(2)}`

    if (onEvent) {
      this.agentHandlers.set(id, onEvent)
    }

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, {
        resolve: (value) => {
          this.agentHandlers.delete(id)
          resolve(value as { reason: string; usage?: import("./types").TokenUsage })
        },
        reject: (error) => {
          this.agentHandlers.delete(id)
          reject(error)
        },
      })

      this.send({
        type: "agent_request",
        id,
        ...request,
      })
    })
  }

  /**
   * Cancel an ongoing agent execution.
   */
  cancelAgent(id: string): void {
    this.send({ type: "agent_cancel", id })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Tool Execution
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Execute a tool directly.
   */
  async executeTool(tool: string, params: Record<string, unknown>): Promise<unknown> {
    const id = `tool-${Date.now()}-${Math.random().toString(36).slice(2)}`

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject })
      this.send({
        type: "tool_request",
        id,
        tool,
        params,
      })
    })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Confirmations
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Respond to a confirmation request.
   */
  respondToConfirmation(id: string, approved: boolean, comment?: string): void {
    this.send({
      type: "confirmation_response",
      id,
      approved,
      comment,
    })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Subscriptions
  // ──────────────────────────────────────────────────────────────────────────

  subscribeToSession(sessionId: string): void {
    this.send({ type: "session_subscribe", session_id: sessionId })
  }

  unsubscribeFromSession(sessionId: string): void {
    this.send({ type: "session_unsubscribe", session_id: sessionId })
  }

  subscribeToObserver(): void {
    this.send({ type: "observer_subscribe" })
  }

  unsubscribeFromObserver(): void {
    this.send({ type: "observer_unsubscribe" })
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Internal
  // ──────────────────────────────────────────────────────────────────────────

  private send(msg: WsClientMessage): void {
    if (!this.isConnected) {
      throw new Error("WebSocket not connected")
    }
    this.ws!.send(JSON.stringify(msg))
  }

  private handleMessage(data: string, connectResolve?: (value: void) => void): void {
    let msg: WsServerMessage
    try {
      msg = JSON.parse(data)
    } catch {
      return
    }

    // Notify all message handlers
    this.messageHandlers.forEach((h) => h(msg))

    // Handle specific message types
    switch (msg.type) {
      case "connected":
        this.connectionId = msg.connection_id
        this.connectHandlers.forEach((h) => h())
        connectResolve?.()
        break

      case "pong":
        // Keepalive response, no action needed
        break

      case "agent_start":
        this.notifyAgentEvent(msg.id, { type: "start", id: msg.id })
        break

      case "agent_text":
        this.notifyAgentEvent(msg.id, { type: "text", id: msg.id, content: msg.content })
        break

      case "agent_reasoning":
        this.notifyAgentEvent(msg.id, { type: "reasoning", id: msg.id, content: msg.content })
        break

      case "agent_tool_call":
        this.notifyAgentEvent(msg.id, {
          type: "tool_call",
          id: msg.id,
          toolCallId: msg.tool_call_id,
          tool: msg.tool,
          arguments: msg.arguments,
        })
        break

      case "agent_tool_result":
        this.notifyAgentEvent(msg.id, {
          type: "tool_result",
          id: msg.id,
          toolCallId: msg.tool_call_id,
          output: msg.output,
          error: msg.error,
        })
        break

      case "agent_complete":
        this.notifyAgentEvent(msg.id, {
          type: "complete",
          id: msg.id,
          reason: msg.reason,
          usage: msg.usage,
        })
        this.resolveRequest(msg.id, { reason: msg.reason, usage: msg.usage })
        break

      case "agent_error":
        this.notifyAgentEvent(msg.id, {
          type: "error",
          id: msg.id,
          code: msg.code,
          message: msg.message,
        })
        this.rejectRequest(msg.id, new Error(`[${msg.code}] ${msg.message}`))
        break

      case "agent_cancelled":
        this.notifyAgentEvent(msg.id, { type: "cancelled", id: msg.id })
        this.rejectRequest(msg.id, new Error("Agent cancelled"))
        break

      case "tool_response":
        if (msg.success) {
          this.resolveRequest(msg.id, msg.result)
        } else {
          this.rejectRequest(msg.id, new Error(msg.error ?? "Tool execution failed"))
        }
        break

      case "confirmation_request":
        this.confirmationHandlers.forEach((h) => h(msg))
        break

      case "error":
        this.errorHandlers.forEach((h) => h(new Error(`[${msg.code}] ${msg.message}`)))
        break
    }
  }

  private notifyAgentEvent(id: string, event: AgentStreamEvent): void {
    const handler = this.agentHandlers.get(id)
    handler?.(event)
  }

  private resolveRequest(id: string, value: unknown): void {
    const pending = this.pendingRequests.get(id)
    if (pending) {
      this.pendingRequests.delete(id)
      pending.resolve(value)
    }
  }

  private rejectRequest(id: string, error: Error): void {
    const pending = this.pendingRequests.get(id)
    if (pending) {
      this.pendingRequests.delete(id)
      pending.reject(error)
    }
  }

  private startPing(): void {
    this.stopPing()
    this.pingTimer = setInterval(() => {
      if (this.isConnected) {
        this.send({ type: "ping" })
      }
    }, this.config.pingInterval)
  }

  private stopPing(): void {
    if (this.pingTimer) {
      clearInterval(this.pingTimer)
      this.pingTimer = null
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.errorHandlers.forEach((h) =>
        h(new Error(`Max reconnect attempts (${this.config.maxReconnectAttempts}) reached`))
      )
      return
    }

    this.reconnectAttempts++
    const delay = this.config.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)

    setTimeout(() => {
      if (!this.isClosed) {
        this.connect().catch(() => {
          // Error will be handled by onError handlers
        })
      }
    }, delay)
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Singleton Instance
// ══════════════════════════════════════════════════════════════════════════════

let defaultClient: WebSocketClient | null = null

/**
 * Get the default WebSocket client singleton.
 */
export function getWebSocketClient(config?: WebSocketClientConfig): WebSocketClient {
  if (!defaultClient) {
    defaultClient = new WebSocketClient(config)
  }
  return defaultClient
}

/**
 * Reset the default WebSocket client.
 */
export function resetWebSocketClient(): void {
  defaultClient?.close()
  defaultClient = null
}
