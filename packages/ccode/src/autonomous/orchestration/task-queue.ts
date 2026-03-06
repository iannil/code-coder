/**
 * Task Queue for Autonomous Mode
 *
 * This module provides a TypeScript wrapper around the native Rust task queue implementation.
 * The native implementation offers:
 * - Priority-based scheduling using a binary heap
 * - Dependency resolution with topological ordering
 * - Retry logic with configurable backoff
 * - Concurrent task execution limits
 *
 * @package autonomous
 */

import { Log } from "@/util/log"
import { AutonomousEvent } from "../events"
import { Bus } from "@/bus"
import { createTaskQueue as rawNativeCreateTaskQueue } from "@codecoder-ai/core"

const log = Log.create({ service: "autonomous.task-queue" })

// ============================================================================
// Native Type Definitions
// ============================================================================

interface NativeTaskQueueConfig {
  maxConcurrent?: number
  maxRetries?: number
  retryDelayMs?: number
}

interface NativeTaskQueueStats {
  total: number
  pending: number
  running: number
  completed: number
  failed: number
  skipped: number
  blocked: number
}

interface NapiTask {
  id: string
  sessionId: string
  subject: string
  description: string
  status: string
  priority: string
  dependencies: string[]
  dependents: string[]
  createdAt: number
  startedAt?: number
  completedAt?: number
  error?: string
  retryCount: number
  maxRetries: number
  agent?: string
}

interface NativeTaskQueueHandle {
  addTask(subject: string, description: string, priority: string): string
  addTaskWithDeps(subject: string, description: string, priority: string, dependencies: string[]): string
  getTask(id: string): NapiTask | null
  allTasks(): NapiTask[]
  runnableTasks(): NapiTask[]
  startTask(id: string): void
  completeTask(id: string): number
  failTask(id: string, error: string, retryable: boolean): boolean
  skipTask(id: string, reason?: string): void
  blockTask(id: string, reason?: string): void
  unblockTask(id: string): void
  retryTask(id: string): void
  stats(): NativeTaskQueueStats
  isComplete(): boolean
  hasFailures(): boolean
  failedTasks(): NapiTask[]
  taskChain(id: string): NapiTask[]
  clear(): void
  sessionId(): string
  serialize(): string
}

type NativeCreateTaskQueue = (sessionId: string, config?: NativeTaskQueueConfig) => NativeTaskQueueHandle

// Cast the raw import to our properly typed function
const nativeCreateTaskQueue = rawNativeCreateTaskQueue as unknown as NativeCreateTaskQueue | undefined

// ============================================================================
// Native Binding Validation
// ============================================================================

if (!nativeCreateTaskQueue) {
  throw new Error(
    "Native task queue bindings not available. Ensure @codecoder-ai/core is built with 'bun run build' in packages/core",
  )
}

// ============================================================================
// Type Conversion Helpers
// ============================================================================

// Priority mapping (TS lowercase → Native PascalCase)
function toNativePriority(priority: TaskPriority): string {
  return priority.charAt(0).toUpperCase() + priority.slice(1)
}

// Convert native task to TS task
function fromNativeTask(t: NapiTask): Task {
  return {
    id: t.id,
    sessionId: t.sessionId,
    subject: t.subject,
    description: t.description,
    status: t.status.toLowerCase() as TaskStatus,
    priority: t.priority.toLowerCase() as TaskPriority,
    dependencies: t.dependencies,
    dependents: t.dependents,
    createdAt: t.createdAt,
    startedAt: t.startedAt,
    completedAt: t.completedAt,
    error: t.error,
    retryCount: t.retryCount,
    maxRetries: t.maxRetries,
    metadata: {},
    agent: t.agent,
  }
}

/**
 * Task priority levels
 */
export type TaskPriority = "critical" | "high" | "medium" | "low"

/**
 * Task status
 */
export type TaskStatus = "pending" | "running" | "completed" | "failed" | "skipped" | "blocked"

/**
 * Task definition
 */
export interface Task {
  id: string
  sessionId: string
  subject: string
  description: string
  status: TaskStatus
  priority: TaskPriority
  dependencies: string[]
  dependents: string[]
  createdAt: number
  startedAt?: number
  completedAt?: number
  error?: string
  retryCount: number
  maxRetries: number
  metadata: Record<string, unknown>
  agent?: string // Which agent should handle this task
}

/**
 * Task queue configuration
 */
export interface TaskQueueConfig {
  maxConcurrent: number
  maxRetries: number
  retryDelay: number
}

/**
 * Task queue for managing Autonomous Mode tasks
 *
 * Handles task dependencies, priorities, and execution order.
 * Uses native Rust implementation via NAPI bindings.
 *
 * @see services/zero-core/src/autonomous/queue.rs - Rust implementation
 * @see services/zero-core/src/napi/autonomous.rs - NAPI bindings
 */
export class TaskQueue {
  private native: NativeTaskQueueHandle
  private config: TaskQueueConfig
  private _sessionId: string

  constructor(sessionId: string, config: Partial<TaskQueueConfig> = {}) {
    this._sessionId = sessionId
    this.config = {
      maxConcurrent: 3,
      maxRetries: 2,
      retryDelay: 1000,
      ...config,
    }

    // Create native task queue (fail-fast if unavailable)
    // Note: nativeCreateTaskQueue is validated as non-null at module load
    this.native = nativeCreateTaskQueue!(sessionId, {
      maxConcurrent: this.config.maxConcurrent,
      maxRetries: this.config.maxRetries,
      retryDelayMs: this.config.retryDelay,
    })

    log.debug("TaskQueue created", { sessionId, config: this.config })
  }

  /**
   * Add a task to the queue
   */
  async add(task: Omit<Task, "id" | "sessionId" | "status" | "createdAt" | "retryCount" | "maxRetries" | "dependents">): Promise<string> {
    const id = task.dependencies.length > 0
      ? this.native.addTaskWithDeps(task.subject, task.description, toNativePriority(task.priority), task.dependencies)
      : this.native.addTask(task.subject, task.description, toNativePriority(task.priority))

    log.info("Task added", { id, subject: task.subject, priority: task.priority })

    await Bus.publish(AutonomousEvent.TaskCreated, {
      sessionId: this._sessionId,
      taskId: id,
      subject: task.subject,
      description: task.description,
      priority: getPriorityValue(task.priority),
      dependencies: task.dependencies,
    })

    return id
  }

  /**
   * Get a task by ID
   */
  get(id: string): Task | undefined {
    const task = this.native.getTask(id)
    return task ? fromNativeTask(task) : undefined
  }

  /**
   * Get all tasks
   */
  getAll(): Task[] {
    return this.native.allTasks().map(fromNativeTask)
  }

  /**
   * Get tasks by status
   */
  getByStatus(status: TaskStatus): Task[] {
    return this.getAll().filter((t) => t.status === status)
  }

  /**
   * Get pending tasks that can run (dependencies satisfied)
   */
  getRunnable(): Task[] {
    return this.native.runnableTasks().map(fromNativeTask)
  }

  /**
   * Mark a task as started
   */
  async start(id: string): Promise<boolean> {
    this.native.startTask(id)
    const task = this.native.getTask(id)

    if (task) {
      log.info("Task started", { id, subject: task.subject })
      await Bus.publish(AutonomousEvent.TaskStarted, {
        sessionId: this._sessionId,
        taskId: id,
      })
      return true
    }

    return false
  }

  /**
   * Mark a task as completed
   */
  async complete(id: string, metadata?: Record<string, unknown>): Promise<boolean> {
    const duration = this.native.completeTask(id)
    const task = this.native.getTask(id)

    if (task) {
      log.info("Task completed", { id, subject: task.subject, duration })
      await Bus.publish(AutonomousEvent.TaskCompleted, {
        sessionId: this._sessionId,
        taskId: id,
        success: true,
        duration,
        metadata,
      })
      return true
    }

    return false
  }

  /**
   * Mark a task as failed
   */
  async fail(id: string, error: string, retryable = true): Promise<boolean> {
    const willRetry = this.native.failTask(id, error, retryable)
    const task = this.native.getTask(id)

    if (task) {
      if (willRetry) {
        log.info("Task failed, scheduling retry", {
          id,
          subject: task.subject,
          retryCount: task.retryCount,
          maxRetries: task.maxRetries,
        })
      } else {
        log.warn("Task failed permanently", { id, subject: task.subject, error })
      }

      await Bus.publish(AutonomousEvent.TaskFailed, {
        sessionId: this._sessionId,
        taskId: id,
        error,
        retryable: willRetry,
        retryCount: task.retryCount,
      })
      return true
    }

    return false
  }

  /**
   * Skip a task
   */
  async skip(id: string, reason = "Skipped"): Promise<boolean> {
    this.native.skipTask(id, reason)
    const task = this.native.getTask(id)

    if (task) {
      log.info("Task skipped", { id, subject: task.subject, reason })
      return true
    }

    return false
  }

  /**
   * Block a task (waiting on external input)
   */
  async block(id: string, reason = "Blocked"): Promise<boolean> {
    this.native.blockTask(id, reason)
    const task = this.native.getTask(id)

    if (task) {
      log.info("Task blocked", { id, subject: task.subject, reason })
      return true
    }

    return false
  }

  /**
   * Retry a failed task
   */
  async retry(id: string): Promise<boolean> {
    this.native.retryTask(id)
    const task = this.native.getTask(id)

    if (task) {
      log.info("Task retry requested", { id, subject: task.subject })
      return true
    }

    return false
  }

  /**
   * Get queue statistics
   */
  getStats(): {
    total: number
    pending: number
    running: number
    completed: number
    failed: number
    skipped: number
    blocked: number
  } {
    const s = this.native.stats()
    return {
      total: s.total,
      pending: s.pending,
      running: s.running,
      completed: s.completed,
      failed: s.failed,
      skipped: s.skipped,
      blocked: s.blocked,
    }
  }

  /**
   * Check if all tasks are complete
   */
  isComplete(): boolean {
    return this.native.isComplete()
  }

  /**
   * Check if queue has failed tasks
   */
  hasFailures(): boolean {
    return this.native.hasFailures()
  }

  /**
   * Get failed tasks
   */
  getFailed(): Task[] {
    return this.native.failedTasks().map(fromNativeTask)
  }

  /**
   * Get task chain (dependencies and dependents)
   */
  getChain(id: string): Task[] {
    return this.native.taskChain(id).map(fromNativeTask)
  }

  /**
   * Clear all tasks
   */
  clear(): void {
    this.native.clear()
  }

  /**
   * Get session ID
   */
  get sessionId(): string {
    return this.native.sessionId()
  }

  /**
   * Serialize queue
   */
  serialize(): { tasks: Task[]; config: TaskQueueConfig } {
    return {
      tasks: this.getAll(),
      config: this.config,
    }
  }

  /**
   * Restore queue from serialized data
   */
  static deserialize(data: { tasks: Task[]; config: TaskQueueConfig }, sessionId: string): TaskQueue {
    const queue = new TaskQueue(sessionId, data.config)

    // Re-add tasks from serialized data
    for (const task of data.tasks) {
      if (task.status === "pending" || task.status === "running") {
        // Re-add pending/running tasks
        const deps = task.dependencies
        if (deps.length > 0) {
          queue.native.addTaskWithDeps(task.subject, task.description, toNativePriority(task.priority), deps)
        } else {
          queue.native.addTask(task.subject, task.description, toNativePriority(task.priority))
        }
      }
    }

    return queue
  }
}

// Helper function
function getPriorityValue(priority: TaskPriority): number {
  switch (priority) {
    case "critical":
      return 4
    case "high":
      return 3
    case "medium":
      return 2
    case "low":
      return 1
  }
}

/**
 * Create a task queue
 */
export function createTaskQueue(sessionId: string, config?: Partial<TaskQueueConfig>): TaskQueue {
  return new TaskQueue(sessionId, config)
}
