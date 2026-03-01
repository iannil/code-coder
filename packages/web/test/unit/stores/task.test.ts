import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useTaskStore,
  useTasks,
  useTask,
  useTaskEvents,
  useTasksLoading,
  useTaskError,
  useTaskInteracting,
  useTaskDeleting,
  useTaskCounts,
  type Task,
  type TaskEvent,
} from "@/stores/task"
import { api } from "@/lib/api"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listTasks: vi.fn(),
    getTask: vi.fn(),
    createTask: vi.fn(),
    interactTask: vi.fn(),
    deleteTask: vi.fn(),
  },
}))

// Mock EventSource
class MockEventSource {
  url: string
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  close = vi.fn()

  constructor(url: string) {
    this.url = url
  }
}

const originalEventSource = globalThis.EventSource

const mockTask1: Task = {
  id: "task-1",
  sessionID: "session-1",
  status: "running",
  agent: "test-agent",
  prompt: "Test prompt 1",
  context: {
    userID: "user-1",
    platform: "web",
    source: "remote",
  },
  createdAt: new Date(Date.now() - 3600000).toISOString(),
  updatedAt: new Date().toISOString(),
}

const mockTask2: Task = {
  id: "task-2",
  sessionID: "session-2",
  status: "completed",
  agent: "test-agent",
  prompt: "Test prompt 2",
  context: {
    userID: "user-1",
    platform: "web",
    source: "remote",
  },
  output: "Task completed successfully",
  createdAt: new Date(Date.now() - 7200000).toISOString(),
  updatedAt: new Date(Date.now() - 3600000).toISOString(),
}

const mockTask3: Task = {
  id: "task-3",
  sessionID: "session-3",
  status: "failed",
  agent: "test-agent",
  prompt: "Test prompt 3",
  context: {
    userID: "user-1",
    platform: "web",
    source: "remote",
  },
  error: "Task failed with error",
  createdAt: new Date(Date.now() - 10800000).toISOString(),
  updatedAt: new Date(Date.now() - 7200000).toISOString(),
}

describe("Task Store", () => {
  beforeEach(() => {
    useTaskStore.getState().reset()
    vi.clearAllMocks()
    // @ts-expect-error - mock EventSource
    globalThis.EventSource = MockEventSource
  })

  afterEach(() => {
    vi.restoreAllMocks()
    globalThis.EventSource = originalEventSource
  })

  describe("initial state", () => {
    it("should have empty tasks", () => {
      const state = useTaskStore.getState()
      expect(state.tasks.size).toBe(0)
    })

    it("should have empty taskEvents", () => {
      const state = useTaskStore.getState()
      expect(state.taskEvents.size).toBe(0)
    })

    it("should have isLoading false", () => {
      const state = useTaskStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useTaskStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have isCreating false", () => {
      const state = useTaskStore.getState()
      expect(state.isCreating).toBe(false)
    })

    it("should have null error", () => {
      const state = useTaskStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("loadTasks", () => {
    it("should load tasks from API", async () => {
      vi.mocked(api.listTasks).mockResolvedValueOnce([mockTask1, mockTask2])

      await useTaskStore.getState().loadTasks()

      const state = useTaskStore.getState()
      expect(state.tasks.size).toBe(2)
      expect(state.tasks.get("task-1")).toEqual(mockTask1)
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should set isLoading during fetch", async () => {
      vi.mocked(api.listTasks).mockImplementationOnce(async () => {
        expect(useTaskStore.getState().isLoading).toBe(true)
        return [mockTask1]
      })

      await useTaskStore.getState().loadTasks()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listTasks).mockRejectedValueOnce(new Error("Network error"))

      await useTaskStore.getState().loadTasks()

      const state = useTaskStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("getTask", () => {
    it("should get task from API and update store", async () => {
      vi.mocked(api.getTask).mockResolvedValueOnce(mockTask1)

      const result = await useTaskStore.getState().getTask("task-1")

      expect(result).toEqual(mockTask1)
      expect(useTaskStore.getState().tasks.get("task-1")).toEqual(mockTask1)
    })

    it("should handle errors and return null", async () => {
      vi.mocked(api.getTask).mockRejectedValueOnce(new Error("Not found"))

      const result = await useTaskStore.getState().getTask("task-1")

      expect(result).toBeNull()
      expect(useTaskStore.getState().error).toBe("Not found")
    })
  })

  describe("createTask", () => {
    it("should create a task", async () => {
      vi.mocked(api.createTask).mockResolvedValueOnce(mockTask1)

      const input = {
        agent: "test-agent",
        prompt: "Test prompt",
        context: {
          userID: "user-1",
          platform: "web",
          source: "remote" as const,
        },
      }
      const result = await useTaskStore.getState().createTask(input)

      expect(result).toEqual(mockTask1)
      expect(useTaskStore.getState().tasks.get("task-1")).toEqual(mockTask1)
      expect(useTaskStore.getState().isCreating).toBe(false)
    })

    it("should set isCreating during creation", async () => {
      vi.mocked(api.createTask).mockImplementationOnce(async () => {
        expect(useTaskStore.getState().isCreating).toBe(true)
        return mockTask1
      })

      await useTaskStore.getState().createTask({
        agent: "test-agent",
        prompt: "Test",
        context: { userID: "user-1", platform: "web", source: "remote" },
      })
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.createTask).mockRejectedValueOnce(new Error("Create failed"))

      await expect(
        useTaskStore.getState().createTask({
          agent: "test-agent",
          prompt: "Test",
          context: { userID: "user-1", platform: "web", source: "remote" },
        })
      ).rejects.toThrow("Create failed")

      expect(useTaskStore.getState().error).toBe("Create failed")
      expect(useTaskStore.getState().isCreating).toBe(false)
    })
  })

  describe("interactTask", () => {
    it("should interact with task", async () => {
      const updatedTask = { ...mockTask1, status: "completed" as const }
      vi.mocked(api.interactTask).mockResolvedValueOnce(updatedTask)

      const result = await useTaskStore.getState().interactTask("task-1", { action: "approve" })

      expect(result).toEqual(updatedTask)
      expect(useTaskStore.getState().tasks.get("task-1")).toEqual(updatedTask)
      expect(useTaskStore.getState().isInteracting.has("task-1")).toBe(false)
    })

    it("should set isInteracting during interaction", async () => {
      vi.mocked(api.interactTask).mockImplementationOnce(async () => {
        expect(useTaskStore.getState().isInteracting.has("task-1")).toBe(true)
        return mockTask1
      })

      await useTaskStore.getState().interactTask("task-1", { action: "approve" })
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.interactTask).mockRejectedValueOnce(new Error("Interact failed"))

      await expect(
        useTaskStore.getState().interactTask("task-1", { action: "approve" })
      ).rejects.toThrow("Interact failed")

      expect(useTaskStore.getState().error).toBe("Interact failed")
      expect(useTaskStore.getState().isInteracting.has("task-1")).toBe(false)
    })
  })

  describe("deleteTask", () => {
    beforeEach(async () => {
      vi.mocked(api.listTasks).mockResolvedValueOnce([mockTask1, mockTask2])
      await useTaskStore.getState().loadTasks()
    })

    it("should delete a task", async () => {
      vi.mocked(api.deleteTask).mockResolvedValueOnce(undefined)

      await useTaskStore.getState().deleteTask("task-1")

      expect(useTaskStore.getState().tasks.has("task-1")).toBe(false)
      expect(useTaskStore.getState().tasks.size).toBe(1)
    })

    it("should set isDeleting during deletion", async () => {
      vi.mocked(api.deleteTask).mockImplementationOnce(async () => {
        expect(useTaskStore.getState().isDeleting.has("task-1")).toBe(true)
      })

      await useTaskStore.getState().deleteTask("task-1")
    })

    it("should handle errors and throw", async () => {
      vi.mocked(api.deleteTask).mockRejectedValueOnce(new Error("Delete failed"))

      await expect(useTaskStore.getState().deleteTask("task-1")).rejects.toThrow("Delete failed")

      expect(useTaskStore.getState().error).toBe("Delete failed")
      expect(useTaskStore.getState().isDeleting.has("task-1")).toBe(false)
    })
  })

  describe("event subscription", () => {
    it("subscribeToEvents should create EventSource", () => {
      useTaskStore.getState().subscribeToEvents("task-1")

      expect(useTaskStore.getState().eventSources.has("task-1")).toBe(true)
    })

    it("subscribeToEvents should not create duplicate subscriptions", () => {
      useTaskStore.getState().subscribeToEvents("task-1")
      useTaskStore.getState().subscribeToEvents("task-1")

      expect(useTaskStore.getState().eventSources.size).toBe(1)
    })

    it("unsubscribeFromEvents should close EventSource", () => {
      useTaskStore.getState().subscribeToEvents("task-1")
      const eventSource = useTaskStore.getState().eventSources.get("task-1") as MockEventSource

      useTaskStore.getState().unsubscribeFromEvents("task-1")

      expect(eventSource.close).toHaveBeenCalled()
      expect(useTaskStore.getState().eventSources.has("task-1")).toBe(false)
    })

    it("clearEvents should remove events for a task", () => {
      const event: TaskEvent = { type: "thought", data: "Test thought" }
      useTaskStore.getState().addEvent("task-1", event)

      useTaskStore.getState().clearEvents("task-1")

      expect(useTaskStore.getState().taskEvents.has("task-1")).toBe(false)
    })
  })

  describe("state management", () => {
    it("updateTask should update task in store", () => {
      const updatedTask = { ...mockTask1, status: "completed" as const }
      useTaskStore.getState().updateTask(updatedTask)

      expect(useTaskStore.getState().tasks.get("task-1")).toEqual(updatedTask)
    })

    it("addEvent should add event to taskEvents", () => {
      const event: TaskEvent = { type: "thought", data: "Test thought" }
      useTaskStore.getState().addEvent("task-1", event)

      const events = useTaskStore.getState().taskEvents.get("task-1")
      expect(events).toHaveLength(1)
      expect(events?.[0]).toEqual(event)
    })

    it("addEvent should append to existing events", () => {
      const event1: TaskEvent = { type: "thought", data: "Thought 1" }
      const event2: TaskEvent = { type: "output", data: "Output 1" }

      useTaskStore.getState().addEvent("task-1", event1)
      useTaskStore.getState().addEvent("task-1", event2)

      const events = useTaskStore.getState().taskEvents.get("task-1")
      expect(events).toHaveLength(2)
    })

    it("setError should set error", () => {
      useTaskStore.getState().setError("Test error")

      expect(useTaskStore.getState().error).toBe("Test error")
    })

    it("clearError should clear error", () => {
      useTaskStore.getState().setError("Test error")
      useTaskStore.getState().clearError()

      expect(useTaskStore.getState().error).toBeNull()
    })

    it("reset should clear all state", async () => {
      vi.mocked(api.listTasks).mockResolvedValueOnce([mockTask1])
      await useTaskStore.getState().loadTasks()
      useTaskStore.getState().addEvent("task-1", { type: "thought", data: "Test" })
      useTaskStore.getState().subscribeToEvents("task-1")

      useTaskStore.getState().reset()

      const state = useTaskStore.getState()
      expect(state.tasks.size).toBe(0)
      expect(state.taskEvents.size).toBe(0)
      expect(state.eventSources.size).toBe(0)
      expect(state.isLoaded).toBe(false)
    })
  })

  describe("hooks", () => {
    beforeEach(async () => {
      vi.mocked(api.listTasks).mockResolvedValueOnce([mockTask1, mockTask2, mockTask3])
      await useTaskStore.getState().loadTasks()
    })

    it("useTasks should return tasks sorted by creation time", () => {
      const { result } = renderHook(() => useTasks())
      expect(result.current).toHaveLength(3)
      // Newest first
      expect(result.current[0].id).toBe("task-1")
    })

    it("useTask should return specific task", () => {
      const { result } = renderHook(() => useTask("task-1"))
      expect(result.current).toEqual(mockTask1)
    })

    it("useTaskEvents should return events for a task", () => {
      useTaskStore.getState().addEvent("task-1", { type: "thought", data: "Test" })

      const { result } = renderHook(() => useTaskEvents("task-1"))
      expect(result.current).toHaveLength(1)
    })

    it("useTaskEvents should return empty array for no events", () => {
      const { result } = renderHook(() => useTaskEvents("task-999"))
      expect(result.current).toEqual([])
    })

    it("useTasksLoading should return loading states", () => {
      const { result } = renderHook(() => useTasksLoading())
      expect(result.current.isLoading).toBe(false)
      expect(result.current.isLoaded).toBe(true)
      expect(result.current.isCreating).toBe(false)
    })

    it("useTaskError should return error", () => {
      useTaskStore.getState().setError("Test error")

      const { result } = renderHook(() => useTaskError())
      expect(result.current).toBe("Test error")
    })

    it("useTaskInteracting should check if task is being interacted with", async () => {
      vi.mocked(api.interactTask).mockImplementationOnce(async () => {
        const { result } = renderHook(() => useTaskInteracting("task-1"))
        expect(result.current).toBe(true)
        return mockTask1
      })

      await useTaskStore.getState().interactTask("task-1", { action: "approve" })
    })

    it("useTaskDeleting should check if task is being deleted", async () => {
      vi.mocked(api.deleteTask).mockImplementationOnce(async () => {
        const { result } = renderHook(() => useTaskDeleting("task-1"))
        expect(result.current).toBe(true)
      })

      await useTaskStore.getState().deleteTask("task-1")
    })

    it("useTaskCounts should return correct counts", () => {
      const { result } = renderHook(() => useTaskCounts())
      expect(result.current.total).toBe(3)
      expect(result.current.running).toBe(1)
      expect(result.current.completed).toBe(1)
      expect(result.current.failed).toBe(1)
    })
  })
})
