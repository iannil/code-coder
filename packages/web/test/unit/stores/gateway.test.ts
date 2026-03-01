import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useGatewayStore,
  useGatewayStatus,
  useGatewayRunning,
  useGatewayEndpoints,
  useGatewayRequests,
  useGatewayLoading,
} from "@/stores/gateway"
import { renderHook } from "@testing-library/react"

describe("Gateway Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useGatewayStore.setState({
      status: null,
      isLoading: false,
      isStarting: false,
      isStopping: false,
      error: null,
    })
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe("initial state", () => {
    it("should have null status", () => {
      const state = useGatewayStore.getState()
      expect(state.status).toBeNull()
    })

    it("should have isLoading false", () => {
      const state = useGatewayStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isStarting false", () => {
      const state = useGatewayStore.getState()
      expect(state.isStarting).toBe(false)
    })

    it("should have isStopping false", () => {
      const state = useGatewayStore.getState()
      expect(state.isStopping).toBe(false)
    })

    it("should have null error", () => {
      const state = useGatewayStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("fetchStatus", () => {
    it("should set isLoading to true during fetch", async () => {
      const fetchPromise = useGatewayStore.getState().fetchStatus()

      expect(useGatewayStore.getState().isLoading).toBe(true)
      expect(useGatewayStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      expect(useGatewayStore.getState().isLoading).toBe(false)
    })

    it("should update status after fetch", async () => {
      const fetchPromise = useGatewayStore.getState().fetchStatus()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      const state = useGatewayStore.getState()
      expect(state.status).toBeDefined()
      expect(state.status?.running).toBe(true)
      expect(state.status?.host).toBe("127.0.0.1")
      expect(state.status?.port).toBe(4402)
      expect(state.status?.endpoints.length).toBeGreaterThan(0)
    })
  })

  describe("start", () => {
    it("should set isStarting to true during start", async () => {
      const startPromise = useGatewayStore.getState().start()

      expect(useGatewayStore.getState().isStarting).toBe(true)
      expect(useGatewayStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(1000)
      await startPromise

      expect(useGatewayStore.getState().isStarting).toBe(false)
    })

    it("should update status to running when no prior status", async () => {
      const startPromise = useGatewayStore.getState().start()
      await vi.advanceTimersByTimeAsync(1000)
      await startPromise

      const state = useGatewayStore.getState()
      expect(state.status?.running).toBe(true)
      expect(state.status?.host).toBe("127.0.0.1")
      expect(state.status?.port).toBe(4402)
      expect(state.status?.uptime).toBe(0)
    })

    it("should update existing status to running", async () => {
      // First fetch status
      const fetchPromise = useGatewayStore.getState().fetchStatus()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      // Then stop
      const stopPromise = useGatewayStore.getState().stop()
      await vi.advanceTimersByTimeAsync(500)
      await stopPromise

      expect(useGatewayStore.getState().status?.running).toBe(false)

      // Then start
      const startPromise = useGatewayStore.getState().start()
      await vi.advanceTimersByTimeAsync(1000)
      await startPromise

      expect(useGatewayStore.getState().status?.running).toBe(true)
    })
  })

  describe("stop", () => {
    it("should set isStopping to true during stop", async () => {
      // First fetch status
      const fetchPromise = useGatewayStore.getState().fetchStatus()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      // Then stop
      const stopPromise = useGatewayStore.getState().stop()

      expect(useGatewayStore.getState().isStopping).toBe(true)
      expect(useGatewayStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(500)
      await stopPromise

      expect(useGatewayStore.getState().isStopping).toBe(false)
    })

    it("should update status to not running", async () => {
      // First fetch status
      const fetchPromise = useGatewayStore.getState().fetchStatus()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      expect(useGatewayStore.getState().status?.running).toBe(true)

      // Then stop
      const stopPromise = useGatewayStore.getState().stop()
      await vi.advanceTimersByTimeAsync(500)
      await stopPromise

      const state = useGatewayStore.getState()
      expect(state.status?.running).toBe(false)
      expect(state.status?.uptime).toBeUndefined()
    })

    it("should return null when no prior status", async () => {
      const stopPromise = useGatewayStore.getState().stop()
      await vi.advanceTimersByTimeAsync(500)
      await stopPromise

      expect(useGatewayStore.getState().status).toBeNull()
    })
  })

  describe("fetchRequests", () => {
    it("should complete without error", async () => {
      const fetchPromise = useGatewayStore.getState().fetchRequests()
      await vi.advanceTimersByTimeAsync(200)
      await fetchPromise

      // Just verify it doesn't throw
      expect(useGatewayStore.getState().error).toBeNull()
    })
  })

  describe("hooks", () => {
    it("useGatewayStatus should return status", () => {
      const mockStatus = {
        running: true,
        host: "127.0.0.1",
        port: 4402,
        endpoints: [],
        requestCount: 0,
        recentRequests: [],
      }
      useGatewayStore.setState({ status: mockStatus })

      const { result } = renderHook(() => useGatewayStatus())
      expect(result.current?.running).toBe(true)
      expect(result.current?.host).toBe("127.0.0.1")
    })

    it("useGatewayRunning should return running state", () => {
      useGatewayStore.setState({
        status: { running: true, host: "127.0.0.1", port: 4402, endpoints: [], requestCount: 0, recentRequests: [] },
      })

      const { result } = renderHook(() => useGatewayRunning())
      expect(result.current).toBe(true)
    })

    it("useGatewayRunning should return false when no status", () => {
      const { result } = renderHook(() => useGatewayRunning())
      expect(result.current).toBe(false)
    })

    it("useGatewayEndpoints should return endpoints", () => {
      const endpoints = [
        { path: "/webhook", method: "POST", description: "Webhook endpoint" },
      ]
      useGatewayStore.setState({
        status: { running: true, host: "127.0.0.1", port: 4402, endpoints, requestCount: 0, recentRequests: [] },
      })

      const { result } = renderHook(() => useGatewayEndpoints())
      expect(result.current).toEqual(endpoints)
    })

    it("useGatewayRequests should return recent requests", () => {
      const recentRequests = [
        { id: "1", method: "POST", path: "/webhook", status: 200, timestamp: Date.now(), duration: 45 },
      ]
      useGatewayStore.setState({
        status: { running: true, host: "127.0.0.1", port: 4402, endpoints: [], requestCount: 1, recentRequests },
      })

      const { result } = renderHook(() => useGatewayRequests())
      expect(result.current).toEqual(recentRequests)
    })

    it("useGatewayLoading should return loading states", () => {
      useGatewayStore.setState({
        isLoading: true,
        isStarting: false,
        isStopping: false,
        error: "Test error",
      })

      const { result } = renderHook(() => useGatewayLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isStarting).toBe(false)
      expect(result.current.isStopping).toBe(false)
      expect(result.current.error).toBe("Test error")
    })
  })
})
