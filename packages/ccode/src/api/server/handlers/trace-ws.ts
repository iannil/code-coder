/**
 * WebSocket Handler for Real-time Trace Log Streaming
 *
 * Provides real-time trace log updates via WebSocket:
 * - Filter by service name
 * - Filter by log level (debug, info, warn, error)
 * - Follow mode for continuous streaming
 */

import type { ServerWebSocket } from "bun"
import os from "os"
import path from "path"
import fs from "fs/promises"
import type { LogEntry } from "../../../observability"

// ============================================================================
// Types
// ============================================================================

interface TraceSubscription {
  socket: ServerWebSocket<TraceWebSocketData>
  filters: TraceFilters
}

export interface TraceWebSocketData {
  subscriptionId: string
  type: "trace"
}

interface TraceFilters {
  service?: string
  level?: "debug" | "info" | "warn" | "error"
  traceId?: string
}

interface TraceWebSocketMessage {
  type: "subscribe" | "unsubscribe" | "ping" | "filter"
  filters?: TraceFilters
}

interface TraceUpdate {
  type: "entry" | "batch" | "connected" | "pong" | "error" | "filtered"
  data?: LogEntry | LogEntry[]
  filters?: TraceFilters
  timestamp: string
  message?: string
}

// Log level priority for filtering
const LEVEL_PRIORITY: Record<string, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
}

// ============================================================================
// Subscription Manager
// ============================================================================

class TraceWebSocketManager {
  private subscriptions = new Map<string, TraceSubscription>()
  private watchInterval: ReturnType<typeof setInterval> | undefined
  private isWatching = false
  private lastReadPosition = 0
  private currentLogFile: string = ""

  /**
   * Get the log directory path.
   */
  private getLogDir(): string {
    return path.join(os.homedir(), ".codecoder", "logs")
  }

  /**
   * Get the current day's log file path.
   */
  private getCurrentLogFile(): string {
    const date = new Date().toISOString().split("T")[0]
    return path.join(this.getLogDir(), `trace-${date}.jsonl`)
  }

  /**
   * Add a new WebSocket connection.
   */
  addConnection(socket: ServerWebSocket<TraceWebSocketData>): void {
    const subscriptionId = socket.data.subscriptionId
    this.subscriptions.set(subscriptionId, {
      socket,
      filters: { level: "info" }, // Default to info level
    })
  }

  /**
   * Remove a WebSocket connection.
   */
  removeConnection(subscriptionId: string): void {
    this.subscriptions.delete(subscriptionId)
  }

  /**
   * Update filters for a connection.
   */
  updateFilters(subscriptionId: string, filters: TraceFilters): void {
    const subscription = this.subscriptions.get(subscriptionId)
    if (subscription) {
      subscription.filters = { ...subscription.filters, ...filters }
    }
  }

  /**
   * Check if an entry passes the filters.
   */
  private passesFilters(entry: LogEntry, filters: TraceFilters): boolean {
    // Service filter
    if (filters.service && entry.service !== filters.service) {
      return false
    }

    // Level filter
    if (filters.level) {
      const minLevel = LEVEL_PRIORITY[filters.level] ?? 1
      const entryLevel = LEVEL_PRIORITY[entry.level] ?? 1
      if (entryLevel < minLevel) {
        return false
      }
    }

    // Trace ID filter
    if (filters.traceId && entry.trace_id !== filters.traceId) {
      return false
    }

    return true
  }

  /**
   * Send an entry to a specific subscription if it passes filters.
   */
  private sendToSubscription(subscription: TraceSubscription, entry: LogEntry): void {
    if (!this.passesFilters(entry, subscription.filters)) {
      return
    }

    const update: TraceUpdate = {
      type: "entry",
      data: entry,
      timestamp: new Date().toISOString(),
    }

    try {
      subscription.socket.send(JSON.stringify(update))
    } catch {
      // Socket may be closed, ignore
    }
  }

  /**
   * Broadcast an entry to all subscribers.
   */
  private broadcast(entry: LogEntry): void {
    for (const [, subscription] of this.subscriptions) {
      this.sendToSubscription(subscription, entry)
    }
  }

  /**
   * Get the number of active connections.
   */
  getConnectionCount(): number {
    return this.subscriptions.size
  }

  /**
   * Start watching the log file for new entries.
   */
  startWatching(): void {
    if (this.isWatching) return

    this.isWatching = true
    this.currentLogFile = this.getCurrentLogFile()
    this.lastReadPosition = 0

    // Initialize position to end of file
    this.initializePosition()

    // Poll for changes every 500ms
    this.watchInterval = setInterval(async () => {
      await this.checkForNewEntries()
    }, 500)
  }

  /**
   * Initialize read position to current end of file.
   */
  private async initializePosition(): Promise<void> {
    try {
      const stats = await fs.stat(this.currentLogFile)
      this.lastReadPosition = stats.size
    } catch {
      // File doesn't exist yet, start from 0
      this.lastReadPosition = 0
    }
  }

  /**
   * Check for new log entries.
   */
  private async checkForNewEntries(): Promise<void> {
    // Check if date changed (new log file)
    const expectedFile = this.getCurrentLogFile()
    if (expectedFile !== this.currentLogFile) {
      this.currentLogFile = expectedFile
      this.lastReadPosition = 0
    }

    try {
      const stats = await fs.stat(this.currentLogFile)
      if (stats.size <= this.lastReadPosition) {
        return // No new data
      }

      // Read new content
      const file = Bun.file(this.currentLogFile)
      const content = await file.text()
      const newContent = content.slice(this.lastReadPosition)
      this.lastReadPosition = stats.size

      // Parse and broadcast new entries
      const lines = newContent.split("\n").filter((line) => line.trim())
      for (const line of lines) {
        try {
          const entry = JSON.parse(line) as LogEntry
          this.broadcast(entry)
        } catch {
          // Skip malformed lines
        }
      }
    } catch {
      // File may not exist yet, ignore
    }
  }

  /**
   * Stop watching the log file.
   */
  stopWatching(): void {
    if (this.watchInterval) {
      clearInterval(this.watchInterval)
      this.watchInterval = undefined
    }
    this.isWatching = false
  }

  /**
   * Send recent entries to a newly connected client.
   */
  async sendRecentEntries(subscriptionId: string, count: number = 50): Promise<void> {
    const subscription = this.subscriptions.get(subscriptionId)
    if (!subscription) return

    try {
      const file = Bun.file(this.currentLogFile)
      if (!(await file.exists())) return

      const content = await file.text()
      const lines = content.trim().split("\n").slice(-count)
      const entries: LogEntry[] = []

      for (const line of lines) {
        try {
          const entry = JSON.parse(line) as LogEntry
          if (this.passesFilters(entry, subscription.filters)) {
            entries.push(entry)
          }
        } catch {
          // Skip malformed
        }
      }

      if (entries.length > 0) {
        const update: TraceUpdate = {
          type: "batch",
          data: entries,
          timestamp: new Date().toISOString(),
        }
        subscription.socket.send(JSON.stringify(update))
      }
    } catch {
      // File may not exist
    }
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

export const traceWsManager = new TraceWebSocketManager()

// ============================================================================
// WebSocket Handlers
// ============================================================================

/**
 * Handle new WebSocket connection.
 */
export function handleTraceOpen(ws: ServerWebSocket<TraceWebSocketData>): void {
  traceWsManager.addConnection(ws)

  // Send welcome message
  const welcome: TraceUpdate = {
    type: "connected",
    timestamp: new Date().toISOString(),
    message: "Connected to trace log stream",
    filters: { level: "info" },
  }
  ws.send(JSON.stringify(welcome))

  // Start watching if this is the first connection
  if (traceWsManager.getConnectionCount() === 1) {
    traceWsManager.startWatching()
  }

  // Send recent entries
  traceWsManager.sendRecentEntries(ws.data.subscriptionId)
}

/**
 * Handle WebSocket message.
 */
export function handleTraceMessage(ws: ServerWebSocket<TraceWebSocketData>, message: string | Buffer): void {
  try {
    const data: TraceWebSocketMessage = JSON.parse(typeof message === "string" ? message : message.toString())

    switch (data.type) {
      case "filter":
        if (data.filters) {
          traceWsManager.updateFilters(ws.data.subscriptionId, data.filters)
          const response: TraceUpdate = {
            type: "filtered",
            filters: data.filters,
            timestamp: new Date().toISOString(),
          }
          ws.send(JSON.stringify(response))
        }
        break

      case "ping":
        const pong: TraceUpdate = {
          type: "pong",
          timestamp: new Date().toISOString(),
        }
        ws.send(JSON.stringify(pong))
        break

      default:
        // Unknown message type
        break
    }
  } catch {
    const error: TraceUpdate = {
      type: "error",
      message: "Invalid message format",
      timestamp: new Date().toISOString(),
    }
    ws.send(JSON.stringify(error))
  }
}

/**
 * Handle WebSocket close.
 */
export function handleTraceClose(ws: ServerWebSocket<TraceWebSocketData>): void {
  traceWsManager.removeConnection(ws.data.subscriptionId)

  // Stop watching if no connections remain
  if (traceWsManager.getConnectionCount() === 0) {
    traceWsManager.stopWatching()
  }
}

/**
 * Generate a unique subscription ID.
 */
export function generateTraceSubscriptionId(): string {
  return `trace-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
}
