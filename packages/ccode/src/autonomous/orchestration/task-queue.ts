import { Log } from "@/util/log"
import { AutonomousEvent } from "../events"
import { Bus } from "@/bus"

const log = Log.create({ service: "autonomous.task-queue" })

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
 * Handles task dependencies, priorities, and execution order
 */
export class TaskQueue {
  private tasks: Map<string, Task> = new Map()
  private config: TaskQueueConfig
  private running: Set<string> = new Set()
  private sessionId: string

  constructor(sessionId: string, config: Partial<TaskQueueConfig> = {}) {
    this.sessionId = sessionId
    this.config = {
      maxConcurrent: 3,
      maxRetries: 2,
      retryDelay: 1000,
      ...config,
    }
  }

  /**
   * Add a task to the queue
   */
  async add(task: Omit<Task, "id" | "sessionId" | "status" | "createdAt" | "retryCount" | "maxRetries" | "dependents">): Promise<string> {
    const id = this.generateTaskId()

    const newTask: Task = {
      id,
      sessionId: this.sessionId,
      status: "pending",
      createdAt: Date.now(),
      retryCount: 0,
      maxRetries: this.config.maxRetries,
      dependents: [],
      ...task,
    }

    this.tasks.set(id, newTask)

    // Register dependencies
    for (const depId of task.dependencies) {
      const dep = this.tasks.get(depId)
      if (dep) {
        dep.dependents.push(id)
      }
    }

    log.info("Task added", { id, subject: task.subject, priority: task.priority })

    await Bus.publish(AutonomousEvent.TaskCreated, {
      sessionId: this.sessionId,
      taskId: id,
      subject: task.subject,
      description: task.description,
      priority: this.getPriorityValue(task.priority),
      dependencies: task.dependencies,
    })

    return id
  }

  /**
   * Get a task by ID
   */
  get(id: string): Task | undefined {
    return this.tasks.get(id)
  }

  /**
   * Get all tasks
   */
  getAll(): Task[] {
    return Array.from(this.tasks.values())
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
    const pending = this.getByStatus("pending")
    const runningCount = this.running.size

    // Check concurrency limit
    if (runningCount >= this.config.maxConcurrent) {
      return []
    }

    // Filter tasks whose dependencies are all completed
    return pending
      .filter((task) => {
        if (task.dependencies.length === 0) return true
        return task.dependencies.every((depId) => {
          const dep = this.tasks.get(depId)
          return dep?.status === "completed"
        })
      })
      .sort((a, b) => {
        // Sort by priority first
        const aPriority = this.getPriorityValue(a.priority)
        const bPriority = this.getPriorityValue(b.priority)
        if (aPriority !== bPriority) {
          return bPriority - aPriority
        }
        // Then by creation time
        return a.createdAt - b.createdAt
      })
      .slice(0, this.config.maxConcurrent - runningCount)
  }

  /**
   * Mark a task as started
   */
  async start(id: string): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task || task.status !== "pending") {
      return false
    }

    task.status = "running"
    task.startedAt = Date.now()
    this.running.add(id)

    log.info("Task started", { id, subject: task.subject })

    await Bus.publish(AutonomousEvent.TaskStarted, {
      sessionId: this.sessionId,
      taskId: id,
    })

    return true
  }

  /**
   * Mark a task as completed
   */
  async complete(id: string, metadata?: Record<string, unknown>): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task || task.status !== "running") {
      return false
    }

    task.status = "completed"
    task.completedAt = Date.now()
    this.running.delete(id)

    if (metadata) {
      task.metadata = { ...task.metadata, ...metadata }
    }

    const duration = task.completedAt - (task.startedAt ?? task.completedAt)

    log.info("Task completed", { id, subject: task.subject, duration })

    await Bus.publish(AutonomousEvent.TaskCompleted, {
      sessionId: this.sessionId,
      taskId: id,
      success: true,
      duration,
      metadata,
    })

    return true
  }

  /**
   * Mark a task as failed
   */
  async fail(id: string, error: string, retryable = true): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task) {
      return false
    }

    task.error = error
    task.retryCount++

    // Check if we should retry
    if (retryable && task.retryCount < task.maxRetries) {
      task.status = "pending"
      this.running.delete(id)

      log.info("Task failed, scheduling retry", {
        id,
        subject: task.subject,
        retryCount: task.retryCount,
        maxRetries: task.maxRetries,
      })

      await Bus.publish(AutonomousEvent.TaskFailed, {
        sessionId: this.sessionId,
        taskId: id,
        error,
        retryable: true,
        retryCount: task.retryCount,
      })

      return true
    }

    // Final failure
    task.status = "failed"
    task.completedAt = Date.now()
    this.running.delete(id)

    log.warn("Task failed permanently", { id, subject: task.subject, error })

    await Bus.publish(AutonomousEvent.TaskFailed, {
      sessionId: this.sessionId,
      taskId: id,
      error,
      retryable: false,
      retryCount: task.retryCount,
    })

    return true
  }

  /**
   * Skip a task
   */
  async skip(id: string, reason = "Skipped"): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task || (task.status !== "pending" && task.status !== "running")) {
      return false
    }

    task.status = "skipped"
    task.completedAt = Date.now()
    task.error = reason
    this.running.delete(id)

    log.info("Task skipped", { id, subject: task.subject, reason })

    return true
  }

  /**
   * Block a task (waiting on external input)
   */
  async block(id: string, reason = "Blocked"): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task || (task.status !== "pending" && task.status !== "running")) {
      return false
    }

    task.status = "blocked"
    task.error = reason
    this.running.delete(id)

    log.info("Task blocked", { id, subject: task.subject, reason })

    return true
  }

  /**
   * Retry a failed task
   */
  async retry(id: string): Promise<boolean> {
    const task = this.tasks.get(id)
    if (!task || task.status !== "failed") {
      return false
    }

    task.status = "pending"
    task.error = undefined

    log.info("Task retry requested", { id, subject: task.subject })

    return true
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
    const tasks = this.getAll()

    return {
      total: tasks.length,
      pending: tasks.filter((t) => t.status === "pending").length,
      running: tasks.filter((t) => t.status === "running").length,
      completed: tasks.filter((t) => t.status === "completed").length,
      failed: tasks.filter((t) => t.status === "failed").length,
      skipped: tasks.filter((t) => t.status === "skipped").length,
      blocked: tasks.filter((t) => t.status === "blocked").length,
    }
  }

  /**
   * Check if all tasks are complete
   */
  isComplete(): boolean {
    const stats = this.getStats()
    return stats.total > 0 && stats.pending === 0 && stats.running === 0 && stats.blocked === 0
  }

  /**
   * Check if queue has failed tasks
   */
  hasFailures(): boolean {
    return this.getStats().failed > 0
  }

  /**
   * Get failed tasks
   */
  getFailed(): Task[] {
    return this.getByStatus("failed")
  }

  /**
   * Get task chain (dependencies and dependents)
   */
  getChain(id: string): Task[] {
    const task = this.tasks.get(id)
    if (!task) return []

    const chain: Set<string> = new Set()

    // Add dependencies
    const addDeps = (taskId: string) => {
      chain.add(taskId)
      const t = this.tasks.get(taskId)
      if (t) {
        for (const depId of t.dependencies) {
          addDeps(depId)
        }
      }
    }

    // Add dependents
    const addDependents = (taskId: string) => {
      chain.add(taskId)
      const t = this.tasks.get(taskId)
      if (t) {
        for (const depId of t.dependents) {
          addDependents(depId)
        }
      }
    }

    addDeps(id)
    addDependents(id)

    return Array.from(chain)
      .map((tid) => this.tasks.get(tid))
      .filter((t): t is Task => t !== undefined)
  }

  /**
   * Clear all tasks
   */
  clear(): void {
    this.tasks.clear()
    this.running.clear()
  }

  /**
   * Get priority as number
   */
  private getPriorityValue(priority: TaskPriority): number {
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
   * Generate task ID
   */
  private generateTaskId(): string {
    return `task_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
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

    for (const task of data.tasks) {
      queue.tasks.set(task.id, task)
      if (task.status === "running") {
        queue.running.add(task.id)
        task.status = "pending" // Reset running tasks to pending on restore
      }
    }

    return queue
  }
}

/**
 * Create a task queue
 */
export function createTaskQueue(sessionId: string, config?: Partial<TaskQueueConfig>): TaskQueue {
  return new TaskQueue(sessionId, config)
}
