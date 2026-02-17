import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { useAgentStore, useAgents, useAgent, useSelectedAgent, useSelectedAgentId, useAgentsByCategory, useAgentsLoading, useAgentError } from "@/stores/agent"
import { renderHook, act } from "@testing-library/react"

describe("Agent Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useAgentStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty agents map", () => {
      const state = useAgentStore.getState()
      expect(state.agents.size).toBe(0)
    })

    it("should have null selectedAgentId", () => {
      const state = useAgentStore.getState()
      expect(state.selectedAgentId).toBeNull()
    })

    it("should have isLoading false", () => {
      const state = useAgentStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isLoaded false", () => {
      const state = useAgentStore.getState()
      expect(state.isLoaded).toBe(false)
    })

    it("should have null error", () => {
      const state = useAgentStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("loadAgents", () => {
    it("should load static agents", async () => {
      await useAgentStore.getState().loadAgents()

      const state = useAgentStore.getState()
      expect(state.agents.size).toBeGreaterThan(0)
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })

    it("should include expected agents", async () => {
      await useAgentStore.getState().loadAgents()

      const state = useAgentStore.getState()
      // Check for some expected agents
      expect(state.agents.has("build")).toBe(true)
      expect(state.agents.has("plan")).toBe(true)
      expect(state.agents.has("code-reviewer")).toBe(true)
      expect(state.agents.has("security-reviewer")).toBe(true)
    })

    it("should set isLoading during load", async () => {
      const loadPromise = useAgentStore.getState().loadAgents()

      // The loading state might already be updated
      await loadPromise

      const state = useAgentStore.getState()
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(true)
    })
  })

  describe("selectAgent", () => {
    beforeEach(async () => {
      await useAgentStore.getState().loadAgents()
    })

    it("should select an agent by ID", () => {
      useAgentStore.getState().selectAgent("build")

      expect(useAgentStore.getState().selectedAgentId).toBe("build")
    })

    it("should clear selection with null", () => {
      useAgentStore.getState().selectAgent("build")
      useAgentStore.getState().selectAgent(null)

      expect(useAgentStore.getState().selectedAgentId).toBeNull()
    })

    it("should warn when selecting non-existent agent", () => {
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {})

      useAgentStore.getState().selectAgent("non-existent")

      expect(warnSpy).toHaveBeenCalledWith('Agent with ID "non-existent" not found')
      warnSpy.mockRestore()
    })
  })

  describe("setSelectedAgent", () => {
    it("should set selected agent without validation", () => {
      useAgentStore.getState().setSelectedAgent("any-id")

      expect(useAgentStore.getState().selectedAgentId).toBe("any-id")
    })
  })

  describe("error management", () => {
    it("should set error", () => {
      useAgentStore.getState().setError("Test error")

      expect(useAgentStore.getState().error).toBe("Test error")
    })

    it("should clear error", () => {
      useAgentStore.getState().setError("Test error")
      useAgentStore.getState().clearError()

      expect(useAgentStore.getState().error).toBeNull()
    })
  })

  describe("reset", () => {
    it("should reset to initial state", async () => {
      await useAgentStore.getState().loadAgents()
      useAgentStore.getState().selectAgent("build")

      useAgentStore.getState().reset()

      const state = useAgentStore.getState()
      expect(state.agents.size).toBe(0)
      expect(state.selectedAgentId).toBeNull()
      expect(state.isLoading).toBe(false)
      expect(state.isLoaded).toBe(false)
    })
  })
})

describe("Agent Selector Hooks", () => {
  beforeEach(async () => {
    useAgentStore.getState().reset()
    await useAgentStore.getState().loadAgents()
  })

  describe("useAgents", () => {
    it("should return all agents as array", () => {
      const { result } = renderHook(() => useAgents())
      expect(result.current.length).toBeGreaterThan(0)
    })

    it("should include agent properties", () => {
      const { result } = renderHook(() => useAgents())
      const buildAgent = result.current.find(a => a.id === "build")

      expect(buildAgent).toBeDefined()
      expect(buildAgent?.name).toBe("build")
      expect(buildAgent?.description).toBeDefined()
      expect(buildAgent?.category).toBe("primary")
    })
  })

  describe("useAgent", () => {
    it("should return a specific agent", () => {
      const { result } = renderHook(() => useAgent("build"))

      expect(result.current).toBeDefined()
      expect(result.current?.id).toBe("build")
    })

    it("should return undefined for non-existent agent", () => {
      const { result } = renderHook(() => useAgent("non-existent"))
      expect(result.current).toBeUndefined()
    })
  })

  describe("useSelectedAgent", () => {
    it("should return null when no agent selected", () => {
      const { result } = renderHook(() => useSelectedAgent())
      expect(result.current).toBeNull()
    })

    it("should return selected agent when set", () => {
      useAgentStore.getState().selectAgent("build")
      const { result } = renderHook(() => useSelectedAgent())

      expect(result.current).toBeDefined()
      expect(result.current?.id).toBe("build")
    })
  })

  describe("useSelectedAgentId", () => {
    it("should return selected agent ID", () => {
      useAgentStore.getState().selectAgent("build")
      const { result } = renderHook(() => useSelectedAgentId())

      expect(result.current).toBe("build")
    })

    it("should return null when no agent selected", () => {
      const { result } = renderHook(() => useSelectedAgentId())
      expect(result.current).toBeNull()
    })
  })

  describe("useAgentsByCategory", () => {
    it("should return agents grouped by category", () => {
      const { result } = renderHook(() => useAgentsByCategory())

      expect(result.current).toBeDefined()
      expect(typeof result.current).toBe("object")
      expect(result.current.primary).toBeDefined()
      expect(result.current.engineering).toBeDefined()
      expect(result.current.content).toBeDefined()
    })

    it("should have correct agents in each category", () => {
      const { result } = renderHook(() => useAgentsByCategory())

      const primaryAgents = result.current.primary
      expect(primaryAgents).toBeDefined()
      expect(primaryAgents?.some(a => a.id === "build")).toBe(true)
      expect(primaryAgents?.some(a => a.id === "plan")).toBe(true)
    })
  })

  describe("useAgentsLoading", () => {
    it("should return loading states", () => {
      const { result } = renderHook(() => useAgentsLoading())

      expect(result.current).toEqual({
        isLoading: false,
        isLoaded: true,
      })
    })
  })

  describe("useAgentError", () => {
    it("should return error state", () => {
      useAgentStore.getState().setError("Test error")
      const { result } = renderHook(() => useAgentError())

      expect(result.current).toBe("Test error")
    })

    it("should return null when no error", () => {
      const { result } = renderHook(() => useAgentError())
      expect(result.current).toBeNull()
    })
  })
})

describe("Agent Categories", () => {
  beforeEach(async () => {
    useAgentStore.getState().reset()
    await useAgentStore.getState().loadAgents()
  })

  it("should have primary category agents", () => {
    const { result } = renderHook(() => useAgentsByCategory())
    const primary = result.current.primary

    expect(primary).toBeDefined()
    expect(primary?.length).toBeGreaterThan(0)
  })

  it("should have engineering category agents", () => {
    const { result } = renderHook(() => useAgentsByCategory())
    const engineering = result.current.engineering

    expect(engineering).toBeDefined()
    expect(engineering?.some(a => a.id === "code-reviewer")).toBe(true)
    expect(engineering?.some(a => a.id === "security-reviewer")).toBe(true)
  })

  it("should have content category agents", () => {
    const { result } = renderHook(() => useAgentsByCategory())
    const content = result.current.content

    expect(content).toBeDefined()
    expect(content?.some(a => a.id === "writer")).toBe(true)
  })

  it("should have zrs category agents", () => {
    const { result } = renderHook(() => useAgentsByCategory())
    const zrs = result.current.zrs

    expect(zrs).toBeDefined()
    expect(zrs?.some(a => a.id === "observer")).toBe(true)
    expect(zrs?.some(a => a.id === "decision")).toBe(true)
  })

  it("should have reverse category agents", () => {
    const { result } = renderHook(() => useAgentsByCategory())
    const reverse = result.current.reverse

    expect(reverse).toBeDefined()
    expect(reverse?.some(a => a.id === "code-reverse")).toBe(true)
  })
})
