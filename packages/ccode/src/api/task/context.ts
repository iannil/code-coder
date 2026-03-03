/**
 * Task Context Registry
 *
 * Maintains the mapping between sessions and their active task IDs,
 * as well as the full TaskContext for channel information retrieval.
 *
 * Uses Instance.state for lifecycle-bound storage (disposed with project).
 */

import { Instance } from "@/project/instance"
import type { TaskContext } from "./types"

interface ContextRegistryState {
  /** Map of sessionID -> taskID */
  sessions: Map<string, string>
  /** Map of sessionID -> TaskContext (for channel info) */
  contexts: Map<string, TaskContext>
}

const state = Instance.state((): ContextRegistryState => {
  return {
    sessions: new Map(),
    contexts: new Map(),
  }
})

export const TaskContextRegistry = {
  /**
   * Register a session with its active task ID and context.
   * Call this when starting task execution via API.
   */
  register(sessionID: string, taskID: string, context?: TaskContext): void {
    const s = state()
    s.sessions.set(sessionID, taskID)
    if (context) {
      s.contexts.set(sessionID, context)
    }
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
   * Get the TaskContext for a session.
   * Returns undefined if no context is registered.
   */
  getContext(sessionID: string): TaskContext | undefined {
    const s = state()
    return s.contexts.get(sessionID)
  },

  /**
   * Get channel info (type and ID) from the session's TaskContext.
   * Returns undefined if no context is registered or conversationId is not set.
   *
   * ConversationId format is typically "platform:channelId" (e.g., "telegram:765318302")
   */
  getChannelInfo(sessionID: string): { channelType: string; channelId: string } | undefined {
    const context = this.getContext(sessionID)
    if (!context) return undefined

    // Try to extract from conversationId (format: "platform:channelId")
    if (context.conversationId) {
      const parts = context.conversationId.split(":")
      if (parts.length >= 2) {
        return {
          channelType: parts[0],
          channelId: parts.slice(1).join(":"), // Handle channel IDs that contain ":"
        }
      }
    }

    // Fallback: use platform as channelType (channelId not available)
    if (context.platform) {
      return {
        channelType: context.platform,
        channelId: context.userID, // Use userID as fallback for direct messages
      }
    }

    return undefined
  },

  /**
   * Unregister a session (cleanup after task completes).
   */
  unregister(sessionID: string): void {
    const s = state()
    s.sessions.delete(sessionID)
    s.contexts.delete(sessionID)
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
