import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useHooksStore,
  useHooks,
  useHooksLoading,
  useHooksSettings,
  useHooksLocations,
  useHooksActionTypes,
  useSelectedLifecycle,
  useHooksByLifecycle,
  useHookCounts,
} from "@/stores/hooks"
import { api } from "@/lib/api"
import type { HookEntry, HookSettings, HookLocation, HookActionTypeInfo, HookLifecycle } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listHooks: vi.fn(),
    getHooksByLifecycle: vi.fn(),
    getHooksSettings: vi.fn(),
    getHookLocations: vi.fn(),
    getHookActionTypes: vi.fn(),
  },
}))

const mockHook1: HookEntry = {
  lifecycle: "PreToolUse",
  name: "validate-input",
  definition: { command: "validate" },
  source: "config",
}

const mockHook2: HookEntry = {
  lifecycle: "PostToolUse",
  name: "format-output",
  definition: { command: "format" },
  source: "config",
}

const mockHook3: HookEntry = {
  lifecycle: "PreToolUse",
  name: "check-permissions",
  definition: { command: "check" },
  source: "project",
}

const mockSettings: HookSettings = {
  enabled: true,
  timeout: 5000,
  retryCount: 3,
}

const mockLocation: HookLocation = {
  path: "/Users/test/.config/hooks",
  type: "global",
}

const mockActionType: HookActionTypeInfo = {
  name: "shell",
  description: "Execute shell command",
}

describe("Hooks Store", () => {
  beforeEach(() => {
    useHooksStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty hooks list", () => {
      const state = useHooksStore.getState()
      expect(state.hooks).toEqual([])
    })

    it("should have hooksLoading false", () => {
      const state = useHooksStore.getState()
      expect(state.hooksLoading).toBe(false)
    })

    it("should have no error", () => {
      const state = useHooksStore.getState()
      expect(state.hooksError).toBeNull()
    })

    it("should have null settings", () => {
      const state = useHooksStore.getState()
      expect(state.settings).toBeNull()
    })

    it("should have no selected lifecycle", () => {
      const state = useHooksStore.getState()
      expect(state.selectedLifecycle).toBeNull()
    })
  })

  describe("fetchHooks", () => {
    it("should fetch hooks from API", async () => {
      vi.mocked(api.listHooks).mockResolvedValueOnce([mockHook1, mockHook2, mockHook3])

      await useHooksStore.getState().fetchHooks()

      const state = useHooksStore.getState()
      expect(state.hooks).toHaveLength(3)
      expect(state.hooksLoading).toBe(false)
    })

    it("should set loading state during fetch", async () => {
      vi.mocked(api.listHooks).mockImplementationOnce(async () => {
        expect(useHooksStore.getState().hooksLoading).toBe(true)
        return [mockHook1]
      })

      await useHooksStore.getState().fetchHooks()
    })

    it("should clear error before fetching", async () => {
      useHooksStore.setState({ hooksError: "Previous error" })
      vi.mocked(api.listHooks).mockResolvedValueOnce([])

      await useHooksStore.getState().fetchHooks()

      expect(useHooksStore.getState().hooksError).toBeNull()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listHooks).mockRejectedValueOnce(new Error("Network error"))

      await useHooksStore.getState().fetchHooks()

      const state = useHooksStore.getState()
      expect(state.hooksError).toBe("Network error")
      expect(state.hooksLoading).toBe(false)
    })

    it("should handle non-Error exceptions", async () => {
      vi.mocked(api.listHooks).mockRejectedValueOnce("Unknown error")

      await useHooksStore.getState().fetchHooks()

      expect(useHooksStore.getState().hooksError).toBe("Failed to fetch hooks")
    })
  })

  describe("fetchHooksByLifecycle", () => {
    it("should fetch hooks by lifecycle", async () => {
      const rawHooks = [
        { name: "hook1", definition: { command: "cmd1" } },
        { name: "hook2", definition: { command: "cmd2" } },
      ]
      vi.mocked(api.getHooksByLifecycle).mockResolvedValueOnce(rawHooks)

      await useHooksStore.getState().fetchHooksByLifecycle("PreToolUse")

      const state = useHooksStore.getState()
      expect(state.hooks).toHaveLength(2)
      expect(state.hooks[0].lifecycle).toBe("PreToolUse")
      expect(state.selectedLifecycle).toBe("PreToolUse")
    })

    it("should handle errors", async () => {
      vi.mocked(api.getHooksByLifecycle).mockRejectedValueOnce(new Error("Failed"))

      await useHooksStore.getState().fetchHooksByLifecycle("PreToolUse")

      expect(useHooksStore.getState().hooksError).toBe("Failed")
    })
  })

  describe("fetchSettings", () => {
    it("should fetch settings from API", async () => {
      vi.mocked(api.getHooksSettings).mockResolvedValueOnce(mockSettings)

      await useHooksStore.getState().fetchSettings()

      expect(useHooksStore.getState().settings).toEqual(mockSettings)
      expect(useHooksStore.getState().settingsLoading).toBe(false)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getHooksSettings).mockRejectedValueOnce(new Error("Failed"))

      await useHooksStore.getState().fetchSettings()

      expect(useHooksStore.getState().settingsLoading).toBe(false)
    })
  })

  describe("fetchLocations", () => {
    it("should fetch locations from API", async () => {
      vi.mocked(api.getHookLocations).mockResolvedValueOnce([mockLocation])

      await useHooksStore.getState().fetchLocations()

      expect(useHooksStore.getState().locations).toHaveLength(1)
      expect(useHooksStore.getState().locationsLoading).toBe(false)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getHookLocations).mockRejectedValueOnce(new Error("Failed"))

      await useHooksStore.getState().fetchLocations()

      expect(useHooksStore.getState().locationsLoading).toBe(false)
    })
  })

  describe("fetchActionTypes", () => {
    it("should fetch action types from API", async () => {
      vi.mocked(api.getHookActionTypes).mockResolvedValueOnce([mockActionType])

      await useHooksStore.getState().fetchActionTypes()

      expect(useHooksStore.getState().actionTypes).toHaveLength(1)
      expect(useHooksStore.getState().actionTypesLoading).toBe(false)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getHookActionTypes).mockRejectedValueOnce(new Error("Failed"))

      await useHooksStore.getState().fetchActionTypes()

      expect(useHooksStore.getState().actionTypesLoading).toBe(false)
    })
  })

  describe("setSelectedLifecycle", () => {
    it("should set selected lifecycle", () => {
      useHooksStore.getState().setSelectedLifecycle("PostToolUse")

      expect(useHooksStore.getState().selectedLifecycle).toBe("PostToolUse")
    })

    it("should clear selection when null", () => {
      useHooksStore.setState({ selectedLifecycle: "PreToolUse" })

      useHooksStore.getState().setSelectedLifecycle(null)

      expect(useHooksStore.getState().selectedLifecycle).toBeNull()
    })
  })

  describe("reset", () => {
    it("should reset to initial state", () => {
      useHooksStore.setState({
        hooks: [mockHook1, mockHook2],
        hooksLoading: true,
        hooksError: "Some error",
        settings: mockSettings,
        locations: [mockLocation],
        actionTypes: [mockActionType],
        selectedLifecycle: "PreToolUse",
      })

      useHooksStore.getState().reset()

      const state = useHooksStore.getState()
      expect(state.hooks).toEqual([])
      expect(state.hooksLoading).toBe(false)
      expect(state.hooksError).toBeNull()
      expect(state.settings).toBeNull()
      expect(state.locations).toEqual([])
      expect(state.actionTypes).toEqual([])
      expect(state.selectedLifecycle).toBeNull()
    })
  })
})

describe("Hooks Selector Hooks", () => {
  beforeEach(() => {
    useHooksStore.setState({
      hooks: [mockHook1, mockHook2, mockHook3],
      hooksLoading: false,
      hooksError: null,
      settings: mockSettings,
      locations: [mockLocation],
      actionTypes: [mockActionType],
      selectedLifecycle: "PreToolUse",
    })
  })

  describe("useHooks", () => {
    it("should return all hooks", () => {
      const { result } = renderHook(() => useHooks())
      expect(result.current).toHaveLength(3)
    })
  })

  describe("useHooksLoading", () => {
    it("should return loading state", () => {
      const { result } = renderHook(() => useHooksLoading())
      expect(result.current).toBe(false)
    })
  })

  describe("useHooksSettings", () => {
    it("should return settings", () => {
      const { result } = renderHook(() => useHooksSettings())
      expect(result.current).toEqual(mockSettings)
    })
  })

  describe("useHooksLocations", () => {
    it("should return locations", () => {
      const { result } = renderHook(() => useHooksLocations())
      expect(result.current).toHaveLength(1)
    })
  })

  describe("useHooksActionTypes", () => {
    it("should return action types", () => {
      const { result } = renderHook(() => useHooksActionTypes())
      expect(result.current).toHaveLength(1)
    })
  })

  describe("useSelectedLifecycle", () => {
    it("should return selected lifecycle", () => {
      const { result } = renderHook(() => useSelectedLifecycle())
      expect(result.current).toBe("PreToolUse")
    })
  })

  describe("useHooksByLifecycle", () => {
    it("should return hooks filtered by lifecycle", () => {
      const { result } = renderHook(() => useHooksByLifecycle("PreToolUse"))
      expect(result.current).toHaveLength(2)
      expect(result.current.every((h) => h.lifecycle === "PreToolUse")).toBe(true)
    })

    it("should return empty array for lifecycle with no hooks", () => {
      const { result } = renderHook(() => useHooksByLifecycle("Stop"))
      expect(result.current).toEqual([])
    })
  })

  describe("useHookCounts", () => {
    it("should return counts by lifecycle", () => {
      const { result } = renderHook(() => useHookCounts())
      expect(result.current).toEqual({
        PreToolUse: 2,
        PostToolUse: 1,
        PreResponse: 0,
        Stop: 0,
      })
    })
  })
})
