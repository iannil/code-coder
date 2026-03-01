/**
 * Autonomous Session Store
 *
 * Redis-backed storage for per-conversation autonomous mode state.
 * Enables IM users to toggle autonomous mode on/off for their conversations.
 */

import Redis from "ioredis"
import { Config } from "@/config/config"
import { Log } from "@/util/log"

const log = Log.create({ service: "store.autonomous-session" })

export namespace AutonomousSessionStore {
  const KEY_PREFIX = "autonomous:"
  let client: Redis | null = null
  let keyPrefix = "codecoder:"

  /**
   * State of autonomous mode for a conversation
   */
  export interface AutonomousState {
    /** Whether autonomous mode is enabled */
    enabled: boolean
    /** Autonomy level (wild, crazy, etc.) */
    autonomyLevel: string
    /** When autonomous mode was enabled */
    enabledAt: number
    /** User who enabled autonomous mode */
    enabledBy: string
  }

  /**
   * Initialize Redis connection.
   * Must be called before using other methods.
   */
  export async function init(redisUrl?: string): Promise<void> {
    if (client) {
      log.warn("AutonomousSessionStore already initialized")
      return
    }

    const config = await Config.get()
    const redisConfig: Config.RedisConfig = config.redis

    const url = redisUrl
      ?? process.env.REDIS_URL
      ?? redisConfig?.url
      ?? "redis://localhost:4410"

    keyPrefix = redisConfig?.keyPrefix ?? "codecoder:"

    client = new Redis(url, {
      password: process.env.REDIS_PASSWORD ?? redisConfig?.password,
      db: redisConfig?.db ?? 0,
      connectTimeout: redisConfig?.connectTimeout ?? 5000,
      commandTimeout: redisConfig?.commandTimeout ?? 3000,
      maxRetriesPerRequest: redisConfig?.maxRetriesPerRequest ?? 3,
      lazyConnect: true,
      retryStrategy: (times) => {
        if (times > 1) return null
        return Math.min(times * 100, 1000)
      },
      enableOfflineQueue: false,
    })

    client.on("error", (err) => {
      log.debug("Redis connection error (handled)", { error: err.message })
    })

    await client.connect()
    await client.ping()

    log.info("AutonomousSessionStore initialized", { url: url.replace(/\/\/.*@/, "//***@") })
  }

  /**
   * Set autonomous mode enabled/disabled for a conversation.
   */
  export async function setEnabled(
    conversationId: string,
    enabled: boolean,
    userId?: string,
    autonomyLevel?: string
  ): Promise<void> {
    if (!client) throw new Error("AutonomousSessionStore not initialized")

    const key = `${keyPrefix}${KEY_PREFIX}${conversationId}`

    if (enabled) {
      const state: AutonomousState = {
        enabled: true,
        autonomyLevel: autonomyLevel ?? "wild",
        enabledAt: Date.now(),
        enabledBy: userId ?? "unknown"
      }
      await client.set(key, JSON.stringify(state))

      log.info("Autonomous mode enabled", {
        conversationId,
        autonomyLevel: state.autonomyLevel,
        userId
      })
    } else {
      await client.del(key)

      log.info("Autonomous mode disabled", {
        conversationId,
        userId
      })
    }
  }

  /**
   * Get autonomous state for a conversation.
   * Returns null if autonomous mode is not enabled.
   */
  export async function getState(conversationId: string): Promise<AutonomousState | null> {
    if (!client) throw new Error("AutonomousSessionStore not initialized")

    const key = `${keyPrefix}${KEY_PREFIX}${conversationId}`
    const data = await client.get(key)

    if (!data) return null

    try {
      return JSON.parse(data) as AutonomousState
    } catch {
      log.warn("Failed to parse autonomous state", { conversationId })
      return null
    }
  }

  /**
   * Check if autonomous mode is enabled for a conversation.
   */
  export async function isEnabled(conversationId: string): Promise<boolean> {
    try {
      const state = await getState(conversationId)
      return state?.enabled ?? false
    } catch {
      return false
    }
  }

  /**
   * Health check - verify Redis connection is alive.
   */
  export async function healthCheck(): Promise<boolean> {
    if (!client) return false
    try {
      const result = await client.ping()
      return result === "PONG"
    } catch {
      return false
    }
  }

  /**
   * Close Redis connection.
   * Call during graceful shutdown.
   */
  export async function close(): Promise<void> {
    if (client) {
      await client.quit()
      client = null
      log.info("AutonomousSessionStore closed")
    }
  }

  /**
   * Check if store is initialized.
   */
  export function isInitialized(): boolean {
    return client !== null
  }
}
