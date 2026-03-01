import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useTunnelStore,
  useTunnelStatus,
  useTunnelConnected,
  useTunnelPublicUrl,
  useAvailableTunnelTypes,
  useTunnelLoading,
} from "@/stores/tunnel"
import { renderHook, act, waitFor } from "@testing-library/react"

describe("Tunnel Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useTunnelStore.setState({
      status: null,
      availableTypes: ["cloudflare", "ngrok", "tailscale", "custom"],
      isLoading: false,
      isConnecting: false,
      isDisconnecting: false,
      error: null,
    })
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe("initial state", () => {
    it("should have null status", () => {
      const state = useTunnelStore.getState()
      expect(state.status).toBeNull()
    })

    it("should have available tunnel types", () => {
      const state = useTunnelStore.getState()
      expect(state.availableTypes).toEqual(["cloudflare", "ngrok", "tailscale", "custom"])
    })

    it("should have isLoading false", () => {
      const state = useTunnelStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isConnecting false", () => {
      const state = useTunnelStore.getState()
      expect(state.isConnecting).toBe(false)
    })

    it("should have isDisconnecting false", () => {
      const state = useTunnelStore.getState()
      expect(state.isDisconnecting).toBe(false)
    })

    it("should have null error", () => {
      const state = useTunnelStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("fetchStatus", () => {
    it("should set isLoading to true during fetch", async () => {
      const fetchPromise = useTunnelStore.getState().fetchStatus()

      expect(useTunnelStore.getState().isLoading).toBe(true)
      expect(useTunnelStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      expect(useTunnelStore.getState().isLoading).toBe(false)
    })

    it("should update status after fetch", async () => {
      const fetchPromise = useTunnelStore.getState().fetchStatus()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      const state = useTunnelStore.getState()
      expect(state.status).toEqual({
        type: "none",
        connected: false,
      })
    })
  })

  describe("connect", () => {
    it("should set isConnecting to true during connection", async () => {
      const connectPromise = useTunnelStore.getState().connect("cloudflare")

      expect(useTunnelStore.getState().isConnecting).toBe(true)
      expect(useTunnelStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      expect(useTunnelStore.getState().isConnecting).toBe(false)
    })

    it("should update status with connected state for cloudflare", async () => {
      const connectPromise = useTunnelStore.getState().connect("cloudflare")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      const state = useTunnelStore.getState()
      expect(state.status?.connected).toBe(true)
      expect(state.status?.type).toBe("cloudflare")
      expect(state.status?.publicUrl).toBe("https://random-words.trycloudflare.com")
      expect(state.status?.localUrl).toBe("http://127.0.0.1:4096")
      expect(state.status?.startedAt).toBeDefined()
    })

    it("should update status with connected state for ngrok", async () => {
      const connectPromise = useTunnelStore.getState().connect("ngrok")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      const state = useTunnelStore.getState()
      expect(state.status?.type).toBe("ngrok")
      expect(state.status?.publicUrl).toBe("https://abc123.ngrok.io")
    })

    it("should update status with connected state for tailscale", async () => {
      const connectPromise = useTunnelStore.getState().connect("tailscale")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      const state = useTunnelStore.getState()
      expect(state.status?.type).toBe("tailscale")
      expect(state.status?.publicUrl).toBe("https://machine.tailnet-name.ts.net")
    })

    it("should update status with connected state for custom", async () => {
      const connectPromise = useTunnelStore.getState().connect("custom")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      const state = useTunnelStore.getState()
      expect(state.status?.type).toBe("custom")
      expect(state.status?.publicUrl).toBe("https://custom.example.com")
    })
  })

  describe("disconnect", () => {
    it("should set isDisconnecting to true during disconnection", async () => {
      // First connect
      const connectPromise = useTunnelStore.getState().connect("cloudflare")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      // Then disconnect
      const disconnectPromise = useTunnelStore.getState().disconnect()

      expect(useTunnelStore.getState().isDisconnecting).toBe(true)
      expect(useTunnelStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(500)
      await disconnectPromise

      expect(useTunnelStore.getState().isDisconnecting).toBe(false)
    })

    it("should reset status after disconnect", async () => {
      // First connect
      const connectPromise = useTunnelStore.getState().connect("cloudflare")
      await vi.advanceTimersByTimeAsync(2000)
      await connectPromise

      // Then disconnect
      const disconnectPromise = useTunnelStore.getState().disconnect()
      await vi.advanceTimersByTimeAsync(500)
      await disconnectPromise

      const state = useTunnelStore.getState()
      expect(state.status?.connected).toBe(false)
      expect(state.status?.type).toBe("none")
    })
  })

  describe("hooks", () => {
    it("useTunnelStatus should return status", () => {
      useTunnelStore.setState({
        status: { type: "cloudflare", connected: true, publicUrl: "https://test.com" },
      })

      const { result } = renderHook(() => useTunnelStatus())
      expect(result.current?.type).toBe("cloudflare")
      expect(result.current?.connected).toBe(true)
    })

    it("useTunnelConnected should return connected state", () => {
      useTunnelStore.setState({
        status: { type: "cloudflare", connected: true },
      })

      const { result } = renderHook(() => useTunnelConnected())
      expect(result.current).toBe(true)
    })

    it("useTunnelConnected should return false when no status", () => {
      const { result } = renderHook(() => useTunnelConnected())
      expect(result.current).toBe(false)
    })

    it("useTunnelPublicUrl should return public URL", () => {
      useTunnelStore.setState({
        status: { type: "cloudflare", connected: true, publicUrl: "https://test.trycloudflare.com" },
      })

      const { result } = renderHook(() => useTunnelPublicUrl())
      expect(result.current).toBe("https://test.trycloudflare.com")
    })

    it("useAvailableTunnelTypes should return available types", () => {
      const { result } = renderHook(() => useAvailableTunnelTypes())
      expect(result.current).toEqual(["cloudflare", "ngrok", "tailscale", "custom"])
    })

    it("useTunnelLoading should return loading states", () => {
      useTunnelStore.setState({
        isLoading: true,
        isConnecting: false,
        isDisconnecting: false,
        error: "Test error",
      })

      const { result } = renderHook(() => useTunnelLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isConnecting).toBe(false)
      expect(result.current.isDisconnecting).toBe(false)
      expect(result.current.error).toBe("Test error")
    })
  })
})
