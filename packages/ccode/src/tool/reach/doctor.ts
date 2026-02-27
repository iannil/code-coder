import type { ChannelId, ChannelInfo, ChannelTier } from "./types"
import { commandExists, commandPath, getInstallInstructions } from "./utils"
import { ReachConfigManager } from "./config"
import { MCP } from "../../mcp"
import { Log } from "@/util/log"

/**
 * Agent Reach - Dependency Diagnostics
 *
 * Checks availability of external tools and services required for each channel
 */

const log = Log.create({ service: "reach.doctor" })

interface ChannelSpec {
  name: string
  description: string
  tier: ChannelTier
  backends: string[]
  check: () => Promise<{ status: ChannelInfo["status"]; message: string }>
}

const CHANNELS: Record<ChannelId, ChannelSpec> = {
  youtube: {
    name: "YouTube",
    description: "YouTube video info, transcripts, and search",
    tier: 0,
    backends: ["yt-dlp"],
    check: checkYtDlp,
  },
  bilibili: {
    name: "Bilibili",
    description: "B站 video info and transcripts",
    tier: 0,
    backends: ["yt-dlp"],
    check: checkYtDlp,
  },
  rss: {
    name: "RSS/Atom",
    description: "Read RSS and Atom feeds",
    tier: 0,
    backends: ["built-in"],
    check: async () => ({ status: "ok", message: "Built-in parser ready" }),
  },
  twitter: {
    name: "Twitter/X",
    description: "Read tweets, search, and timeline",
    tier: 1,
    backends: ["bird CLI"],
    check: checkTwitter,
  },
  reddit: {
    name: "Reddit",
    description: "Read posts and comments",
    tier: 1,
    backends: ["JSON API"],
    check: checkReddit,
  },
  xiaohongshu: {
    name: "小红书",
    description: "Read and search notes",
    tier: 2,
    backends: ["MCP"],
    check: () => checkMcp("xiaohongshu"),
  },
  douyin: {
    name: "抖音",
    description: "Read and search videos",
    tier: 2,
    backends: ["MCP"],
    check: () => checkMcp("douyin"),
  },
  linkedin: {
    name: "LinkedIn",
    description: "Read profiles and posts",
    tier: 2,
    backends: ["MCP"],
    check: () => checkMcp("linkedin"),
  },
  bosszhipin: {
    name: "Boss直聘",
    description: "Search job listings",
    tier: 2,
    backends: ["MCP"],
    check: () => checkMcp("bosszhipin"),
  },
}

async function checkYtDlp(): Promise<{ status: ChannelInfo["status"]; message: string }> {
  const exists = await commandExists("yt-dlp")
  if (!exists) {
    return {
      status: "off",
      message: `yt-dlp not found. Install: ${getInstallInstructions("yt-dlp")}`,
    }
  }

  // Check version
  const path = await commandPath("yt-dlp")
  return {
    status: "ok",
    message: `yt-dlp found at ${path}`,
  }
}

async function checkTwitter(): Promise<{ status: ChannelInfo["status"]; message: string }> {
  // Check if bird CLI exists
  const exists = await commandExists("bird")
  if (!exists) {
    return {
      status: "off",
      message: `bird CLI not found. Install: ${getInstallInstructions("bird")}`,
    }
  }

  // Check if cookies are configured
  const hasConfig = await ReachConfigManager.hasChannelConfig("twitter")
  if (!hasConfig) {
    return {
      status: "warn",
      message: "bird CLI found, but no cookies configured. Run: codecoder reach configure twitter",
    }
  }

  return {
    status: "ok",
    message: "bird CLI and cookies configured",
  }
}

async function checkReddit(): Promise<{ status: ChannelInfo["status"]; message: string }> {
  // Reddit uses JSON API, check if proxy is available for restricted regions
  const proxy = await ReachConfigManager.getProxy()

  // Try a simple API request
  try {
    const response = await fetch("https://www.reddit.com/r/all.json?limit=1", {
      signal: AbortSignal.timeout(5000),
      headers: {
        "User-Agent": "CodeCoder/1.0",
      },
    })

    if (response.ok) {
      return {
        status: "ok",
        message: "Reddit API accessible",
      }
    }

    if (response.status === 403) {
      return {
        status: proxy ? "warn" : "off",
        message: proxy
          ? "Reddit API blocked, proxy configured but may not be working"
          : "Reddit API blocked. Configure proxy: codecoder reach configure reddit",
      }
    }

    return {
      status: "warn",
      message: `Reddit API returned ${response.status}`,
    }
  } catch (error) {
    return {
      status: proxy ? "warn" : "off",
      message: proxy
        ? "Reddit API unreachable, proxy may not be working"
        : "Reddit API unreachable. May need proxy configuration",
    }
  }
}

async function checkMcp(channelId: string): Promise<{ status: ChannelInfo["status"]; message: string }> {
  // Check if MCP server is configured
  const mcpName = await ReachConfigManager.getMcpName(channelId)
  if (!mcpName) {
    return {
      status: "off",
      message: `No MCP server configured for ${channelId}. Configure in ~/.codecoder/reach.json`,
    }
  }

  // Check MCP connection status
  try {
    const statuses = await MCP.status()
    const serverStatus = statuses[mcpName]

    if (!serverStatus) {
      return {
        status: "off",
        message: `MCP server '${mcpName}' not found in config`,
      }
    }

    if (serverStatus.status === "connected") {
      return {
        status: "ok",
        message: `MCP server '${mcpName}' connected`,
      }
    }

    if (serverStatus.status === "disabled") {
      return {
        status: "warn",
        message: `MCP server '${mcpName}' is disabled`,
      }
    }

    if (serverStatus.status === "failed") {
      return {
        status: "error",
        message: `MCP server '${mcpName}' failed: ${serverStatus.error}`,
      }
    }

    if (serverStatus.status === "needs_auth") {
      return {
        status: "warn",
        message: `MCP server '${mcpName}' needs authentication`,
      }
    }

    return {
      status: "error",
      message: `MCP server '${mcpName}' status: ${serverStatus.status}`,
    }
  } catch (error) {
    log.error("failed to check MCP status", { channelId, mcpName, error })
    return {
      status: "error",
      message: `Failed to check MCP server '${mcpName}'`,
    }
  }
}

/**
 * Check all channels and return their status
 */
export async function checkAll(): Promise<Record<ChannelId, ChannelInfo>> {
  const results: Partial<Record<ChannelId, ChannelInfo>> = {}

  await Promise.all(
    (Object.entries(CHANNELS) as [ChannelId, ChannelSpec][]).map(async ([id, spec]) => {
      const { status, message } = await spec.check().catch((error) => ({
        status: "error" as const,
        message: `Check failed: ${error.message}`,
      }))

      results[id] = {
        name: spec.name,
        description: spec.description,
        status,
        message,
        tier: spec.tier,
        backends: spec.backends,
      }
    }),
  )

  return results as Record<ChannelId, ChannelInfo>
}

/**
 * Check a specific channel
 */
export async function checkChannel(channelId: ChannelId): Promise<ChannelInfo> {
  const spec = CHANNELS[channelId]
  if (!spec) {
    return {
      name: channelId,
      description: "Unknown channel",
      status: "error",
      message: `Unknown channel: ${channelId}`,
      tier: 0,
      backends: [],
    }
  }

  const { status, message } = await spec.check().catch((error) => ({
    status: "error" as const,
    message: `Check failed: ${error.message}`,
  }))

  return {
    name: spec.name,
    description: spec.description,
    status,
    message,
    tier: spec.tier,
    backends: spec.backends,
  }
}

/**
 * Format doctor results as a human-readable report
 */
export function formatReport(results: Record<ChannelId, ChannelInfo>): string {
  const lines: string[] = ["Agent Reach - Dependency Diagnostics", "=" + "=".repeat(39), ""]

  const statusEmoji: Record<ChannelInfo["status"], string> = {
    ok: "✓",
    warn: "!",
    off: "✗",
    error: "✗",
  }

  const tierLabels: Record<ChannelTier, string> = {
    0: "Tier 0 (Zero Config)",
    1: "Tier 1 (Needs Config)",
    2: "Tier 2 (Needs MCP)",
  }

  // Group by tier
  const byTier = new Map<ChannelTier, [ChannelId, ChannelInfo][]>()

  for (const [id, info] of Object.entries(results) as [ChannelId, ChannelInfo][]) {
    const tier = info.tier
    if (!byTier.has(tier)) {
      byTier.set(tier, [])
    }
    byTier.get(tier)!.push([id, info])
  }

  for (const tier of [0, 1, 2] as ChannelTier[]) {
    const channels = byTier.get(tier)
    if (!channels) continue

    lines.push(tierLabels[tier])
    lines.push("-".repeat(tierLabels[tier].length))

    for (const [_id, info] of channels) {
      const emoji = statusEmoji[info.status]
      lines.push(`  ${emoji} ${info.name.padEnd(12)} ${info.message}`)
    }

    lines.push("")
  }

  // Summary
  const total = Object.keys(results).length
  const ok = Object.values(results).filter((r) => r.status === "ok").length
  const warn = Object.values(results).filter((r) => r.status === "warn").length
  const off = Object.values(results).filter((r) => r.status === "off" || r.status === "error").length

  lines.push("Summary")
  lines.push("-------")
  lines.push(`  ${ok}/${total} channels ready`)
  if (warn > 0) lines.push(`  ${warn} channels need attention`)
  if (off > 0) lines.push(`  ${off} channels unavailable`)

  return lines.join("\n")
}

/**
 * Get list of available (ok status) channels
 */
export async function getAvailableChannels(): Promise<ChannelId[]> {
  const results = await checkAll()
  return (Object.entries(results) as [ChannelId, ChannelInfo][])
    .filter(([_, info]) => info.status === "ok")
    .map(([id]) => id)
}
