/**
 * Task Store
 * In-memory storage for task state with optional persistence
 */

import { Instance } from "@/project/instance"
import { Identifier } from "@/id/id"
import { Log } from "@/util/log"
import type { Task, TaskStatus, TaskContext } from "./types"

export namespace TaskStore {
  const log = Log.create({ service: "task-store" })

  // ============================================================================
  // State Management
  // ============================================================================

  interface PendingConfirmationInfo {
    requestID: string
    permission: string
  }

  interface TaskStoreState {
    tasks: Map<string, Task>
    /** Map of taskID to pending confirmation info */
    pendingConfirmations: Map<string, PendingConfirmationInfo>
  }

  const state = Instance.state((): TaskStoreState => {
    return {
      tasks: new Map(),
      pendingConfirmations: new Map(),
    }
  })

  // ============================================================================
  // CRUD Operations
  // ============================================================================

  export interface CreateTaskInput {
    agent: string
    prompt: string
    context: TaskContext
    sessionID: string
  }

  export function create(input: CreateTaskInput): Task {
    const s = state()
    const now = new Date().toISOString()
    const task: Task = {
      id: Identifier.ascending("task"),
      sessionID: input.sessionID,
      status: "pending",
      agent: input.agent,
      prompt: input.prompt,
      context: input.context,
      createdAt: now,
      updatedAt: now,
    }

    s.tasks.set(task.id, task)
    log.info("task created", { taskID: task.id, agent: input.agent })
    return task
  }

  export function get(taskID: string): Task | undefined {
    return state().tasks.get(taskID)
  }

  export function list(): Task[] {
    return [...state().tasks.values()].sort((a, b) => b.createdAt.localeCompare(a.createdAt))
  }

  export function listBySession(sessionID: string): Task[] {
    return list().filter((task) => task.sessionID === sessionID)
  }

  export function listByStatus(status: TaskStatus): Task[] {
    return list().filter((task) => task.status === status)
  }

  export function listByUser(userID: string): Task[] {
    return list().filter((task) => task.context.userID === userID)
  }

  // ============================================================================
  // State Transitions
  // ============================================================================

  export function updateStatus(taskID: string, status: TaskStatus): Task | undefined {
    const s = state()
    const task = s.tasks.get(taskID)
    if (!task) {
      log.warn("task not found for status update", { taskID })
      return undefined
    }

    const updated: Task = {
      ...task,
      status,
      updatedAt: new Date().toISOString(),
    }
    s.tasks.set(taskID, updated)
    log.info("task status updated", { taskID, from: task.status, to: status })
    return updated
  }

  export function setRunning(taskID: string): Task | undefined {
    return updateStatus(taskID, "running")
  }

  export function setAwaitingApproval(taskID: string, confirmationRequestID: string, permission: string): Task | undefined {
    const s = state()
    s.pendingConfirmations.set(taskID, { requestID: confirmationRequestID, permission })
    return updateStatus(taskID, "awaiting_approval")
  }

  export function complete(taskID: string, output: string): Task | undefined {
    const s = state()
    const task = s.tasks.get(taskID)
    if (!task) {
      log.warn("task not found for completion", { taskID })
      return undefined
    }

    const updated: Task = {
      ...task,
      status: "completed",
      output,
      updatedAt: new Date().toISOString(),
    }
    s.tasks.set(taskID, updated)
    s.pendingConfirmations.delete(taskID)
    log.info("task completed", { taskID })
    return updated
  }

  export function fail(taskID: string, error: string): Task | undefined {
    const s = state()
    const task = s.tasks.get(taskID)
    if (!task) {
      log.warn("task not found for failure", { taskID })
      return undefined
    }

    const updated: Task = {
      ...task,
      status: "failed",
      error,
      updatedAt: new Date().toISOString(),
    }
    s.tasks.set(taskID, updated)
    s.pendingConfirmations.delete(taskID)
    log.info("task failed", { taskID, error })
    return updated
  }

  // ============================================================================
  // Confirmation Handling
  // ============================================================================

  export function getPendingConfirmation(taskID: string): string | undefined {
    return state().pendingConfirmations.get(taskID)?.requestID
  }

  export function getPendingConfirmationInfo(taskID: string): PendingConfirmationInfo | undefined {
    return state().pendingConfirmations.get(taskID)
  }

  export function clearPendingConfirmation(taskID: string): void {
    const s = state()
    s.pendingConfirmations.delete(taskID)
    const task = s.tasks.get(taskID)
    if (task && task.status === "awaiting_approval") {
      updateStatus(taskID, "running")
    }
  }

  // ============================================================================
  // Cleanup
  // ============================================================================

  export function remove(taskID: string): boolean {
    const s = state()
    const existed = s.tasks.has(taskID)
    s.tasks.delete(taskID)
    s.pendingConfirmations.delete(taskID)
    if (existed) {
      log.info("task removed", { taskID })
    }
    return existed
  }

  export function cleanup(maxAgeMs: number = 24 * 60 * 60 * 1000): number {
    const s = state()
    const cutoff = Date.now() - maxAgeMs
    let removed = 0

    for (const [taskID, task] of s.tasks) {
      const taskTime = new Date(task.updatedAt).getTime()
      const isTerminal = task.status === "completed" || task.status === "failed"
      if (isTerminal && taskTime < cutoff) {
        s.tasks.delete(taskID)
        s.pendingConfirmations.delete(taskID)
        removed++
      }
    }

    if (removed > 0) {
      log.info("cleaned up old tasks", { removed })
    }
    return removed
  }

  // ============================================================================
  // Stats
  // ============================================================================

  export function stats(): {
    total: number
    pending: number
    running: number
    awaitingApproval: number
    completed: number
    failed: number
  } {
    const tasks = list()
    return {
      total: tasks.length,
      pending: tasks.filter((t) => t.status === "pending").length,
      running: tasks.filter((t) => t.status === "running").length,
      awaitingApproval: tasks.filter((t) => t.status === "awaiting_approval").length,
      completed: tasks.filter((t) => t.status === "completed").length,
      failed: tasks.filter((t) => t.status === "failed").length,
    }
  }
}
