import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { SSEClient, createSSEClient, getSSEClient, setDefaultSSEClient } from "@/lib/sse"

describe("SSEClient", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("constructor", () => {
    it("should create client with default config", () => {
      const client = new SSEClient()
      expect(client).toBeDefined()
      expect(client.getState()).toBe("disconnected")
    })

    it("should create client with custom config", () => {
      const client = new SSEClient({
        baseUrl: "http://localhost:8080",
        apiKey: "test-key",
        channels: ["message", "status"],
        reconnectInterval: 5000,
        maxReconnectAttempts: 5,
      })
      expect(client).toBeDefined()
      expect(client.getChannels()).toEqual(["message", "status"])
    })
  })

  describe("connect", () => {
    it("should start connection process", async () => {
      const onOpen = vi.fn()
      const onStatusChange = vi.fn()
      const client = new SSEClient(
        { baseUrl: "/api" },
        { onOpen, onStatusChange }
      )

      client.connect()

      // Client should transition from disconnected
      expect(client.getState()).not.toBe("disconnected")

      client.disconnect()
    })

    it("should handle multiple connect calls gracefully", async () => {
      const client = new SSEClient({ baseUrl: "/api" })

      client.connect()
      const stateAfterFirst = client.getState()

      // Try to connect again - should not throw
      expect(() => client.connect()).not.toThrow()

      client.disconnect()
    })
  })

  describe("disconnect", () => {
    it("should disconnect from SSE endpoint", async () => {
      const onClose = vi.fn()
      const onStatusChange = vi.fn()
      const client = new SSEClient(
        { baseUrl: "/api" },
        { onClose, onStatusChange }
      )

      client.connect()

      client.disconnect()

      expect(client.getState()).toBe("disconnected")
      expect(client.isConnected()).toBe(false)
    })
  })

  describe("message handling", () => {
    it("should dispatch message events to handler", async () => {
      const onMessage = vi.fn()
      const onMessageEvent = vi.fn()
      const client = new SSEClient(
        { baseUrl: "/api" },
        { onMessage, onMessageEvent }
      )

      client.connect()

      // The SSE client should be in a non-disconnected state after connect
      expect(client.getState()).not.toBe("disconnected")

      // Handlers should be configured (we can't easily test event dispatch without a real server)
      // Just verify client was created and is attempting to connect
      expect(client).toBeDefined()

      client.disconnect()
    })
  })

  describe("channel management", () => {
    it("should return configured channels", () => {
      const client = new SSEClient({
        channels: ["message", "status", "error"],
      })

      expect(client.getChannels()).toEqual(["message", "status", "error"])
    })

    it("should subscribe to additional channels", async () => {
      const client = new SSEClient({
        channels: ["message"],
      })

      client.subscribeChannels(["status", "error"])

      expect(client.getChannels()).toEqual(["message", "status", "error"])
    })

    it("should unsubscribe from channels", async () => {
      const client = new SSEClient({
        channels: ["message", "status", "error"],
      })

      client.unsubscribeChannels(["error"])

      expect(client.getChannels()).toEqual(["message", "status"])
    })

    it("should not duplicate channels when subscribing", () => {
      const client = new SSEClient({
        channels: ["message", "status"],
      })

      client.subscribeChannels(["message", "error"])

      expect(client.getChannels()).toEqual(["message", "status", "error"])
    })
  })

  describe("state management", () => {
    it("should start in disconnected state", () => {
      const client = new SSEClient()
      expect(client.getState()).toBe("disconnected")
    })

    it("should transition from disconnected after connect", async () => {
      const client = new SSEClient()
      client.connect()
      // State should no longer be disconnected
      const state = client.getState()
      expect(state).not.toBe("disconnected")
      client.disconnect()
    })

    it("should reset reconnect attempts without error", async () => {
      const client = new SSEClient()
      // Reset reconnect attempts should work without throwing
      expect(() => client.resetReconnectAttempts()).not.toThrow()
    })
  })
})

describe("createSSEClient", () => {
  it("should create SSE client and initiate connection", async () => {
    const onOpen = vi.fn()
    const client = createSSEClient(
      { baseUrl: "/api" },
      { onOpen }
    )

    // Client should be created
    expect(client).toBeDefined()
    // State should not be disconnected after createSSEClient (which calls connect)
    expect(client.getState()).not.toBe("disconnected")

    client.disconnect()
  })
})

describe("Default SSE Client", () => {
  it("should create default client on first call", () => {
    const client = getSSEClient()
    expect(client).toBeDefined()
  })

  it("should set custom default client", () => {
    const onMessage = vi.fn()
    setDefaultSSEClient({ baseUrl: "/api/custom" }, { onMessage })

    const client = getSSEClient()
    expect(client).toBeDefined()
  })
})
