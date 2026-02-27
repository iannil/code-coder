import { ReachConfig } from "./types"
import { Global } from "../../global"
import { Log } from "@/util/log"
import path from "path"
import fs from "fs/promises"

/**
 * Agent Reach - Configuration Management
 *
 * Manages reach-specific config stored in ~/.codecoder/reach.json
 * This file is separate from the main config to keep sensitive data (cookies, tokens) isolated.
 */

const log = Log.create({ service: "reach.config" })

const CONFIG_FILENAME = "reach.json"
const CONFIG_PERMISSIONS = 0o600 // Owner read/write only

export namespace ReachConfigManager {
  function configPath(): string {
    return path.join(Global.Path.config, CONFIG_FILENAME)
  }

  /**
   * Load reach configuration from ~/.codecoder/reach.json
   * Returns default empty config if file doesn't exist
   */
  export async function load(): Promise<ReachConfig> {
    const filepath = configPath()

    try {
      const content = await Bun.file(filepath).text()
      const data = JSON.parse(content)
      const parsed = ReachConfig.safeParse(data)

      if (!parsed.success) {
        log.warn("invalid reach config, using defaults", { issues: parsed.error.issues })
        return {}
      }

      return parsed.data
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return {}
      }
      log.error("failed to load reach config", { error })
      return {}
    }
  }

  /**
   * Save reach configuration to ~/.codecoder/reach.json
   * Sets restrictive file permissions (600) for security
   */
  export async function save(config: ReachConfig): Promise<void> {
    const filepath = configPath()

    try {
      // Ensure config directory exists
      await fs.mkdir(path.dirname(filepath), { recursive: true })

      // Write config with pretty formatting
      await Bun.write(filepath, JSON.stringify(config, null, 2))

      // Set restrictive permissions
      await fs.chmod(filepath, CONFIG_PERMISSIONS)

      log.info("saved reach config", { path: filepath })
    } catch (error) {
      log.error("failed to save reach config", { error })
      throw error
    }
  }

  /**
   * Update specific fields in reach configuration
   */
  export async function update(updates: Partial<ReachConfig>): Promise<ReachConfig> {
    const current = await load()
    const merged = { ...current, ...updates }
    await save(merged)
    return merged
  }

  /**
   * Get proxy configuration if set
   */
  export async function getProxy(): Promise<string | undefined> {
    const config = await load()
    return config.proxy
  }

  /**
   * Check if a specific channel has required configuration
   */
  export async function hasChannelConfig(channelId: string): Promise<boolean> {
    const config = await load()

    switch (channelId) {
      case "twitter":
        return !!config.twitter?.cookies
      case "xiaohongshu":
        return !!config.xiaohongshu?.mcpName
      case "douyin":
        return !!config.douyin?.mcpName
      case "linkedin":
        return !!config.linkedin?.mcpName
      case "bosszhipin":
        return !!config.bosszhipin?.mcpName
      default:
        return true // Tier 0 channels don't need config
    }
  }

  /**
   * Get MCP server name for a channel
   */
  export async function getMcpName(channelId: string): Promise<string | undefined> {
    const config = await load()

    switch (channelId) {
      case "xiaohongshu":
        return config.xiaohongshu?.mcpName
      case "douyin":
        return config.douyin?.mcpName
      case "linkedin":
        return config.linkedin?.mcpName
      case "bosszhipin":
        return config.bosszhipin?.mcpName
      default:
        return undefined
    }
  }
}
