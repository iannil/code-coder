import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { useSessionStore, useSessions, useSession, useActiveSession, useActiveSessionId, useSessionsLoading, useSessionError, useSessionDeleting } from "@/stores/session"
import { api } from "@/lib/api"
import type { SessionInfo } from "@/lib/types"
import { renderHook, act } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listSessions: vi.fn(),
    createSession: vi.fn(),
    deleteSession: vi.fn(),
    getSession: vi.fn(),
  },
}))

const mockSession: SessionInfo = {
  id: "test-session-1",
  title: "Test Session",
  time: {
    created: Date.now() - 3600000,
    updated: Date.now(),
  },
}

const mockSession2: SessionInfo = {
  id: "test-session-2",
  title: "Test Session 2",
  time: {
    created: Date.now() - 7200000,
    updated: Date.now() - 3600000,
  },
}

describe("Session Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useSessionStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty sessions map", () => {
      const state = useSessionStore.getState()
      expect(state.sessions.size).toBe(0)
    })

    it("should have null activeSessionId", () => {
      const state = useSessionStore.getState()
      expect(state.activeSessionId).toBeNull()
    })

    it("should have isLoading false", () => {
      const state = useSessionStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useSessionStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have isCreating false", () => {
      const state = useSessionStore.getState()
      expect(state.isCreating).toBe(false)
    })

    it("should have null error", () => {
      const state = useSessionStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("loadSessions", () => {
    it("should load sessions from API", async () => {
      vi.mocked(api.listSessions).mockResolvedValueOnce([mockSession, mockSession2])

      await useSessionStore.getState().loadSessions()

      const state = useSessionStore.getState()
      expect(state.sessions.size).toBe(2)
      expect(state.sessions.get(mockSession.id)).toEqual(mockSession)
      expect(state.sessions.get(mockSession2.id)).toEqual(mockSession2)
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should set isLoading during load", async () => {
      vi.mocked(api.listSessions).mockImplementationOnce(async () => {
        // Check state while loading
        expect(useSessionStore.getState().isLoading).toBe(true)
        return [mockSession]
      })

      await useSessionStore.getState().loadSessions()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listSessions).mockRejectedValueOnce(new Error("Network error"))

      await useSessionStore.getState().loadSessions()

      const state = useSessionStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("createSession", () => {
    it("should create a session", async () => {
      const newSession: SessionInfo = { ...mockSession, id: "new-session" }
      vi.mocked(api.createSession).mockResolvedValueOnce(newSession)

      const result = await useSessionStore.getState().createSession({ title: "New Session" })

      expect(result).toEqual(newSession)
      const state = useSessionStore.getState()
      expect(state.sessions.get(newSession.id)).toEqual(newSession)
      expect(state.activeSessionId).toBe(newSession.id)
      expect(state.isCreating).toBe(false)
    })

    it("should set isCreating during creation", async () => {
      vi.mocked(api.createSession).mockImplementationOnce(async () => {
        expect(useSessionStore.getState().isCreating).toBe(true)
        return mockSession
      })

      await useSessionStore.getState().createSession()
    })

    it("should handle creation errors", async () => {
      vi.mocked(api.createSession).mockRejectedValueOnce(new Error("Failed to create"))

      await expect(useSessionStore.getState().createSession()).rejects.toThrow("Failed to create")

      const state = useSessionStore.getState()
      expect(state.error).toBe("Failed to create")
      expect(state.isCreating).toBe(false)
    })
  })

  describe("deleteSession", () => {
    beforeEach(async () => {
      // Load a session first
      vi.mocked(api.listSessions).mockResolvedValueOnce([mockSession])
      await useSessionStore.getState().loadSessions()
    })

    it("should delete a session", async () => {
      vi.mocked(api.deleteSession).mockResolvedValueOnce(undefined)

      await useSessionStore.getState().deleteSession(mockSession.id)

      const state = useSessionStore.getState()
      expect(state.sessions.has(mockSession.id)).toBe(false)
    })

    it("should clear activeSessionId if deleted session was active", async () => {
      useSessionStore.getState().setActiveSession(mockSession.id)
      vi.mocked(api.deleteSession).mockResolvedValueOnce(undefined)

      await useSessionStore.getState().deleteSession(mockSession.id)

      const state = useSessionStore.getState()
      expect(state.activeSessionId).toBeNull()
    })

    it("should track deleting state", async () => {
      vi.mocked(api.deleteSession).mockImplementationOnce(async () => {
        expect(useSessionStore.getState().isDeleting.has(mockSession.id)).toBe(true)
        return undefined
      })

      await useSessionStore.getState().deleteSession(mockSession.id)

      expect(useSessionStore.getState().isDeleting.has(mockSession.id)).toBe(false)
    })

    it("should handle deletion errors", async () => {
      vi.mocked(api.deleteSession).mockRejectedValueOnce(new Error("Failed to delete"))

      await expect(useSessionStore.getState().deleteSession(mockSession.id)).rejects.toThrow("Failed to delete")

      const state = useSessionStore.getState()
      expect(state.error).toBe("Failed to delete")
      expect(state.sessions.has(mockSession.id)).toBe(true) // Session should still exist
    })
  })

  describe("refreshSession", () => {
    it("should refresh a single session", async () => {
      const updatedSession = { ...mockSession, title: "Updated Title" }
      vi.mocked(api.getSession).mockResolvedValueOnce(updatedSession)

      await useSessionStore.getState().refreshSession(mockSession.id)

      const state = useSessionStore.getState()
      expect(state.sessions.get(mockSession.id)).toEqual(updatedSession)
    })

    it("should handle refresh errors", async () => {
      vi.mocked(api.getSession).mockRejectedValueOnce(new Error("Not found"))

      await useSessionStore.getState().refreshSession("non-existent")

      const state = useSessionStore.getState()
      expect(state.error).toBe("Not found")
    })
  })

  describe("setActiveSession", () => {
    it("should set active session ID", () => {
      useSessionStore.getState().setActiveSession("test-id")

      expect(useSessionStore.getState().activeSessionId).toBe("test-id")
    })

    it("should clear active session ID", () => {
      useSessionStore.getState().setActiveSession("test-id")
      useSessionStore.getState().setActiveSession(null)

      expect(useSessionStore.getState().activeSessionId).toBeNull()
    })
  })

  describe("error management", () => {
    it("should set error", () => {
      useSessionStore.getState().setError("Test error")

      expect(useSessionStore.getState().error).toBe("Test error")
    })

    it("should clear error", () => {
      useSessionStore.getState().setError("Test error")
      useSessionStore.getState().clearError()

      expect(useSessionStore.getState().error).toBeNull()
    })
  })

  describe("reset", () => {
    it("should reset to initial state", async () => {
      // Modify state
      vi.mocked(api.listSessions).mockResolvedValueOnce([mockSession])
      await useSessionStore.getState().loadSessions()
      useSessionStore.getState().setActiveSession(mockSession.id)
      useSessionStore.getState().setError("Some error")

      // Reset
      useSessionStore.getState().reset()

      const state = useSessionStore.getState()
      expect(state.sessions.size).toBe(0)
      expect(state.activeSessionId).toBeNull()
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(false)
      expect(state.error).toBeNull()
    })
  })
})

describe("Session Selector Hooks", () => {
  beforeEach(async () => {
    useSessionStore.getState().reset()
    vi.mocked(api.listSessions).mockResolvedValueOnce([mockSession, mockSession2])
    await useSessionStore.getState().loadSessions()
  })

  describe("useSessions", () => {
    it("should return all sessions as array", () => {
      const { result } = renderHook(() => useSessions())
      expect(result.current).toHaveLength(2)
      expect(result.current).toContainEqual(mockSession)
      expect(result.current).toContainEqual(mockSession2)
    })
  })

  describe("useSession", () => {
    it("should return a specific session", () => {
      const { result } = renderHook(() => useSession(mockSession.id))
      expect(result.current).toEqual(mockSession)
    })

    it("should return undefined for non-existent session", () => {
      const { result } = renderHook(() => useSession("non-existent"))
      expect(result.current).toBeUndefined()
    })
  })

  describe("useActiveSession", () => {
    it("should return null when no active session", () => {
      const { result } = renderHook(() => useActiveSession())
      expect(result.current).toBeNull()
    })

    it("should return active session when set", () => {
      useSessionStore.getState().setActiveSession(mockSession.id)
      const { result } = renderHook(() => useActiveSession())
      expect(result.current).toEqual(mockSession)
    })
  })

  describe("useActiveSessionId", () => {
    it("should return active session ID", () => {
      useSessionStore.getState().setActiveSession(mockSession.id)
      const { result } = renderHook(() => useActiveSessionId())
      expect(result.current).toBe(mockSession.id)
    })
  })

  describe("useSessionsLoading", () => {
    it("should return loading states", () => {
      const { result } = renderHook(() => useSessionsLoading())
      expect(result.current).toEqual({
        isLoading: false,
        isLoaded: true,
        isCreating: false,
      })
    })
  })

  describe("useSessionError", () => {
    it("should return error state", () => {
      useSessionStore.getState().setError("Test error")
      const { result } = renderHook(() => useSessionError())
      expect(result.current).toBe("Test error")
    })
  })

  describe("useSessionDeleting", () => {
    it("should return false when session is not being deleted", () => {
      const { result } = renderHook(() => useSessionDeleting(mockSession.id))
      expect(result.current).toBe(false)
    })
  })
})
