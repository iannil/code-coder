import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useProviderStore,
  useProviders,
  useConnectedProviders,
  useSelectedModel,
  useModelFavorites,
  useModelRecents,
  useProviderLoading,
} from "@/stores/provider"
import { api } from "@/lib/api"
import type { ProviderInfo, ProviderListResponse, ModelSelection } from "@/lib/types"
import { renderHook, act } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listProviders: vi.fn(),
    getProviderAuthMethods: vi.fn(),
  },
}))

// Mock localStorage
const mockLocalStorage = new Map<string, string>()
beforeEach(() => {
  mockLocalStorage.clear()
  vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockLocalStorage.get(key) ?? null)
  vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
    mockLocalStorage.set(key, value)
  })
})

const mockProvider: ProviderInfo = {
  id: "openai",
  name: "OpenAI",
  models: {
    "gpt-4": {
      id: "gpt-4",
      name: "GPT-4",
      contextWindow: 128000,
    },
    "gpt-3.5-turbo": {
      id: "gpt-3.5-turbo",
      name: "GPT-3.5 Turbo",
      contextWindow: 16000,
    },
  },
}

const mockProvider2: ProviderInfo = {
  id: "anthropic",
  name: "Anthropic",
  models: {
    "claude-3": {
      id: "claude-3",
      name: "Claude 3",
      contextWindow: 200000,
    },
  },
}

const mockProviderListResponse: ProviderListResponse = {
  all: [mockProvider, mockProvider2],
  connected: ["openai"],
  default: {
    openai: "gpt-4",
    anthropic: "claude-3",
  },
}

describe("Provider Store", () => {
  beforeEach(() => {
    // Reset store state
    useProviderStore.setState({
      all: [],
      connected: [],
      defaults: {},
      authMethods: {},
      selectedModel: null,
      favorites: [],
      recents: [],
      isLoading: false,
      error: null,
    })
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty providers list", () => {
      const state = useProviderStore.getState()
      expect(state.all).toEqual([])
    })

    it("should have empty connected list", () => {
      const state = useProviderStore.getState()
      expect(state.connected).toEqual([])
    })

    it("should have isLoading false", () => {
      const state = useProviderStore.getState()
      expect(state.isLoading).toBe(false)
    })
  })

  describe("fetchProviders", () => {
    it("should fetch providers from API", async () => {
      vi.mocked(api.listProviders).mockResolvedValueOnce(mockProviderListResponse)

      await useProviderStore.getState().fetchProviders()

      const state = useProviderStore.getState()
      expect(state.all).toHaveLength(2)
      expect(state.connected).toEqual(["openai"])
      expect(state.defaults).toEqual({ openai: "gpt-4", anthropic: "claude-3" })
      expect(state.isLoading).toBe(false)
    })

    it("should set loading state during fetch", async () => {
      vi.mocked(api.listProviders).mockImplementationOnce(async () => {
        expect(useProviderStore.getState().isLoading).toBe(true)
        return mockProviderListResponse
      })

      await useProviderStore.getState().fetchProviders()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listProviders).mockRejectedValueOnce(new Error("Network error"))

      await useProviderStore.getState().fetchProviders()

      const state = useProviderStore.getState()
      expect(state.error).toBe("Network error")
      expect(state.isLoading).toBe(false)
    })
  })

  describe("fetchAuthMethods", () => {
    it("should fetch auth methods from API", async () => {
      const mockAuthMethods = {
        openai: [{ type: "api_key", name: "API Key" }],
      }
      vi.mocked(api.getProviderAuthMethods).mockResolvedValueOnce(mockAuthMethods)

      await useProviderStore.getState().fetchAuthMethods()

      const state = useProviderStore.getState()
      expect(state.authMethods).toEqual(mockAuthMethods)
    })

    it("should handle auth methods errors silently", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {})
      vi.mocked(api.getProviderAuthMethods).mockRejectedValueOnce(new Error("Failed"))

      await useProviderStore.getState().fetchAuthMethods()

      expect(consoleSpy).toHaveBeenCalled()
      consoleSpy.mockRestore()
    })
  })

  describe("selectModel", () => {
    it("should select a model", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }

      useProviderStore.getState().selectModel(selection)

      expect(useProviderStore.getState().selectedModel).toEqual(selection)
    })

    it("should save selection to localStorage", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }

      useProviderStore.getState().selectModel(selection)

      expect(mockLocalStorage.get("codecoder:selected-model")).toBe(JSON.stringify(selection))
    })

    it("should add selection to recents", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }

      useProviderStore.getState().selectModel(selection)

      expect(useProviderStore.getState().recents).toContainEqual(selection)
    })
  })

  describe("clearSelection", () => {
    it("should clear selected model", () => {
      useProviderStore.setState({ selectedModel: { providerID: "openai", modelID: "gpt-4" } })

      useProviderStore.getState().clearSelection()

      expect(useProviderStore.getState().selectedModel).toBeNull()
    })
  })

  describe("toggleFavorite", () => {
    it("should add model to favorites", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }

      useProviderStore.getState().toggleFavorite(selection)

      expect(useProviderStore.getState().favorites).toContainEqual(selection)
    })

    it("should remove model from favorites if already favorited", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }
      useProviderStore.setState({ favorites: [selection] })

      useProviderStore.getState().toggleFavorite(selection)

      expect(useProviderStore.getState().favorites).not.toContainEqual(selection)
    })
  })

  describe("addRecent", () => {
    it("should add to recents", () => {
      const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }

      useProviderStore.getState().addRecent(selection)

      expect(useProviderStore.getState().recents[0]).toEqual(selection)
    })

    it("should move existing to front", () => {
      const selection1: ModelSelection = { providerID: "openai", modelID: "gpt-4" }
      const selection2: ModelSelection = { providerID: "anthropic", modelID: "claude-3" }
      useProviderStore.setState({ recents: [selection1, selection2] })

      useProviderStore.getState().addRecent(selection2)

      expect(useProviderStore.getState().recents[0]).toEqual(selection2)
    })

    it("should limit recents to 10", () => {
      const selections = Array.from({ length: 12 }, (_, i) => ({
        providerID: `provider-${i}`,
        modelID: `model-${i}`,
      }))

      for (const selection of selections) {
        useProviderStore.getState().addRecent(selection)
      }

      expect(useProviderStore.getState().recents).toHaveLength(10)
    })
  })

  describe("clearRecents", () => {
    it("should clear all recents", () => {
      useProviderStore.setState({
        recents: [
          { providerID: "openai", modelID: "gpt-4" },
          { providerID: "anthropic", modelID: "claude-3" },
        ],
      })

      useProviderStore.getState().clearRecents()

      expect(useProviderStore.getState().recents).toEqual([])
    })
  })

  describe("helper methods", () => {
    beforeEach(async () => {
      vi.mocked(api.listProviders).mockResolvedValueOnce(mockProviderListResponse)
      await useProviderStore.getState().fetchProviders()
    })

    describe("getProvider", () => {
      it("should return provider by ID", () => {
        const provider = useProviderStore.getState().getProvider("openai")
        expect(provider).toEqual(mockProvider)
      })

      it("should return undefined for non-existent provider", () => {
        const provider = useProviderStore.getState().getProvider("non-existent")
        expect(provider).toBeUndefined()
      })
    })

    describe("getModel", () => {
      it("should return model by provider and model ID", () => {
        const model = useProviderStore.getState().getModel("openai", "gpt-4")
        expect(model?.id).toBe("gpt-4")
      })

      it("should return undefined for non-existent model", () => {
        const model = useProviderStore.getState().getModel("openai", "non-existent")
        expect(model).toBeUndefined()
      })
    })

    describe("isConnected", () => {
      it("should return true for connected provider", () => {
        expect(useProviderStore.getState().isConnected("openai")).toBe(true)
      })

      it("should return false for non-connected provider", () => {
        expect(useProviderStore.getState().isConnected("anthropic")).toBe(false)
      })
    })

    describe("isFavorite", () => {
      it("should return true for favorited model", () => {
        const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }
        useProviderStore.setState({ favorites: [selection] })

        expect(useProviderStore.getState().isFavorite(selection)).toBe(true)
      })

      it("should return false for non-favorited model", () => {
        const selection: ModelSelection = { providerID: "openai", modelID: "gpt-4" }
        expect(useProviderStore.getState().isFavorite(selection)).toBe(false)
      })
    })
  })
})

describe("Provider Selector Hooks", () => {
  beforeEach(async () => {
    useProviderStore.setState({
      all: [mockProvider, mockProvider2],
      connected: ["openai"],
      defaults: { openai: "gpt-4" },
      selectedModel: { providerID: "openai", modelID: "gpt-4" },
      favorites: [{ providerID: "openai", modelID: "gpt-4" }],
      recents: [{ providerID: "anthropic", modelID: "claude-3" }],
      isLoading: false,
      error: null,
      authMethods: {},
    })
  })

  describe("useProviders", () => {
    it("should return all providers", () => {
      const { result } = renderHook(() => useProviders())
      expect(result.current).toHaveLength(2)
    })
  })

  describe("useConnectedProviders", () => {
    it("should return connected providers", () => {
      const { result } = renderHook(() => useConnectedProviders())
      expect(result.current).toEqual(["openai"])
    })
  })

  describe("useSelectedModel", () => {
    it("should return selected model", () => {
      const { result } = renderHook(() => useSelectedModel())
      expect(result.current).toEqual({ providerID: "openai", modelID: "gpt-4" })
    })
  })

  describe("useModelFavorites", () => {
    it("should return favorites", () => {
      const { result } = renderHook(() => useModelFavorites())
      expect(result.current).toContainEqual({ providerID: "openai", modelID: "gpt-4" })
    })
  })

  describe("useModelRecents", () => {
    it("should return recents", () => {
      const { result } = renderHook(() => useModelRecents())
      expect(result.current).toContainEqual({ providerID: "anthropic", modelID: "claude-3" })
    })
  })

  describe("useProviderLoading", () => {
    it("should return loading state", () => {
      const { result } = renderHook(() => useProviderLoading())
      expect(result.current).toEqual({ isLoading: false, error: null })
    })
  })
})
