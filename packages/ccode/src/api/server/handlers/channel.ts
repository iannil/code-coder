/**
 * Channel API Handler
 * Handles /api/channels endpoints for messaging channel management
 *
 * Reads channel configuration from ~/.codecoder/config.json under zerobot.channels
 *
 * Endpoints:
 * - GET /api/channels - List all configured channels with status
 * - GET /api/channels/:name - Get specific channel status
 * - POST /api/channels/:name/health - Check channel health
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { homedir } from "os"
import { join } from "path"

// ============================================================================
// Types
// ============================================================================

type ChannelType = "cli" | "telegram" | "discord" | "slack" | "matrix" | "whatsapp" | "imessage" | "email" | "feishu"
type ChannelHealth = "healthy" | "degraded" | "unhealthy"

interface ChannelStatus {
  name: string
  type: ChannelType
  enabled: boolean
  health: ChannelHealth
  config: Record<string, unknown>
  lastPing?: number
  error?: string
}

// Matches ZeroBotChannels in config.ts
interface ZeroBotChannels {
  cli?: boolean
  telegram?: {
    bot_token: string
    allowed_users?: string[]
  }
  discord?: {
    bot_token: string
    guild_id?: string
    allowed_users?: string[]
  }
  slack?: {
    bot_token: string
    app_token?: string
    channel_id?: string
  }
  whatsapp?: {
    access_token: string
    phone_number_id: string
    verify_token: string
    allowed_numbers?: string[]
  }
  feishu?: {
    app_id: string
    app_secret: string
    encrypt_key?: string
    verification_token?: string
    allowed_users?: string[]
  }
}

// Matches ZeroBotGateway in config.ts
interface ZeroBotGateway {
  port?: number
  host?: string
}

interface ZeroBotConfig {
  channels?: ZeroBotChannels
  gateway?: ZeroBotGateway
}

interface CodeCoderConfig {
  zerobot?: ZeroBotConfig
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Read CodeCoder config from ~/.codecoder/config.json
 */
async function readConfig(): Promise<CodeCoderConfig> {
  const configPath = join(homedir(), ".codecoder", "config.json")

  try {
    const file = Bun.file(configPath)
    const exists = await file.exists()

    if (!exists) {
      return {}
    }

    const content = await file.text()
    return JSON.parse(content) as CodeCoderConfig
  } catch {
    return {}
  }
}

/**
 * Check if ZeroBot daemon is running by calling its health endpoint
 */
async function checkZeroBotHealth(gateway?: ZeroBotGateway): Promise<boolean> {
  const host = gateway?.host ?? "127.0.0.1"
  const port = gateway?.port ?? 8080  // ZeroBot default port
  const url = `http://${host}:${port}`

  try {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), 2000)

    const response = await fetch(`${url}/health`, {
      signal: controller.signal,
    })

    clearTimeout(timeoutId)
    return response.ok
  } catch {
    return false
  }
}

/**
 * Build channel status list from config
 */
async function buildChannelStatuses(config: CodeCoderConfig): Promise<ChannelStatus[]> {
  const channels: ChannelStatus[] = []
  const channelsConfig = config.zerobot?.channels ?? {}

  // CLI channel
  if (channelsConfig.cli !== undefined) {
    channels.push({
      name: "cli",
      type: "cli",
      enabled: channelsConfig.cli === true,
      health: channelsConfig.cli ? "healthy" : "unhealthy",
      config: {},
    })
  }

  // Telegram channel
  if (channelsConfig.telegram) {
    const tg = channelsConfig.telegram
    const hasToken = !!tg.bot_token && tg.bot_token !== "YOUR_BOT_TOKEN_HERE"

    channels.push({
      name: "telegram",
      type: "telegram",
      enabled: hasToken,
      health: hasToken ? "healthy" : "unhealthy",
      config: {
        token: tg.bot_token ? "***" + tg.bot_token.slice(-4) : undefined,
        allowedUsers: tg.allowed_users,
      },
      error: hasToken ? undefined : "Bot token not configured",
    })
  }

  // Discord channel
  if (channelsConfig.discord) {
    const dc = channelsConfig.discord
    const hasToken = !!dc.bot_token

    channels.push({
      name: "discord",
      type: "discord",
      enabled: hasToken,
      health: hasToken ? "healthy" : "unhealthy",
      config: {
        token: dc.bot_token ? "***" + dc.bot_token.slice(-4) : undefined,
        guildId: dc.guild_id,
      },
      error: hasToken ? undefined : "Bot token not configured",
    })
  }

  // Slack channel
  if (channelsConfig.slack) {
    const sl = channelsConfig.slack
    const hasToken = !!sl.bot_token

    channels.push({
      name: "slack",
      type: "slack",
      enabled: hasToken,
      health: hasToken ? "healthy" : "unhealthy",
      config: {
        token: sl.bot_token ? "***" : undefined,
      },
      error: hasToken ? undefined : "Bot token not configured",
    })
  }

  // WhatsApp channel
  if (channelsConfig.whatsapp) {
    const wa = channelsConfig.whatsapp
    const hasConfig = !!wa.access_token && !!wa.phone_number_id

    channels.push({
      name: "whatsapp",
      type: "whatsapp",
      enabled: hasConfig,
      health: hasConfig ? "healthy" : "unhealthy",
      config: {
        phoneNumberId: wa.phone_number_id,
      },
      error: hasConfig ? undefined : "WhatsApp configuration incomplete",
    })
  }

  // Feishu channel
  if (channelsConfig.feishu) {
    const fs = channelsConfig.feishu
    const hasConfig = !!fs.app_id && !!fs.app_secret

    channels.push({
      name: "feishu",
      type: "feishu",
      enabled: hasConfig,
      health: hasConfig ? "healthy" : "unhealthy",
      config: {
        appId: fs.app_id ? "***" + fs.app_id.slice(-4) : undefined,
        allowedUsers: fs.allowed_users,
      },
      error: hasConfig ? undefined : "Feishu app credentials not configured",
    })
  }

  return channels
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/channels
 * List all configured channels with status
 */
export async function listChannels(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await readConfig()
    const channels = await buildChannelStatuses(config)

    // Check if ZeroBot is running
    const zeroBotRunning = await checkZeroBotHealth(config.zerobot?.gateway)

    // Update health status based on daemon status
    for (const channel of channels) {
      if (channel.enabled && !zeroBotRunning) {
        channel.health = "degraded"
        channel.error = channel.error ?? "ZeroBot daemon not running"
      }
    }

    return jsonResponse({
      success: true,
      data: channels,
      meta: {
        total: channels.length,
        enabled: channels.filter((c) => c.enabled).length,
        healthy: channels.filter((c) => c.health === "healthy").length,
        zeroBotRunning,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/channels/:name
 * Get specific channel status
 */
export async function getChannel(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("Channel name is required", 400)
    }

    const config = await readConfig()
    const channels = await buildChannelStatuses(config)
    const channel = channels.find((c) => c.name === name)

    if (!channel) {
      return errorResponse(`Channel "${name}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: channel,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/channels/:name/health
 * Check channel health
 */
export async function checkChannelHealth(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("Channel name is required", 400)
    }

    const config = await readConfig()
    const channels = await buildChannelStatuses(config)
    const channel = channels.find((c) => c.name === name)

    if (!channel) {
      return errorResponse(`Channel "${name}" not found`, 404)
    }

    // For now, just return the current health status
    // In the future, we could add actual health checks per channel type
    const updatedChannel: ChannelStatus = {
      ...channel,
      lastPing: Date.now(),
    }

    return jsonResponse({
      success: true,
      data: updatedChannel,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
