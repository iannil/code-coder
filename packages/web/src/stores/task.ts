/**
 * Task Store
 * Manages async task state including tasks list, loading states, and SSE subscriptions
 */

import { create } from "zustand"
import { immer } from "zustand/middleware/immer"
import { enableMapSet } from "immer"
import { useShallow } from "zustand/react/shallow"
import { api } from "../lib/api"

// Enable Immer support for Map and Set
enableMapSet()

// ============================================================================
// Type Definitions (mirroring backend types)
// ============================================================================

export type TaskStatus = "pending" | "running" | "awaiting_approval" | "completed" | "failed"

export interface TaskContext {
  userID: string
  platform: string
  chatHistory?: unknown[]
  source: "remote"
}

export interface Task {
  id: string
  sessionID: string
  status: TaskStatus
  agent: string
  prompt: string
  context: TaskContext
  output?: string
  error?: string
  createdAt: string
  updatedAt: string
}

export interface CreateTaskInput {
  agent: string
  prompt: string
  context: TaskContext
  sessionID?: string
  model?: string
}

export interface InteractTaskInput {
  action: "approve" | "reject"
  reason?: string
  reply?: "once" | "always" | "reject"
}

export type TaskEventType = "thought" | "tool_use" | "output" | "confirmation" | "finish" | "progress"

export interface ThoughtEvent {
  type: "thought"
  data: string
}

export interface ToolUseEvent {
  type: "tool_use"
  data: {
    tool: string
    args: unknown
    result?: unknown
  }
}

export interface OutputEvent {
  type: "output"
  data: string
}

export interface ConfirmationEvent {
  type: "confirmation"
  data: {
    requestID: string
    tool: string
    description: string
    args: unknown
    actions: string[]
  }
}

export interface FinishEvent {
  type: "finish"
  data: {
    success: boolean
    output?: string
    error?: string
  }
}

export interface ProgressEvent {
  type: "progress"
  data: {
    stage: string
    message: string
    percentage?: number
  }
}

export type TaskEvent = ThoughtEvent | ToolUseEvent | OutputEvent | ConfirmationEvent | FinishEvent | ProgressEvent

// ============================================================================
// State Interface
// ============================================================================

interface TaskState {
  // Data
  tasks: Map<string, Task>
  taskEvents: Map<string, TaskEvent[]>

  // Loading states
  isLoading: boolean
  isLoaded: boolean
  isCreating: boolean
  isInteracting: Set<string>
  isDeleting: Set<string>

  // SSE connections
  eventSources: Map<string, EventSource>

  // Error state
  error: string | null
}

interface TaskActions {
  // Task management
  loadTasks: () => Promise<void>
  getTask: (taskId: string) => Promise<Task | null>
  createTask: (input: CreateTaskInput) => Promise<Task>
  interactTask: (taskId: string, input: InteractTaskInput) => Promise<Task>
  deleteTask: (taskId: string) => Promise<void>

  // Event subscription
  subscribeToEvents: (taskId: string) => void
  unsubscribeFromEvents: (taskId: string) => void
  clearEvents: (taskId: string) => void

  // State management
  updateTask: (task: Task) => void
  addEvent: (taskId: string, event: TaskEvent) => void
  setError: (error: string | null) => void
  clearError: () => void
  reset: () => void
}

type TaskStore = TaskState & TaskActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<TaskState, "tasks" | "taskEvents" | "isInteracting" | "isDeleting" | "eventSources"> = {
  isLoading: false,
  isLoaded: false,
  isCreating: false,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

const useTaskStoreBase = create<TaskStore>()(
  immer((set, get) => ({
    // Initial state
    tasks: new Map(),
    taskEvents: new Map(),
    isInteracting: new Set(),
    isDeleting: new Set(),
    eventSources: new Map(),
    ...initialState,

    // ======================================================================
    // Task Management
    // ======================================================================

    loadTasks: async () => {
      set((state) => {
        state.isLoading = true
        state.error = null
      })

      try {
        const tasks = await api.listTasks()

        set((state) => {
          state.tasks.clear()
          for (const task of tasks) {
            state.tasks.set(task.id, task)
          }
          state.isLoading = false
          state.isLoaded = true
        })
      } catch (error) {
        set((state) => {
          state.isLoading = false
          state.error = error instanceof Error ? error.message : "Failed to load tasks"
        })
      }
    },

    getTask: async (taskId) => {
      try {
        const task = await api.getTask(taskId)
        set((state) => {
          state.tasks.set(taskId, task)
        })
        return task
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : "Failed to get task"
        })
        return null
      }
    },

    createTask: async (input) => {
      set((state) => {
        state.isCreating = true
        state.error = null
      })

      try {
        const task = await api.createTask(input)

        set((state) => {
          state.tasks.set(task.id, task)
          state.isCreating = false
        })

        return task
      } catch (error) {
        set((state) => {
          state.isCreating = false
          state.error = error instanceof Error ? error.message : "Failed to create task"
        })
        throw error
      }
    },

    interactTask: async (taskId, input) => {
      set((state) => {
        state.isInteracting.add(taskId)
        state.error = null
      })

      try {
        const task = await api.interactTask(taskId, input)

        set((state) => {
          state.tasks.set(taskId, task)
          state.isInteracting.delete(taskId)
        })

        return task
      } catch (error) {
        set((state) => {
          state.isInteracting.delete(taskId)
          state.error = error instanceof Error ? error.message : "Failed to interact with task"
        })
        throw error
      }
    },

    deleteTask: async (taskId) => {
      set((state) => {
        state.isDeleting.add(taskId)
        state.error = null
      })

      try {
        await api.deleteTask(taskId)

        set((state) => {
          state.tasks.delete(taskId)
          state.taskEvents.delete(taskId)
          state.isDeleting.delete(taskId)
        })

        // Clean up SSE connection if exists
        const eventSource = get().eventSources.get(taskId)
        if (eventSource) {
          eventSource.close()
          set((state) => {
            state.eventSources.delete(taskId)
          })
        }
      } catch (error) {
        set((state) => {
          state.isDeleting.delete(taskId)
          state.error = error instanceof Error ? error.message : "Failed to delete task"
        })
        throw error
      }
    },

    // ======================================================================
    // Event Subscription
    // ======================================================================

    subscribeToEvents: (taskId) => {
      const existing = get().eventSources.get(taskId)
      if (existing) return // Already subscribed

      const eventSource = new EventSource(`/api/v1/tasks/${taskId}/events`)

      eventSource.onmessage = (event) => {
        try {
          const taskEvent = JSON.parse(event.data) as TaskEvent
          get().addEvent(taskId, taskEvent)

          // Update task status based on finish event
          if (taskEvent.type === "finish") {
            get().getTask(taskId)
          }
        } catch {
          // Ignore parse errors
        }
      }

      eventSource.onerror = () => {
        // Reconnect logic could be added here
        eventSource.close()
        set((state) => {
          state.eventSources.delete(taskId)
        })
      }

      set((state) => {
        state.eventSources.set(taskId, eventSource)
      })
    },

    unsubscribeFromEvents: (taskId) => {
      const eventSource = get().eventSources.get(taskId)
      if (eventSource) {
        eventSource.close()
        set((state) => {
          state.eventSources.delete(taskId)
        })
      }
    },

    clearEvents: (taskId) => {
      set((state) => {
        state.taskEvents.delete(taskId)
      })
    },

    // ======================================================================
    // State Management
    // ======================================================================

    updateTask: (task) => {
      set((state) => {
        state.tasks.set(task.id, task)
      })
    },

    addEvent: (taskId, event) => {
      set((state) => {
        const events = state.taskEvents.get(taskId) ?? []
        state.taskEvents.set(taskId, [...events, event])
      })
    },

    setError: (error) => {
      set((state) => {
        state.error = error
      })
    },

    clearError: () => {
      set((state) => {
        state.error = null
      })
    },

    reset: () => {
      // Close all event sources
      for (const eventSource of get().eventSources.values()) {
        eventSource.close()
      }

      set((state) => {
        state.tasks.clear()
        state.taskEvents.clear()
        state.isInteracting.clear()
        state.isDeleting.clear()
        state.eventSources.clear()
        Object.assign(state, initialState)
      })
    },
  }))
)

// ============================================================================
// Selector Hooks
// ============================================================================

/**
 * Get all tasks as an array sorted by creation time (newest first)
 */
export const useTasks = () =>
  useTaskStoreBase(
    useShallow((state) =>
      Array.from(state.tasks.values()).sort(
        (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
      )
    )
  )

/**
 * Get a task by ID
 */
export const useTask = (taskId: string) => useTaskStoreBase((state) => state.tasks.get(taskId))

/**
 * Get task events by task ID
 */
export const useTaskEvents = (taskId: string) =>
  useTaskStoreBase(useShallow((state) => state.taskEvents.get(taskId) ?? []))

/**
 * Get loading state
 */
export const useTasksLoading = () =>
  useTaskStoreBase(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isLoaded: state.isLoaded,
      isCreating: state.isCreating,
    }))
  )

/**
 * Get error state
 */
export const useTaskError = () => useTaskStoreBase((state) => state.error)

/**
 * Check if a task is being interacted with
 */
export const useTaskInteracting = (taskId: string) =>
  useTaskStoreBase((state) => state.isInteracting.has(taskId))

/**
 * Check if a task is being deleted
 */
export const useTaskDeleting = (taskId: string) => useTaskStoreBase((state) => state.isDeleting.has(taskId))

/**
 * Get tasks filtered by status
 */
export const useTasksByStatus = (status: TaskStatus) =>
  useTaskStoreBase(
    useShallow((state) =>
      Array.from(state.tasks.values())
        .filter((t) => t.status === status)
        .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    )
  )

/**
 * Get task counts by status
 */
export const useTaskCounts = () =>
  useTaskStoreBase(
    useShallow((state) => {
      const tasks = Array.from(state.tasks.values())
      return {
        total: tasks.length,
        pending: tasks.filter((t) => t.status === "pending").length,
        running: tasks.filter((t) => t.status === "running").length,
        awaitingApproval: tasks.filter((t) => t.status === "awaiting_approval").length,
        completed: tasks.filter((t) => t.status === "completed").length,
        failed: tasks.filter((t) => t.status === "failed").length,
      }
    })
  )

// ============================================================================
// Export Store
// ============================================================================

export { useTaskStoreBase as useTaskStore }
