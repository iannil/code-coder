/**
 * Task Context Registry
 *
 * Maintains the mapping between sessions and their active task IDs.
 * This enables SessionProcessor to emit SSE events to the correct task.
 *
 * Uses Instance.state for lifecycle-bound storage (disposed with project).
 */

import { Instance } from "@/project/instance"

interface ContextRegistryState {
  /** Map of sessionID -> taskID */
  sessions: Map<string, string>
}

const state = Instance.state((): ContextRegistryState => {
  return {
    sessions: new Map(),
  }
})

export const TaskContextRegistry = {
  /**
   * Register a session with its active task ID.
   * Call this when starting task execution via API.
   */
  register(sessionID: string, taskID: string): void {
    const s = state()
    s.sessions.set(sessionID, taskID)
  },

  /**
   * Get the active task ID for a session.
   * Returns undefined if no task is registered.
   */
  getTaskID(sessionID: string): string | undefined {
    const s = state()
    return s.sessions.get(sessionID)
  },

  /**
   * Unregister a session (cleanup after task completes).
   */
  unregister(sessionID: string): void {
    const s = state()
    s.sessions.delete(sessionID)
  },

  /**
   * Check if a session has an active task.
   */
  has(sessionID: string): boolean {
    const s = state()
    return s.sessions.has(sessionID)
  },

  /**
   * Get the number of active registrations.
   */
  get size(): number {
    const s = state()
    return s.sessions.size
  },
}
