/**
 * WebSocket Client - Real-time communication with zero-api
 */

import z from "zod"
import { getClient } from "./index"

// ============================================================================
// WebSocket Message Types
// ============================================================================

/** WebSocket message types */
export const WsMessageType = z.enum([
  "ping",
  "pong",
  "tool_request",
  "tool_response",
  "stream",
  "error",
])

export type WsMessageType = z.infer<typeof WsMessageType>

/** Ping message */
export const WsPing = z.object({
  type: z.literal("ping"),
})

/** Pong message */
export const WsPong = z.object({
  type: z.literal("pong"),
})

/** Tool request message */
export const WsToolRequest = z.object({
  type: z.literal("tool_request"),
  id: z.string(),
  tool: z.string(),
  params: z.record(z.string(), z.unknown()),
})

export type WsToolRequest = z.infer<typeof WsToolRequest>

/** Tool response message */
export const WsToolResponse = z.object({
  type: z.literal("tool_response"),
  id: z.string(),
  success: z.boolean(),
  result: z.unknown().optional(),
  error: z.string().optional(),
})

export type WsToolResponse = z.infer<typeof WsToolResponse>

/** Stream message */
export const WsStream = z.object({
  type: z.literal("stream"),
  id: z.string(),
  content: z.string(),
  done: z.boolean(),
})

export type WsStream = z.infer<typeof WsStream>

/** Error message */
export const WsError = z.object({
  type: z.literal("error"),
  message: z.string(),
})

export type WsError = z.infer<typeof WsError>

/** All WebSocket message types */
export const WsMessage = z.discriminatedUnion("type", [
  WsPing,
  WsPong,
  WsToolRequest,
  WsToolResponse,
  WsStream,
  WsError,
])

export type WsMessage = z.infer<typeof WsMessage>

// ============================================================================
// WebSocket Client
// ============================================================================

export type WsEventHandler = (message: WsMessage) => void
export type WsErrorHandler = (error: Error) => void
export type WsCloseHandler = (code: number, reason: string) => void

export interface WsClientOptions {
  /** Auto-reconnect on disconnect */
  autoReconnect?: boolean
  /** Reconnect delay in milliseconds */
  reconnectDelay?: number
  /** Maximum reconnect attempts */
  maxReconnectAttempts?: number
  /** Ping interval in milliseconds */
  pingInterval?: number
}

/**
 * WebSocket client for zero-api
 */
export class ZeroWebSocketClient {
  private ws: WebSocket | null = null
  private url: string
  private options: Required<WsClientOptions>
  private reconnectAttempts = 0
  private pingIntervalId: ReturnType<typeof setInterval> | null = null
  private pendingRequests = new Map<string, { resolve: (value: unknown) => void; reject: (error: Error) => void }>()
  private streamHandlers = new Map<string, (content: string, done: boolean) => void>()
  private eventHandlers: WsEventHandler[] = []
  private errorHandlers: WsErrorHandler[] = []
  private closeHandlers: WsCloseHandler[] = []
  private connected = false

  constructor(url?: string, options: WsClientOptions = {}) {
    this.url = url ?? getClient().wsUrl
    this.options = {
      autoReconnect: options.autoReconnect ?? true,
      reconnectDelay: options.reconnectDelay ?? 1000,
      maxReconnectAttempts: options.maxReconnectAttempts ?? 5,
      pingInterval: options.pingInterval ?? 30000,
    }
  }

  /**
   * Check if connected
   */
  get isConnected(): boolean {
    return this.connected && this.ws?.readyState === WebSocket.OPEN
  }

  /**
   * Connect to the WebSocket server
   */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        resolve()
        return
      }

      this.ws = new WebSocket(this.url)

      this.ws.onopen = () => {
        this.connected = true
        this.reconnectAttempts = 0
        this.startPingInterval()
        resolve()
      }

      this.ws.onerror = (event) => {
        const error = new Error("WebSocket error")
        this.errorHandlers.forEach((h) => h(error))
        if (!this.connected) {
          reject(error)
        }
      }

      this.ws.onclose = (event) => {
        this.connected = false
        this.stopPingInterval()
        this.closeHandlers.forEach((h) => h(event.code, event.reason))

        // Reject all pending requests
        this.pendingRequests.forEach(({ reject }) => {
          reject(new Error("WebSocket connection closed"))
        })
        this.pendingRequests.clear()

        // Auto-reconnect
        if (this.options.autoReconnect && this.reconnectAttempts < this.options.maxReconnectAttempts) {
          this.reconnectAttempts++
          setTimeout(() => this.connect().catch(() => {}), this.options.reconnectDelay)
        }
      }

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data as string)
          const message = WsMessage.parse(data)
          this.handleMessage(message)
        } catch (error) {
          console.error("Failed to parse WebSocket message:", error)
        }
      }
    })
  }

  /**
   * Disconnect from the WebSocket server
   */
  disconnect(): void {
    this.options.autoReconnect = false
    this.stopPingInterval()
    this.ws?.close()
    this.ws = null
    this.connected = false
  }

  /**
   * Send a message
   */
  send(message: WsMessage): void {
    if (!this.isConnected) {
      throw new Error("WebSocket not connected")
    }
    this.ws?.send(JSON.stringify(message))
  }

  /**
   * Send a ping
   */
  ping(): void {
    this.send({ type: "ping" })
  }

  /**
   * Execute a tool via WebSocket
   */
  async executeTool<T = unknown>(tool: string, params: Record<string, unknown>): Promise<T> {
    const id = crypto.randomUUID()

    return new Promise<T>((resolve, reject) => {
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      })

      this.send({
        type: "tool_request",
        id,
        tool,
        params,
      })

      // Timeout after 60 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id)
          reject(new Error("Tool execution timeout"))
        }
      }, 60000)
    })
  }

  /**
   * Execute a tool with streaming response
   */
  async executeToolStream(
    tool: string,
    params: Record<string, unknown>,
    onContent: (content: string, done: boolean) => void
  ): Promise<void> {
    const id = crypto.randomUUID()

    return new Promise<void>((resolve, reject) => {
      this.streamHandlers.set(id, (content, done) => {
        onContent(content, done)
        if (done) {
          this.streamHandlers.delete(id)
          resolve()
        }
      })

      this.pendingRequests.set(id, {
        resolve: () => {
          this.streamHandlers.delete(id)
          resolve()
        },
        reject: (error) => {
          this.streamHandlers.delete(id)
          reject(error)
        },
      })

      this.send({
        type: "tool_request",
        id,
        tool,
        params,
      })
    })
  }

  /**
   * Add event handler
   */
  onMessage(handler: WsEventHandler): () => void {
    this.eventHandlers.push(handler)
    return () => {
      const index = this.eventHandlers.indexOf(handler)
      if (index >= 0) {
        this.eventHandlers.splice(index, 1)
      }
    }
  }

  /**
   * Add error handler
   */
  onError(handler: WsErrorHandler): () => void {
    this.errorHandlers.push(handler)
    return () => {
      const index = this.errorHandlers.indexOf(handler)
      if (index >= 0) {
        this.errorHandlers.splice(index, 1)
      }
    }
  }

  /**
   * Add close handler
   */
  onClose(handler: WsCloseHandler): () => void {
    this.closeHandlers.push(handler)
    return () => {
      const index = this.closeHandlers.indexOf(handler)
      if (index >= 0) {
        this.closeHandlers.splice(index, 1)
      }
    }
  }

  private handleMessage(message: WsMessage): void {
    // Notify event handlers
    this.eventHandlers.forEach((h) => h(message))

    switch (message.type) {
      case "pong":
        // Ping/pong handled
        break

      case "tool_response": {
        const pending = this.pendingRequests.get(message.id)
        if (pending) {
          this.pendingRequests.delete(message.id)
          if (message.success) {
            pending.resolve(message.result)
          } else {
            pending.reject(new Error(message.error ?? "Tool execution failed"))
          }
        }
        break
      }

      case "stream": {
        const handler = this.streamHandlers.get(message.id)
        if (handler) {
          handler(message.content, message.done)
        }
        break
      }

      case "error":
        console.error("WebSocket error:", message.message)
        break
    }
  }

  private startPingInterval(): void {
    if (this.options.pingInterval > 0) {
      this.pingIntervalId = setInterval(() => {
        if (this.isConnected) {
          this.ping()
        }
      }, this.options.pingInterval)
    }
  }

  private stopPingInterval(): void {
    if (this.pingIntervalId) {
      clearInterval(this.pingIntervalId)
      this.pingIntervalId = null
    }
  }
}

/**
 * Singleton WebSocket client
 */
let wsClient: ZeroWebSocketClient | null = null

/**
 * Get the WebSocket client
 */
export function getWsClient(options?: WsClientOptions): ZeroWebSocketClient {
  if (!wsClient) {
    wsClient = new ZeroWebSocketClient(undefined, options)
  }
  return wsClient
}

/**
 * Reset the WebSocket client
 */
export function resetWsClient(): void {
  wsClient?.disconnect()
  wsClient = null
}

/**
 * Namespace for WebSocket API
 */
export namespace ZeroWs {
  export const connect = () => getWsClient().connect()
  export const disconnect = () => getWsClient().disconnect()
  export const isConnected = () => getWsClient().isConnected
  export const executeTool = <T = unknown>(tool: string, params: Record<string, unknown>) =>
    getWsClient().executeTool<T>(tool, params)
  export const executeToolStream = (
    tool: string,
    params: Record<string, unknown>,
    onContent: (content: string, done: boolean) => void
  ) => getWsClient().executeToolStream(tool, params, onContent)
  export const onMessage = (handler: WsEventHandler) => getWsClient().onMessage(handler)
  export const onError = (handler: WsErrorHandler) => getWsClient().onError(handler)
  export const onClose = (handler: WsCloseHandler) => getWsClient().onClose(handler)
}
