import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useChannelStore,
  useChannels,
  useChannelLoading,
  useZeroBotStatus,
  useChannelCounts,
} from "@/stores/channel"
import { api } from "@/lib/api"
import type { ChannelStatus } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listChannels: vi.fn(),
    checkChannelHealth: vi.fn(),
  },
}))

const mockChannel1: ChannelStatus = {
  name: "telegram-main",
  type: "telegram",
  enabled: true,
  health: "healthy",
  lastCheck: Date.now() - 60000,
}

const mockChannel2: ChannelStatus = {
  name: "discord-server",
  type: "discord",
  enabled: true,
  health: "unhealthy",
  lastCheck: Date.now() - 120000,
}

const mockChannel3: ChannelStatus = {
  name: "slack-workspace",
  type: "slack",
  enabled: false,
  health: "unknown",
  lastCheck: Date.now() - 180000,
}

describe("Channel Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useChannelStore.setState({
      channels: [],
      isLoading: false,
      isToggling: null,
      error: null,
      zeroBotRunning: false,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty channels array", () => {
      const state = useChannelStore.getState()
      expect(state.channels).toEqual([])
    })

    it("should have isLoading false", () => {
      const state = useChannelStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isToggling null", () => {
      const state = useChannelStore.getState()
      expect(state.isToggling).toBeNull()
    })

    it("should have null error", () => {
      const state = useChannelStore.getState()
      expect(state.error).toBeNull()
    })

    it("should have zeroBotRunning false", () => {
      const state = useChannelStore.getState()
      expect(state.zeroBotRunning).toBe(false)
    })
  })

  describe("fetchChannels", () => {
    it("should load channels from API", async () => {
      vi.mocked(api.listChannels).mockResolvedValueOnce([mockChannel1, mockChannel2])

      await useChannelStore.getState().fetchChannels()

      const state = useChannelStore.getState()
      expect(state.channels).toHaveLength(2)
      expect(state.channels[0].name).toBe("telegram-main")
      expect(state.isLoading).toBe(false)
    })

    it("should set zeroBotRunning to true when healthy channel exists", async () => {
      vi.mocked(api.listChannels).mockResolvedValueOnce([mockChannel1])

      await useChannelStore.getState().fetchChannels()

      expect(useChannelStore.getState().zeroBotRunning).toBe(true)
    })

    it("should set zeroBotRunning to false when no healthy channels", async () => {
      vi.mocked(api.listChannels).mockResolvedValueOnce([mockChannel2, mockChannel3])

      await useChannelStore.getState().fetchChannels()

      expect(useChannelStore.getState().zeroBotRunning).toBe(false)
    })

    it("should set isLoading during fetch", async () => {
      vi.mocked(api.listChannels).mockImplementationOnce(async () => {
        expect(useChannelStore.getState().isLoading).toBe(true)
        return [mockChannel1]
      })

      await useChannelStore.getState().fetchChannels()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listChannels).mockRejectedValueOnce(new Error("Network error"))

      await useChannelStore.getState().fetchChannels()

      const state = useChannelStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("addChannel", () => {
    it("should call fetchChannels after add", async () => {
      vi.mocked(api.listChannels).mockResolvedValueOnce([mockChannel1])

      await useChannelStore.getState().addChannel("telegram", "new-channel", {})

      expect(api.listChannels).toHaveBeenCalled()
      expect(useChannelStore.getState().isLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.listChannels).mockRejectedValueOnce(new Error("Add failed"))

      await useChannelStore.getState().addChannel("telegram", "new-channel", {})

      expect(useChannelStore.getState().error).toBe("Add failed")
    })
  })

  describe("removeChannel", () => {
    it("should call fetchChannels after remove", async () => {
      vi.mocked(api.listChannels).mockResolvedValueOnce([])

      await useChannelStore.getState().removeChannel("telegram-main")

      expect(api.listChannels).toHaveBeenCalled()
      expect(useChannelStore.getState().isLoading).toBe(false)
    })

    it("should handle errors", async () => {
      vi.mocked(api.listChannels).mockRejectedValueOnce(new Error("Remove failed"))

      await useChannelStore.getState().removeChannel("telegram-main")

      expect(useChannelStore.getState().error).toBe("Remove failed")
    })
  })

  describe("toggleChannel", () => {
    it("should update channel after toggle", async () => {
      useChannelStore.setState({ channels: [mockChannel1] })
      const updatedChannel = { ...mockChannel1, health: "unhealthy" as const }
      vi.mocked(api.checkChannelHealth).mockResolvedValueOnce(updatedChannel)

      await useChannelStore.getState().toggleChannel("telegram-main")

      const state = useChannelStore.getState()
      expect(state.channels[0].health).toBe("unhealthy")
      expect(state.isToggling).toBeNull()
    })

    it("should set isToggling during toggle", async () => {
      useChannelStore.setState({ channels: [mockChannel1] })
      vi.mocked(api.checkChannelHealth).mockImplementationOnce(async () => {
        expect(useChannelStore.getState().isToggling).toBe("telegram-main")
        return mockChannel1
      })

      await useChannelStore.getState().toggleChannel("telegram-main")
    })

    it("should handle errors", async () => {
      vi.mocked(api.checkChannelHealth).mockRejectedValueOnce(new Error("Toggle failed"))

      await useChannelStore.getState().toggleChannel("telegram-main")

      expect(useChannelStore.getState().error).toBe("Toggle failed")
      expect(useChannelStore.getState().isToggling).toBeNull()
    })
  })

  describe("checkHealth", () => {
    it("should update channel health", async () => {
      useChannelStore.setState({ channels: [mockChannel1] })
      const updatedChannel = { ...mockChannel1, health: "unhealthy" as const }
      vi.mocked(api.checkChannelHealth).mockResolvedValueOnce(updatedChannel)

      await useChannelStore.getState().checkHealth("telegram-main")

      expect(useChannelStore.getState().channels[0].health).toBe("unhealthy")
    })

    it("should handle errors", async () => {
      vi.mocked(api.checkChannelHealth).mockRejectedValueOnce(new Error("Health check failed"))

      await useChannelStore.getState().checkHealth("telegram-main")

      expect(useChannelStore.getState().error).toBe("Health check failed")
    })
  })

  describe("getChannelsByHealth", () => {
    it("should return channels with specified health", () => {
      useChannelStore.setState({ channels: [mockChannel1, mockChannel2, mockChannel3] })

      const healthyChannels = useChannelStore.getState().getChannelsByHealth("healthy")
      const unhealthyChannels = useChannelStore.getState().getChannelsByHealth("unhealthy")

      expect(healthyChannels).toHaveLength(1)
      expect(healthyChannels[0].name).toBe("telegram-main")
      expect(unhealthyChannels).toHaveLength(1)
      expect(unhealthyChannels[0].name).toBe("discord-server")
    })
  })

  describe("getEnabledChannels", () => {
    it("should return only enabled channels", () => {
      useChannelStore.setState({ channels: [mockChannel1, mockChannel2, mockChannel3] })

      const enabledChannels = useChannelStore.getState().getEnabledChannels()

      expect(enabledChannels).toHaveLength(2)
      expect(enabledChannels.every((c) => c.enabled)).toBe(true)
    })
  })

  describe("hooks", () => {
    it("useChannels should return channels", () => {
      useChannelStore.setState({ channels: [mockChannel1, mockChannel2] })

      const { result } = renderHook(() => useChannels())
      expect(result.current).toHaveLength(2)
    })

    it("useChannelLoading should return loading states", () => {
      useChannelStore.setState({
        isLoading: true,
        isToggling: "telegram-main",
        error: "Test error",
      })

      const { result } = renderHook(() => useChannelLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isToggling).toBe("telegram-main")
      expect(result.current.error).toBe("Test error")
    })

    it("useZeroBotStatus should return zeroBotRunning", () => {
      useChannelStore.setState({ zeroBotRunning: true })

      const { result } = renderHook(() => useZeroBotStatus())
      expect(result.current).toBe(true)
    })

    it("useChannelCounts should return counts", () => {
      useChannelStore.setState({ channels: [mockChannel1, mockChannel2, mockChannel3] })

      const { result } = renderHook(() => useChannelCounts())
      expect(result.current.total).toBe(3)
      expect(result.current.enabled).toBe(2)
      expect(result.current.healthy).toBe(1)
    })
  })
})
