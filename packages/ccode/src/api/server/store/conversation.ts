/**
 * Conversation Store
 *
 * Redis-backed storage for conversation_id to session_id mappings.
 * Enables multi-instance deployment by sharing conversation state.
 */

import Redis from "ioredis"
import { Config } from "@/config/config"
import { Log } from "@/util/log"

const log = Log.create({ service: "conversation-store" })

export namespace ConversationStore {
  const KEY_PREFIX = "conv:"
  let client: Redis | null = null
  let keyPrefix = "codecoder:"

  /**
   * Initialize Redis connection.
   * Must be called before using other methods.
   */
  export async function init(redisUrl?: string): Promise<void> {
    if (client) {
      log.warn("ConversationStore already initialized")
      return
    }

    const config = await Config.get()
    const redisConfig: Config.RedisConfig = config.redis

    const url = redisUrl
      ?? process.env.REDIS_URL
      ?? redisConfig?.url
      ?? "redis://localhost:6379"

    keyPrefix = redisConfig?.keyPrefix ?? "codecoder:"

    client = new Redis(url, {
      password: process.env.REDIS_PASSWORD ?? redisConfig?.password,
      db: redisConfig?.db ?? 0,
      connectTimeout: redisConfig?.connectTimeout ?? 5000,
      commandTimeout: redisConfig?.commandTimeout ?? 3000,
      maxRetriesPerRequest: redisConfig?.maxRetriesPerRequest ?? 3,
      lazyConnect: true,
    })

    // Connect and verify
    await client.connect()
    await client.ping()

    log.info("ConversationStore initialized", { url: url.replace(/\/\/.*@/, "//***@") })
  }

  /**
   * Get session_id for a conversation.
   * Returns null if not found.
   */
  export async function get(conversationId: string): Promise<string | null> {
    if (!client) throw new Error("ConversationStore not initialized")
    return client.get(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
  }

  /**
   * Set mapping from conversation_id to session_id.
   */
  export async function set(conversationId: string, sessionId: string): Promise<void> {
    if (!client) throw new Error("ConversationStore not initialized")
    await client.set(`${keyPrefix}${KEY_PREFIX}${conversationId}`, sessionId)
  }

  /**
   * Delete a conversation mapping.
   * Returns true if key existed, false otherwise.
   */
  export async function delete_(conversationId: string): Promise<boolean> {
    if (!client) throw new Error("ConversationStore not initialized")
    const result = await client.del(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
    return result > 0
  }

  /**
   * Check if a conversation mapping exists.
   */
  export async function exists(conversationId: string): Promise<boolean> {
    if (!client) throw new Error("ConversationStore not initialized")
    const result = await client.exists(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
    return result > 0
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
      log.info("ConversationStore closed")
    }
  }

  /**
   * Check if store is initialized.
   */
  export function isInitialized(): boolean {
    return client !== null
  }
}
