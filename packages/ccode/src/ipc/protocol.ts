/**
 * IPC Protocol Handler for JSON-RPC 2.0 over Unix Socket.
 *
 * Handles message framing (newline-delimited JSON), request/response
 * correlation, and notification dispatch.
 */

import { Socket } from "net"
import { EventEmitter } from "events"
import type {
  IpcId,
  IpcRequest,
  IpcResponse,
  IpcEvents,
  IpcEventName,
  SessionUpdateNotification,
  ToolRequestNotification,
  LlmRequestNotification,
  StreamTokenNotification,
  AgentStreamNotification,
  ErrorNotification,
} from "./types"
import { IpcMethods, isErrorResponse } from "./types"

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

type PendingRequest = {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  timeout: ReturnType<typeof setTimeout>
}

type IpcEventHandler<T extends IpcEventName> = (...args: IpcEvents[T]) => void

// ══════════════════════════════════════════════════════════════════════════════
// IpcProtocol Class
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Protocol handler for IPC communication.
 *
 * Manages:
 * - Message framing (newline-delimited JSON)
 * - Request/response correlation
 * - Timeout handling
 * - Notification dispatch
 */
export class IpcProtocol extends EventEmitter {
  private socket: Socket | null = null
  private buffer = ""
  private requestId = 0
  private pending = new Map<string, PendingRequest>()
  private defaultTimeout: number

  constructor(options?: { timeout?: number }) {
    super()
    this.defaultTimeout = options?.timeout ?? 30000
  }

  /** Connect to the IPC socket */
  async connect(socketPath: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.socket = new Socket()

      const onError = (err: Error) => {
        this.socket?.removeListener("error", onError)
        reject(err)
      }

      this.socket.once("error", onError)

      this.socket.connect(socketPath, () => {
        this.socket?.removeListener("error", onError)
        this.setupListeners()
        resolve()
      })
    })
  }

  /** Set up socket event listeners */
  private setupListeners(): void {
    if (!this.socket) return

    this.socket.on("data", (chunk: Buffer) => {
      this.buffer += chunk.toString("utf8")
      this.processBuffer()
    })

    this.socket.on("close", () => {
      this.cleanup()
      this.emit("disconnected")
    })

    this.socket.on("error", (err: Error) => {
      this.emit("error", { code: -32603, message: err.message })
    })
  }

  /** Process the receive buffer for complete messages */
  private processBuffer(): void {
    let newlineIndex: number
    while ((newlineIndex = this.buffer.indexOf("\n")) !== -1) {
      const line = this.buffer.slice(0, newlineIndex).trim()
      this.buffer = this.buffer.slice(newlineIndex + 1)

      if (line.length > 0) {
        this.handleMessage(line)
      }
    }
  }

  /** Handle a single JSON-RPC message */
  private handleMessage(json: string): void {
    let message: IpcResponse | IpcRequest

    try {
      message = JSON.parse(json)
    } catch {
      this.emit("error", { code: -32700, message: "Parse error" })
      return
    }

    // Check if this is a response to a pending request
    if ("result" in message || "error" in message) {
      const response = message as IpcResponse
      if (response.id !== undefined) {
        const idStr = String(response.id)
        const pending = this.pending.get(idStr)
        if (pending) {
          this.pending.delete(idStr)
          clearTimeout(pending.timeout)

          if (isErrorResponse(response)) {
            pending.reject(new Error(response.error.message))
          } else {
            pending.resolve(response.result)
          }
        }
      }
      return
    }

    // Otherwise, it's a notification
    const notification = message as IpcRequest
    this.handleNotification(notification)
  }

  /** Handle incoming notification */
  private handleNotification(notification: IpcRequest): void {
    const { method, params } = notification

    switch (method) {
      case IpcMethods.SESSION_UPDATE:
        this.emit("session_update", params as SessionUpdateNotification)
        break
      case IpcMethods.TOOL_REQUEST:
        this.emit("tool_request", params as ToolRequestNotification)
        break
      case IpcMethods.LLM_REQUEST:
        this.emit("llm_request", params as LlmRequestNotification)
        break
      case IpcMethods.STREAM_TOKEN:
        this.emit("stream_token", params as StreamTokenNotification)
        break
      case IpcMethods.AGENT_STREAM:
        this.emit("agent_stream", params as AgentStreamNotification)
        break
      case IpcMethods.ERROR:
        this.emit("error", params as ErrorNotification)
        break
      default:
        // Unknown notification - ignore
        break
    }
  }

  /** Send a request and wait for response */
  async request<T>(method: string, params?: unknown, timeout?: number): Promise<T> {
    if (!this.socket) {
      throw new Error("Not connected")
    }

    const id = ++this.requestId
    const idStr = String(id)

    const request: IpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
    }

    if (params !== undefined) {
      request.params = params
    }

    return new Promise((resolve, reject) => {
      const timeoutMs = timeout ?? this.defaultTimeout

      const timeoutHandle = setTimeout(() => {
        this.pending.delete(idStr)
        reject(new Error(`Request timeout: ${method}`))
      }, timeoutMs)

      this.pending.set(idStr, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout: timeoutHandle,
      })

      const json = JSON.stringify(request) + "\n"
      this.socket?.write(json, (err) => {
        if (err) {
          this.pending.delete(idStr)
          clearTimeout(timeoutHandle)
          reject(err)
        }
      })
    })
  }

  /** Send a notification (no response expected) */
  notify(method: string, params?: unknown): void {
    if (!this.socket) {
      throw new Error("Not connected")
    }

    const notification: IpcRequest = {
      jsonrpc: "2.0",
      method,
    }

    if (params !== undefined) {
      notification.params = params
    }

    const json = JSON.stringify(notification) + "\n"
    this.socket.write(json)
  }

  /** Check if connected */
  get connected(): boolean {
    return this.socket !== null && !this.socket.destroyed
  }

  /** Close the connection */
  close(): void {
    this.cleanup()
    this.socket?.destroy()
    this.socket = null
  }

  /** Clean up pending requests */
  private cleanup(): void {
    for (const [id, pending] of this.pending) {
      clearTimeout(pending.timeout)
      pending.reject(new Error("Connection closed"))
    }
    this.pending.clear()
    this.buffer = ""
  }

  // Type-safe event emitter methods
  override on<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.on(event, listener as (...args: unknown[]) => void)
  }

  override once<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.once(event, listener as (...args: unknown[]) => void)
  }

  override off<T extends IpcEventName>(event: T, listener: IpcEventHandler<T>): this {
    return super.off(event, listener as (...args: unknown[]) => void)
  }

  override emit<T extends IpcEventName>(event: T, ...args: IpcEvents[T]): boolean {
    return super.emit(event, ...args)
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

/** Check if a Unix socket exists and is connectable */
export async function isSocketReady(socketPath: string, timeout = 1000): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = new Socket()

    const timer = setTimeout(() => {
      socket.destroy()
      resolve(false)
    }, timeout)

    socket.once("connect", () => {
      clearTimeout(timer)
      socket.destroy()
      resolve(true)
    })

    socket.once("error", () => {
      clearTimeout(timer)
      socket.destroy()
      resolve(false)
    })

    socket.connect(socketPath)
  })
}

/** Wait for a socket to become available */
export async function waitForSocket(
  socketPath: string,
  options?: { timeout?: number; interval?: number }
): Promise<void> {
  const timeout = options?.timeout ?? 10000
  const interval = options?.interval ?? 100
  const deadline = Date.now() + timeout

  while (Date.now() < deadline) {
    if (await isSocketReady(socketPath)) {
      return
    }
    await new Promise((r) => setTimeout(r, interval))
  }

  throw new Error(`Socket not available after ${timeout}ms: ${socketPath}`)
}
