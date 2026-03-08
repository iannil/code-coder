/**
 * Channels Client for Observer Network
 *
 * Provides HTTP client for sending messages through zero-hub channels.
 * Supports Telegram, Discord, Slack, Feishu, WeChat Work, and other IM channels.
 *
 * @module observer/integration/channels-client
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "observer.integration.channels" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type ChannelType =
  | "telegram"
  | "discord"
  | "slack"
  | "feishu"
  | "wecom"
  | "dingtalk"
  | "whatsapp"
  | "email"

export interface SendMessageRequest {
  channelType: ChannelType
  channelId: string
  content: MessageContent
}

export interface MessageContent {
  type: "text" | "markdown"
  text: string
}

export interface SendMessageResponse {
  success: boolean
  messageId?: string
  error?: string
}

export interface InlineButton {
  text: string
  callbackData: string
}

export interface SendWithButtonsRequest {
  channelType: ChannelType
  channelId: string
  text: string
  buttons: InlineButton[][]
}

export interface ChannelsClientConfig {
  /** Base URL for zero-hub channels API */
  baseUrl: string
  /** Request timeout in milliseconds */
  timeoutMs: number
  /** Default channel type */
  defaultChannel?: ChannelType
  /** Default channel ID (chat/group ID) */
  defaultChannelId?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: ChannelsClientConfig = {
  baseUrl: "http://127.0.0.1:4402",
  timeoutMs: 30000,
}

// ─────────────────────────────────────────────────────────────────────────────
// Channels Client
// ─────────────────────────────────────────────────────────────────────────────

/**
 * HTTP client for zero-hub channels API.
 */
export class ChannelsClient {
  private config: ChannelsClientConfig

  constructor(config: Partial<ChannelsClientConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Send a message to a channel.
   */
  async send(request: SendMessageRequest): Promise<SendMessageResponse> {
    const url = `${this.config.baseUrl}/api/v1/send`

    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          channel_type: request.channelType,
          channel_id: request.channelId,
          content: request.content,
        }),
        signal: AbortSignal.timeout(this.config.timeoutMs),
      })

      const data = (await response.json()) as SendMessageResponse

      if (!response.ok) {
        log.error("Failed to send message", {
          status: response.status,
          error: data.error,
        })
        return {
          success: false,
          error: data.error ?? `HTTP ${response.status}`,
        }
      }

      log.debug("Message sent", {
        channelType: request.channelType,
        channelId: request.channelId,
        messageId: data.messageId,
      })

      return data
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      log.error("Failed to send message", { error: errorMessage })
      return {
        success: false,
        error: errorMessage,
      }
    }
  }

  /**
   * Send a text message to the default channel.
   */
  async sendText(text: string): Promise<SendMessageResponse> {
    if (!this.config.defaultChannel || !this.config.defaultChannelId) {
      return {
        success: false,
        error: "Default channel not configured",
      }
    }

    return this.send({
      channelType: this.config.defaultChannel,
      channelId: this.config.defaultChannelId,
      content: { type: "text", text },
    })
  }

  /**
   * Send a markdown message to the default channel.
   */
  async sendMarkdown(markdown: string): Promise<SendMessageResponse> {
    if (!this.config.defaultChannel || !this.config.defaultChannelId) {
      return {
        success: false,
        error: "Default channel not configured",
      }
    }

    return this.send({
      channelType: this.config.defaultChannel,
      channelId: this.config.defaultChannelId,
      content: { type: "markdown", text: markdown },
    })
  }

  /**
   * Send a message with inline buttons (for Telegram).
   * This requires direct Telegram API call since zero-hub doesn't expose buttons yet.
   */
  async sendWithButtons(request: SendWithButtonsRequest): Promise<SendMessageResponse> {
    // For now, fall back to regular text with options listed
    // TODO: Implement direct Telegram API call for inline buttons
    const optionsText = request.buttons
      .flat()
      .map((btn, i) => `[${i + 1}] ${btn.text}`)
      .join("\n")

    return this.send({
      channelType: request.channelType,
      channelId: request.channelId,
      content: {
        type: "text",
        text: `${request.text}\n\n${optionsText}`,
      },
    })
  }

  /**
   * Check if the channels service is available.
   */
  async isAvailable(): Promise<boolean> {
    try {
      const response = await fetch(`${this.config.baseUrl}/health`, {
        signal: AbortSignal.timeout(5000),
      })
      return response.ok
    } catch {
      return false
    }
  }

  /**
   * Update configuration.
   */
  configure(config: Partial<ChannelsClientConfig>): void {
    this.config = { ...this.config, ...config }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let clientInstance: ChannelsClient | null = null

/**
 * Get or create the channels client instance.
 */
export function getChannelsClient(config?: Partial<ChannelsClientConfig>): ChannelsClient {
  if (!clientInstance) {
    clientInstance = new ChannelsClient(config)
  } else if (config) {
    clientInstance.configure(config)
  }
  return clientInstance
}

/**
 * Reset the channels client instance.
 */
export function resetChannelsClient(): void {
  clientInstance = null
}

/**
 * Create a new channels client instance.
 */
export function createChannelsClient(config?: Partial<ChannelsClientConfig>): ChannelsClient {
  return new ChannelsClient(config)
}
