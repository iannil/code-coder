/**
 * Provider Store
 *
 * Manages provider state including:
 * - All available providers
 * - Connected providers
 * - Default models per provider
 * - Model selection state
 */

import { create } from "zustand"
import { api } from "@/lib/api"
import type {
  ProviderInfo,
  ProviderModel,
  ProviderAuthMethod,
  ModelSelection,
} from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface ProviderState {
  // Data
  all: ProviderInfo[]
  connected: string[]
  defaults: Record<string, string>
  authMethods: Record<string, ProviderAuthMethod[]>

  // UI State
  selectedModel: ModelSelection | null
  favorites: ModelSelection[]
  recents: ModelSelection[]

  // Loading State
  isLoading: boolean
  error: string | null
}

interface ProviderActions {
  // Data Loading
  fetchProviders: () => Promise<void>
  fetchAuthMethods: () => Promise<void>

  // Model Selection
  selectModel: (selection: ModelSelection) => void
  clearSelection: () => void

  // Favorites & Recents
  toggleFavorite: (selection: ModelSelection) => void
  addRecent: (selection: ModelSelection) => void
  clearRecents: () => void

  // Helpers
  getProvider: (providerId: string) => ProviderInfo | undefined
  getModel: (providerId: string, modelId: string) => ProviderModel | undefined
  isConnected: (providerId: string) => boolean
  isFavorite: (selection: ModelSelection) => boolean
}

type ProviderStore = ProviderState & ProviderActions

// ============================================================================
// Local Storage Keys
// ============================================================================

const FAVORITES_KEY = "codecoder:model-favorites"
const RECENTS_KEY = "codecoder:model-recents"
const SELECTED_KEY = "codecoder:selected-model"

// ============================================================================
// Helpers
// ============================================================================

function loadFromStorage<T>(key: string, defaultValue: T): T {
  if (typeof window === "undefined") return defaultValue
  try {
    const stored = localStorage.getItem(key)
    return stored ? JSON.parse(stored) : defaultValue
  } catch {
    return defaultValue
  }
}

function saveToStorage<T>(key: string, value: T): void {
  if (typeof window === "undefined") return
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Ignore storage errors
  }
}

function isSameModel(a: ModelSelection, b: ModelSelection): boolean {
  return a.providerID === b.providerID && a.modelID === b.modelID
}

// ============================================================================
// Store
// ============================================================================

export const useProviderStore = create<ProviderStore>((set, get) => ({
  // Initial State
  all: [],
  connected: [],
  defaults: {},
  authMethods: {},
  selectedModel: loadFromStorage<ModelSelection | null>(SELECTED_KEY, null),
  favorites: loadFromStorage<ModelSelection[]>(FAVORITES_KEY, []),
  recents: loadFromStorage<ModelSelection[]>(RECENTS_KEY, []),
  isLoading: false,
  error: null,

  // Actions
  fetchProviders: async () => {
    set({ isLoading: true, error: null })
    try {
      const response = await api.listProviders()
      set({
        all: response.all,
        connected: response.connected,
        defaults: response.default,
        isLoading: false,
      })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch providers",
        isLoading: false,
      })
    }
  },

  fetchAuthMethods: async () => {
    try {
      const authMethods = await api.getProviderAuthMethods()
      set({ authMethods })
    } catch (error) {
      console.error("Failed to fetch auth methods:", error)
    }
  },

  selectModel: (selection) => {
    set({ selectedModel: selection })
    saveToStorage(SELECTED_KEY, selection)
    get().addRecent(selection)
  },

  clearSelection: () => {
    set({ selectedModel: null })
    saveToStorage(SELECTED_KEY, null)
  },

  toggleFavorite: (selection) => {
    const state = get()
    const isFav = state.favorites.some((f) => isSameModel(f, selection))
    const newFavorites = isFav
      ? state.favorites.filter((f) => !isSameModel(f, selection))
      : [...state.favorites, selection]

    set({ favorites: newFavorites })
    saveToStorage(FAVORITES_KEY, newFavorites)
  },

  addRecent: (selection) => {
    const state = get()
    // Remove if already in recents
    const filtered = state.recents.filter((r) => !isSameModel(r, selection))
    // Add to front, keep max 10
    const newRecents = [selection, ...filtered].slice(0, 10)

    set({ recents: newRecents })
    saveToStorage(RECENTS_KEY, newRecents)
  },

  clearRecents: () => {
    set({ recents: [] })
    saveToStorage(RECENTS_KEY, [])
  },

  getProvider: (providerId) => {
    return get().all.find((p) => p.id === providerId)
  },

  getModel: (providerId, modelId) => {
    const provider = get().getProvider(providerId)
    return provider?.models[modelId]
  },

  isConnected: (providerId) => {
    return get().connected.includes(providerId)
  },

  isFavorite: (selection) => {
    return get().favorites.some((f) => isSameModel(f, selection))
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useProviders = () => useProviderStore((state) => state.all)
export const useConnectedProviders = () => useProviderStore((state) => state.connected)
export const useSelectedModel = () => useProviderStore((state) => state.selectedModel)
export const useModelFavorites = () => useProviderStore((state) => state.favorites)
export const useModelRecents = () => useProviderStore((state) => state.recents)

export const useProviderLoading = () =>
  useProviderStore((state) => ({
    isLoading: state.isLoading,
    error: state.error,
  }))
