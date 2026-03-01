/**
 * Task Stream Consumer
 *
 * Consumes tasks from Redis Streams and executes them.
 * This replaces the direct HTTP task creation with queue-based processing.
 *
 * ## Consumer Group Pattern
 *
 * Multiple workers can run simultaneously, each in the same consumer group.
 * Redis ensures each task is delivered to exactly one consumer.
 *
 * ## Reliability Features
 *
 * - **XACK**: Acknowledges successful processing
 * - **Auto-claim**: Takes over tasks from crashed consumers
 * - **Heartbeat**: Keeps task alive during long execution
 */

import { Log } from "@/util/log"
import {
  RedisStreamClient,
  streamKeys,
} from "@/infrastructure/redis"
import type { StreamMessage } from "@/infrastructure/redis"
import {
  StreamEventEnvelope,
  StreamTaskEvent,
  TaskStateProjection,
  createStreamEvent,
  createInitialState,
  applyEventToState,
  stateToHashFields,
  stateFromHashFields,
} from "./events"
import { TaskStore } from "./store"
import { TaskEmitter } from "./emitter"
import { Config } from "@/config/config"

const log = Log.create({ service: "task-consumer" })

// ============================================================================
// Configuration
// ============================================================================

export interface TaskConsumerConfig {
  /** Consumer group name. */
  consumerGroup: string
  /** Consumer name (unique per worker). */
  consumerName: string
  /** Number of tasks to fetch per poll. */
  batchSize: number
  /** Block timeout for XREADGROUP (ms). */
  blockTimeoutMs: number
  /** Heartbeat interval (ms). */
  heartbeatIntervalMs: number
  /** Pending task timeout (ms) for auto-claim. */
  pendingTimeoutMs: number
  /** Maximum retries before moving to dead letter. */
  maxRetries: number
  /** Per-tool execution timeout (ms). */
  toolTimeoutMs: number
  /** Global task timeout (ms). */
  globalTimeoutMs: number
}

const DEFAULT_CONFIG: TaskConsumerConfig = {
  consumerGroup: "ccode-workers",
  consumerName: `worker-${crypto.randomUUID()}`,
  batchSize: 1, // Process one task at a time for reliability
  blockTimeoutMs: 5000,
  heartbeatIntervalMs: 30_000,
  pendingTimeoutMs: 300_000, // 5 minutes
  maxRetries: 3,
  toolTimeoutMs: 60_000, // 1 minute per tool
  globalTimeoutMs: 30 * 60_000, // 30 minutes total
}

// ============================================================================
// Task Request (from Stream)
// ============================================================================

export interface TaskRequest {
  taskId: string
  userId: string
  channel: string
  channelId: string
  prompt: string
  agent: string
  traceId: string
  chatHistory: unknown[]
  replyToMessageId?: string
  createdAt: number
}

// ============================================================================
// Consumer State
// ============================================================================

interface ConsumerState {
  running: boolean
  currentTaskId: string | null
  heartbeatInterval: ReturnType<typeof setInterval> | null
  eventSeq: number
}

// ============================================================================
// Task Consumer
// ============================================================================

export class TaskConsumer {
  private config: TaskConsumerConfig
  private client: RedisStreamClient | null = null
  private state: ConsumerState = {
    running: false,
    currentTaskId: null,
    heartbeatInterval: null,
    eventSeq: 0,
  }

  constructor(config: Partial<TaskConsumerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Initialize the consumer.
   */
  async init(): Promise<void> {
    this.client = new RedisStreamClient({
      consumerGroup: this.config.consumerGroup,
      consumerName: this.config.consumerName,
      blockTimeoutMs: this.config.blockTimeoutMs,
    })

    await this.client.init()

    // Ensure consumer group exists
    await this.client.ensureConsumerGroup(streamKeys.TASKS_PENDING)

    log.info("Task consumer initialized", {
      consumerGroup: this.config.consumerGroup,
      consumerName: this.config.consumerName,
    })
  }

  /**
   * Start consuming tasks.
   * This runs in a loop until stop() is called.
   */
  async start(): Promise<void> {
    if (!this.client) {
      throw new Error("Consumer not initialized")
    }

    this.state.running = true
    log.info("Starting task consumer loop")

    while (this.state.running) {
      try {
        // First, try to claim any abandoned tasks
        await this.claimAbandonedTasks()

        // Then, fetch new tasks
        const messages = await this.client.xreadgroup(
          streamKeys.TASKS_PENDING,
          ">", // Only new messages
          this.config.batchSize,
        )

        for (const message of messages) {
          if (!this.state.running) break
          await this.processMessage(message)
        }
      } catch (error) {
        log.error("Error in consumer loop", {
          error: error instanceof Error ? error.message : String(error),
        })
        // Wait before retrying
        await new Promise((resolve) => setTimeout(resolve, 1000))
      }
    }

    log.info("Task consumer stopped")
  }

  /**
   * Stop the consumer.
   */
  stop(): void {
    this.state.running = false
    this.stopHeartbeat()
    log.info("Stopping task consumer")
  }

  /**
   * Claim abandoned tasks from crashed consumers.
   */
  private async claimAbandonedTasks(): Promise<void> {
    if (!this.client) return

    try {
      const pending = await this.client.xpending(
        streamKeys.TASKS_PENDING,
        10, // Check up to 10 pending
      )

      const abandonedIds = pending
        .filter((p) => p.idleMs > this.config.pendingTimeoutMs)
        .filter((p) => p.deliveryCount < this.config.maxRetries)
        .map((p) => p.id)

      if (abandonedIds.length > 0) {
        const claimed = await this.client.xclaim(
          streamKeys.TASKS_PENDING,
          abandonedIds,
          this.config.pendingTimeoutMs,
        )

        for (const message of claimed) {
          log.info("Claimed abandoned task", { messageId: message.id })
          await this.processMessage(message)
        }
      }

      // Move tasks with too many retries to dead letter
      const deadLetters = pending
        .filter((p) => p.deliveryCount >= this.config.maxRetries)
        .map((p) => p.id)

      if (deadLetters.length > 0) {
        for (const id of deadLetters) {
          await this.moveToDeadLetter(id)
        }
      }
    } catch (error) {
      log.warn("Error claiming abandoned tasks", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Move a message to dead letter queue.
   */
  private async moveToDeadLetter(messageId: string): Promise<void> {
    if (!this.client) return

    try {
      // Read the message
      const messages = await this.client.xread(
        streamKeys.TASKS_PENDING,
        messageId,
        1,
      )

      if (messages.length > 0) {
        const message = messages[0]
        // Add to dead letter
        await this.client.xaddWithFields(streamKeys.TASKS_DEAD_LETTER, {
          ...message.fields,
          original_id: messageId,
          reason: "max_retries_exceeded",
        })
      }

      // ACK the original
      await this.client.xack(streamKeys.TASKS_PENDING, messageId)

      log.warn("Moved task to dead letter", { messageId })
    } catch (error) {
      log.error("Error moving to dead letter", {
        messageId,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Process a single task message.
   */
  private async processMessage(message: StreamMessage): Promise<void> {
    const taskRequest = this.client!.parsePayload<TaskRequest>(message)
    const taskId = taskRequest.taskId

    log.info("Processing task", {
      taskId,
      agent: taskRequest.agent,
      traceId: taskRequest.traceId,
    })

    this.state.currentTaskId = taskId
    this.state.eventSeq = 0

    // Start heartbeat
    this.startHeartbeat(taskId)

    try {
      // Publish task_started event
      await this.publishEvent(taskId, {
        type: "task_started",
        data: {
          agent: taskRequest.agent,
          sessionId: "", // Will be set during execution
          traceId: taskRequest.traceId,
        },
      })

      // Update state to running
      await this.updateTaskState(taskId, { status: "running" })

      // Execute the task
      await this.executeTask(taskRequest)

      // ACK the message on success
      await this.client!.xack(streamKeys.TASKS_PENDING, message.id)

      log.info("Task completed successfully", { taskId })
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)

      log.error("Task execution failed", { taskId, error: errorMessage })

      // Publish failure event
      await this.publishEvent(taskId, {
        type: "task_failed",
        data: {
          error: errorMessage,
          recoverable: false,
        },
      })

      // Update state to failed
      await this.updateTaskState(taskId, {
        status: "failed",
        error: errorMessage,
      })

      // ACK the message (don't retry after max attempts handled elsewhere)
      await this.client!.xack(streamKeys.TASKS_PENDING, message.id)
    } finally {
      this.stopHeartbeat()
      this.state.currentTaskId = null
    }
  }

  /**
   * Execute a task (integrate with existing task execution).
   */
  private async executeTask(request: TaskRequest): Promise<void> {
    // This integrates with the existing TaskStore and execution logic
    // For now, we'll use the existing task handler pattern

    const task = await TaskStore.create({
      agent: request.agent,
      prompt: request.prompt,
      sessionID: request.taskId,
      context: {
        userID: request.userId,
        platform: request.channel,
        conversationId: `${request.channel}:${request.channelId}`,
        source: "remote",
      },
    })

    // Forward events from existing emitter to Redis Stream
    // This is a bridge until we fully migrate

    // The actual execution is handled by the existing handler
    // which will be refactored in Phase 2

    // For now, emit a placeholder
    await this.publishEvent(request.taskId, {
      type: "progress",
      data: {
        stage: "executing",
        message: "Task execution started",
        percentage: 10,
      },
    })
  }

  /**
   * Publish an event to the task's event stream.
   */
  private async publishEvent(
    taskId: string,
    event: StreamTaskEvent,
  ): Promise<void> {
    if (!this.client) return

    this.state.eventSeq++
    const envelope = createStreamEvent(
      this.state.eventSeq,
      event,
      undefined, // trace_id handled elsewhere
      undefined, // span_id handled elsewhere
    )

    await this.client.xadd(streamKeys.taskEvents(taskId), envelope)

    // Also update state projection
    const currentState = await this.getTaskState(taskId)
    const newState = applyEventToState(currentState, envelope)
    await this.client.hset(
      streamKeys.taskState(taskId),
      stateToHashFields(newState),
    )
  }

  /**
   * Get current task state.
   */
  private async getTaskState(taskId: string): Promise<TaskStateProjection> {
    if (!this.client) {
      return createInitialState(taskId)
    }

    const fields = await this.client.hgetall(streamKeys.taskState(taskId))

    if (Object.keys(fields).length === 0) {
      return createInitialState(taskId)
    }

    return stateFromHashFields(fields)
  }

  /**
   * Update task state fields.
   */
  private async updateTaskState(
    taskId: string,
    updates: Partial<TaskStateProjection>,
  ): Promise<void> {
    if (!this.client) return

    const fields: Record<string, string> = {
      updatedAt: new Date().toISOString(),
    }

    if (updates.status) fields.status = updates.status
    if (updates.currentAgent) fields.currentAgent = updates.currentAgent
    if (updates.progressPct !== undefined)
      fields.progressPct = updates.progressPct.toString()
    if (updates.error) fields.error = updates.error

    await this.client.hset(streamKeys.taskState(taskId), fields)
  }

  /**
   * Start heartbeat for the current task.
   */
  private startHeartbeat(taskId: string): void {
    this.stopHeartbeat() // Clear any existing

    const startTime = Date.now()

    this.state.heartbeatInterval = setInterval(async () => {
      const elapsedMs = Date.now() - startTime

      // Check global timeout
      if (elapsedMs > this.config.globalTimeoutMs) {
        log.warn("Task exceeded global timeout", { taskId, elapsedMs })
        this.stop()
        return
      }

      // Publish heartbeat event
      await this.publishEvent(taskId, {
        type: "heartbeat",
        data: {
          elapsedMs,
        },
      })
    }, this.config.heartbeatIntervalMs)
  }

  /**
   * Stop heartbeat.
   */
  private stopHeartbeat(): void {
    if (this.state.heartbeatInterval) {
      clearInterval(this.state.heartbeatInterval)
      this.state.heartbeatInterval = null
    }
  }

  /**
   * Get consumer health status.
   */
  async getHealth(): Promise<{
    healthy: boolean
    running: boolean
    currentTask: string | null
    queueDepth: number
  }> {
    const queueDepth = this.client
      ? await this.client.xlen(streamKeys.TASKS_PENDING)
      : 0

    return {
      healthy: this.client?.isInitialized() ?? false,
      running: this.state.running,
      currentTask: this.state.currentTaskId,
      queueDepth,
    }
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

let instance: TaskConsumer | null = null

/**
 * Get or create the global task consumer.
 */
export async function getTaskConsumer(): Promise<TaskConsumer> {
  if (!instance) {
    const config = await Config.get()
    const taskQueueConfig = config.taskQueue

    instance = new TaskConsumer({
      consumerGroup: taskQueueConfig?.consumerGroup,
      pendingTimeoutMs: taskQueueConfig?.pendingTimeoutMs,
      heartbeatIntervalMs: taskQueueConfig?.heartbeatIntervalMs,
      maxRetries: taskQueueConfig?.maxRetries,
    })

    await instance.init()
  }

  return instance
}

/**
 * Start the task consumer in the background.
 */
export async function startTaskConsumer(): Promise<void> {
  const consumer = await getTaskConsumer()
  // Don't await - runs in background
  consumer.start().catch((error) => {
    log.error("Task consumer crashed", {
      error: error instanceof Error ? error.message : String(error),
    })
  })
}

/**
 * Stop the task consumer.
 */
export function stopTaskConsumer(): void {
  if (instance) {
    instance.stop()
    instance = null
  }
}
