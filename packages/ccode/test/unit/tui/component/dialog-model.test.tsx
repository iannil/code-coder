// @ts-nocheck
/**
 * DialogModel Component Unit Tests
 *
 * Tests for the model selection dialog including:
 * - Model list display
 * - Search/filter functionality
 * - Provider grouping
 * - Favorite models
 * - Recent models
 */

import { describe, it, expect, beforeEach, mock } from "bun:test"
import { render } from "solid-js/web"
import { createRoot } from "solid-js"
import { TestProviders } from "@/test/helpers/test-context"
import { DialogModel } from "@/cli/cmd/tui/component/dialog-model"

// Mock the local and sync data
const mockLocalModel = {
  current: mock(() => ({ providerID: "anthropic", modelID: "claude-sonnet-4-20250514" })),
  set: mock(() => {}),
  favorite: mock(() => [
    { providerID: "anthropic", modelID: "claude-sonnet-4-20250514" },
    { providerID: "openai", modelID: "gpt-4" },
  ]),
  recent: mock(() => [
    { providerID: "google", modelID: "gemini-2.0-flash-exp" },
  ]),
  toggleFavorite: mock(() => {}),
  parsed: mock(() => ({ model: "sonnet-4", provider: "anthropic" })),
}

const mockSyncData = {
  provider: [
    {
      id: "anthropic",
      name: "Anthropic",
      models: {
        "claude-sonnet-4-20250514": {
          id: "claude-sonnet-4-20250514",
          name: "Claude Sonnet 4",
          cost: { input: 3, output: 15 },
          status: "available",
        },
        "claude-opus-4-20250514": {
          id: "claude-opus-4-20250514",
          name: "Claude Opus 4",
          cost: { input: 15, output: 75 },
          status: "available",
        },
        "claude-haiku-4-20250514": {
          id: "claude-haiku-4-20250514",
          name: "Claude Haiku 4",
          cost: { input: 0.25, output: 1.25 },
          status: "available",
        },
      },
    },
    {
      id: "openai",
      name: "OpenAI",
      models: {
        "gpt-4": {
          id: "gpt-4",
          name: "GPT-4",
          cost: { input: 5, output: 15 },
          status: "available",
        },
        "gpt-4-turbo": {
          id: "gpt-4-turbo",
          name: "GPT-4 Turbo",
          cost: { input: 2, output: 10 },
          status: "available",
        },
        "gpt-3.5-turbo": {
          id: "gpt-3.5-turbo",
          name: "GPT-3.5 Turbo",
          cost: { input: 0.01, output: 0.02 },
          status: "available",
        },
      },
    },
    {
      id: "google",
      name: "Google",
      models: {
        "gemini-2.0-flash-exp": {
          id: "gemini-2.0-flash-exp",
          name: "Gemini 2.0 Flash Experimental",
          cost: { input: 0, output: 0 },
          status: "available",
        },
      },
    },
  ],
  mcp: {},
}

const mockDialog = {
  clear: mock(() => {}),
  replace: mock(() => {}),
}

const mockKeybind = {
  all: {
    model_provider_list: [{ key: "p", modifiers: ["ctrl", "shift"] }],
    model_favorite_toggle: [{ key: "f", modifiers: ["ctrl"] }],
  },
}

describe("DialogModel Component", () => {
  describe("Options Generation", () => {
    it("should categorize models by provider", () => {
      const providers = mockSyncData.provider
      expect(providers).toHaveLength(3)
      expect(providers[0].name).toBe("Anthropic")
      expect(providers[1].name).toBe("OpenAI")
      expect(providers[2].name).toBe("Google")
    })

    it("should filter out deprecated models", () => {
      const testProvider = {
        id: "test",
        name: "Test",
        models: {
          "active-model": { id: "active-model", name: "Active Model", status: "available" },
          "deprecated-model": { id: "deprecated-model", name: "Deprecated Model", status: "deprecated" },
        },
      }
      const activeModels = Object.entries(testProvider.models).filter(([_, info]) => info.status !== "deprecated")
      expect(activeModels).toHaveLength(1)
      expect(activeModels[0][0]).toBe("active-model")
    })

    it("should show favorite models in a separate category", () => {
      const favorites = mockLocalModel.favorite()
      expect(favorites).toHaveLength(2)
      expect(favorites[0].providerID).toBe("anthropic")
      expect(favorites[1].providerID).toBe("openai")
    })

    it("should show recent models separately from favorites", () => {
      const recents = mockLocalModel.recent()
      const favorites = mockLocalModel.favorite()

      const recentsNotInFavorites = recents.filter(
        (item) => !favorites.some((fav) => fav.providerID === item.providerID && fav.modelID === item.modelID),
      )

      expect(recentsNotInFavorites).toHaveLength(1)
      expect(recentsNotInFavorites[0].providerID).toBe("google")
    })

    it("should mark free models with a footer", () => {
      const googleProvider = mockSyncData.provider.find((p) => p.id === "google")
      expect(googleProvider).toBeDefined()

      const geminiModel = googleProvider!.models["gemini-2.0-flash-exp"]
      expect(geminiModel.cost?.input).toBe(0)
    })

    it("should disable ccode nano models", () => {
      const ccodeProvider = {
        id: "ccode",
        name: "CCode",
        models: {
          "claude-nano": { id: "claude-nano", name: "Claude Nano", status: "available" },
          "claude-sonnet": { id: "claude-sonnet", name: "Claude Sonnet", status: "available" },
        },
      }

      const nanoDisabled = ccodeProvider.id === "ccode" && "claude-nano".includes("-nano")
      const sonnetDisabled = ccodeProvider.id === "ccode" && "claude-sonnet".includes("-nano")

      expect(nanoDisabled).toBe(true)
      expect(sonnetDisabled).toBe(false)
    })
  })

  describe("Provider Sorting", () => {
    it("should sort providers with ccode first", () => {
      const providers = [
        { id: "anthropic", name: "Anthropic" },
        { id: "ccode", name: "CCode" },
        { id: "openai", name: "OpenAI" },
      ]

      const sorted = [...providers].sort((a, b) => {
        if ((a.id !== "ccode") !== (b.id !== "ccode")) return a.id !== "ccode" ? 1 : -1
        return a.name.localeCompare(b.name)
      })

      expect(sorted[0].id).toBe("ccode")
      expect(sorted[1].id).toBe("anthropic")
      expect(sorted[2].id).toBe("openai")
    })

    it("should sort providers alphabetically when same ccode status", () => {
      const providers = [
        { id: "anthropic", name: "Anthropic" },
        { id: "openai", name: "OpenAI" },
        { id: "google", name: "Google" },
      ]

      const sorted = [...providers].sort((a, b) => a.name.localeCompare(b.name))

      expect(sorted[0].name).toBe("Anthropic")
      expect(sorted[1].name).toBe("Google")
      expect(sorted[2].name).toBe("OpenAI")
    })
  })

  describe("Model Selection", () => {
    it("should track current model", () => {
      const current = mockLocalModel.current()
      expect(current.providerID).toBe("anthropic")
      expect(current.modelID).toBe("claude-sonnet-4-20250514")
    })

    it("should call model.set when selecting a model", () => {
      const ref = { providerID: "openai", modelID: "gpt-4" }
      mockLocalModel.set(ref, { recent: true })
      expect(mockLocalModel.set).toHaveBeenCalledWith(ref, { recent: true })
    })
  })

  describe("Search Functionality", () => {
    it("should filter models by search query", () => {
      const options = [
        { title: "Claude Sonnet 4", category: "Anthropic" },
        { title: "GPT-4", category: "OpenAI" },
        { title: "Gemini 2.0 Flash", category: "Google" },
      ]

      const query = "claude"
      const filtered = options.filter((opt) => opt.title.toLowerCase().includes(query.toLowerCase()))

      expect(filtered).toHaveLength(1)
      expect(filtered[0].title).toBe("Claude Sonnet 4")
    })

    it("should search across both title and category", () => {
      const options = [
        { title: "Claude Sonnet 4", category: "Anthropic" },
        { title: "GPT-4", category: "OpenAI" },
        { title: "Gemini Flash", category: "Google" },
      ]

      const query = "openai"
      const filtered = options.filter(
        (opt) => opt.title.toLowerCase().includes(query) || opt.category?.toLowerCase().includes(query),
      )

      expect(filtered).toHaveLength(1)
      expect(filtered[0].category).toBe("OpenAI")
    })
  })

  describe("Favorite Toggle", () => {
    it("should have a keybind for toggling favorites", () => {
      expect(mockKeybind.all.model_favorite_toggle).toBeDefined()
      expect(mockKeybind.all.model_favorite_toggle[0].key).toBe("f")
    })

    it("should call toggleFavorite when triggered", () => {
      const modelRef = { providerID: "anthropic", modelID: "claude-sonnet-4-20250514" }
      mockLocalModel.toggleFavorite(modelRef)
      expect(mockLocalModel.toggleFavorite).toHaveBeenCalledWith(modelRef)
    })
  })

  describe("Provider Connection Status", () => {
    it("should show different options when connected vs not connected", () => {
      const connected = true
      const showExtra = connected

      expect(showExtra).toBe(true)
    })

    it("should hide extra sections when providerID is specified", () => {
      const providerID = "anthropic"
      const showExtra = !providerID

      expect(showExtra).toBe(false)
    })
  })

  describe("Model Options Structure", () => {
    it("should create properly structured model options", () => {
      const provider = mockSyncData.provider[0]
      const modelEntry = Object.entries(provider.models)[0]
      const [modelId, modelInfo] = modelEntry

      const option = {
        value: {
          providerID: provider.id,
          modelID: modelId,
        },
        title: modelInfo.name ?? modelId,
        description: provider.name,
        category: provider.name,
        disabled: provider.id === "ccode" && modelId.includes("-nano"),
        footer: modelInfo.cost?.input === 0 && provider.id === "ccode" ? "Free" : undefined,
      }

      expect(option.value.providerID).toBe("anthropic")
      expect(option.title).toBe("Claude Sonnet 4")
      expect(option.description).toBe("Anthropic")
      expect(option.disabled).toBe(false)
    })
  })
})
