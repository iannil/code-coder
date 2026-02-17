import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useSSEStore,
  useSSEConnectionState,
  useSSEConnected,
  useSSEConnecting,
  useSSESubscribedChannels,
  useSSEEvents,
  useSSEError,
  useSSEReconnectInfo,
} from "@/stores/sse"
import type { SSEClient } from "@/lib/sse"
import type { SSEDataEvent, SSEEventType } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock SSE client
const createMockSSEClient = (): SSEClient => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
  subscribeChannels: vi.fn(),
  unsubscribeChannels: vi.fn(),
  getState: vi.fn().mockReturnValue("disconnected"),
  isConnected: vi.fn().mockReturnValue(false),
})

describe("SSE Store", () => {
  beforeEach(() => {
    // Reset store state
    useSSEStore.setState({
      connectionState: "disconnected",
      lastConnected: null,
      reconnectAttempts: 0,
      subscribedChannels: new Set(),
      events: [],
      maxEventBufferSize: 100,
      error: null,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should be disconnected", () => {
      const state = useSSEStore.getState()
      expect(state.connectionState).toBe("disconnected")
    })

    it("should have empty subscribed channels", () => {
      const state = useSSEStore.getState()
      expect(state.subscribedChannels.size).toBe(0)
    })

    it("should have empty events buffer", () => {
      const state = useSSEStore.getState()
      expect(state.events).toEqual([])
    })

    it("should have no error", () => {
      const state = useSSEStore.getState()
      expect(state.error).toBeNull()
    })

    it("should have zero reconnect attempts", () => {
      const state = useSSEStore.getState()
      expect(state.reconnectAttempts).toBe(0)
    })
  })

  describe("connect", () => {
    it("should connect using client factory", () => {
      const mockClient = createMockSSEClient()
      const clientFactory = vi.fn().mockReturnValue(mockClient)

      useSSEStore.getState().connect(clientFactory)

      expect(clientFactory).toHaveBeenCalled()
      expect(mockClient.connect).toHaveBeenCalled()
      expect(useSSEStore.getState().connectionState).toBe("connecting")
    })

    it("should clear error when connecting", () => {
      useSSEStore.setState({ error: "Previous error" })
      const mockClient = createMockSSEClient()

      useSSEStore.getState().connect(() => mockClient)

      expect(useSSEStore.getState().error).toBeNull()
    })

    it("should disconnect existing client before connecting", () => {
      const oldClient = createMockSSEClient()
      const newClient = createMockSSEClient()

      // Connect first client
      useSSEStore.getState().connect(() => oldClient)

      // Connect second client
      useSSEStore.getState().connect(() => newClient)

      expect(oldClient.disconnect).toHaveBeenCalled()
      expect(newClient.connect).toHaveBeenCalled()
    })
  })

  describe("disconnect", () => {
    it("should disconnect and reset state", () => {
      const mockClient = createMockSSEClient()
      useSSEStore.getState().connect(() => mockClient)
      useSSEStore.setState({
        connectionState: "connected",
        lastConnected: Date.now(),
        reconnectAttempts: 3,
      })

      useSSEStore.getState().disconnect()

      const state = useSSEStore.getState()
      expect(mockClient.disconnect).toHaveBeenCalled()
      expect(state.connectionState).toBe("disconnected")
      expect(state.lastConnected).toBeNull()
      expect(state.reconnectAttempts).toBe(0)
    })

    it("should handle disconnect when no client", () => {
      expect(() => {
        useSSEStore.getState().disconnect()
      }).not.toThrow()
    })
  })

  describe("reconnect", () => {
    it("should set reconnecting state and increment attempts", () => {
      const mockClient = createMockSSEClient()
      useSSEStore.getState().connect(() => mockClient)
      useSSEStore.setState({ reconnectAttempts: 2 })

      useSSEStore.getState().reconnect()

      const state = useSSEStore.getState()
      expect(state.connectionState).toBe("reconnecting")
      expect(state.reconnectAttempts).toBe(3)
    })

    it("should disconnect and reconnect client", () => {
      const mockClient = createMockSSEClient()
      useSSEStore.getState().connect(() => mockClient)
      vi.clearAllMocks()

      useSSEStore.getState().reconnect()

      expect(mockClient.disconnect).toHaveBeenCalled()
      expect(mockClient.connect).toHaveBeenCalled()
    })
  })

  describe("channel management", () => {
    describe("subscribeChannels", () => {
      it("should add channels to subscribed set", () => {
        const channels: SSEEventType[] = ["message", "status"]

        useSSEStore.getState().subscribeChannels(channels)

        const state = useSSEStore.getState()
        expect(state.subscribedChannels.has("message")).toBe(true)
        expect(state.subscribedChannels.has("status")).toBe(true)
      })

      it("should update client when connected", () => {
        const mockClient = createMockSSEClient()
        useSSEStore.getState().connect(() => mockClient)
        vi.clearAllMocks()

        const channels: SSEEventType[] = ["message", "error"]
        useSSEStore.getState().subscribeChannels(channels)

        expect(mockClient.subscribeChannels).toHaveBeenCalledWith(channels)
      })
    })

    describe("unsubscribeChannels", () => {
      it("should remove channels from subscribed set", () => {
        useSSEStore.setState({
          subscribedChannels: new Set(["message", "status", "error"] as SSEEventType[]),
        })

        useSSEStore.getState().unsubscribeChannels(["message"])

        const state = useSSEStore.getState()
        expect(state.subscribedChannels.has("message")).toBe(false)
        expect(state.subscribedChannels.has("status")).toBe(true)
      })

      it("should update client when connected", () => {
        const mockClient = createMockSSEClient()
        useSSEStore.getState().connect(() => mockClient)
        useSSEStore.setState({
          subscribedChannels: new Set(["message", "status"] as SSEEventType[]),
        })
        vi.clearAllMocks()

        useSSEStore.getState().unsubscribeChannels(["message"])

        expect(mockClient.unsubscribeChannels).toHaveBeenCalledWith(["message"])
      })
    })
  })

  describe("event handling", () => {
    describe("addEvent", () => {
      it("should add event to buffer", () => {
        const event: SSEDataEvent = {
          type: "message",
          data: { content: "test" },
          timestamp: Date.now(),
        }

        useSSEStore.getState().addEvent(event)

        expect(useSSEStore.getState().events).toHaveLength(1)
        expect(useSSEStore.getState().events[0]).toEqual(event)
      })

      it("should limit buffer size", () => {
        useSSEStore.setState({ maxEventBufferSize: 3 })

        for (let i = 0; i < 5; i++) {
          useSSEStore.getState().addEvent({
            type: "message",
            data: { index: i },
            timestamp: Date.now(),
          })
        }

        const events = useSSEStore.getState().events
        expect(events).toHaveLength(3)
        expect(events[0].data.index).toBe(2)
        expect(events[2].data.index).toBe(4)
      })
    })

    describe("clearEvents", () => {
      it("should clear all events", () => {
        useSSEStore.setState({
          events: [
            { type: "message", data: {}, timestamp: Date.now() },
            { type: "status", data: {}, timestamp: Date.now() },
          ],
        })

        useSSEStore.getState().clearEvents()

        expect(useSSEStore.getState().events).toEqual([])
      })
    })
  })

  describe("error management", () => {
    it("should set error", () => {
      useSSEStore.getState().setError("Connection failed")
      expect(useSSEStore.getState().error).toBe("Connection failed")
    })

    it("should clear error", () => {
      useSSEStore.setState({ error: "Some error" })

      useSSEStore.getState().clearError()

      expect(useSSEStore.getState().error).toBeNull()
    })
  })

  describe("reset", () => {
    it("should reset all state", () => {
      const mockClient = createMockSSEClient()
      useSSEStore.getState().connect(() => mockClient)
      useSSEStore.setState({
        connectionState: "connected",
        lastConnected: Date.now(),
        reconnectAttempts: 5,
        subscribedChannels: new Set(["message"] as SSEEventType[]),
        events: [{ type: "message", data: {}, timestamp: Date.now() }],
        error: "Some error",
      })

      useSSEStore.getState().reset()

      const state = useSSEStore.getState()
      expect(state.connectionState).toBe("disconnected")
      expect(state.lastConnected).toBeNull()
      expect(state.reconnectAttempts).toBe(0)
      expect(state.subscribedChannels.size).toBe(0)
      expect(state.events).toEqual([])
      expect(state.error).toBeNull()
    })
  })
})

describe("SSE Selector Hooks", () => {
  beforeEach(() => {
    useSSEStore.setState({
      connectionState: "connected",
      lastConnected: 1234567890,
      reconnectAttempts: 2,
      subscribedChannels: new Set(["message", "status"] as SSEEventType[]),
      events: [{ type: "message", data: { test: true }, timestamp: Date.now() }],
      maxEventBufferSize: 100,
      error: null,
    })
  })

  describe("useSSEConnectionState", () => {
    it("should return connection state", () => {
      const { result } = renderHook(() => useSSEConnectionState())
      expect(result.current).toBe("connected")
    })
  })

  describe("useSSEConnected", () => {
    it("should return true when connected", () => {
      const { result } = renderHook(() => useSSEConnected())
      expect(result.current).toBe(true)
    })

    it("should return false when not connected", () => {
      useSSEStore.setState({ connectionState: "disconnected" })
      const { result } = renderHook(() => useSSEConnected())
      expect(result.current).toBe(false)
    })
  })

  describe("useSSEConnecting", () => {
    it("should return true when connecting", () => {
      useSSEStore.setState({ connectionState: "connecting" })
      const { result } = renderHook(() => useSSEConnecting())
      expect(result.current).toBe(true)
    })

    it("should return true when reconnecting", () => {
      useSSEStore.setState({ connectionState: "reconnecting" })
      const { result } = renderHook(() => useSSEConnecting())
      expect(result.current).toBe(true)
    })

    it("should return false when connected", () => {
      const { result } = renderHook(() => useSSEConnecting())
      expect(result.current).toBe(false)
    })
  })

  describe("useSSESubscribedChannels", () => {
    it("should return subscribed channels as array", () => {
      const { result } = renderHook(() => useSSESubscribedChannels())
      expect(result.current).toContain("message")
      expect(result.current).toContain("status")
    })
  })

  describe("useSSEEvents", () => {
    it("should return events", () => {
      const { result } = renderHook(() => useSSEEvents())
      expect(result.current).toHaveLength(1)
      expect(result.current[0].data.test).toBe(true)
    })
  })

  describe("useSSEError", () => {
    it("should return error state", () => {
      useSSEStore.setState({ error: "Connection error" })
      const { result } = renderHook(() => useSSEError())
      expect(result.current).toBe("Connection error")
    })

    it("should return null when no error", () => {
      const { result } = renderHook(() => useSSEError())
      expect(result.current).toBeNull()
    })
  })

  describe("useSSEReconnectInfo", () => {
    it("should return reconnect info", () => {
      const { result } = renderHook(() => useSSEReconnectInfo())
      expect(result.current).toEqual({
        attempts: 2,
        lastConnected: 1234567890,
      })
    })
  })
})
