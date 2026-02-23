/**
 * WebSocket Handler for Executive Dashboard
 *
 * Provides real-time updates for executive metrics:
 * - Token usage updates
 * - Active users count
 * - Cost changes
 * - Alert notifications
 * - Git statistics (commits, PRs, issues)
 */

import type { ServerWebSocket } from "bun"
import { exec } from "child_process"
import { promisify } from "util"

const execAsync = promisify(exec)

// ============================================================================
// Types
// ============================================================================

interface ExecutiveSubscription {
  socket: ServerWebSocket<WebSocketData>
  subscribedChannels: Set<string>
}

interface WebSocketData {
  subscriptionId: string
  userId?: string
}

interface WebSocketMessage {
  type: "subscribe" | "unsubscribe" | "ping"
  channel?: string
  channels?: string[]
}

interface ExecutiveUpdate {
  channel: string
  data: unknown
  timestamp: string
}

interface GitStats {
  commits_today: number
  commits_this_week: number
  open_prs: number
  merged_prs_this_week: number
  open_issues: number
  contributors: number
  last_commit_at: string | null
  repository: string | null
}

// Available channels for subscription
export const EXECUTIVE_CHANNELS = {
  METRICS: "executive.metrics",
  ALERTS: "executive.alerts",
  ACTIVITY: "executive.activity",
  COST: "executive.cost",
  GIT: "executive.git",
} as const

// ============================================================================
// Subscription Manager
// ============================================================================

class ExecutiveWebSocketManager {
  private subscriptions = new Map<string, ExecutiveSubscription>()
  private updateInterval: ReturnType<typeof setInterval> | undefined
  private isRunning = false

  /**
   * Add a new WebSocket connection.
   */
  addConnection(socket: ServerWebSocket<WebSocketData>): void {
    const subscriptionId = socket.data.subscriptionId
    this.subscriptions.set(subscriptionId, {
      socket,
      subscribedChannels: new Set(),
    })
  }

  /**
   * Remove a WebSocket connection.
   */
  removeConnection(subscriptionId: string): void {
    this.subscriptions.delete(subscriptionId)
  }

  /**
   * Subscribe a connection to a channel.
   */
  subscribe(subscriptionId: string, channel: string): void {
    const subscription = this.subscriptions.get(subscriptionId)
    if (subscription) {
      subscription.subscribedChannels.add(channel)
    }
  }

  /**
   * Unsubscribe a connection from a channel.
   */
  unsubscribe(subscriptionId: string, channel: string): void {
    const subscription = this.subscriptions.get(subscriptionId)
    if (subscription) {
      subscription.subscribedChannels.delete(channel)
    }
  }

  /**
   * Broadcast an update to all subscribers of a channel.
   */
  broadcast(channel: string, data: unknown): void {
    const update: ExecutiveUpdate = {
      channel,
      data,
      timestamp: new Date().toISOString(),
    }

    const message = JSON.stringify(update)

    for (const [, subscription] of this.subscriptions) {
      if (subscription.subscribedChannels.has(channel)) {
        try {
          subscription.socket.send(message)
        } catch {
          // Socket may be closed, ignore
        }
      }
    }
  }

  /**
   * Get the number of active connections.
   */
  getConnectionCount(): number {
    return this.subscriptions.size
  }

  /**
   * Start the update interval for pushing periodic updates.
   */
  startUpdates(intervalMs: number = 5000): void {
    if (this.isRunning) return

    this.isRunning = true
    this.updateInterval = setInterval(async () => {
      await this.pushUpdates()
    }, intervalMs)
  }

  /**
   * Stop the update interval.
   */
  stopUpdates(): void {
    if (this.updateInterval) {
      clearInterval(this.updateInterval)
      this.updateInterval = undefined
    }
    this.isRunning = false
  }

  /**
   * Push current metrics to subscribers.
   */
  private async pushUpdates(): Promise<void> {
    // Only push if there are subscribers
    if (this.subscriptions.size === 0) return

    // Get current metrics from metering
    const metricsUpdate = await this.fetchCurrentMetrics()
    this.broadcast(EXECUTIVE_CHANNELS.METRICS, metricsUpdate)

    // Get Git statistics
    const gitStats = await this.fetchGitStats()
    this.broadcast(EXECUTIVE_CHANNELS.GIT, gitStats)
  }

  /**
   * Fetch current metrics for broadcasting.
   */
  private async fetchCurrentMetrics(): Promise<unknown> {
    try {
      // Try to fetch from metering API
      const response = await fetch("http://localhost:4400/api/v1/metering/usage")
      if (response.ok) {
        return await response.json()
      }
    } catch {
      // Fall back to basic metrics
    }

    // Return basic metrics if metering unavailable
    return {
      active_connections: this.subscriptions.size,
      timestamp: new Date().toISOString(),
      status: "ok",
    }
  }

  /**
   * Fetch Git statistics using gh CLI.
   */
  private async fetchGitStats(): Promise<GitStats> {
    const defaultStats: GitStats = {
      commits_today: 0,
      commits_this_week: 0,
      open_prs: 0,
      merged_prs_this_week: 0,
      open_issues: 0,
      contributors: 0,
      last_commit_at: null,
      repository: null,
    }

    try {
      // Get repository info
      const repoResult = await this.runGhCommand("gh repo view --json nameWithOwner -q .nameWithOwner")
      const repository = repoResult.trim() || null
      defaultStats.repository = repository

      if (!repository) {
        return defaultStats
      }

      // Run multiple gh commands in parallel for efficiency
      const [commitsToday, commitsWeek, openPrs, mergedPrs, openIssues, lastCommit] = await Promise.all([
        // Commits today
        this.runGhCommand(`git log --oneline --since="midnight" 2>/dev/null | wc -l`).then((r) =>
          parseInt(r.trim()) || 0
        ),
        // Commits this week
        this.runGhCommand(`git log --oneline --since="1 week ago" 2>/dev/null | wc -l`).then((r) =>
          parseInt(r.trim()) || 0
        ),
        // Open PRs
        this.runGhCommand(`gh pr list --state open --json number -q 'length'`).then((r) =>
          parseInt(r.trim()) || 0
        ),
        // Merged PRs this week
        this.runGhCommand(
          `gh pr list --state merged --search "merged:>=$(date -v-7d +%Y-%m-%d 2>/dev/null || date -d '7 days ago' +%Y-%m-%d 2>/dev/null)" --json number -q 'length'`
        )
          .then((r) => parseInt(r.trim()) || 0)
          .catch(() => 0),
        // Open issues
        this.runGhCommand(`gh issue list --state open --json number -q 'length'`).then((r) =>
          parseInt(r.trim()) || 0
        ),
        // Last commit timestamp
        this.runGhCommand(`git log -1 --format=%cI 2>/dev/null`).then((r) => r.trim() || null),
      ])

      return {
        commits_today: commitsToday,
        commits_this_week: commitsWeek,
        open_prs: openPrs,
        merged_prs_this_week: mergedPrs,
        open_issues: openIssues,
        contributors: 0, // Would need additional API call
        last_commit_at: lastCommit,
        repository,
      }
    } catch (error) {
      // Return default stats if gh CLI fails
      return defaultStats
    }
  }

  /**
   * Run a gh CLI command and return stdout.
   */
  private async runGhCommand(command: string): Promise<string> {
    try {
      const { stdout } = await execAsync(command, {
        timeout: 5000,
        encoding: "utf-8",
      })
      return stdout
    } catch {
      return ""
    }
  }

  /**
   * Send an alert to all alert channel subscribers.
   */
  sendAlert(alert: { type: string; message: string; metric?: string; value?: number }): void {
    this.broadcast(EXECUTIVE_CHANNELS.ALERTS, alert)
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

export const executiveWsManager = new ExecutiveWebSocketManager()

// ============================================================================
// WebSocket Handlers
// ============================================================================

/**
 * Handle new WebSocket connection.
 */
export function handleOpen(ws: ServerWebSocket<WebSocketData>): void {
  executiveWsManager.addConnection(ws)

  // Send welcome message
  ws.send(
    JSON.stringify({
      type: "connected",
      subscriptionId: ws.data.subscriptionId,
      availableChannels: Object.values(EXECUTIVE_CHANNELS),
      timestamp: new Date().toISOString(),
    })
  )

  // Start updates if this is the first connection
  if (executiveWsManager.getConnectionCount() === 1) {
    executiveWsManager.startUpdates()
  }
}

/**
 * Handle WebSocket message.
 */
export function handleMessage(ws: ServerWebSocket<WebSocketData>, message: string | Buffer): void {
  try {
    const data: WebSocketMessage = JSON.parse(typeof message === "string" ? message : message.toString())

    switch (data.type) {
      case "subscribe":
        if (data.channel) {
          executiveWsManager.subscribe(ws.data.subscriptionId, data.channel)
          ws.send(JSON.stringify({ type: "subscribed", channel: data.channel }))
        }
        if (data.channels) {
          for (const channel of data.channels) {
            executiveWsManager.subscribe(ws.data.subscriptionId, channel)
          }
          ws.send(JSON.stringify({ type: "subscribed", channels: data.channels }))
        }
        break

      case "unsubscribe":
        if (data.channel) {
          executiveWsManager.unsubscribe(ws.data.subscriptionId, data.channel)
          ws.send(JSON.stringify({ type: "unsubscribed", channel: data.channel }))
        }
        break

      case "ping":
        ws.send(JSON.stringify({ type: "pong", timestamp: new Date().toISOString() }))
        break
    }
  } catch {
    ws.send(JSON.stringify({ type: "error", message: "Invalid message format" }))
  }
}

/**
 * Handle WebSocket close.
 */
export function handleClose(ws: ServerWebSocket<WebSocketData>): void {
  executiveWsManager.removeConnection(ws.data.subscriptionId)

  // Stop updates if no connections remain
  if (executiveWsManager.getConnectionCount() === 0) {
    executiveWsManager.stopUpdates()
  }
}

/**
 * Generate a unique subscription ID.
 */
export function generateSubscriptionId(): string {
  return `exec-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
}
