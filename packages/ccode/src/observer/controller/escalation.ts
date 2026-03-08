/**
 * Escalation Module
 *
 * Handles escalation to human decision-makers when automatic
 * mode control encounters critical situations.
 *
 * This implements the "评估权" (Evaluation Authority) concept from
 * 祝融说 - preserving human intervention points for critical decisions.
 *
 * @module observer/controller/escalation
 */

import { Log } from "@/util/log"
import type { OperatingMode, Anomaly, Opportunity } from "../types"
import type { CLOSEEvaluation } from "./close-evaluator"
import { ObserverEvent } from "../events"
import {
  getChannelsClient,
  type ChannelType,
  type InlineButton,
} from "../integration/channels-client"

const log = Log.create({ service: "observer.controller.escalation" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type EscalationPriority = "critical" | "high" | "medium" | "low"
export type EscalationStatus = "pending" | "acknowledged" | "resolved" | "dismissed" | "expired"

export interface EscalationContext {
  currentMode: OperatingMode
  recommendedMode: OperatingMode
  closeEvaluation: CLOSEEvaluation
  anomalies: Anomaly[]
  opportunities: Opportunity[]
  trigger: string
}

export interface Escalation {
  id: string
  priority: EscalationPriority
  title: string
  description: string
  context: EscalationContext
  status: EscalationStatus
  createdAt: Date
  updatedAt: Date
  expiresAt: Date
  resolution?: HumanDecision
}

export interface HumanDecision {
  action: "approve" | "reject" | "modify" | "defer"
  chosenMode?: OperatingMode
  reason?: string
  timestamp: Date
}

export interface EscalationConfig {
  /** Timeout for pending escalations (ms) */
  timeoutMs: number
  /** Channel for notifications */
  notificationChannel?: "im" | "tui" | "webhook"
  /** Maximum pending escalations */
  maxPending: number
  /** Auto-resolve low priority after timeout */
  autoResolveLow: boolean
  /** IM channel configuration */
  imChannel?: {
    type: ChannelType
    channelId: string
    baseUrl?: string
  }
  /** Webhook URL for webhook notifications */
  webhookUrl?: string
  /** Enable inline buttons for IM escalations */
  enableInlineButtons: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: EscalationConfig = {
  timeoutMs: 300000, // 5 minutes
  notificationChannel: "tui",
  maxPending: 10,
  autoResolveLow: true,
  enableInlineButtons: true,
}

// ─────────────────────────────────────────────────────────────────────────────
// Escalation Manager
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Manages escalations to human decision-makers.
 */
export class EscalationManager {
  private config: EscalationConfig
  private escalations: Map<string, Escalation> = new Map()
  private idCounter = 0
  private expirationTimer: ReturnType<typeof setInterval> | null = null

  constructor(config: Partial<EscalationConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Start the escalation manager.
   */
  start(): void {
    // Check for expirations every minute
    this.expirationTimer = setInterval(() => {
      this.checkExpirations()
    }, 60000)
  }

  /**
   * Stop the escalation manager.
   */
  stop(): void {
    if (this.expirationTimer) {
      clearInterval(this.expirationTimer)
      this.expirationTimer = null
    }
  }

  /**
   * Create a new escalation.
   */
  async escalate(
    priority: EscalationPriority,
    title: string,
    description: string,
    context: EscalationContext,
  ): Promise<Escalation> {
    // Check max pending
    const pending = this.getPending()
    if (pending.length >= this.config.maxPending) {
      // Expire oldest low-priority escalation
      const lowPriority = pending.filter((e) => e.priority === "low").sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime())
      if (lowPriority.length > 0) {
        await this.expire(lowPriority[0].id)
      } else {
        throw new Error("Maximum pending escalations reached")
      }
    }

    const now = new Date()
    const escalation: Escalation = {
      id: this.generateId(),
      priority,
      title,
      description,
      context,
      status: "pending",
      createdAt: now,
      updatedAt: now,
      expiresAt: new Date(now.getTime() + this.config.timeoutMs),
    }

    this.escalations.set(escalation.id, escalation)

    // Publish event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.EscalationCreated, {
      escalationId: escalation.id,
      priority,
      title,
      currentMode: context.currentMode,
      recommendedMode: context.recommendedMode,
      expiresAt: escalation.expiresAt,
    })

    log.warn("Escalation created", {
      id: escalation.id,
      priority,
      title,
      expiresAt: escalation.expiresAt,
    })

    // Send notification based on channel
    await this.notify(escalation)

    return escalation
  }

  /**
   * Resolve an escalation with human decision.
   */
  async resolve(escalationId: string, decision: HumanDecision): Promise<Escalation | null> {
    const escalation = this.escalations.get(escalationId)
    if (!escalation) return null

    escalation.status = "resolved"
    escalation.resolution = decision
    escalation.updatedAt = new Date()

    // Publish event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.EscalationResolved, {
      escalationId,
      decision,
      resolvedAt: escalation.updatedAt,
    })

    log.info("Escalation resolved", {
      id: escalationId,
      action: decision.action,
      chosenMode: decision.chosenMode,
    })

    return escalation
  }

  /**
   * Acknowledge an escalation (human has seen it).
   */
  acknowledge(escalationId: string): Escalation | null {
    const escalation = this.escalations.get(escalationId)
    if (!escalation || escalation.status !== "pending") return null

    escalation.status = "acknowledged"
    escalation.updatedAt = new Date()

    log.info("Escalation acknowledged", { id: escalationId })

    return escalation
  }

  /**
   * Dismiss an escalation without resolution.
   */
  async dismiss(escalationId: string, reason?: string): Promise<Escalation | null> {
    const escalation = this.escalations.get(escalationId)
    if (!escalation) return null

    escalation.status = "dismissed"
    escalation.updatedAt = new Date()
    escalation.resolution = {
      action: "reject",
      reason: reason ?? "Dismissed by user",
      timestamp: new Date(),
    }

    log.info("Escalation dismissed", { id: escalationId, reason })

    return escalation
  }

  /**
   * Get pending escalations.
   */
  getPending(): Escalation[] {
    return Array.from(this.escalations.values())
      .filter((e) => e.status === "pending" || e.status === "acknowledged")
      .sort((a, b) => {
        // Priority order: critical > high > medium > low
        const priorityOrder: Record<EscalationPriority, number> = {
          critical: 0,
          high: 1,
          medium: 2,
          low: 3,
        }
        const priorityDiff = priorityOrder[a.priority] - priorityOrder[b.priority]
        if (priorityDiff !== 0) return priorityDiff
        return a.createdAt.getTime() - b.createdAt.getTime()
      })
  }

  /**
   * Get escalation by ID.
   */
  get(escalationId: string): Escalation | null {
    return this.escalations.get(escalationId) ?? null
  }

  /**
   * Get all escalations.
   */
  getAll(): Escalation[] {
    return Array.from(this.escalations.values()).sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
  }

  /**
   * Clear all escalations.
   */
  clear(): void {
    this.escalations.clear()
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private generateId(): string {
    return `esc_${Date.now()}_${++this.idCounter}`
  }

  private async expire(escalationId: string): Promise<void> {
    const escalation = this.escalations.get(escalationId)
    if (!escalation || escalation.status === "resolved") return

    escalation.status = "expired"
    escalation.updatedAt = new Date()

    // Auto-resolve low priority if configured
    if (this.config.autoResolveLow && escalation.priority === "low") {
      escalation.resolution = {
        action: "defer",
        reason: "Auto-deferred due to expiration",
        timestamp: new Date(),
      }
    }

    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.EscalationExpired, {
      escalationId,
      expiredAt: escalation.updatedAt,
    })

    log.warn("Escalation expired", { id: escalationId, priority: escalation.priority })
  }

  private checkExpirations(): void {
    const now = Date.now()
    for (const escalation of this.escalations.values()) {
      if (
        (escalation.status === "pending" || escalation.status === "acknowledged") &&
        escalation.expiresAt.getTime() < now
      ) {
        void this.expire(escalation.id)
      }
    }
  }

  private async notify(escalation: Escalation): Promise<void> {
    const { notificationChannel, imChannel, webhookUrl, enableInlineButtons } = this.config

    // Build notification message
    const message = this.buildNotificationMessage(escalation)

    switch (notificationChannel) {
      case "im":
        await this.notifyIM(escalation, message)
        break

      case "tui":
        // TUI will pick up from the event stream
        log.debug("TUI notification via event", { escalationId: escalation.id })
        break

      case "webhook":
        await this.notifyWebhook(escalation, message)
        break

      default:
        log.debug("No notification channel configured")
    }
  }

  private async notifyIM(escalation: Escalation, message: string): Promise<void> {
    const { imChannel, enableInlineButtons } = this.config

    if (!imChannel) {
      log.warn("IM channel not configured for escalation", { id: escalation.id })
      return
    }

    const channelsClient = getChannelsClient({
      baseUrl: imChannel.baseUrl,
      defaultChannel: imChannel.type,
      defaultChannelId: imChannel.channelId,
    })

    try {
      if (enableInlineButtons && imChannel.type === "telegram") {
        // Send with inline buttons for Telegram
        const buttons: InlineButton[][] = [
          [
            { text: "✅ 批准", callbackData: `esc:${escalation.id}:approve` },
            { text: "❌ 拒绝", callbackData: `esc:${escalation.id}:reject` },
          ],
          [
            { text: "⏸️ 延迟", callbackData: `esc:${escalation.id}:defer` },
            { text: "🔄 切换到手动", callbackData: `esc:${escalation.id}:manual` },
          ],
        ]

        const result = await channelsClient.sendWithButtons({
          channelType: imChannel.type,
          channelId: imChannel.channelId,
          text: message,
          buttons,
        })

        if (!result.success) {
          log.warn("Failed to send IM escalation with buttons", {
            id: escalation.id,
            error: result.error,
          })
        } else {
          log.info("IM escalation sent with buttons", {
            id: escalation.id,
            messageId: result.messageId,
          })
        }
      } else {
        // Send plain message
        const result = await channelsClient.send({
          channelType: imChannel.type,
          channelId: imChannel.channelId,
          content: {
            type: "markdown",
            text: message,
          },
        })

        if (!result.success) {
          log.warn("Failed to send IM escalation", {
            id: escalation.id,
            error: result.error,
          })
        } else {
          log.info("IM escalation sent", {
            id: escalation.id,
            messageId: result.messageId,
          })
        }
      }
    } catch (error) {
      log.error("IM escalation error", {
        id: escalation.id,
        error: String(error),
      })
    }
  }

  private async notifyWebhook(escalation: Escalation, message: string): Promise<void> {
    const { webhookUrl } = this.config

    if (!webhookUrl) {
      log.warn("Webhook URL not configured for escalation", { id: escalation.id })
      return
    }

    try {
      const response = await fetch(webhookUrl, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          type: "escalation",
          id: escalation.id,
          priority: escalation.priority,
          title: escalation.title,
          description: escalation.description,
          message,
          context: {
            currentMode: escalation.context.currentMode,
            recommendedMode: escalation.context.recommendedMode,
            closeScore: escalation.context.closeEvaluation.total,
          },
          createdAt: escalation.createdAt.toISOString(),
          expiresAt: escalation.expiresAt.toISOString(),
        }),
        signal: AbortSignal.timeout(10000),
      })

      if (!response.ok) {
        log.warn("Webhook escalation failed", {
          id: escalation.id,
          status: response.status,
        })
      } else {
        log.info("Webhook escalation sent", { id: escalation.id })
      }
    } catch (error) {
      log.error("Webhook escalation error", {
        id: escalation.id,
        error: String(error),
      })
    }
  }

  private buildNotificationMessage(escalation: Escalation): string {
    const { priority, title, description, context } = escalation
    const { closeEvaluation } = context

    const lines: string[] = []
    lines.push(`🚨 [${priority.toUpperCase()}] ${title}`)
    lines.push("")
    lines.push(description)
    lines.push("")
    lines.push(`Current Mode: ${context.currentMode}`)
    lines.push(`Recommended: ${context.recommendedMode}`)
    lines.push("")
    lines.push(`CLOSE Score: ${closeEvaluation.total.toFixed(2)}/10`)
    lines.push(`Risk Level: ${closeEvaluation.risk.toFixed(2)}/10`)
    lines.push(`Confidence: ${(closeEvaluation.confidence * 100).toFixed(0)}%`)

    if (context.anomalies.length > 0) {
      lines.push("")
      lines.push(`Anomalies: ${context.anomalies.length}`)
    }

    if (context.opportunities.length > 0) {
      lines.push(`Opportunities: ${context.opportunities.length}`)
    }

    lines.push("")
    lines.push(`Expires: ${escalation.expiresAt.toISOString()}`)

    return lines.join("\n")
  }
}

/**
 * Create an escalation manager.
 */
export function createEscalationManager(config?: Partial<EscalationConfig>): EscalationManager {
  return new EscalationManager(config)
}
