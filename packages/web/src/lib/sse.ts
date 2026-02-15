/**
 * SSE (Server-Sent Events) Client for CodeCoder Web
 * Manages real-time event streaming from the CodeCoder API
 */

import type {
  SSEDataEvent,
  SSEEventType,
  SSEMessageEvent,
  SSEStatusEvent,
  SSEErrorEvent,
  SSEPermissionEvent,
  SSEProgressEvent,
} from "./types"

// ============================================================================
// SSE Client Configuration
// ============================================================================

export interface SSEClientConfig {
  baseUrl?: string
  apiKey?: string
  channels?: SSEEventType[]
  reconnectInterval?: number
  maxReconnectAttempts?: number
  headers?: Record<string, string>
}

const DEFAULT_SSE_CONFIG: Required<Omit<SSEClientConfig, "apiKey" | "channels" | "headers">> = {
  baseUrl: "/api",
  reconnectInterval: 3000,
  maxReconnectAttempts: 10,
}

// ============================================================================
// SSE Event Handlers
// ============================================================================

export interface SSEEventHandlers {
  onOpen?: (event: Event) => void
  onError?: (error: Error) => void
  onClose?: () => void
  onMessage?: (event: SSEDataEvent) => void
  onStatusChange?: (connected: boolean) => void
  // Specific event type handlers
  onMessageEvent?: (event: SSEMessageEvent) => void
  onStatusEvent?: (event: SSEStatusEvent) => void
  onErrorEvent?: (event: SSEErrorEvent) => void
  onPermissionEvent?: (event: SSEPermissionEvent) => void
  onProgressEvent?: (event: SSEProgressEvent) => void
}

// ============================================================================
// SSE Client State
// ============================================================================

type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting" | "error"

// ============================================================================
// SSE Client Class
// ============================================================================

export class SSEClient {
  private config: Required<SSEClientConfig>
  private handlers: SSEEventHandlers
  private eventSource: EventSource | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private reconnectAttempts = 0
  private state: ConnectionState = "disconnected"
  private manualClose = false

  constructor(config: SSEClientConfig = {}, handlers: SSEEventHandlers = {}) {
    this.config = {
      ...DEFAULT_SSE_CONFIG,
      channels: config.channels ?? ["message", "status", "error", "permission", "progress"],
      headers: {
        ...config.headers,
      },
      apiKey: config.apiKey ?? "",
    }
    this.handlers = handlers
  }

  // ========================================================================
  // Connection Management
  // ========================================================================

  /**
   * Connect to the SSE endpoint
   */
  connect(): void {
    if (this.state === "connected" || this.state === "connecting") {
      return
    }

    this.manualClose = false
    this.connectInternal()
  }

  private connectInternal(): void {
    if (this.manualClose) {
      return
    }

    this.setState("connecting")

    try {
      // Build URL with channels parameter
      const url = new URL(`${this.config.baseUrl.replace(/\/+$/, "")}/events`)
      if (this.config.channels && this.config.channels.length > 0) {
        url.searchParams.append("channels", this.config.channels.join(","))
      }

      // Create EventSource with custom headers if needed
      // Note: EventSource doesn't support custom headers directly,
      // so we use URL params for API key
      if (this.config.apiKey) {
        url.searchParams.append("api_key", this.config.apiKey)
      }

      this.eventSource = new EventSource(url.toString())

      // Set up event listeners
      this.eventSource.onopen = this.handleOpen.bind(this)
      this.eventSource.onerror = this.handleError.bind(this)
      this.eventSource.onmessage = this.handleMessage.bind(this)

      // Set up specific event type listeners
      const eventTypes = ["message", "status", "error", "permission", "progress"]
      for (const eventType of eventTypes) {
        this.eventSource.addEventListener(eventType, (e) => this.handleTypedEvent(eventType, e))
      }
    } catch (error) {
      this.setState("error")
      this.scheduleReconnect()
      this.handlers.onError?.(
        error instanceof Error ? error : new Error("Failed to create EventSource"),
      )
    }
  }

  /**
   * Disconnect from the SSE endpoint
   */
  disconnect(): void {
    this.manualClose = true
    this.clearReconnectTimer()
    this.closeEventSource()
    this.setState("disconnected")
  }

  private closeEventSource(): void {
    if (this.eventSource) {
      this.eventSource.close()
      this.eventSource = null
    }
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }

  // ========================================================================
  // State Management
  // ========================================================================

  private setState(newState: ConnectionState): void {
    const wasConnected = this.state === "connected"
    this.state = newState
    const isConnected = newState === "connected"

    if (wasConnected !== isConnected) {
      this.handlers.onStatusChange?.(isConnected)
    }
  }

  getState(): ConnectionState {
    return this.state
  }

  isConnected(): boolean {
    return this.state === "connected"
  }

  // ========================================================================
  // Event Handlers
  // ========================================================================

  private handleOpen(event: Event): void {
    this.setState("connected")
    this.reconnectAttempts = 0
    this.handlers.onOpen?.(event)
  }

  private handleError(_event: Event): void {
    this.setState("error")

    const error = new Error(
      `EventSource error: ${this.eventSource?.readyState === EventSource.CLOSED ? "Connection closed" : "Unknown error"}`,
    )
    this.handlers.onError?.(error)

    // Schedule reconnect if not manually closed
    if (!this.manualClose) {
      this.scheduleReconnect()
    }
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const data = JSON.parse(event.data) as SSEDataEvent
      this.dispatchDataEvent(data)
    } catch (error) {
      console.error("Failed to parse SSE message:", error)
    }
  }

  private handleTypedEvent(type: string, event: Event): void {
    const messageEvent = event as MessageEvent
    try {
      const data = JSON.parse(messageEvent.data) as SSEDataEvent
      // Create a new object with the correct type, avoiding spread of the original
      const typedEvent: SSEDataEvent = { ...data, type: type as SSEEventType } as SSEDataEvent
      this.dispatchDataEvent(typedEvent)
    } catch (error) {
      console.error(`Failed to parse SSE ${type} event:`, error)
    }
  }

  private dispatchDataEvent(data: SSEDataEvent): void {
    this.handlers.onMessage?.(data)

    switch (data.type) {
      case "message":
        this.handlers.onMessageEvent?.(data as SSEMessageEvent)
        break
      case "status":
        this.handlers.onStatusEvent?.(data as SSEStatusEvent)
        break
      case "error":
        this.handlers.onErrorEvent?.(data as SSEErrorEvent)
        break
      case "permission":
        this.handlers.onPermissionEvent?.(data as SSEPermissionEvent)
        break
      case "progress":
        this.handlers.onProgressEvent?.(data as SSEProgressEvent)
        break
    }
  }

  // ========================================================================
  // Reconnection Logic
  // ========================================================================

  private scheduleReconnect(): void {
    this.clearReconnectTimer()

    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.setState("error")
      this.handlers.onError?.(new Error("Max reconnection attempts reached"))
      return
    }

    this.setState("reconnecting")
    this.reconnectAttempts++

    this.reconnectTimer = setTimeout(() => {
      this.closeEventSource()
      this.connectInternal()
    }, this.config.reconnectInterval)
  }

  /**
   * Reset reconnection attempts (call after successful operation)
   */
  resetReconnectAttempts(): void {
    this.reconnectAttempts = 0
  }

  // ========================================================================
  // Channel Management
  // ========================================================================

  /**
   * Subscribe to additional channels
   * Note: This requires reconnection
   */
  subscribeChannels(channels: SSEEventType[]): void {
    const newChannels = [...new Set([...this.config.channels, ...channels])]
    if (newChannels.length !== this.config.channels.length) {
      this.config.channels = newChannels
      if (this.isConnected()) {
        this.disconnect()
        this.connect()
      }
    }
  }

  /**
   * Unsubscribe from channels
   * Note: This requires reconnection
   */
  unsubscribeChannels(channels: SSEEventType[]): void {
    this.config.channels = this.config.channels.filter((c) => !channels.includes(c))
    if (this.isConnected()) {
      this.disconnect()
      this.connect()
    }
  }

  getChannels(): SSEEventType[] {
    return [...this.config.channels]
  }
}

// ============================================================================
// Default SSE Client Instance
// ============================================================================

let defaultSSEClient: SSEClient | null = null

export function setDefaultSSEClient(config: SSEClientConfig, handlers?: SSEEventHandlers): void {
  if (defaultSSEClient) {
    defaultSSEClient.disconnect()
  }
  defaultSSEClient = new SSEClient(config, handlers)
}

export function getSSEClient(): SSEClient {
  if (!defaultSSEClient) {
    defaultSSEClient = new SSEClient()
  }
  return defaultSSEClient
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create an SSE client with auto-connection
 */
export function createSSEClient(config: SSEClientConfig, handlers: SSEEventHandlers): SSEClient {
  const client = new SSEClient(config, handlers)
  client.connect()
  return client
}

/**
 * Connect to SSE and return a promise that resolves when connected
 */
export function connectSSE(
  config: SSEClientConfig,
  handlers?: SSEEventHandlers,
): Promise<SSEClient> {
  return new Promise((resolve, reject) => {
    const client = new SSEClient(config, {
      ...handlers,
      onOpen: (event) => {
        handlers?.onOpen?.(event)
        resolve(client)
      },
      onError: (error) => {
        handlers?.onError?.(error)
        reject(error)
      },
    })

    client.connect()

    // Timeout after 10 seconds
    setTimeout(() => {
      if (!client.isConnected()) {
        client.disconnect()
        reject(new Error("SSE connection timeout"))
      }
    }, 10000)
  })
}
