import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useConfigStore,
  useConfig,
  useConfigValue,
  useConfigLoading,
  useConfigError,
} from "@/stores/config"
import { api } from "@/lib/api"
import type { ConfigData } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    getConfig: vi.fn(),
    updateConfig: vi.fn(),
  },
}))

const mockConfig: ConfigData = {
  apiEndpoint: "http://localhost:4096",
  theme: "dark",
  language: "en",
  autoSave: true,
  maxTokens: 4096,
}

describe("Config Store", () => {
  beforeEach(() => {
    // Reset store state
    useConfigStore.setState({
      config: null,
      isLoading: false,
      isLoaded: false,
      isUpdating: false,
      error: null,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have null config", () => {
      const state = useConfigStore.getState()
      expect(state.config).toBeNull()
    })

    it("should have isLoading false", () => {
      const state = useConfigStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useConfigStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have isUpdating false", () => {
      const state = useConfigStore.getState()
      expect(state.isUpdating).toBe(false)
    })

    it("should have no error", () => {
      const state = useConfigStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("loadConfig", () => {
    it("should load config from API", async () => {
      vi.mocked(api.getConfig).mockResolvedValueOnce(mockConfig)

      await useConfigStore.getState().loadConfig()

      const state = useConfigStore.getState()
      expect(state.config).toEqual(mockConfig)
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should set loading state during load", async () => {
      vi.mocked(api.getConfig).mockImplementationOnce(async () => {
        expect(useConfigStore.getState().isLoading).toBe(true)
        return mockConfig
      })

      await useConfigStore.getState().loadConfig()
    })

    it("should clear error before loading", async () => {
      useConfigStore.setState({ error: "Previous error" })
      vi.mocked(api.getConfig).mockResolvedValueOnce(mockConfig)

      await useConfigStore.getState().loadConfig()

      expect(useConfigStore.getState().error).toBeNull()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getConfig).mockRejectedValueOnce(new Error("Network error"))

      await useConfigStore.getState().loadConfig()

      const state = useConfigStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(false)
    })

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.getConfig).mockRejectedValueOnce("Unknown error")

      await useConfigStore.getState().loadConfig()

      const state = useConfigStore.getState()
      expect(state.error).toBe("Failed to load config")
    })
  })

  describe("updateConfig", () => {
    it("should update config via API", async () => {
      const updates = { theme: "light" } as Partial<ConfigData>
      const updatedConfig = { ...mockConfig, ...updates }
      vi.mocked(api.updateConfig).mockResolvedValueOnce(updatedConfig)

      const result = await useConfigStore.getState().updateConfig(updates)

      expect(result).toEqual(updatedConfig)
      expect(useConfigStore.getState().config).toEqual(updatedConfig)
      expect(useConfigStore.getState().isUpdating).toBe(false)
    })

    it("should set updating state during update", async () => {
      vi.mocked(api.updateConfig).mockImplementationOnce(async (updates) => {
        expect(useConfigStore.getState().isUpdating).toBe(true)
        return { ...mockConfig, ...updates }
      })

      await useConfigStore.getState().updateConfig({ theme: "light" })
    })

    it("should handle update errors", async () => {
      vi.mocked(api.updateConfig).mockRejectedValueOnce(new Error("Update failed"))

      await expect(useConfigStore.getState().updateConfig({ theme: "light" })).rejects.toThrow("Update failed")

      const state = useConfigStore.getState()
      expect(state.error).toBe("Update failed")
      expect(state.isUpdating).toBe(false)
    })

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.updateConfig).mockRejectedValueOnce("Unknown error")

      await expect(useConfigStore.getState().updateConfig({ theme: "light" })).rejects.toBeDefined()

      expect(useConfigStore.getState().error).toBe("Failed to update config")
    })
  })

  describe("setConfig", () => {
    it("should set config locally", () => {
      useConfigStore.getState().setConfig(mockConfig)

      const state = useConfigStore.getState()
      expect(state.config).toEqual(mockConfig)
      expect(state.isLoaded).toBe(true)
    })
  })

  describe("resetConfig", () => {
    it("should reset config to null", () => {
      useConfigStore.setState({ config: mockConfig, isLoaded: true })

      useConfigStore.getState().resetConfig()

      const state = useConfigStore.getState()
      expect(state.config).toBeNull()
      expect(state.isLoaded).toBe(false)
    })
  })

  describe("error management", () => {
    it("should set error", () => {
      useConfigStore.getState().setError("Test error")
      expect(useConfigStore.getState().error).toBe("Test error")
    })

    it("should clear error", () => {
      useConfigStore.setState({ error: "Some error" })

      useConfigStore.getState().clearError()

      expect(useConfigStore.getState().error).toBeNull()
    })
  })
})

describe("Config Selector Hooks", () => {
  beforeEach(() => {
    useConfigStore.setState({
      config: mockConfig,
      isLoading: false,
      isLoaded: true,
      isUpdating: false,
      error: null,
    })
  })

  describe("useConfig", () => {
    it("should return the config", () => {
      const { result } = renderHook(() => useConfig())
      expect(result.current).toEqual(mockConfig)
    })

    it("should return null when no config", () => {
      useConfigStore.setState({ config: null })
      const { result } = renderHook(() => useConfig())
      expect(result.current).toBeNull()
    })
  })

  describe("useConfigValue", () => {
    it("should return a specific config value", () => {
      const { result } = renderHook(() => useConfigValue("theme"))
      expect(result.current).toBe("dark")
    })

    it("should return null when config is null", () => {
      useConfigStore.setState({ config: null })
      const { result } = renderHook(() => useConfigValue("theme"))
      expect(result.current).toBeNull()
    })
  })

  describe("useConfigLoading", () => {
    it("should return loading states", () => {
      useConfigStore.setState({ isLoading: true, isLoaded: false, isUpdating: true })
      const { result } = renderHook(() => useConfigLoading())
      expect(result.current).toEqual({
        isLoading: true,
        isLoaded: false,
        isUpdating: true,
      })
    })
  })

  describe("useConfigError", () => {
    it("should return error state", () => {
      useConfigStore.setState({ error: "Test error" })
      const { result } = renderHook(() => useConfigError())
      expect(result.current).toBe("Test error")
    })

    it("should return null when no error", () => {
      const { result } = renderHook(() => useConfigError())
      expect(result.current).toBeNull()
    })
  })
})
