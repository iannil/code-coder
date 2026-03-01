/**
 * Redis Streams Infrastructure
 *
 * Provides persistent task queue and event sourcing capabilities using Redis Streams.
 * This is the TypeScript counterpart to the Rust implementation in zero-common.
 *
 * ## Key Features
 *
 * - **Persistent Queue**: Tasks survive restarts, no message loss
 * - **Consumer Groups**: Multiple workers can process tasks concurrently
 * - **Acknowledgement**: Explicit ACK ensures reliable processing
 * - **Replay**: Events can be replayed from any point (checkpoint resume)
 *
 * ## Stream Keys
 *
 * - `tasks:pending` - Pending task queue (entry point)
 * - `tasks:events:{task_id}` - Per-task event stream (event sourcing)
 * - `tasks:state:{task_id}` - Task state projection (Redis Hash)
 */

import Redis from "ioredis"
import { Config } from "@/config/config"
import { Log } from "@/util/log"

const log = Log.create({ service: "redis-streams" })

// ============================================================================
// Configuration
// ============================================================================

export interface RedisStreamConfig {
  /** Redis URL (redis://host:port). */
  url: string
  /** Key prefix for namespacing. */
  keyPrefix: string
  /** Consumer group name. */
  consumerGroup: string
  /** Consumer name (unique per worker). */
  consumerName: string
  /** Pending timeout in milliseconds (for auto-claim). */
  pendingTimeoutMs: number
  /** Heartbeat interval in milliseconds. */
  heartbeatIntervalMs: number
  /** Maximum retries before moving to dead letter. */
  maxRetries: number
  /** Maximum stream length (MAXLEN for trimming). */
  maxStreamLength?: number
  /** Block timeout for XREADGROUP (0 = forever). */
  blockTimeoutMs: number
}

const DEFAULT_CONFIG: RedisStreamConfig = {
  url: "redis://localhost:4410",
  keyPrefix: "codecoder:",
  consumerGroup: "ccode-workers",
  consumerName: `worker-${crypto.randomUUID()}`,
  pendingTimeoutMs: 300_000, // 5 minutes
  heartbeatIntervalMs: 30_000, // 30 seconds
  maxRetries: 3,
  maxStreamLength: 10_000,
  blockTimeoutMs: 5_000, // 5 seconds
}

// ============================================================================
// Stream Message Types
// ============================================================================

/** A message read from a stream. */
export interface StreamMessage {
  /** Stream message ID (e.g., "1234567890123-0"). */
  id: string
  /** Field-value pairs from the message. */
  fields: Record<string, string>
}

/** Pending message info (from XPENDING). */
export interface PendingMessage {
  /** Message ID. */
  id: string
  /** Consumer that owns this message. */
  consumer: string
  /** Idle time in milliseconds. */
  idleMs: number
  /** Delivery count. */
  deliveryCount: number
}

// ============================================================================
// Stream Keys Helper
// ============================================================================

export const streamKeys = {
  /** Pending task queue (entry point). */
  TASKS_PENDING: "tasks:pending",

  /** Per-task event stream. */
  taskEvents: (taskId: string): string => `tasks:events:${taskId}`,

  /** Per-task state projection. */
  taskState: (taskId: string): string => `tasks:state:${taskId}`,

  /** Dead letter queue for failed tasks. */
  TASKS_DEAD_LETTER: "tasks:dead_letter",
}

// ============================================================================
// Redis Stream Client
// ============================================================================

export class RedisStreamClient {
  private client: Redis | null = null
  private config: RedisStreamConfig

  constructor(config: Partial<RedisStreamConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Initialize Redis connection.
   */
  async init(redisUrl?: string): Promise<void> {
    if (this.client) {
      log.warn("RedisStreamClient already initialized")
      return
    }

    const appConfig = await Config.get()
    const redisConfig = appConfig.redis

    const url =
      redisUrl ?? process.env.REDIS_URL ?? redisConfig?.url ?? this.config.url

    // Merge task queue config if available
    const taskQueueConfig = appConfig.taskQueue
    if (taskQueueConfig) {
      this.config.consumerGroup =
        taskQueueConfig.consumerGroup ?? this.config.consumerGroup
      this.config.pendingTimeoutMs =
        taskQueueConfig.pendingTimeoutMs ?? this.config.pendingTimeoutMs
      this.config.heartbeatIntervalMs =
        taskQueueConfig.heartbeatIntervalMs ?? this.config.heartbeatIntervalMs
      this.config.maxRetries =
        taskQueueConfig.maxRetries ?? this.config.maxRetries
    }

    this.client = new Redis(url, {
      password: process.env.REDIS_PASSWORD ?? redisConfig?.password,
      db: redisConfig?.db ?? 0,
      connectTimeout: redisConfig?.connectTimeout ?? 5000,
      commandTimeout: redisConfig?.commandTimeout ?? 10000,
      maxRetriesPerRequest: redisConfig?.maxRetriesPerRequest ?? 3,
      lazyConnect: true,
    })

    await this.client.connect()
    await this.client.ping()

    log.info("RedisStreamClient initialized", {
      url: url.replace(/\/\/.*@/, "//***@"),
      consumerGroup: this.config.consumerGroup,
      consumerName: this.config.consumerName,
    })
  }

  /** Get the configuration. */
  getConfig(): RedisStreamConfig {
    return this.config
  }

  /** Get prefixed key. */
  private key(name: string): string {
    return `${this.config.keyPrefix}${name}`
  }

  /** Check if connected. */
  isInitialized(): boolean {
    return this.client !== null
  }

  /** Health check. */
  async isHealthy(): Promise<boolean> {
    if (!this.client) return false
    try {
      const result = await this.client.ping()
      return result === "PONG"
    } catch {
      return false
    }
  }

  /** Close connection. */
  async close(): Promise<void> {
    if (this.client) {
      await this.client.quit()
      this.client = null
      log.info("RedisStreamClient closed")
    }
  }

  // ==========================================================================
  // Stream Operations
  // ==========================================================================

  /**
   * Add a message to a stream (XADD).
   * @returns The message ID.
   */
  async xadd<T>(stream: string, payload: T): Promise<string> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)
    const payloadJson = JSON.stringify(payload)

    let result: string | null

    if (this.config.maxStreamLength) {
      result = await this.client.xadd(
        key,
        "MAXLEN",
        "~",
        this.config.maxStreamLength,
        "*",
        "payload",
        payloadJson,
      )
    } else {
      result = await this.client.xadd(key, "*", "payload", payloadJson)
    }

    if (!result) {
      throw new Error("Failed to add message to stream")
    }

    log.debug("Added message to stream", { stream: key, id: result })
    return result
  }

  /**
   * Add a message with additional fields.
   */
  async xaddWithFields(
    stream: string,
    fields: Record<string, string>,
  ): Promise<string> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)
    const args: string[] = []

    for (const [k, v] of Object.entries(fields)) {
      args.push(k, v)
    }

    let result: string | null

    if (this.config.maxStreamLength) {
      result = await this.client.xadd(
        key,
        "MAXLEN",
        "~",
        this.config.maxStreamLength,
        "*",
        ...args,
      )
    } else {
      result = await this.client.xadd(key, "*", ...args)
    }

    if (!result) {
      throw new Error("Failed to add message to stream")
    }

    return result
  }

  /**
   * Read messages from a stream (XREAD).
   * Use `lastId` = "0" to read from beginning, "$" to read only new messages.
   */
  async xread(
    stream: string,
    lastId: string,
    count: number,
    blockMs?: number,
  ): Promise<StreamMessage[]> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)

    let result: [string, [string, string[]][]][] | null

    if (blockMs !== undefined) {
      result = await this.client.xread(
        "COUNT",
        count,
        "BLOCK",
        blockMs,
        "STREAMS",
        key,
        lastId,
      )
    } else {
      result = await this.client.xread("COUNT", count, "STREAMS", key, lastId)
    }

    if (!result) return []

    return this.parseStreamEntries(result)
  }

  /**
   * Ensure consumer group exists (XGROUP CREATE).
   */
  async ensureConsumerGroup(stream: string): Promise<void> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)

    try {
      await this.client.xgroup("CREATE", key, this.config.consumerGroup, "0", "MKSTREAM")
      log.info("Created consumer group", {
        stream: key,
        group: this.config.consumerGroup,
      })
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      // BUSYGROUP means group already exists - that's fine
      if (errorMessage.includes("BUSYGROUP")) {
        log.debug("Consumer group already exists", {
          stream: key,
          group: this.config.consumerGroup,
        })
      } else {
        throw error
      }
    }
  }

  /**
   * Read messages as a consumer group member (XREADGROUP).
   * Use `lastId` = ">" to read only new messages assigned to this consumer.
   * Use "0" to re-read pending messages.
   */
  async xreadgroup(
    stream: string,
    lastId: string,
    count: number,
  ): Promise<StreamMessage[]> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)

    const result = await this.client.xreadgroup(
      "GROUP",
      this.config.consumerGroup,
      this.config.consumerName,
      "COUNT",
      count,
      "BLOCK",
      this.config.blockTimeoutMs,
      "STREAMS",
      key,
      lastId,
    )

    if (!result) return []

    return this.parseStreamEntries(result as [string, [string, string[]][]][])
  }

  /**
   * Acknowledge message processing (XACK).
   */
  async xack(stream: string, messageId: string): Promise<void> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)
    await this.client.xack(key, this.config.consumerGroup, messageId)

    log.debug("Acknowledged message", { stream: key, id: messageId })
  }

  /**
   * Get pending messages (XPENDING).
   */
  async xpending(stream: string, count: number): Promise<PendingMessage[]> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)

    // XPENDING stream group [start end count] [consumer]
    const result = (await this.client.xpending(
      key,
      this.config.consumerGroup,
      "-",
      "+",
      count,
    )) as [string, string, number, number][]

    return result.map(([id, consumer, idleMs, deliveryCount]) => ({
      id,
      consumer,
      idleMs,
      deliveryCount,
    }))
  }

  /**
   * Claim pending messages that have been idle too long (XCLAIM).
   */
  async xclaim(
    stream: string,
    messageIds: string[],
    minIdleMs: number,
  ): Promise<StreamMessage[]> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")
    if (messageIds.length === 0) return []

    const key = this.key(stream)

    const result = (await this.client.xclaim(
      key,
      this.config.consumerGroup,
      this.config.consumerName,
      minIdleMs,
      ...messageIds,
    )) as [string, string[]][]

    return result.map(([id, fields]) => ({
      id,
      fields: this.parseFields(fields),
    }))
  }

  // ==========================================================================
  // State Projection (Hash Operations)
  // ==========================================================================

  /**
   * Set task state fields (HSET).
   */
  async hset(key: string, fields: Record<string, string>): Promise<void> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    const args: string[] = []

    for (const [k, v] of Object.entries(fields)) {
      args.push(k, v)
    }

    await this.client.hset(fullKey, ...args)
  }

  /**
   * Get all task state fields (HGETALL).
   */
  async hgetall(key: string): Promise<Record<string, string>> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    return this.client.hgetall(fullKey)
  }

  /**
   * Get specific field from hash (HGET).
   */
  async hget(key: string, field: string): Promise<string | null> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    return this.client.hget(fullKey, field)
  }

  /**
   * Increment a hash field (HINCRBY).
   */
  async hincrby(key: string, field: string, incr: number): Promise<number> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    return this.client.hincrby(fullKey, field, incr)
  }

  /**
   * Delete a key (DEL).
   */
  async del(key: string): Promise<boolean> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    const result = await this.client.del(fullKey)
    return result > 0
  }

  /**
   * Set key expiration (EXPIRE).
   */
  async expire(key: string, seconds: number): Promise<boolean> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const fullKey = this.key(key)
    const result = await this.client.expire(fullKey, seconds)
    return result === 1
  }

  /**
   * Get stream length (XLEN).
   */
  async xlen(stream: string): Promise<number> {
    if (!this.client) throw new Error("RedisStreamClient not initialized")

    const key = this.key(stream)
    return this.client.xlen(key)
  }

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  /** Parse stream entries from XREAD/XREADGROUP result. */
  private parseStreamEntries(
    result: [string, [string, string[]][]][],
  ): StreamMessage[] {
    const messages: StreamMessage[] = []

    for (const [, entries] of result) {
      for (const [id, fields] of entries) {
        messages.push({
          id,
          fields: this.parseFields(fields),
        })
      }
    }

    return messages
  }

  /** Parse field array to record. */
  private parseFields(fields: string[]): Record<string, string> {
    const result: Record<string, string> = {}

    for (let i = 0; i < fields.length; i += 2) {
      result[fields[i]] = fields[i + 1]
    }

    return result
  }

  /**
   * Parse payload from stream message.
   */
  parsePayload<T>(message: StreamMessage): T {
    const payload = message.fields["payload"]
    if (!payload) {
      throw new Error("No payload field in message")
    }
    return JSON.parse(payload) as T
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

let instance: RedisStreamClient | null = null

/**
 * Get the global Redis Stream client instance.
 */
export async function getRedisStreamClient(): Promise<RedisStreamClient> {
  if (!instance) {
    instance = new RedisStreamClient()
    await instance.init()
  }
  return instance
}

/**
 * Close the global Redis Stream client instance.
 */
export async function closeRedisStreamClient(): Promise<void> {
  if (instance) {
    await instance.close()
    instance = null
  }
}

/**
 * Check if Redis Streams are available.
 */
export async function isRedisStreamsAvailable(): Promise<boolean> {
  try {
    const client = await getRedisStreamClient()
    return client.isHealthy()
  } catch {
    return false
  }
}
