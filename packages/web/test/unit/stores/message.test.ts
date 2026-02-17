import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { useMessageStore, useMessages, useMessage, useMessagesLoading, useMessagesLoaded, useMessagesError } from "@/stores/message"
import { api } from "@/lib/api"
import type { MessageWithParts, MessagePart } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    getSessionMessages: vi.fn(),
  },
}))

const mockMessage: MessageWithParts = {
  info: {
    id: "msg-1",
    sessionID: "session-1",
    role: "user",
    time: {
      created: Date.now() - 60000,
      updated: Date.now(),
    },
    parentID: "",
  },
  parts: [
    {
      id: "part-1",
      type: "text",
      text: "Hello, world!",
    },
  ],
}

const mockMessage2: MessageWithParts = {
  info: {
    id: "msg-2",
    sessionID: "session-1",
    role: "assistant",
    time: {
      created: Date.now() - 30000,
      updated: Date.now(),
    },
    parentID: "msg-1",
  },
  parts: [
    {
      id: "part-2",
      type: "text",
      text: "Hi there!",
    },
  ],
}

describe("Message Store", () => {
  beforeEach(() => {
    useMessageStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty messages map", () => {
      const state = useMessageStore.getState()
      expect(state.messagesBySession.size).toBe(0)
    })

    it("should have empty loading states", () => {
      const state = useMessageStore.getState()
      expect(state.loadingStates.size).toBe(0)
    })

    it("should have empty errors", () => {
      const state = useMessageStore.getState()
      expect(state.errors.size).toBe(0)
    })
  })

  describe("loadMessages", () => {
    it("should load messages from API", async () => {
      vi.mocked(api.getSessionMessages).mockResolvedValueOnce([mockMessage, mockMessage2])

      await useMessageStore.getState().loadMessages("session-1")

      const state = useMessageStore.getState()
      const messages = state.messagesBySession.get("session-1")
      expect(messages).toHaveLength(2)
      expect(messages?.[0]).toEqual(mockMessage)
      expect(state.loadingStates.get("session-1")).toBe(false)
      expect(state.loadedStates.get("session-1")).toBe(true)
    })

    it("should set loading state during load", async () => {
      vi.mocked(api.getSessionMessages).mockImplementationOnce(async () => {
        expect(useMessageStore.getState().loadingStates.get("session-1")).toBe(true)
        return [mockMessage]
      })

      await useMessageStore.getState().loadMessages("session-1")
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getSessionMessages).mockRejectedValueOnce(new Error("Network error"))

      await useMessageStore.getState().loadMessages("session-1")

      const state = useMessageStore.getState()
      expect(state.errors.get("session-1")).toBe("Network error")
      expect(state.loadingStates.get("session-1")).toBe(false)
    })

    it("should pass limit parameter", async () => {
      vi.mocked(api.getSessionMessages).mockResolvedValueOnce([mockMessage])

      await useMessageStore.getState().loadMessages("session-1", 10)

      expect(api.getSessionMessages).toHaveBeenCalledWith("session-1", { limit: 10 })
    })
  })

  describe("addMessage", () => {
    it("should add a message to a session", () => {
      useMessageStore.getState().addMessage("session-1", mockMessage)

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages).toHaveLength(1)
      expect(messages?.[0]).toEqual(mockMessage)
    })

    it("should append to existing messages", () => {
      useMessageStore.getState().addMessage("session-1", mockMessage)
      useMessageStore.getState().addMessage("session-1", mockMessage2)

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages).toHaveLength(2)
    })
  })

  describe("updateMessage", () => {
    beforeEach(() => {
      useMessageStore.getState().addMessage("session-1", mockMessage)
    })

    it("should update a message", () => {
      useMessageStore.getState().updateMessage("session-1", "msg-1", { role: "system" })

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].info.role).toBe("system")
    })

    it("should do nothing if message not found", () => {
      useMessageStore.getState().updateMessage("session-1", "non-existent", { role: "system" })

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].info.role).toBe("user")
    })

    it("should do nothing if session not found", () => {
      expect(() => {
        useMessageStore.getState().updateMessage("non-existent", "msg-1", { role: "system" })
      }).not.toThrow()
    })
  })

  describe("appendPart", () => {
    beforeEach(() => {
      useMessageStore.getState().addMessage("session-1", mockMessage)
    })

    it("should append a part to a message", () => {
      const newPart: MessagePart = {
        id: "part-new",
        type: "text",
        text: "Additional content",
      }

      useMessageStore.getState().appendPart("session-1", "msg-1", newPart)

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].parts).toHaveLength(2)
    })

    it("should do nothing if message not found", () => {
      const newPart: MessagePart = {
        id: "part-new",
        type: "text",
        text: "Additional content",
      }

      useMessageStore.getState().appendPart("session-1", "non-existent", newPart)

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].parts).toHaveLength(1)
    })
  })

  describe("updatePart", () => {
    beforeEach(() => {
      useMessageStore.getState().addMessage("session-1", mockMessage)
    })

    it("should update a part", () => {
      useMessageStore.getState().updatePart("session-1", "msg-1", "part-1", { text: "Updated text" })

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].parts[0].text).toBe("Updated text")
    })

    it("should do nothing if part not found", () => {
      useMessageStore.getState().updatePart("session-1", "msg-1", "non-existent", { text: "Updated" })

      const messages = useMessageStore.getState().messagesBySession.get("session-1")
      expect(messages?.[0].parts[0].text).toBe("Hello, world!")
    })
  })

  describe("clearMessages", () => {
    beforeEach(async () => {
      vi.mocked(api.getSessionMessages).mockResolvedValueOnce([mockMessage])
      await useMessageStore.getState().loadMessages("session-1")
    })

    it("should clear all messages for a session", () => {
      useMessageStore.getState().clearMessages("session-1")

      const state = useMessageStore.getState()
      expect(state.messagesBySession.has("session-1")).toBe(false)
      expect(state.loadedStates.has("session-1")).toBe(false)
      expect(state.loadingStates.has("session-1")).toBe(false)
      expect(state.errors.has("session-1")).toBe(false)
    })
  })

  describe("error management", () => {
    it("should set error for session", () => {
      useMessageStore.getState().setError("session-1", "Test error")

      expect(useMessageStore.getState().errors.get("session-1")).toBe("Test error")
    })

    it("should clear all errors", () => {
      useMessageStore.getState().setError("session-1", "Error 1")
      useMessageStore.getState().setError("session-2", "Error 2")

      useMessageStore.getState().clearSessionErrors()

      expect(useMessageStore.getState().errors.size).toBe(0)
    })
  })

  describe("reset", () => {
    it("should reset to initial state", async () => {
      vi.mocked(api.getSessionMessages).mockResolvedValueOnce([mockMessage])
      await useMessageStore.getState().loadMessages("session-1")
      useMessageStore.getState().setError("session-1", "Some error")

      useMessageStore.getState().reset()

      const state = useMessageStore.getState()
      expect(state.messagesBySession.size).toBe(0)
      expect(state.loadingStates.size).toBe(0)
      expect(state.loadedStates.size).toBe(0)
      expect(state.errors.size).toBe(0)
    })
  })
})

describe("Message Selector Hooks", () => {
  beforeEach(async () => {
    useMessageStore.getState().reset()
    vi.mocked(api.getSessionMessages).mockResolvedValueOnce([mockMessage, mockMessage2])
    await useMessageStore.getState().loadMessages("session-1")
  })

  describe("useMessages", () => {
    it("should return messages for a session", () => {
      const { result } = renderHook(() => useMessages("session-1"))
      expect(result.current).toHaveLength(2)
    })

    it("should return empty array for non-existent session", () => {
      const { result } = renderHook(() => useMessages("non-existent"))
      expect(result.current).toEqual([])
    })
  })

  describe("useMessage", () => {
    it("should return a specific message", () => {
      const { result } = renderHook(() => useMessage("session-1", "msg-1"))
      expect(result.current).toEqual(mockMessage)
    })

    it("should return undefined for non-existent message", () => {
      const { result } = renderHook(() => useMessage("session-1", "non-existent"))
      expect(result.current).toBeUndefined()
    })
  })

  describe("useMessagesLoading", () => {
    it("should return loading state", () => {
      const { result } = renderHook(() => useMessagesLoading("session-1"))
      expect(result.current).toBe(false)
    })

    it("should return false for non-existent session", () => {
      const { result } = renderHook(() => useMessagesLoading("non-existent"))
      expect(result.current).toBe(false)
    })
  })

  describe("useMessagesLoaded", () => {
    it("should return loaded state", () => {
      const { result } = renderHook(() => useMessagesLoaded("session-1"))
      expect(result.current).toBe(true)
    })

    it("should return false for non-existent session", () => {
      const { result } = renderHook(() => useMessagesLoaded("non-existent"))
      expect(result.current).toBe(false)
    })
  })

  describe("useMessagesError", () => {
    it("should return error state", () => {
      useMessageStore.getState().setError("session-1", "Test error")
      const { result } = renderHook(() => useMessagesError("session-1"))
      expect(result.current).toBe("Test error")
    })

    it("should return null for no error", () => {
      const { result } = renderHook(() => useMessagesError("session-1"))
      expect(result.current).toBeNull()
    })
  })
})
