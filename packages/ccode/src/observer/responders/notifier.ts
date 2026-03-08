/**
 * Notifier Responder
 *
 * Sends notifications based on observation events.
 *
 * @module observer/responders/notifier
 */

import { Log } from "@/util/log"
import type { Observation, Anomaly, Opportunity, EmergentPattern, GearPreset } from "../types"
import type { Escalation } from "../controller"
import { ObserverEvent } from "../events"
import {
  getChannelsClient,
  type ChannelType,
  type ChannelsClientConfig,
} from "../integration/channels-client"
import { getDialPanel, type DialPanel } from "../panel"

const log = Log.create({ service: "observer.responders.notifier" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type NotificationPriority = "low" | "medium" | "high" | "urgent"
export type NotificationChannel = "tui" | "im" | "webhook" | "email" | "log"

export interface Notification {
  id: string
  title: string
  body: string
  priority: NotificationPriority
  channels: NotificationChannel[]
  triggeredBy: string // Observation, anomaly, or pattern ID
  triggerType: "observation" | "anomaly" | "pattern" | "opportunity" | "escalation"
  metadata?: Record<string, unknown>
  createdAt: Date
  sentAt?: Date
  status: "pending" | "sent" | "failed"
}

export interface NotificationRule {
  id: string
  name: string
  enabled: boolean
  trigger: {
    type: "observation" | "anomaly" | "pattern" | "opportunity" | "escalation"
    filter?: (item: unknown) => boolean
  }
  priority: NotificationPriority
  channels: NotificationChannel[]
  template: (item: unknown) => { title: string; body: string }
}

export interface NotifierConfig {
  /** Enabled channels */
  enabledChannels: NotificationChannel[]
  /** Notification rules */
  rules: NotificationRule[]
  /** Minimum priority for notifications */
  minPriority: NotificationPriority
  /** Maximum notifications per minute */
  rateLimit: number
  /** Cooldown for duplicate notifications (ms) */
  cooldownMs: number
  /** IM channel configuration */
  imChannel?: {
    type: ChannelType
    channelId: string
    baseUrl?: string
  }
  /** Webhook URL for webhook notifications */
  webhookUrl?: string
  /** Email configuration */
  emailConfig?: {
    to: string[]
    from?: string
  }
  /** Use dial-based control (new architecture) */
  useDialControl: boolean
  /** Observe dial threshold for proactive notifications (0-100) */
  observeThreshold: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: NotifierConfig = {
  enabledChannels: ["tui", "log"],
  rules: [],
  minPriority: "medium",
  rateLimit: 30,
  cooldownMs: 60000,
  useDialControl: true,
  observeThreshold: 50, // Observe dial > 50% enables proactive notifications
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Rules
// ─────────────────────────────────────────────────────────────────────────────

const BUILT_IN_RULES: NotificationRule[] = [
  {
    id: "critical-anomaly",
    name: "Critical Anomaly Alert",
    enabled: true,
    trigger: {
      type: "anomaly",
      filter: (a) => (a as Anomaly).severity === "critical",
    },
    priority: "urgent",
    channels: ["tui", "im"],
    template: (a) => {
      const anomaly = a as Anomaly
      return {
        title: `🚨 Critical Anomaly: ${anomaly.type}`,
        body: anomaly.description,
      }
    },
  },
  {
    id: "high-impact-opportunity",
    name: "High Impact Opportunity",
    enabled: true,
    trigger: {
      type: "opportunity",
      filter: (o) => (o as Opportunity).impact === "high",
    },
    priority: "high",
    channels: ["tui"],
    template: (o) => {
      const opp = o as Opportunity
      return {
        title: `💡 Opportunity: ${opp.type}`,
        body: opp.description,
      }
    },
  },
  {
    id: "escalation-alert",
    name: "Escalation Alert",
    enabled: true,
    trigger: {
      type: "escalation",
    },
    priority: "urgent",
    channels: ["tui", "im"],
    template: (e) => {
      const esc = e as Escalation
      return {
        title: `⚠️ [${esc.priority.toUpperCase()}] ${esc.title}`,
        body: esc.description,
      }
    },
  },
  {
    id: "strong-pattern",
    name: "Strong Pattern Detected",
    enabled: true,
    trigger: {
      type: "pattern",
      filter: (p) => (p as EmergentPattern).strength > 0.8,
    },
    priority: "medium",
    channels: ["tui"],
    template: (p) => {
      const pattern = p as EmergentPattern
      return {
        title: `📊 Pattern: ${pattern.name}`,
        body: `${pattern.type} pattern with ${(pattern.strength * 100).toFixed(0)}% strength`,
      }
    },
  },
]

// ─────────────────────────────────────────────────────────────────────────────
// Notifier
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Sends notifications based on observer events.
 */
export class Notifier {
  private config: NotifierConfig
  private notifications: Map<string, Notification> = new Map()
  private recentNotifications: string[] = [] // For deduplication
  private idCounter = 0
  private running = false
  private rateLimitCounter = 0
  private rateLimitResetTimer: ReturnType<typeof setInterval> | null = null
  private eventSubscriptions: Array<() => void> = []
  private dialPanel: DialPanel | null = null

  constructor(config: Partial<NotifierConfig> = {}) {
    this.config = {
      ...DEFAULT_CONFIG,
      ...config,
      rules: [...BUILT_IN_RULES, ...(config.rules ?? [])],
    }
    if (this.config.useDialControl) {
      this.dialPanel = getDialPanel()
    }
  }

  /**
   * Get the current observe dial value.
   */
  getObserveDialValue(): number {
    return this.dialPanel?.getDial("observe") ?? 50
  }

  /**
   * Check if notifications should be proactive.
   * When observe dial < threshold, stay silent for non-urgent notifications.
   */
  shouldNotifyProactively(): boolean {
    if (!this.config.useDialControl || !this.dialPanel) {
      return true // Default to proactive
    }
    return this.dialPanel.shouldObserve()
  }

  /**
   * Get the notification mode based on observe dial.
   * Returns 'silent' for low observation, 'proactive' for high observation.
   */
  getNotificationMode(): "silent" | "proactive" {
    const observeValue = this.getObserveDialValue()
    return observeValue >= this.config.observeThreshold ? "proactive" : "silent"
  }

  /**
   * Start the notifier.
   */
  async start(): Promise<void> {
    if (this.running) return

    this.running = true

    // Reset rate limit counter every minute
    this.rateLimitResetTimer = setInterval(() => {
      this.rateLimitCounter = 0
    }, 60000)

    // Subscribe to events
    const Bus = (await import("@/bus")).Bus

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.AnomalyDetected, async (event) => {
        await this.handleTrigger("anomaly", event.properties)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.OpportunityIdentified, async (event) => {
        await this.handleTrigger("opportunity", event.properties)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.PatternDetected, async (event) => {
        await this.handleTrigger("pattern", event.properties)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.EscalationCreated, async (event) => {
        await this.handleTrigger("escalation", event.properties)
      }),
    )

    log.info("Notifier started", {
      channels: this.config.enabledChannels,
      ruleCount: this.config.rules.length,
    })
  }

  /**
   * Stop the notifier.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    if (this.rateLimitResetTimer) {
      clearInterval(this.rateLimitResetTimer)
      this.rateLimitResetTimer = null
    }

    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []

    log.info("Notifier stopped")
  }

  /**
   * Send a notification manually.
   */
  async notify(
    title: string,
    body: string,
    options: {
      priority?: NotificationPriority
      channels?: NotificationChannel[]
      triggeredBy?: string
      triggerType?: Notification["triggerType"]
      metadata?: Record<string, unknown>
    } = {},
  ): Promise<Notification> {
    const notification = this.createNotification(
      title,
      body,
      options.priority ?? "medium",
      options.channels ?? this.config.enabledChannels,
      options.triggeredBy ?? "manual",
      options.triggerType ?? "observation",
      options.metadata,
    )

    await this.send(notification)
    return notification
  }

  /**
   * Get notification history.
   */
  getHistory(limit?: number): Notification[] {
    const notifications = Array.from(this.notifications.values())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
    return notifications.slice(0, limit ?? 50)
  }

  /**
   * Get pending notifications.
   */
  getPending(): Notification[] {
    return Array.from(this.notifications.values())
      .filter((n) => n.status === "pending")
  }

  /**
   * Add a custom rule.
   */
  addRule(rule: NotificationRule): void {
    this.config.rules.push(rule)
  }

  /**
   * Remove a rule.
   */
  removeRule(ruleId: string): boolean {
    const index = this.config.rules.findIndex((r) => r.id === ruleId)
    if (index >= 0) {
      this.config.rules.splice(index, 1)
      return true
    }
    return false
  }

  /**
   * Clear notification history.
   */
  clear(): void {
    this.notifications.clear()
    this.recentNotifications = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async handleTrigger(type: Notification["triggerType"], item: unknown): Promise<void> {
    const matchingRules = this.config.rules.filter((rule) => {
      if (!rule.enabled) return false
      if (rule.trigger.type !== type) return false
      if (rule.trigger.filter && !rule.trigger.filter(item)) return false
      return true
    })

    for (const rule of matchingRules) {
      const { title, body } = rule.template(item)
      const notification = this.createNotification(
        title,
        body,
        rule.priority,
        rule.channels,
        (item as any)?.id ?? "unknown",
        type,
      )

      await this.send(notification)
    }
  }

  private createNotification(
    title: string,
    body: string,
    priority: NotificationPriority,
    channels: NotificationChannel[],
    triggeredBy: string,
    triggerType: Notification["triggerType"],
    metadata?: Record<string, unknown>,
  ): Notification {
    return {
      id: `notif_${Date.now()}_${++this.idCounter}`,
      title,
      body,
      priority,
      channels: channels.filter((c) => this.config.enabledChannels.includes(c)),
      triggeredBy,
      triggerType,
      metadata,
      createdAt: new Date(),
      status: "pending",
    }
  }

  private async send(notification: Notification): Promise<void> {
    // Check priority
    const priorityOrder: Record<NotificationPriority, number> = {
      low: 0,
      medium: 1,
      high: 2,
      urgent: 3,
    }
    const minPriorityOrder = priorityOrder[this.config.minPriority]
    if (priorityOrder[notification.priority] < minPriorityOrder) {
      log.debug("Notification below minimum priority", {
        id: notification.id,
        priority: notification.priority,
        minPriority: this.config.minPriority,
      })
      return
    }

    // Check observe dial - only urgent notifications bypass silent mode
    const notificationMode = this.getNotificationMode()
    if (notificationMode === "silent" && notification.priority !== "urgent") {
      log.debug("Silent mode active, skipping non-urgent notification", {
        id: notification.id,
        priority: notification.priority,
        observeDial: this.getObserveDialValue(),
        threshold: this.config.observeThreshold,
      })
      return
    }

    // Check rate limit
    if (this.rateLimitCounter >= this.config.rateLimit) {
      log.warn("Rate limit exceeded, dropping notification", { id: notification.id })
      return
    }

    // Check cooldown (deduplication)
    const dedupeKey = `${notification.triggerType}:${notification.triggeredBy}`
    if (this.recentNotifications.includes(dedupeKey)) {
      log.debug("Notification in cooldown", { id: notification.id, key: dedupeKey })
      return
    }

    this.notifications.set(notification.id, notification)
    this.rateLimitCounter++
    this.recentNotifications.push(dedupeKey)

    // Remove from recent after cooldown
    setTimeout(() => {
      const index = this.recentNotifications.indexOf(dedupeKey)
      if (index >= 0) {
        this.recentNotifications.splice(index, 1)
      }
    }, this.config.cooldownMs)

    // Send to each channel
    for (const channel of notification.channels) {
      try {
        await this.sendToChannel(notification, channel)
      } catch (error) {
        log.error("Failed to send notification to channel", {
          id: notification.id,
          channel,
          error: String(error),
        })
      }
    }

    notification.status = "sent"
    notification.sentAt = new Date()

    // Publish event
    const Bus = (await import("@/bus")).Bus
    await Bus.publish(ObserverEvent.NotificationSent, {
      channel: notification.channels.join(","),
      message: notification.body,
      priority: notification.priority,
      triggeredBy: notification.triggeredBy,
    })
  }

  private async sendToChannel(notification: Notification, channel: NotificationChannel): Promise<void> {
    switch (channel) {
      case "log":
        this.sendToLog(notification)
        break

      case "tui":
        // TUI picks up from events; no direct action needed
        log.debug("TUI notification", { id: notification.id })
        break

      case "im":
        await this.sendToIM(notification)
        break

      case "webhook":
        await this.sendToWebhook(notification)
        break

      case "email":
        await this.sendToEmail(notification)
        break
    }
  }

  private async sendToIM(notification: Notification): Promise<void> {
    const { imChannel } = this.config
    if (!imChannel) {
      log.debug("IM channel not configured", { id: notification.id })
      return
    }

    try {
      const channelsClient = getChannelsClient({
        baseUrl: imChannel.baseUrl,
        defaultChannel: imChannel.type,
        defaultChannelId: imChannel.channelId,
      })

      // Format message with priority indicator
      const priorityEmoji = this.getPriorityEmoji(notification.priority)
      const message = `${priorityEmoji} *${notification.title}*\n\n${notification.body}`

      const result = await channelsClient.send({
        channelType: imChannel.type,
        channelId: imChannel.channelId,
        content: {
          type: "markdown",
          text: message,
        },
      })

      if (!result.success) {
        log.warn("Failed to send IM notification", {
          id: notification.id,
          error: result.error,
        })
      } else {
        log.debug("IM notification sent", {
          id: notification.id,
          messageId: result.messageId,
        })
      }
    } catch (error) {
      log.error("IM notification error", {
        id: notification.id,
        error: String(error),
      })
    }
  }

  private async sendToWebhook(notification: Notification): Promise<void> {
    const { webhookUrl } = this.config
    if (!webhookUrl) {
      log.debug("Webhook URL not configured", { id: notification.id })
      return
    }

    try {
      const response = await fetch(webhookUrl, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          id: notification.id,
          title: notification.title,
          body: notification.body,
          priority: notification.priority,
          triggerType: notification.triggerType,
          triggeredBy: notification.triggeredBy,
          createdAt: notification.createdAt.toISOString(),
          metadata: notification.metadata,
        }),
        signal: AbortSignal.timeout(10000),
      })

      if (!response.ok) {
        log.warn("Webhook notification failed", {
          id: notification.id,
          status: response.status,
        })
      } else {
        log.debug("Webhook notification sent", { id: notification.id })
      }
    } catch (error) {
      log.error("Webhook notification error", {
        id: notification.id,
        error: String(error),
      })
    }
  }

  private async sendToEmail(notification: Notification): Promise<void> {
    const { emailConfig, imChannel } = this.config
    if (!emailConfig) {
      log.debug("Email not configured", { id: notification.id })
      return
    }

    try {
      // Use channels client to send email
      const channelsClient = getChannelsClient({
        baseUrl: imChannel?.baseUrl,
      })

      const result = await channelsClient.send({
        channelType: "email",
        channelId: emailConfig.to.join(","),
        content: {
          type: "text",
          text: `Subject: [${notification.priority.toUpperCase()}] ${notification.title}\n\n${notification.body}`,
        },
      })

      if (!result.success) {
        log.warn("Failed to send email notification", {
          id: notification.id,
          error: result.error,
        })
      } else {
        log.debug("Email notification sent", { id: notification.id })
      }
    } catch (error) {
      log.error("Email notification error", {
        id: notification.id,
        error: String(error),
      })
    }
  }

  private getPriorityEmoji(priority: NotificationPriority): string {
    switch (priority) {
      case "urgent":
        return "🚨"
      case "high":
        return "⚠️"
      case "medium":
        return "📢"
      case "low":
        return "📝"
    }
  }

  private sendToLog(notification: Notification): void {
    const logFn =
      notification.priority === "urgent" || notification.priority === "high"
        ? log.warn
        : log.info

    logFn.call(log, `[${notification.priority.toUpperCase()}] ${notification.title}`, {
      body: notification.body,
      triggeredBy: notification.triggeredBy,
    })
  }
}

/**
 * Create a notifier.
 */
export function createNotifier(config?: Partial<NotifierConfig>): Notifier {
  return new Notifier(config)
}
